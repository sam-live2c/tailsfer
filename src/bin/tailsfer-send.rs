use std::env;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use quinn::{ClientConfig, Endpoint, RecvStream, SendStream};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::ring;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use tailsfer_core::transfer::decision::{TransferDecision, parse_verified_frame};
use tailsfer_core::transfer::offer::TransferOffer;
use tailsfer_core::transport::protocol::{ALPN, Frame, FrameType};

const CHUNK_SIZE: usize = 1024 * 1024;
const MAX_FRAME_SIZE: usize = 1024 * 1024 + 4096;

#[derive(Debug)]
struct DevVerifier;

impl ServerCertVerifier for DevVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
        ]
    }
}

async fn write_frame(
    send: &mut SendStream,
    frame: &Frame,
) -> Result<(), Box<dyn std::error::Error>> {
    let encoded = frame.encode()?;
    send.write_all(&encoded).await?;
    Ok(())
}

async fn read_frame(recv: &mut RecvStream) -> Result<Frame, Box<dyn std::error::Error>> {
    let mut header = [0u8; 5];

    recv.read_exact(&mut header).await?;

    let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    if payload_len > MAX_FRAME_SIZE {
        return Err("frame payload too large".into());
    }

    let mut payload = vec![0u8; payload_len];

    recv.read_exact(&mut payload).await?;

    let mut frame_bytes = Vec::with_capacity(5 + payload_len);
    frame_bytes.extend_from_slice(&header);
    frame_bytes.extend_from_slice(&payload);

    Ok(Frame::decode(&frame_bytes)?)
}

fn generate_transfer_id() -> [u8; 16] {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let mut id = [0u8; 16];

    id[..8].copy_from_slice(&(now as u64).to_le_bytes());
    id[8..].copy_from_slice(&((now >> 64) as u64).to_le_bytes());

    id
}

fn detect_mime(path: &Path) -> String {
    match path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "txt" => "text/plain",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn hash_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{:02x}", byte)).collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: tailsfer-send <server-ip:port> <file>");
        return Ok(());
    }

    let server: SocketAddr = args[1].parse()?;
    let path = Path::new(&args[2]);

    if !path.exists() {
        return Err(format!("file does not exist: {}", path.display()).into());
    }

    let metadata = tokio::fs::metadata(path).await?;

    if !metadata.is_file() {
        return Err(format!("not a regular file: {}", path.display()).into());
    }

    let file_name = path
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or("invalid file name")?
        .to_string();

    let transfer_id = generate_transfer_id();

    let offer = TransferOffer::new(
        transfer_id,
        file_name.clone(),
        metadata.len(),
        detect_mime(path),
    )?;

    let mut endpoint = Endpoint::client("[::]:0".parse()?)?;

    let mut crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(DevVerifier))
        .with_no_client_auth();

    crypto.alpn_protocols.push(ALPN.to_vec());

    let config = ClientConfig::new(Arc::new(quinn::crypto::rustls::QuicClientConfig::try_from(
        crypto,
    )?));

    endpoint.set_default_client_config(config);

    println!("Connecting to {server}...");

    let connection = endpoint.connect(server, "tailsfer.local")?.await?;

    println!("Connected to {server}");

    let (mut send, mut recv) = connection.open_bi().await?;

    println!();
    println!("Sending transfer offer:");
    println!("  File: {}", offer.file_name);
    println!("  Size: {} bytes", offer.file_size);
    println!("  Type: {}", offer.mime_type);
    println!();

    let offer_frame = offer.to_frame()?;
    write_frame(&mut send, &offer_frame).await?;

    println!("Waiting for receiver decision...");

    let decision = read_frame(&mut recv).await?;

    match decision.frame_type {
        FrameType::Accept => {
            let (accepted_id, _) = TransferDecision::from_frame(&decision)?;

            if accepted_id != transfer_id {
                return Err("receiver accepted a different transfer ID".into());
            }

            println!("Receiver ACCEPTED the transfer.");
        }

        FrameType::Reject => {
            let (rejected_id, _) = TransferDecision::from_frame(&decision)?;

            if rejected_id != transfer_id {
                return Err("receiver rejected a different transfer ID".into());
            }

            println!("Receiver REJECTED the transfer.");

            connection.close(0u32.into(), b"rejected");

            return Ok(());
        }

        _ => {
            return Err("unexpected frame received instead of decision".into());
        }
    }

    let mut file = File::open(path).await?;
    let mut buffer = vec![0u8; CHUNK_SIZE];

    let mut total = 0u64;
    let mut hasher = blake3::Hasher::new();

    let mut last_reported_mib = 0u64;

    loop {
        let n = file.read(&mut buffer).await?;

        if n == 0 {
            break;
        }

        send.write_all(&buffer[..n]).await?;

        hasher.update(&buffer[..n]);

        total += n as u64;

        let current_mib = total / 1_048_576;

        if current_mib >= last_reported_mib + 64 || total == offer.file_size {
            println!(
                "Sent: {:.2} / {:.2} MiB ({:.1}%)",
                total as f64 / 1_048_576.0,
                offer.file_size as f64 / 1_048_576.0,
                if offer.file_size == 0 {
                    100.0
                } else {
                    total as f64 / offer.file_size as f64 * 100.0
                }
            );

            last_reported_mib = current_mib;
        }
    }

    if total != offer.file_size {
        return Err(format!(
            "local file size changed during transfer: expected {}, sent {}",
            offer.file_size, total
        )
        .into());
    }

    let local_hash = *hasher.finalize().as_bytes();

    send.finish()?;

    println!();
    println!("Transfer data sent: {} bytes", total);
    println!("Sender BLAKE3: {}", hash_hex(&local_hash));
    println!("Waiting for receiver verification...");

    match read_frame(&mut recv).await {
        Ok(frame) => match frame.frame_type {
            FrameType::Verified => {
                let (verified_id, receiver_hash) = parse_verified_frame(&frame)?;

                if verified_id != transfer_id {
                    return Err("receiver verified a different transfer ID".into());
                }

                println!("Receiver VERIFIED the transfer.");
                println!("Receiver BLAKE3: {}", hash_hex(&receiver_hash));

                if receiver_hash != local_hash {
                    return Err(
                        "BLAKE3 verification failed: sender and receiver hashes differ".into(),
                    );
                }

                println!("BLAKE3 hashes MATCH.");
                println!("Transfer ID confirmed.");
            }

            FrameType::Reject => {
                let (rejected_id, _) = TransferDecision::from_frame(&frame)?;

                if rejected_id != transfer_id {
                    return Err("receiver rejected a different transfer ID".into());
                }

                return Err("receiver reported transfer verification failure".into());
            }

            other => {
                return Err(
                    format!("unexpected frame received after transfer: {:?}", other).into(),
                );
            }
        },

        Err(error) => {
            return Err(format!("receiver connection ended before verification: {error}").into());
        }
    }

    println!();
    println!("======================================");
    println!("       TRANSFER COMPLETE");
    println!("======================================");
    println!("File : {}", offer.file_name);
    println!("Size : {} bytes", total);
    println!("BLAKE3: {}", hash_hex(&local_hash));
    println!("Receiver: VERIFIED");
    println!("Hash: MATCH");
    println!("======================================");

    endpoint.wait_idle().await;

    Ok(())
}
