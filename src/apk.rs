//! APK/ZIP parser: central directory, local headers, stored + deflate entries.

use anyhow::{Context, Result};
use flate2::read::DeflateDecoder;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const EOCD_SIG: u32 = 0x0605_4b50;
const CD_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;

/// One central-directory entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    /// 0 = stored, 8 = deflate.
    pub method: u16,
    pub crc: u32,
    /// Offset of the local file header (needed to read the data).
    pub header_offset: u32,
    /// True for directories (name ends with '/').
    pub is_dir: bool,
}

/// Parsed APK.
pub struct Apk {
    pub path: PathBuf,
    pub entries: BTreeMap<String, Entry>,
}

impl Apk {
    /// Parse the central directory.
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = fs::File::open(path).context("open APK")?;
        let mut entries = BTreeMap::new();

        let len = file.seek(SeekFrom::End(0))?;
        if len < 22 {
            anyhow::bail!("file too small");
        }

        // EOCD is at the end, but a trailing comment may follow it: scan back.
        let max_back = 22u64 + 0xffff;
        let window = 22u64.min(len);
        let mut eocd_pos = None;
        let mut back = window;
        while back <= max_back && back <= len {
            file.seek(SeekFrom::End(-(back as i64)))?;
            let mut buf = [0u8; 22];
            file.read_exact(&mut buf)?;
            if u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) == EOCD_SIG {
                eocd_pos = Some(len - back);
                break;
            }
            back += 1;
        }
        let eocd_pos = eocd_pos.context("EOCD not found (not a zip?)")?;

        file.seek(SeekFrom::Start(eocd_pos))?;
        let mut buf = [0u8; 22];
        file.read_exact(&mut buf)?;
        let cd_entries_total = u16::from_le_bytes([buf[10], buf[11]]);
        let cd_size = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]) as u64;
        let cd_offset = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]) as u64;

        file.seek(SeekFrom::Start(cd_offset))?;
        let mut pos = cd_offset;
        let end = cd_offset + cd_size;
        let mut guard = 0usize;

        while pos < end {
            guard += 1;
            if guard > cd_entries_total as usize + 1024 {
                anyhow::bail!("central directory overruns");
            }

            let mut hdr = [0u8; 46];
            file.read_exact(&mut hdr)?;
            if u32::from_le_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) != CD_SIG {
                anyhow::bail!("bad central directory signature");
            }

            let name_len = u16::from_le_bytes([hdr[28], hdr[29]]) as usize;
            let extra_len = u16::from_le_bytes([hdr[30], hdr[31]]) as usize;
            let comment_len = u16::from_le_bytes([hdr[32], hdr[33]]) as usize;

            let mut name = vec![0u8; name_len];
            file.read_exact(&mut name)?;
            let name = String::from_utf8_lossy(&name).to_string();

            file.seek(SeekFrom::Current((extra_len + comment_len) as i64))?;

            let is_dir = name.ends_with('/');
            entries.insert(
                name.clone(),
                Entry {
                    name,
                    size: u32::from_le_bytes([hdr[24], hdr[25], hdr[26], hdr[27]]) as u64,
                    compressed_size: u32::from_le_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]) as u64,
                    method: u16::from_le_bytes([hdr[10], hdr[11]]),
                    crc: u32::from_le_bytes([hdr[16], hdr[17], hdr[18], hdr[19]]),
                    header_offset: u32::from_le_bytes([hdr[42], hdr[43], hdr[44], hdr[45]]),
                    is_dir,
                },
            );

            pos += 46 + name_len as u64 + extra_len as u64 + comment_len as u64;
        }

        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    /// Extract one entry to memory.
    ///
    /// shortcut: reads the whole entry into RAM. For a 40 MB asset that is a
    /// 40 MB allocation; switch to a streaming reader when editing >100 MB files.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self
            .entries
            .get(name)
            .with_context(|| format!("entry not found: {name}"))?;

        let mut f = fs::File::open(&self.path)?;
        f.seek(SeekFrom::Start(entry.header_offset as u64))?;

        let mut lh = [0u8; 30];
        f.read_exact(&mut lh)?;
        if u32::from_le_bytes([lh[0], lh[1], lh[2], lh[3]]) != LOCAL_SIG {
            anyhow::bail!("bad local header signature for {name}");
        }
        let name_len = u16::from_le_bytes([lh[26], lh[27]]) as usize;
        let extra_len = u16::from_le_bytes([lh[28], lh[29]]) as usize;
        let data_off = entry.header_offset as u64 + 30 + name_len as u64 + extra_len as u64;

        let mut raw = vec![0u8; entry.compressed_size as usize];
        f.seek(SeekFrom::Start(data_off))?;
        f.read_exact(&mut raw)?;

        match entry.method {
            0 => Ok(raw),
            8 => {
                let mut out = Vec::with_capacity(entry.size as usize);
                DeflateDecoder::new(&raw[..]).read_to_end(&mut out)?;
                Ok(out)
            }
            m => anyhow::bail!("unsupported compression method {m} for {name}"),
        }
    }

    /// Verify the CRC of an entry against the central directory.
    pub fn verify(&self, name: &str) -> Result<bool> {
        let data = self.read(name)?;
        Ok(crc32(&data) == self.entries[name].crc)
    }
}

/// Standard CRC-32 (IEEE 802.3), matching zip.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a zip with one stored entry "a.txt" -> "hi".
    fn fake_zip() -> Vec<u8> {
        let name = b"a.txt";
        let body = b"hi";
        let crc = crc32(body);

        let mut local = Vec::new();
        local.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes()); // stored
        local.extend_from_slice(&[0u8; 4]); // time, date
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(body.len() as u32).to_le_bytes());
        local.extend_from_slice(&(body.len() as u32).to_le_bytes());
        local.extend_from_slice(&(name.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name);
        local.extend_from_slice(body);

        let cd_off = local.len() as u32;
        let mut cd = Vec::new();
        cd.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        cd.extend_from_slice(&[0u8; 4]);
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&[0u8; 4]); // time, date
        cd.extend_from_slice(&crc.to_le_bytes());
        cd.extend_from_slice(&(body.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(body.len() as u32).to_le_bytes());
        cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&0u32.to_le_bytes());
        cd.extend_from_slice(&0u32.to_le_bytes()); // header offset = 0
        cd.extend_from_slice(name);

        let mut z = local;
        z.extend_from_slice(&cd);
        z.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z.extend_from_slice(&1u16.to_le_bytes());
        z.extend_from_slice(&1u16.to_le_bytes());
        z.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        z.extend_from_slice(&cd_off.to_le_bytes());
        z.extend_from_slice(&0u16.to_le_bytes());
        z
    }

    #[test]
    fn rejects_non_zip() {
        let p = std::env::temp_dir().join("mtman-notzip.bin");
        std::fs::write(&p, [0u8; 100]).unwrap();
        assert!(Apk::open(&p).is_err());
    }

    #[test]
    fn lists_entries() {
        let p = std::env::temp_dir().join("mtman-fake.zip");
        std::fs::write(&p, fake_zip()).unwrap();
        let apk = Apk::open(&p).unwrap();
        assert_eq!(apk.entries.len(), 1);
        let e = apk.entries.get("a.txt").expect("a.txt");
        assert_eq!(e.size, 2);
        assert_eq!(e.method, 0);
        assert!(!e.is_dir);
    }

    #[test]
    fn reads_stored_entry() {
        let p = std::env::temp_dir().join("mtman-fake2.zip");
        std::fs::write(&p, fake_zip()).unwrap();
        let apk = Apk::open(&p).unwrap();
        assert_eq!(apk.read("a.txt").unwrap(), b"hi");
    }

    #[test]
    fn crc32_known_vector() {
        // CRC-32("123456789") = 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn verifies_crc() {
        let p = std::env::temp_dir().join("mtman-fake3.zip");
        std::fs::write(&p, fake_zip()).unwrap();
        let apk = Apk::open(&p).unwrap();
        assert!(apk.verify("a.txt").unwrap());
    }
}