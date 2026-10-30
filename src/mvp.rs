use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedDevice {
    pub device_id: String,
    pub display_name: String,
    pub fingerprint: String,
    pub paired_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct TrustedDeviceStore {
    devices: HashMap<String, TrustedDevice>,
}

impl TrustedDeviceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, device: TrustedDevice) {
        self.devices.insert(device.device_id.clone(), device);
    }

    pub fn remove(&mut self, device_id: &str) {
        self.devices.remove(device_id);
    }

    pub fn get(&self, device_id: &str) -> Option<&TrustedDevice> {
        self.devices.get(device_id)
    }

    pub fn list(&self) -> Vec<&TrustedDevice> {
        self.devices.values().collect()
    }

    pub fn is_trusted(&self, device_id: &str) -> bool {
        self.devices.contains_key(device_id)
    }
}

pub fn default_trusted_store_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tailsfer").join("trusted_devices.json")
}

pub fn load_trusted_devices<P: AsRef<Path>>(path: P) -> Result<TrustedDeviceStore, Box<dyn std::error::Error>> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(TrustedDeviceStore::new());
    }

    let bytes = fs::read(path)?;
    let text = String::from_utf8(bytes)?;
    let devices: Vec<TrustedDevice> = serde_json::from_str(&text)?;

    let mut store = TrustedDeviceStore::new();
    for device in devices {
        store.add(device);
    }

    Ok(store)
}

pub fn save_trusted_devices<P: AsRef<Path>>(
    path: P,
    store: &TrustedDeviceStore,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let devices = store.list();
    let serialized = serde_json::to_string_pretty(&devices)?;
    fs::write(path, serialized)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_store_adds_and_lists_devices() {
        let mut store = TrustedDeviceStore::new();
        let device = TrustedDevice {
            device_id: "abc".to_string(),
            display_name: "Pixel 9".to_string(),
            fingerprint: "aa bb cc".to_string(),
            paired_at: "2026-01-01T00:00:00Z".to_string(),
        };

        store.add(device.clone());

        assert!(store.is_trusted("abc"));
        assert_eq!(store.get("abc").unwrap().display_name, "Pixel 9");
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn trusted_store_removes_device() {
        let mut store = TrustedDeviceStore::new();
        store.add(TrustedDevice {
            device_id: "x1".to_string(),
            display_name: "Laptop".to_string(),
            fingerprint: "ff".to_string(),
            paired_at: "2026-01-01T00:00:00Z".to_string(),
        });

        store.remove("x1");
        assert!(!store.is_trusted("x1"));
    }
}
