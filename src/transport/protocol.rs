use std::io::{self, Cursor, Read};

pub const PROTOCOL_VERSION: u8 = 1;

pub const ALPN: &[u8] = b"tailsfer/1";

pub const MAX_FRAME_SIZE: usize = 1024 * 1024 + 4096;

pub const MAX_FILE_NAME_LEN: usize = 255;
pub const MAX_MIME_TYPE_LEN: usize = 127;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Hello = 1,
    Session = 2,
    TransferOffer = 3,
    Accept = 4,
    Reject = 5,
    Chunk = 6,
    Complete = 7,
    Cancel = 8,
    Verified = 9,
}

impl TryFrom<u8> for FrameType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Session),
            3 => Ok(Self::TransferOffer),
            4 => Ok(Self::Accept),
            5 => Ok(Self::Reject),
            6 => Ok(Self::Chunk),
            7 => Ok(Self::Complete),
            8 => Ok(Self::Cancel),
            9 => Ok(Self::Verified),
            _ => Err("unknown frame type"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Result<Self, &'static str> {
        if payload.len() > MAX_FRAME_SIZE {
            return Err("frame payload exceeds maximum size");
        }

        Ok(Self {
            frame_type,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, &'static str> {
        if self.payload.len() > MAX_FRAME_SIZE {
            return Err("frame payload exceeds maximum size");
        }

        let payload_len =
            u32::try_from(self.payload.len()).map_err(|_| "payload length does not fit in u32")?;

        let mut output = Vec::with_capacity(5 + self.payload.len());

        output.push(self.frame_type as u8);
        output.extend_from_slice(&payload_len.to_be_bytes());
        output.extend_from_slice(&self.payload);

        Ok(output)
    }

    pub fn decode(data: &[u8]) -> Result<Self, &'static str> {
        let mut reader = Cursor::new(data);

        let frame_type = read_u8(&mut reader).map_err(|_| "truncated frame type")?;

        let frame_type = FrameType::try_from(frame_type)?;

        let payload_len = read_u32(&mut reader).map_err(|_| "truncated frame length")? as usize;

        if payload_len > MAX_FRAME_SIZE {
            return Err("frame payload exceeds maximum size");
        }

        let expected_total = 5usize
            .checked_add(payload_len)
            .ok_or("frame size overflow")?;

        if data.len() != expected_total {
            return Err("frame length does not match payload");
        }

        let mut payload = vec![0u8; payload_len];

        reader
            .read_exact(&mut payload)
            .map_err(|_| "truncated frame payload")?;

        Ok(Self {
            frame_type,
            payload,
        })
    }
}

fn read_u8(reader: &mut impl Read) -> io::Result<u8> {
    let mut buffer = [0u8; 1];

    reader.read_exact(&mut buffer)?;

    Ok(buffer[0])
}

fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut buffer = [0u8; 4];

    reader.read_exact(&mut buffer)?;

    Ok(u32::from_be_bytes(buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_types_roundtrip() {
        for value in 1u8..=9 {
            let frame = FrameType::try_from(value).unwrap();

            assert_eq!(frame as u8, value);
        }
    }

    #[test]
    fn invalid_frame_type_is_rejected() {
        assert!(FrameType::try_from(0).is_err());
        assert!(FrameType::try_from(10).is_err());
    }

    #[test]
    fn frame_roundtrip() {
        let frame = Frame::new(FrameType::TransferOffer, b"test payload".to_vec()).unwrap();

        let encoded = frame.encode().unwrap();
        let decoded = Frame::decode(&encoded).unwrap();

        assert_eq!(decoded, frame);
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let payload = vec![0u8; MAX_FRAME_SIZE + 1];

        assert!(Frame::new(FrameType::Chunk, payload).is_err());
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let frame = Frame::new(FrameType::Accept, vec![1, 2, 3]).unwrap();

        let mut encoded = frame.encode().unwrap();

        encoded.pop();

        assert!(Frame::decode(&encoded).is_err());
    }

    #[test]
    fn mismatched_length_is_rejected() {
        let mut encoded = vec![FrameType::Accept as u8, 0, 0, 0, 10, 1, 2, 3];

        assert!(Frame::decode(&encoded).is_err());

        encoded[4] = 3;

        assert!(Frame::decode(&encoded).is_ok());
    }
}
