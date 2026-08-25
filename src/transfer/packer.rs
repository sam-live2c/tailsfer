use std::io::{self, Read, Write};

use super::chunk::CHUNK_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zstd,
}

pub fn should_compress(data: &[u8]) -> bool {
    if data.len() < 4096 {
        return false;
    }

    let sample = &data[..data.len().min(64 * 1024)];

    let compressed = match zstd::stream::encode_all(sample, 1) {
        Ok(value) => value,
        Err(_) => return false,
    };

    compressed.len() + (compressed.len() / 20) < sample.len()
}

pub fn pack_chunk(data: &[u8]) -> io::Result<(Compression, Vec<u8>)> {
    if should_compress(data) {
        let compressed = zstd::stream::encode_all(data, 1)?;

        if compressed.len() < data.len() {
            return Ok((Compression::Zstd, compressed));
        }
    }

    Ok((Compression::None, data.to_vec()))
}

pub fn pack_reader<R: Read, W: Write>(mut input: R, mut output: W) -> io::Result<u64> {
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total = 0u64;

    loop {
        let read = input.read(&mut buffer)?;

        if read == 0 {
            break;
        }

        let (_, packed) = pack_chunk(&buffer[..read])?;

        output.write_all(&packed)?;

        total += read as u64;
    }

    Ok(total)
}
