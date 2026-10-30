use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub display_name: String,
    pub fingerprint: String,
    pub trusted: bool,
}

impl DeviceInfo {
    pub fn new(device_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let device_id = device_id.into();
        let display_name = display_name.into();
        let fingerprint = format_fingerprint(device_id.as_bytes());

        Self {
            device_id,
            display_name,
            fingerprint,
            trusted: false,
        }
    }

    pub fn trusted(device_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let mut info = Self::new(device_id, display_name);
        info.trusted = true;
        info
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
}

impl PairingCode {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 4];
        rand::rng().fill_bytes(&mut bytes);
        let code = bytes
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<Vec<_>>()
            .join("");

        Self { code }
    }

    pub fn from_value(value: impl Into<String>) -> Self {
        Self {
            code: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    Queued,
    WaitingForApproval,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferSession {
    pub session_id: String,
    pub peer_id: String,
    pub direction: TransferDirection,
    pub file_name: String,
    pub file_size: u64,
    pub bytes_transferred: u64,
    pub state: TransferState,
}

impl TransferSession {
    pub fn new(
        peer_id: impl Into<String>,
        direction: TransferDirection,
        file_name: impl Into<String>,
        file_size: u64,
    ) -> Self {
        let session_id = generate_session_id();
        Self {
            session_id,
            peer_id: peer_id.into(),
            direction,
            file_name: file_name.into(),
            file_size,
            bytes_transferred: 0,
            state: TransferState::Queued,
        }
    }

    pub fn progress_percentage(&self) -> u8 {
        if self.file_size == 0 {
            return 100;
        }

        let percentage = ((self.bytes_transferred as f64 / self.file_size as f64) * 100.0).round();
        percentage.min(100.0) as u8
    }

    pub fn mark_in_progress(&mut self) {
        self.state = TransferState::InProgress;
    }

    pub fn mark_completed(&mut self) {
        self.state = TransferState::Completed;
        self.bytes_transferred = self.file_size;
    }

    pub fn mark_failed(&mut self) {
        self.state = TransferState::Failed;
    }

    pub fn approve(&mut self) {
        self.state = TransferState::WaitingForApproval;
    }

    pub fn reject(&mut self) {
        self.state = TransferState::Rejected;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRequest {
    pub session_id: String,
    pub peer_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
    pub require_approval: bool,
}

impl TransferRequest {
    pub fn new(
        peer_id: impl Into<String>,
        file_name: impl Into<String>,
        file_size: u64,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            session_id: generate_session_id(),
            peer_id: peer_id.into(),
            file_name: file_name.into(),
            file_size,
            mime_type: mime_type.into(),
            require_approval: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferProgress {
    pub session_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub percentage: u8,
}

impl TransferProgress {
    pub fn from_session(session: &TransferSession) -> Self {
        Self {
            session_id: session.session_id.clone(),
            bytes_transferred: session.bytes_transferred,
            total_bytes: session.file_size,
            percentage: session.progress_percentage(),
        }
    }
}

pub fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<_>>()
        .join("")
}

fn format_fingerprint(input: &[u8]) -> String {
    let mut out = String::new();
    for (idx, byte) in input.iter().enumerate() {
        if idx > 0 && idx % 4 == 0 {
            out.push(' ');
        }
        out.push_str(&format!("{:02x}", byte));
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_code_is_generated() {
        let code = PairingCode::generate();
        assert_eq!(code.code.len(), 8);
        assert!(code.code.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn transfer_session_tracks_progress() {
        let mut session = TransferSession::new("peer-a", TransferDirection::Send, "photo.jpg", 1_000);
        session.mark_in_progress();
        session.bytes_transferred = 500;

        assert_eq!(session.progress_percentage(), 50);
        assert_eq!(session.state, TransferState::InProgress);
    }

    #[test]
    fn transfer_session_can_approve_and_complete() {
        let mut session = TransferSession::new("peer-b", TransferDirection::Receive, "notes.txt", 200);
        session.approve();
        session.mark_in_progress();
        session.mark_completed();

        assert_eq!(session.state, TransferState::Completed);
        assert_eq!(session.bytes_transferred, 200);
    }

    #[test]
    fn transfer_request_is_created() {
        let request = TransferRequest::new("peer-1", "hello.txt", 42, "text/plain");
        assert_eq!(request.file_name, "hello.txt");
        assert!(request.require_approval);
    }

    #[test]
    fn trusted_device_marks_pairing_state() {
        let device = DeviceInfo::trusted("device-123", "Pixel 9");
        assert!(device.trusted);
        assert!(!device.fingerprint.is_empty());
    }
}
