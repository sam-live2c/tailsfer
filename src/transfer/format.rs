use std::io::{self, Read, Write};

pub const MAGIC: &[u8; 8] = b"TLSFR001";
pub const HEADER_SIZE: usize = 8 + 4 + 8 + 8;

#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    pub version: u32,
    pub original_size: u64,
    pub chunk_size: u64,
}

impl FileHeader {
    pub fn new(original_size: u64, chunk_size: u64) -> Self {
        Self {
            version: 1,
            original_size,
            chunk_size,
        }
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(MAGIC)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.original_size.to_le_bytes())?;
        writer.write_all(&self.chunk_size.to_le_bytes())?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;

        if &magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Tailsfer file",
            ));
        }

        let mut version = [0u8; 4];
        let mut original_size = [0u8; 8];
        let mut chunk_size = [0u8; 8];

        reader.read_exact(&mut version)?;
        reader.read_exact(&mut original_size)?;
        reader.read_exact(&mut chunk_size)?;

        Ok(Self {
            version: u32::from_le_bytes(version),
            original_size: u64::from_le_bytes(original_size),
            chunk_size: u64::from_le_bytes(chunk_size),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let header = FileHeader::new(100_000_000_000, 1024 * 1024);

        let mut data = Vec::new();
        header.write_to(&mut data).unwrap();

        let decoded = FileHeader::read_from(&data[..]).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.original_size, 100_000_000_000);
        assert_eq!(decoded.chunk_size, 1024 * 1024);
    }
}
