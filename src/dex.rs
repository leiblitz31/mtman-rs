//! DEX parser: header, string table, type/proto/field/method ids, class defs.

use anyhow::Result;

/// Parsed DEX file header (112 bytes).
#[derive(Debug, Clone)]
pub struct DexHeader {
    pub magic: [u8; 8],
    pub checksum: u32,
    pub signature: [u8; 20],
    pub file_size: u32,
    pub header_size: u32,
    pub endian_tag: u32,
    pub link_size: u32,
    pub link_off: u32,
    pub map_off: u32,
    pub string_ids_size: u32,
    pub string_ids_off: u32,
    pub type_ids_size: u32,
    pub type_ids_off: u32,
    pub proto_ids_size: u32,
    pub proto_ids_off: u32,
    pub field_ids_size: u32,
    pub field_ids_off: u32,
    pub method_ids_size: u32,
    pub method_ids_off: u32,
    pub class_defs_size: u32,
    pub class_defs_off: u32,
    pub data_size: u32,
    pub data_off: u32,
}

/// A field_id entry (8 bytes).
#[derive(Debug, Clone)]
pub struct FieldId {
    pub class_idx: u16,
    pub type_idx: u16,
    pub name_idx: u32,
}

/// A method_id entry (8 bytes).
#[derive(Debug, Clone)]
pub struct MethodId {
    pub class_idx: u16,
    pub proto_idx: u16,
    pub name_idx: u32,
}

/// A class_def entry (32 bytes).
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub class_idx: u32,
    pub access_flags: u32,
    pub superclass_idx: u32,
    pub interfaces_off: u32,
    pub source_file_idx: u32,
    pub annotations_off: u32,
    pub class_data_off: u32,
    pub static_values_off: u32,
}

/// Parsed DEX.
pub struct Dex<'a> {
    pub data: &'a [u8],
    pub header: DexHeader,
}

impl<'a> Dex<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < 112 {
            anyhow::bail!("file too small");
        }
        let magic = &data[0..8];
        if magic != b"dex\n035\0" && magic != b"dex\n037\0" && magic != b"dex\n038\0" {
            anyhow::bail!("bad magic: {:?}", magic);
        }

        let header = DexHeader {
            magic: magic.try_into()?,
            checksum: u32::from_le_bytes(data[8..12].try_into()?),
            signature: data[12..32].try_into()?,
            file_size: u32::from_le_bytes(data[32..36].try_into()?),
            header_size: u32::from_le_bytes(data[36..40].try_into()?),
            endian_tag: u32::from_le_bytes(data[40..44].try_into()?),
            link_size: u32::from_le_bytes(data[44..48].try_into()?),
            link_off: u32::from_le_bytes(data[48..52].try_into()?),
            map_off: u32::from_le_bytes(data[52..56].try_into()?),
            string_ids_size: u32::from_le_bytes(data[56..60].try_into()?),
            string_ids_off: u32::from_le_bytes(data[60..64].try_into()?),
            type_ids_size: u32::from_le_bytes(data[64..68].try_into()?),
            type_ids_off: u32::from_le_bytes(data[68..72].try_into()?),
            proto_ids_size: u32::from_le_bytes(data[72..76].try_into()?),
            proto_ids_off: u32::from_le_bytes(data[76..80].try_into()?),
            field_ids_size: u32::from_le_bytes(data[80..84].try_into()?),
            field_ids_off: u32::from_le_bytes(data[84..88].try_into()?),
            method_ids_size: u32::from_le_bytes(data[88..92].try_into()?),
            method_ids_off: u32::from_le_bytes(data[92..96].try_into()?),
            class_defs_size: u32::from_le_bytes(data[96..100].try_into()?),
            class_defs_off: u32::from_le_bytes(data[100..104].try_into()?),
            data_size: u32::from_le_bytes(data[104..108].try_into()?),
            data_off: u32::from_le_bytes(data[108..112].try_into()?),
        };

        Ok(Self { data, header })
    }

    fn u32_at(&self, off: usize) -> Result<u32> {
        let b = self
            .data
            .get(off..off + 4)
            .ok_or_else(|| anyhow::anyhow!("offset {off} out of range"))?;
        Ok(u32::from_le_bytes(b.try_into()?))
    }

    fn u16_at(&self, off: usize) -> Result<u16> {
        let b = self
            .data
            .get(off..off + 2)
            .ok_or_else(|| anyhow::anyhow!("offset {off} out of range"))?;
        Ok(u16::from_le_bytes(b.try_into()?))
    }

    /// Read the ULEB128 at `p` (returns value, new position).
    pub fn uleb128(&self, p: usize) -> Result<(u32, usize)> {
        let mut result = 0u32;
        let mut shift = 0;
        let mut pos = p;
        loop {
            let b = *self
                .data
                .get(pos)
                .ok_or_else(|| anyhow::anyhow!("uleb128 past end"))?;
            pos += 1;
            result |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 28 {
                anyhow::bail!("uleb128 too long");
            }
        }
        Ok((result, pos))
    }

    /// String table entry -> MUTF-8 string.
    pub fn string(&self, id: u32) -> Result<String> {
        let off = self.header.string_ids_off as usize + (id as usize) * 4;
        let str_off = self.u32_at(off)? as usize;

        // uleb128: utf16 length, then MUTF-8 bytes up to NUL
        let (_utf16_len, p) = self.uleb128(str_off)?;
        let rest = self
            .data
            .get(p..)
            .ok_or_else(|| anyhow::anyhow!("string {id} out of range"))?;
        let nul = rest
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("unterminated string {id}"))?;
        // MUTF-8 differs from UTF-8 only for NUL and supplementary chars;
        // lossy decode is correct for all BMP text and never panics.
        Ok(String::from_utf8_lossy(&rest[..nul]).into_owned())
    }

    /// type_ids entry -> descriptor string.
    pub fn type_string(&self, id: u32) -> Result<String> {
        let off = self.header.type_ids_off as usize + (id as usize) * 4;
        self.string(self.u32_at(off)?)
    }

    /// method_id -> (class descriptor, method name).
    pub fn method(&self, id: u32) -> Result<(String, String)> {
        let off = self.header.method_ids_off as usize + (id as usize) * 8;
        let class_idx = self.u16_at(off)?;
        let name_idx = self.u32_at(off + 4)?;
        Ok((self.type_string(class_idx as u32)?, self.string(name_idx)?))
    }

    /// field_id -> (class descriptor, field name).
    pub fn field(&self, id: u32) -> Result<(String, String)> {
        let off = self.header.field_ids_off as usize + (id as usize) * 8;
        let class_idx = self.u16_at(off)?;
        let name_idx = self.u32_at(off + 4)?;
        Ok((self.type_string(class_idx as u32)?, self.string(name_idx)?))
    }

    /// class_def entry.
    pub fn class_def(&self, id: u32) -> Result<ClassDef> {
        let off = self.header.class_defs_off as usize + (id as usize) * 32;
        Ok(ClassDef {
            class_idx: self.u32_at(off)?,
            access_flags: self.u32_at(off + 4)?,
            superclass_idx: self.u32_at(off + 8)?,
            interfaces_off: self.u32_at(off + 12)?,
            source_file_idx: self.u32_at(off + 16)?,
            annotations_off: self.u32_at(off + 20)?,
            class_data_off: self.u32_at(off + 24)?,
            static_values_off: self.u32_at(off + 28)?,
        })
    }

    /// class_def -> class descriptor.
    pub fn class_name(&self, id: u32) -> Result<String> {
        let def = self.class_def(id)?;
        self.type_string(def.class_idx)
    }

    /// All class descriptors in order.
    pub fn class_names(&self) -> Result<Vec<String>> {
        (0..self.header.class_defs_size)
            .map(|i| self.class_name(i))
            .collect()
    }

    /// Iterate class_def + class_data (fields/methods with access flags).
    /// Returns (class descriptor, class_data_off) pairs.
    pub fn classes_with_data(&self) -> Result<Vec<(String, u32)>> {
        (0..self.header.class_defs_size)
            .map(|i| {
                let def = self.class_def(i)?;
                Ok((self.type_string(def.class_idx)?, def.class_data_off))
            })
            .collect()
    }

    /// Parse class_data at `off` -> (static fields, instance fields, direct methods, virtual methods) as indices.
    pub fn class_data(&self, off: u32) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>)> {
        let mut p = off as usize;
        let (sf, p1) = self.uleb128(p)?;
        p = p1;
        let (inf, p1) = self.uleb128(p)?;
        p = p1;
        let (dm, p1) = self.uleb128(p)?;
        p = p1;
        let (vm, p1) = self.uleb128(p)?;
        p = p1;

        let read_idx = |p: &mut usize, prev: &mut u32| -> Result<u32> {
            let (v, np) = self.uleb128(*p)?;
            *p = np;
            *prev = prev.wrapping_add(v);
            Ok(*prev)
        };

        let read_fields = |p: &mut usize, count: u32| -> Result<Vec<u32>> {
            let mut idx = 0u32;
            let mut out = Vec::with_capacity(count as usize);
            for _ in 0..count {
                out.push(read_idx(p, &mut idx)?);
                let (_a, np) = self.uleb128(*p)?;
                *p = np;
            }
            Ok(out)
        };

        let read_methods = |p: &mut usize, count: u32| -> Result<Vec<u32>> {
            let mut idx = 0u32;
            let mut out = Vec::with_capacity(count as usize);
            for _ in 0..count {
                out.push(read_idx(p, &mut idx)?);
                let (_f, np) = self.uleb128(*p)?;
                *p = np;
                let (_c, np) = self.uleb128(*p)?;
                *p = np;
            }
            Ok(out)
        };

        Ok((
            read_fields(&mut p, sf)?,
            read_fields(&mut p, inf)?,
            read_methods(&mut p, dm)?,
            read_methods(&mut p, vm)?,
        ))
    }

    /// Search all strings for a needle.
    pub fn find_strings(&self, needle: &str) -> Result<Vec<(u32, String)>> {
        let mut hits = Vec::new();
        for i in 0..self.header.string_ids_size {
            let s = self.string(i)?;
            if s.contains(needle) {
                hits.push((i, s));
            }
        }
        Ok(hits)
    }

    /// Map-list entry: (type_idx, unused, size, offset).
    pub fn map_list(&self) -> Result<Vec<(u16, u32, u32)>> {
        let off = self.header.map_off as usize;
        let n = self.u32_at(off)? as usize;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let p = off + 4 + i * 12;
            out.push((
                self.u16_at(p)?,
                self.u16_at(p + 2)? as u32,
                self.u32_at(p + 4)?,
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal DEX laid out as:
    ///   0..112   header
    ///   112..116 string_ids[0] -> 116
    ///   116..123 ULEB128(5) + "hello"
    ///   124..156 class_def (class_idx -> type 0)
    ///   156..160 type_ids[0] -> string 0
    fn fake_dex() -> Vec<u8> {
        let mut d = vec![0u8; 160];
        d[0..8].copy_from_slice(b"dex\n035\0");

        // string_ids: size 1, off 112
        d[56..60].copy_from_slice(&1u32.to_le_bytes());
        d[60..64].copy_from_slice(&112u32.to_le_bytes());
        // type_ids: size 1, off 156
        d[64..68].copy_from_slice(&1u32.to_le_bytes());
        d[68..72].copy_from_slice(&156u32.to_le_bytes());
        // class_defs: size 1, off 124
        d[96..100].copy_from_slice(&1u32.to_le_bytes());
        d[100..104].copy_from_slice(&124u32.to_le_bytes());

        // string_ids[0] -> 116
        d[112..116].copy_from_slice(&116u32.to_le_bytes());
        // string data at 116: ULEB128(5) + "hello"
        d[116] = 5;
        d[117..122].copy_from_slice(b"hello");
        // class_def at 124: class_idx = 0
        d[124..128].copy_from_slice(&0u32.to_le_bytes());
        // type_ids[0] -> string 0
        d[156..160].copy_from_slice(&0u32.to_le_bytes());
        d
    }

    fn dex_bytes() -> Vec<u8> {
        fake_dex()
    }

    #[test]
    fn rejects_short_input() {
        assert!(Dex::parse(&[0u8; 10]).is_err());
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(Dex::parse(&vec![0u8; 112]).is_err());
    }

    #[test]
    fn parses_header() {
        let d = dex_bytes();
        let dex = Dex::parse(&d).unwrap();
        assert_eq!(dex.header.magic, *b"dex\n035\0");
        assert_eq!(dex.header.string_ids_size, 1);
    }

    #[test]
    fn reads_string_by_id() {
        let d = dex_bytes();
        let dex = Dex::parse(&d).unwrap();
        assert_eq!(dex.string(0).unwrap(), "hello");
    }

    #[test]
    fn reads_class_name() {
        let d = dex_bytes();
        let dex = Dex::parse(&d).unwrap();
        assert_eq!(dex.class_name(0).unwrap(), "hello");
    }

    #[test]
    fn uleb128_roundtrip() {
        let d = dex_bytes();
        let dex = Dex::parse(&d).unwrap();
        // ULEB128(5) at offset 116
        let (v, p) = dex.uleb128(116).unwrap();
        assert_eq!(v, 5);
        assert_eq!(p, 117);
    }

    #[test]
    fn uleb128_multibyte() {
        // 0xAC 0x02 = 300
        let mut d = fake_dex();
        d.extend_from_slice(&[0xAC, 0x02, 0]);
        let dex = Dex::parse(&d).unwrap();
        let (v, _) = dex.uleb128(160).unwrap();
        assert_eq!(v, 300);
    }
}