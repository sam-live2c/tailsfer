use std::io::{self, Cursor, Read};

use crate::transport::protocol::{Frame, FrameType, MAX_FILE_NAME_LEN, MAX_MIME_TYPE_LEN};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferOffer {
    pub transfer_id: [u8; 16],
    pub file_name: String,
    pub file_size: u64,
    pub mime_type: String,
}

impl TransferOffer {
    pub fn new(
        transfer_id: [u8; 16],
        file_name: String,
        file_size: u64,
        mime_type: String,
    ) -> Result<Self, &'static str> {
        validate_file_name(&file_name)?;
        validate_mime_type(&mime_type)?;

        Ok(Self {
            transfer_id,
            file_name,
            file_size,
            mime_type,
        })
    }

    pub fn encode_payload(&self) -> Result<Vec<u8>, &'static str> {
        validate_file_name(&self.file_name)?;
        validate_mime_type(&self.mime_type)?;

        let file_name = self.file_name.as_bytes();
        let mime_type = self.mime_type.as_bytes();

        let file_name_len = u16::try_from(file_name.len()).map_err(|_| "file name is too long")?;

        let mime_type_len = u16::try_from(mime_type.len()).map_err(|_| "MIME type is too long")?;

        let mut output = Vec::with_capacity(16 + 2 + file_name.len() + 8 + 2 + mime_type.len());

        output.extend_from_slice(&self.transfer_id);
        output.extend_from_slice(&file_name_len.to_be_bytes());
        output.extend_from_slice(file_name);
        output.extend_from_slice(&self.file_size.to_be_bytes());
        output.extend_from_slice(&mime_type_len.to_be_bytes());
        output.extend_from_slice(mime_type);

        Ok(output)
    }

    pub fn decode_payload(payload: &[u8]) -> Result<Self, &'static str> {
        let mut reader = Cursor::new(payload);

        let transfer_id = read_transfer_id(&mut reader)?;

        let file_name_len = read_u16(&mut reader)? as usize;

        if file_name_len > MAX_FILE_NAME_LEN {
            return Err("file name is too long");
        }

        let file_name_bytes = read_bytes(&mut reader, file_name_len)?;

        let file_name =
            String::from_utf8(file_name_bytes).map_err(|_| "file name is not valid UTF-8")?;

        let file_size = read_u64(&mut reader)?;

        let mime_type_len = read_u16(&mut reader)? as usize;

        if mime_type_len > MAX_MIME_TYPE_LEN {
            return Err("MIME type is too long");
        }

        let mime_type_bytes = read_bytes(&mut reader, mime_type_len)?;

        let mime_type =
            String::from_utf8(mime_type_bytes).map_err(|_| "MIME type is not valid UTF-8")?;

        if reader.position() as usize != payload.len() {
            return Err("unexpected bytes after transfer offer");
        }

        Self::new(transfer_id, file_name, file_size, mime_type)
    }

    pub fn to_frame(&self) -> Result<Frame, &'static str> {
        Frame::new(FrameType::TransferOffer, self.encode_payload()?)
    }

    pub fn from_frame(frame: &Frame) -> Result<Self, &'static str> {
        if frame.frame_type != FrameType::TransferOffer {
            return Err("frame is not a transfer offer");
        }

        Self::decode_payload(&frame.payload)
    }
}

fn validate_file_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("file name cannot be empty");
    }

    if name.len() > MAX_FILE_NAME_LEN {
        return Err("file name is too long");
    }

    if name.contains('\0') {
        return Err("file name contains NUL byte");
    }

    if name == "." || name == ".." {
        return Err("invalid file name");
    }

    if name.contains('/') || name.contains('\\') {
        return Err("file name must not contain path separators");
    }

    Ok(())
}

fn validate_mime_type(mime_type: &str) -> Result<(), &'static str> {
    if mime_type.is_empty() {
        return Err("MIME type cannot be empty");
    }

    if mime_type.len() > MAX_MIME_TYPE_LEN {
        return Err("MIME type is too long");
    }

    if mime_type.contains('\0') {
        return Err("MIME type contains NUL byte");
    }

    Ok(())
}

fn read_transfer_id(reader: &mut impl Read) -> Result<[u8; 16], &'static str> {
    let mut id = [0u8; 16];

    reader
        .read_exact(&mut id)
        .map_err(|_| "truncated transfer ID")?;

    Ok(id)
}

fn read_u16(reader: &mut impl Read) -> Result<u16, &'static str> {
    let mut buffer = [0u8; 2];

    reader
        .read_exact(&mut buffer)
        .map_err(|_| "truncated u16")?;

    Ok(u16::from_be_bytes(buffer))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, &'static str> {
    let mut buffer = [0u8; 8];

    reader
        .read_exact(&mut buffer)
        .map_err(|_| "truncated u64")?;

    Ok(u64::from_be_bytes(buffer))
}

fn read_bytes(reader: &mut impl Read, length: usize) -> Result<Vec<u8>, &'static str> {
    let mut buffer = vec![0u8; length];

    reader
        .read_exact(&mut buffer)
        .map_err(|_| "truncated field")?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_creation() {
        let transfer_id = [7u8; 16];

        let offer = TransferOffer::new(
            transfer_id,
            "hello.bin".to_string(),
            10 * 1024 * 1024,
            "application/octet-stream".to_string(),
        )
        .unwrap();

        assert_eq!(offer.transfer_id, transfer_id);
        assert_eq!(offer.file_name, "hello.bin");
        assert_eq!(offer.file_size, 10 * 1024 * 1024);
        assert_eq!(offer.mime_type, "application/octet-stream");
    }

    #[test]
    fn payload_roundtrip() {
        let original = TransferOffer::new(
            [42u8; 16],
            "hello.bin".to_string(),
            10_485_760,
            "application/octet-stream".to_string(),
        )
        .unwrap();

        let encoded = original.encode_payload().unwrap();
        let decoded = TransferOffer::decode_payload(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn frame_roundtrip() {
        let original = TransferOffer::new(
            [99u8; 16],
            "photo.jpg".to_string(),
            123_456,
            "image/jpeg".to_string(),
        )
        .unwrap();

        let frame = original.to_frame().unwrap();
        let decoded = TransferOffer::from_frame(&frame).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(frame.frame_type, FrameType::TransferOffer);
    }

    #[test]
    fn path_traversal_is_rejected() {
        assert!(
            TransferOffer::new(
                [1u8; 16],
                "../evil.txt".to_string(),
                100,
                "text/plain".to_string(),
            )
            .is_err()
        );

        assert!(
            TransferOffer::new(
                [1u8; 16],
                "folder/evil.txt".to_string(),
                100,
                "text/plain".to_string(),
            )
            .is_err()
        );

        assert!(
            TransferOffer::new(
                [1u8; 16],
                "folder\\evil.txt".to_string(),
                100,
                "text/plain".to_string(),
            )
            .is_err()
        );
    }

    #[test]
    fn empty_file_name_is_rejected() {
        assert!(
            TransferOffer::new([1u8; 16], String::new(), 100, "text/plain".to_string(),).is_err()
        );
    }

    #[test]
    fn malformed_payload_is_rejected() {
        assert!(TransferOffer::decode_payload(&[1, 2, 3]).is_err());
    }

    #[test]
    fn wrong_frame_type_is_rejected() {
        let frame = Frame::new(FrameType::Accept, vec![0u8; 16]).unwrap();

        assert!(TransferOffer::from_frame(&frame).is_err());
    }
}
