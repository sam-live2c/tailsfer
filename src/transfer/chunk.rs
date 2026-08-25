pub const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB

#[derive(Debug, Clone, Copy)]
pub struct ChunkInfo {
    pub index: u64,
    pub plaintext_len: u32,
}

impl ChunkInfo {
    pub fn new(index: u64, plaintext_len: usize) -> Self {
        Self {
            index,
            plaintext_len: plaintext_len as u32,
        }
    }
}
