use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, KeyInit},
};
use rand::RngCore;

pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

pub fn generate_key() -> [u8; KEY_SIZE] {
    let mut key = [0u8; KEY_SIZE];
    rand::rng().fill_bytes(&mut key);
    key
}

pub fn encrypt(
    key: &[u8; KEY_SIZE],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; NONCE_SIZE]), String> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| "invalid encryption key".to_string())?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);

    let ciphertext = cipher
        .encrypt((&nonce_bytes).into(), plaintext)
        .map_err(|_| "encryption failed".to_string())?;

    Ok((ciphertext, nonce_bytes))
}

pub fn decrypt(
    key: &[u8; KEY_SIZE],
    nonce_bytes: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|_| "invalid encryption key".to_string())?;

    cipher
        .decrypt(nonce_bytes.into(), ciphertext)
        .map_err(|_| "authentication failed".to_string())
}

pub fn parse_hex_key(value: &str) -> Result<[u8; KEY_SIZE], String> {
    let value = value.trim();

    if value.len() != KEY_SIZE * 2 {
        return Err(format!(
            "key must contain exactly {} hexadecimal characters",
            KEY_SIZE * 2
        ));
    }

    let mut key = [0u8; KEY_SIZE];

    for i in 0..KEY_SIZE {
        let start = i * 2;
        let end = start + 2;

        key[i] = u8::from_str_radix(&value[start..end], 16)
            .map_err(|_| "key contains invalid hexadecimal characters".to_string())?;
    }

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_roundtrip() {
        let key = generate_key();

        let plaintext = b"Tailsfer encrypted transfer test";

        let (ciphertext, nonce) = encrypt(&key, plaintext).unwrap();

        assert_ne!(ciphertext, plaintext);

        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let key = generate_key();

        let plaintext = b"Tailsfer authentication test";

        let (mut ciphertext, nonce) = encrypt(&key, plaintext).unwrap();

        ciphertext[0] ^= 0x01;

        assert!(decrypt(&key, &nonce, &ciphertext).is_err());
    }

    #[test]
    fn invalid_key_is_rejected() {
        assert!(parse_hex_key("abcd").is_err());
    }

    #[test]
    fn hex_key_roundtrip() {
        let key = parse_hex_key(
            "000102030405060708090a0b0c0d0e0f\
             101112131415161718191a1b1c1d1e1f",
        )
        .unwrap();

        assert_eq!(key[0], 0);
        assert_eq!(key[31], 31);
    }
}
