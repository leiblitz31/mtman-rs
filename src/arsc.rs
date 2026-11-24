//! Android resource table (resources.arsc) parser.
//!
//! Layout: RES_TABLE_TYPE header, then the global string pool, then packages.
//! We only need the string pool and package/type names for a resource browser,
//! so that is what this implements.

use anyhow::Result;

const RES_TABLE_TYPE: u16 = 0x0002;
const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_TABLE_PACKAGE_TYPE: u16 = 0x0200;
const RES_TABLE_TYPE_TYPE: u16 = 0x0201;

/// A resource type (e.g. "string", "drawable").
#[derive(Debug, Clone)]
pub struct ResType {
    pub id: u8,
    pub name: String,
    pub entry_count: usize,
}

/// A package in the table.
#[derive(Debug, Clone)]
pub struct Package {
    pub id: u32,
    pub name: String,
    pub types: Vec<ResType>,
}

/// Parsed resource table.
pub struct Arsc<'a> {
    pub data: &'a [u8],
    pub global_strings: Vec<String>,
    pub packages: Vec<Package>,
    /// Entries that could not be decoded (kept as empty strings). A real APK
    /// can contain a few hundred; a browser must not abort on them.
    pub bad_strings: usize,
}

impl<'a> Arsc<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 12 {
            anyhow::bail!("file too small");
        }
        let typ = u16::from_le_bytes(data[0..2].try_into()?);
        if typ != RES_TABLE_TYPE {
            anyhow::bail!("not an ARSC file (type {typ:#06x})");
        }
        let header_size = u16::from_le_bytes(data[2..4].try_into()?) as usize;
        let total_size = u32::from_le_bytes(data[4..8].try_into()?) as usize;

        let mut pos = header_size;
        let end = total_size.min(data.len());

        let mut global_strings = Vec::new();
        let mut packages = Vec::new();
        let mut bad_strings = 0usize;

        while pos + 8 <= end {
            let chunk_type = u16::from_le_bytes(data[pos..pos + 2].try_into()?);
            let chunk_header = u16::from_le_bytes(data[pos + 2..pos + 4].try_into()?) as usize;
            let chunk_size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into()?) as usize;
            if chunk_size < 8 || pos + chunk_size > end {
                break;
            }

            match chunk_type {
                RES_STRING_POOL_TYPE => {
                    // parse_string_pool reads the chunk header itself (counts live
                    // inside it), so hand it the whole chunk.
                    let _ = chunk_header;
                    let (pool, bad) = parse_string_pool_lenient(&data[pos..pos + chunk_size]);
                    global_strings = pool;
                    bad_strings += bad;
                }
                RES_TABLE_PACKAGE_TYPE => {
                    if let Some(pkg) = parse_package(&data[pos..pos + chunk_size], chunk_header)? {
                        packages.push(pkg);
                    }
                }
                _ => {}
            }

            pos += chunk_size;
        }

        Ok(Self {
            data,
            global_strings,
            packages,
            bad_strings,
        })
    }

    /// Look up a string by global pool index.
    pub fn string(&self, id: u32) -> Option<&str> {
        self.global_strings.get(id as usize).map(|s| s.as_str())
    }

    /// All package names.
    pub fn package_names(&self) -> Vec<String> {
        self.packages.iter().map(|p| p.name.clone()).collect()
    }
}

/// Lenient variant used for real APKs: undecodable entries become empty
/// strings instead of failing the whole table.
fn parse_string_pool_lenient(buf: &[u8]) -> (Vec<String>, usize) {
    let (count, is_utf8, offsets, strings_start) = match string_pool_layout(buf) {
        Some(v) => v,
        None => return (Vec::new(), 1),
    };

    let mut out = Vec::with_capacity(count);
    let mut bad = 0usize;
    for off in offsets {
        let p = strings_start as usize + off as usize;
        let s = if is_utf8 {
            read_utf8_string(buf, p)
        } else {
            read_utf16_string(buf, p)
        };
        match s {
            Ok(v) => out.push(v),
            Err(_) => {
                bad += 1;
                out.push(String::new());
            }
        }
    }
    (out, bad)
}

/// Shared header/offset-array decode for a string pool chunk.
fn string_pool_layout(buf: &[u8]) -> Option<(usize, bool, Vec<u32>, u32)> {
    if buf.len() < 28 {
        return None;
    }
    let string_count = u32::from_le_bytes(buf[8..12].try_into().ok()?) as usize;
    let flags = u32::from_le_bytes(buf[16..20].try_into().ok()?);
    let strings_start = u32::from_le_bytes(buf[20..24].try_into().ok()?);
    let is_utf8 = flags & (1 << 8) != 0;

    let mut offsets = Vec::with_capacity(string_count);
    for i in 0..string_count {
        let p = 28 + i * 4;
        if p + 4 > buf.len() {
            break;
        }
        offsets.push(u32::from_le_bytes(buf[p..p + 4].try_into().ok()?));
    }
    Some((offsets.len(), is_utf8, offsets, strings_start))
}

/// Parse a RES_STRING_POOL_TYPE chunk body.
fn parse_string_pool(buf: &[u8]) -> Result<Vec<String>> {
    if buf.len() < 28 {
        anyhow::bail!("string pool too small");
    }
    let string_count = u32::from_le_bytes(buf[8..12].try_into()?) as usize;
    let flags = u32::from_le_bytes(buf[16..20].try_into()?);
    let strings_start = u32::from_le_bytes(buf[20..24].try_into()?) as usize;
    let is_utf8 = flags & (1 << 8) != 0;

    let mut offsets = Vec::with_capacity(string_count);
    for i in 0..string_count {
        let p = 28 + i * 4;
        if p + 4 > buf.len() {
            break;
        }
        offsets.push(u32::from_le_bytes(buf[p..p + 4].try_into()?));
    }

    let mut out = Vec::with_capacity(offsets.len());
    for off in offsets {
        let p = strings_start as usize + off as usize;
        let s = if is_utf8 {
            read_utf8_string(buf, p)?
        } else {
            read_utf16_string(buf, p)?
        };
        out.push(s);
    }
    Ok(out)
}

/// UTF-8 pool entry: uleb128 utf16_len, uleb128 byte_len, then bytes.
fn read_utf8_string(buf: &[u8], p: usize) -> Result<String> {
    let mut pos = p;
    let (_utf16, np) = read_uleb128(buf, pos)?;
    pos = np;
    let (byte_len, np) = read_uleb128(buf, pos)?;
    pos = np;
    let end = pos + byte_len as usize;
    if end > buf.len() {
        anyhow::bail!("utf8 string overruns pool");
    }
    Ok(String::from_utf8_lossy(&buf[pos..end]).into_owned())
}

/// UTF-16 pool entry: u16 length (high bit = 2-word form), then UTF-16LE units.
fn read_utf16_string(buf: &[u8], p: usize) -> Result<String> {
    let mut pos = p;
    let mut len = u16::from_le_bytes(buf[pos..pos + 2].try_into()?) as usize;
    pos += 2;
    if len & 0x8000 != 0 {
        len = ((len & 0x7fff) << 16) | u16::from_le_bytes(buf[pos..pos + 2].try_into()?) as usize;
        pos += 2;
    }
    let end = pos + len * 2;
    if end > buf.len() {
        anyhow::bail!("utf16 string overruns pool");
    }
    let units: Vec<u16> = buf[pos..end]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok(String::from_utf16_lossy(&units))
}

fn read_uleb128(buf: &[u8], p: usize) -> Result<(usize, usize)> {
    let mut result = 0usize;
    let mut shift = 0;
    let mut pos = p;
    loop {
        let b = *buf.get(pos).ok_or_else(|| anyhow::anyhow!("uleb128 past end"))?;
        pos += 1;
        result |= ((b & 0x7f) as usize) << shift;
        if b & 0x80 == 0 {
            return Ok((result, pos));
        }
        shift += 7;
        if shift > 28 {
            anyhow::bail!("uleb128 too long");
        }
    }
}

/// Parse a RES_TABLE_PACKAGE_TYPE chunk.
fn parse_package(buf: &[u8], header_size: usize) -> Result<Option<Package>> {
    if buf.len() < header_size || header_size < 284 {
        anyhow::bail!("package chunk too small");
    }
    let id = u32::from_le_bytes(buf[8..12].try_into()?);

    // name: 128 u16 chars at offset 12, NUL padded
    let name_units: Vec<u16> = buf[12..12 + 256]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let name: String = name_units
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| char::from_u32(c as u32).unwrap_or('\u{fffd}'))
        .collect();

    let mut pos = header_size;
    let mut type_strings: Vec<String> = Vec::new();
    let mut key_strings: Vec<String> = Vec::new();
    let mut types: Vec<ResType> = Vec::new();
    let mut saw_type_pool = false;

    while pos + 8 <= buf.len() {
        let chunk_type = u16::from_le_bytes(buf[pos..pos + 2].try_into()?);
        let _chunk_header = u16::from_le_bytes(buf[pos + 2..pos + 4].try_into()?) as usize;
        let chunk_size = u32::from_le_bytes(buf[pos + 4..pos + 8].try_into()?) as usize;
        if chunk_size < 8 || pos + chunk_size > buf.len() {
            break;
        }

        match chunk_type {
            RES_STRING_POOL_TYPE => {
                let pool = parse_string_pool(&buf[pos..pos + chunk_size])?;
                if !saw_type_pool {
                    type_strings = pool;
                    saw_type_pool = true;
                } else {
                    key_strings = pool;
                }
            }
            RES_TABLE_TYPE_TYPE => {
                // Chunk layout: header(8) | id(1) flags(1) reserved(2) |
                // entryCount(4) entriesStart(4) | config...
                if chunk_size >= 20 {
                    let type_id = buf[pos + 8];
                    let entry_count =
                        u32::from_le_bytes(buf[pos + 12..pos + 16].try_into()?) as usize;
                    let type_name = type_strings
                        .get(type_id.saturating_sub(1) as usize)
                        .cloned()
                        .unwrap_or_else(|| format!("type_{type_id}"));
                    types.push(ResType {
                        id: type_id,
                        name: type_name,
                        entry_count,
                    });
                }
            }
            _ => {}
        }
        pos += chunk_size;
    }

    // key_strings parsed for completeness: entry names need key-index lookup,
    // which this browser does not surface yet.
    let _ = key_strings;

    Ok(Some(Package { id, name, types }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal ARSC: table header + one UTF-8 string pool holding "hello".
    /// Layout: table header (12) | pool chunk (28 hdr + 4 offset + 7 data = 39).
    fn fake_arsc() -> Vec<u8> {
        let mut d = vec![0u8; 12];
        d[0..2].copy_from_slice(&RES_TABLE_TYPE.to_le_bytes());
        d[2..4].copy_from_slice(&12u16.to_le_bytes()); // header size
        d[4..8].copy_from_slice(&51u32.to_le_bytes()); // total size = 12 + 39

        let mut pool = vec![0u8; 28];
        pool[0..2].copy_from_slice(&RES_STRING_POOL_TYPE.to_le_bytes());
        pool[2..4].copy_from_slice(&28u16.to_le_bytes()); // header size
        pool[4..8].copy_from_slice(&39u32.to_le_bytes()); // chunk size
        pool[8..12].copy_from_slice(&1u32.to_le_bytes()); // string count
        pool[16..20].copy_from_slice(&(1u32 << 8).to_le_bytes()); // UTF-8 flag
        pool[20..24].copy_from_slice(&32u32.to_le_bytes()); // strings start = 28+4

        pool.extend_from_slice(&0u32.to_le_bytes()); // offsets[0] -> 0
        pool.extend_from_slice(&[5, 5]); // utf16 len, byte len
        pool.extend_from_slice(b"hello");

        d.extend_from_slice(&pool);
        d
    }

    #[test]
    fn rejects_short_input() {
        assert!(Arsc::parse(&[0u8; 4]).is_err());
    }

    #[test]
    fn rejects_wrong_type() {
        assert!(Arsc::parse(&vec![0u8; 80]).is_err());
    }

    #[test]
    fn parses_global_strings() {
        let d = fake_arsc();
        let arsc = Arsc::parse(&d).unwrap();
        assert_eq!(arsc.global_strings.len(), 1);
        assert_eq!(arsc.string(0), Some("hello"));
    }
}