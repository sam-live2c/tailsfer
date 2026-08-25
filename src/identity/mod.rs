use rand::RngCore;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEVICE_ID_SIZE: usize = 16;
pub const DEVICE_KEY_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct DeviceIdentity {
    pub device_id: [u8; DEVICE_ID_SIZE],
    pub private_key: [u8; DEVICE_KEY_SIZE],
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        let mut device_id = [0u8; DEVICE_ID_SIZE];
        let mut private_key = [0u8; DEVICE_KEY_SIZE];

        rand::rng().fill_bytes(&mut device_id);
        rand::rng().fill_bytes(&mut private_key);

        Self {
            device_id,
            private_key,
        }
    }

    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.as_ref();

        if path.exists() {
            return Self::load(path);
        }

        let identity = Self::generate();
        identity.save(path)?;

        Ok(identity)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut data = Vec::with_capacity(DEVICE_ID_SIZE + DEVICE_KEY_SIZE);

        data.extend_from_slice(&self.device_id);
        data.extend_from_slice(&self.private_key);

        fs::write(path, data)?;

        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let data = fs::read(path)?;

        if data.len() != DEVICE_ID_SIZE + DEVICE_KEY_SIZE {
            return Err("invalid Tailsfer identity file".into());
        }

        let mut device_id = [0u8; DEVICE_ID_SIZE];
        let mut private_key = [0u8; DEVICE_KEY_SIZE];

        device_id.copy_from_slice(&data[..DEVICE_ID_SIZE]);

        private_key.copy_from_slice(&data[DEVICE_ID_SIZE..]);

        Ok(Self {
            device_id,
            private_key,
        })
    }

    pub fn device_id_hex(&self) -> String {
        self.device_id
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

pub fn default_identity_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home).join(".tailsfer").join("identity")
}
