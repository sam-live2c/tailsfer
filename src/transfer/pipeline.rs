use std::io::{self, Read, Write};

use zstd::stream::{decode_all, encode_all};

use crate::crypto::encryption::{KEY_SIZE, decrypt, encrypt};

pub const MAX_CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zstd,
}

pub fn pack_chunk(
    key: &[u8; KEY_SIZE],
    plaintext: &[u8],
) -> Result<(Vec<u8>, Compression, [u8; 12]), String> {
    let compressed = encode_all(plaintext, 1).map_err(|e| format!("compression failed: {e}"))?;

    let (payload, compression) = if compressed.len() < plaintext.len() {
        (compressed, Compression::Zstd)
    } else {
        (plaintext.to_vec(), Compression::None)
    };

    let (encrypted, nonce) = encrypt(key, &payload)?;

    Ok((encrypted, compression, nonce))
}

pub fn unpack_chunk(
    key: &[u8; KEY_SIZE],
    ciphertext: &[u8],
    nonce: &[u8; 12],
    compression: Compression,
) -> Result<Vec<u8>, String> {
    let decrypted = decrypt(key, nonce, ciphertext)?;

    match compression {
        Compression::None => Ok(decrypted),
        Compression::Zstd => {
            decode_all(decrypted.as_slice()).map_err(|e| format!("decompression failed: {e}"))
        }
    }
}

/// Streams a file without loading the complete file into memory.
pub fn process_reader<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    key: &[u8; KEY_SIZE],
) -> Result<u64, String> {
    let mut buffer = vec![0u8; MAX_CHUNK_SIZE];
    let mut total = 0u64;

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("read failed: {e}"))?;

        if read == 0 {
            break;
        }

        let (encrypted, compression, nonce) = pack_chunk(key, &buffer[..read])?;

        // Temporary local framing.
        writer
            .write_all(&(encrypted.len() as u32).to_be_bytes())
            .map_err(|e| format!("write failed: {e}"))?;

        writer
            .write_all(&[compression as u8])
            .map_err(|e| format!("write failed: {e}"))?;

        writer
            .write_all(&nonce)
            .map_err(|e| format!("write failed: {e}"))?;

        writer
            .write_all(&encrypted)
            .map_err(|e| format!("write failed: {e}"))?;

        total += read as u64;
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_roundtrip() {
        let key = crate::crypto::encryption::generate_key();

        let original = b"Tailsfer secure transfer test data ".repeat(1000);

        let (encrypted, compression, nonce) = pack_chunk(&key, &original).unwrap();

        let recovered = unpack_chunk(&key, &encrypted, &nonce, compression).unwrap();

        assert_eq!(original, recovered);
    }
}
