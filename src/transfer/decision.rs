use crate::transport::protocol::{Frame, FrameType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDecision {
    Accept,
    Reject,
}

impl TransferDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::Reject => "REJECT",
        }
    }

    pub fn frame_type(self) -> FrameType {
        match self {
            Self::Accept => FrameType::Accept,
            Self::Reject => FrameType::Reject,
        }
    }

    pub fn to_frame(self, transfer_id: [u8; 16]) -> Result<Frame, &'static str> {
        Frame::new(self.frame_type(), transfer_id.to_vec())
    }

    pub fn from_frame(frame: &Frame) -> Result<([u8; 16], Self), &'static str> {
        let decision = match frame.frame_type {
            FrameType::Accept => Self::Accept,
            FrameType::Reject => Self::Reject,
            _ => return Err("frame is not an accept/reject decision"),
        };

        if frame.payload.len() != 16 {
            return Err("invalid transfer decision payload length");
        }

        let mut transfer_id = [0u8; 16];
        transfer_id.copy_from_slice(&frame.payload);

        Ok((transfer_id, decision))
    }
}

/*
 * Complete frame payload:
 *
 * [ transfer_id: 16 bytes ]
 * [ blake3_hash: 32 bytes ]
 *
 * Total: 48 bytes
 */
pub fn complete_frame(transfer_id: [u8; 16], hash: [u8; 32]) -> Result<Frame, &'static str> {
    let mut payload = Vec::with_capacity(48);

    payload.extend_from_slice(&transfer_id);
    payload.extend_from_slice(&hash);

    Frame::new(FrameType::Complete, payload)
}

pub fn parse_complete_frame(frame: &Frame) -> Result<([u8; 16], [u8; 32]), &'static str> {
    if frame.frame_type != FrameType::Complete {
        return Err("frame is not a complete frame");
    }

    if frame.payload.len() != 48 {
        return Err("invalid complete frame payload length");
    }

    let mut transfer_id = [0u8; 16];
    let mut hash = [0u8; 32];

    transfer_id.copy_from_slice(&frame.payload[..16]);
    hash.copy_from_slice(&frame.payload[16..48]);

    Ok((transfer_id, hash))
}

/*
 * Verified frame payload:
 *
 * [ transfer_id: 16 bytes ]
 */
pub fn verified_frame(transfer_id: [u8; 16]) -> Result<Frame, &'static str> {
    Frame::new(FrameType::Verified, transfer_id.to_vec())
}

pub fn parse_verified_frame(frame: &Frame) -> Result<[u8; 16], &'static str> {
    if frame.frame_type != FrameType::Verified {
        return Err("frame is not a verified frame");
    }

    if frame.payload.len() != 16 {
        return Err("invalid verified frame payload length");
    }

    let mut transfer_id = [0u8; 16];
    transfer_id.copy_from_slice(&frame.payload);

    Ok(transfer_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Offered,
    Accepted,
    Rejected,
    Receiving,
    Completed,
    Cancelled,
}

impl TransferState {
    pub fn can_receive_chunks(self) -> bool {
        matches!(self, Self::Accepted | Self::Receiving)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferSession {
    state: TransferState,
}

impl TransferSession {
    pub fn new() -> Self {
        Self {
            state: TransferState::Offered,
        }
    }

    pub fn state(&self) -> TransferState {
        self.state
    }

    pub fn accept(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransferState::Offered => {
                self.state = TransferState::Accepted;
                Ok(())
            }
            _ => Err("transfer cannot be accepted in current state"),
        }
    }

    pub fn reject(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransferState::Offered => {
                self.state = TransferState::Rejected;
                Ok(())
            }
            _ => Err("transfer cannot be rejected in current state"),
        }
    }

    pub fn begin_receiving(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransferState::Accepted => {
                self.state = TransferState::Receiving;
                Ok(())
            }
            _ => Err("transfer has not been accepted"),
        }
    }

    pub fn complete(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransferState::Receiving => {
                self.state = TransferState::Completed;
                Ok(())
            }
            _ => Err("transfer cannot be completed"),
        }
    }

    pub fn cancel(&mut self) -> Result<(), &'static str> {
        match self.state {
            TransferState::Offered | TransferState::Accepted | TransferState::Receiving => {
                self.state = TransferState::Cancelled;
                Ok(())
            }
            _ => Err("transfer cannot be cancelled"),
        }
    }
}

impl Default for TransferSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decisions_have_expected_names() {
        assert_eq!(TransferDecision::Accept.as_str(), "ACCEPT");
        assert_eq!(TransferDecision::Reject.as_str(), "REJECT");
    }

    #[test]
    fn accept_frame_roundtrip() {
        let transfer_id = [7u8; 16];

        let frame = TransferDecision::Accept.to_frame(transfer_id).unwrap();

        let (decoded_id, decoded_decision) = TransferDecision::from_frame(&frame).unwrap();

        assert_eq!(decoded_id, transfer_id);
        assert_eq!(decoded_decision, TransferDecision::Accept);
        assert_eq!(frame.frame_type, FrameType::Accept);
    }

    #[test]
    fn reject_frame_roundtrip() {
        let transfer_id = [9u8; 16];

        let frame = TransferDecision::Reject.to_frame(transfer_id).unwrap();

        let (decoded_id, decoded_decision) = TransferDecision::from_frame(&frame).unwrap();

        assert_eq!(decoded_id, transfer_id);
        assert_eq!(decoded_decision, TransferDecision::Reject);
    }

    #[test]
    fn complete_frame_roundtrip() {
        let transfer_id = [7u8; 16];
        let hash = [42u8; 32];

        let frame = complete_frame(transfer_id, hash).unwrap();

        let (decoded_id, decoded_hash) = parse_complete_frame(&frame).unwrap();

        assert_eq!(decoded_id, transfer_id);
        assert_eq!(decoded_hash, hash);
    }

    #[test]
    fn verified_frame_roundtrip() {
        let transfer_id = [8u8; 16];

        let frame = verified_frame(transfer_id).unwrap();

        let decoded_id = parse_verified_frame(&frame).unwrap();

        assert_eq!(decoded_id, transfer_id);
    }

    #[test]
    fn invalid_complete_length_is_rejected() {
        let frame = Frame::new(FrameType::Complete, vec![0u8; 47]).unwrap();

        assert!(parse_complete_frame(&frame).is_err());
    }

    #[test]
    fn invalid_verified_length_is_rejected() {
        let frame = Frame::new(FrameType::Verified, vec![0u8; 15]).unwrap();

        assert!(parse_verified_frame(&frame).is_err());
    }

    #[test]
    fn new_transfer_starts_as_offered() {
        let session = TransferSession::new();

        assert_eq!(session.state(), TransferState::Offered);
        assert!(!session.state().can_receive_chunks());
    }

    #[test]
    fn accepted_transfer_can_receive() {
        let mut session = TransferSession::new();

        session.accept().unwrap();

        assert_eq!(session.state(), TransferState::Accepted);
        assert!(session.state().can_receive_chunks());

        session.begin_receiving().unwrap();

        assert_eq!(session.state(), TransferState::Receiving);
    }
}
