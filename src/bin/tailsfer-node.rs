use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use quinn::{Endpoint, RecvStream, ServerConfig};
use rustls::crypto::ring;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use tailsfer_core::identity::DeviceIdentity;
use tailsfer_core::identity::default_identity_path;
use tailsfer_core::transfer::decision::{TransferDecision, verified_frame};
use tailsfer_core::transfer::offer::TransferOffer;
use tailsfer_core::transport::protocol::{ALPN, Frame};

const TAILSFER_PORT: u16 = 47691;
const BUFFER_SIZE: usize = 1024 * 1024;
const MAX_FRAME_SIZE: usize = 1024 * 1024 + 4096;
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceivePolicy {
    Manual,
    Auto,
}

impl ReceivePolicy {
    fn from_env() -> Self {
        match std::env::var("TAILSFER_RECEIVE_POLICY")
            .unwrap_or_else(|_| "manual".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "auto" => Self::Auto,
            _ => Self::Manual,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Auto => "auto",
        }
    }
}

fn load_device_identity() -> Result<DeviceIdentity, Box<dyn std::error::Error>> {
    let path = default_identity_path();

    let identity = DeviceIdentity::load_or_create(&path)?;

    println!("======================================");
    println!("          TAILSFER DEVICE");
    println!("======================================");
    println!("Identity : {}", path.display());
    println!("Device ID: {}", identity.device_id_hex());
    println!("======================================");

    Ok(identity)
}

fn safe_filename(name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(name);

    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid filename")?;

    if filename.is_empty() {
        return Err("empty filename".into());
    }

    if filename == "." || filename == ".." {
        return Err("invalid filename".into());
    }

    if filename.contains('\0') {
        return Err("filename contains NUL byte".into());
    }

    Ok(filename.to_string())
}

fn transfer_id_hex(id: &[u8; 16]) -> String {
    id.iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn hash_hex(hash: &[u8; 32]) -> String {
    hash.iter().map(|byte| format!("{:02x}", byte)).collect()
}

async fn read_offer(
    recv: &mut RecvStream,
) -> Result<TransferOffer, Box<dyn std::error::Error + Send + Sync>> {
    let mut header = [0u8; 5];

    recv.read_exact(&mut header).await?;

    let payload_len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

    if payload_len > MAX_FRAME_SIZE {
        return Err("offer frame too large".into());
    }

    let mut payload = vec![0u8; payload_len];

    recv.read_exact(&mut payload).await?;

    let mut frame_bytes = Vec::with_capacity(5 + payload_len);

    frame_bytes.extend_from_slice(&header);
    frame_bytes.extend_from_slice(&payload);

    let frame = Frame::decode(&frame_bytes)?;

    TransferOffer::from_frame(&frame).map_err(|e| e.into())
}

async fn receive_file(
    mut recv: RecvStream,
    offer: TransferOffer,
) -> Result<(u64, [u8; 32]), Box<dyn std::error::Error + Send + Sync>> {
    let receiver_dir =
        PathBuf::from(std::env::var("TAILSFER_RECEIVER_DIR").unwrap_or_else(|_| ".".to_string()));

    tokio::fs::create_dir_all(&receiver_dir).await?;

    let filename = safe_filename(&offer.file_name)?;

    let output_path = receiver_dir.join(&filename);

    if tokio::fs::try_exists(&output_path).await? {
        return Err(format!("destination already exists: {}", output_path.display()).into());
    }

    let transfer_id = transfer_id_hex(&offer.transfer_id);

    let temp_name = format!(".{}.{}.tailsfer.part", filename, transfer_id);

    let temp_path = receiver_dir.join(temp_name);

    let mut file = File::create(&temp_path).await?;

    let result = timeout(TRANSFER_TIMEOUT, async {
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total = 0u64;
        let mut hasher = blake3::Hasher::new();

        loop {
            match recv.read(&mut buffer).await? {
                Some(n) if n > 0 => {
                    file.write_all(&buffer[..n]).await?;

                    hasher.update(&buffer[..n]);

                    total += n as u64;

                    if total > offer.file_size {
                        return Err(format!(
                            "received more data than expected: {} > {}",
                            total, offer.file_size
                        )
                        .into());
                    }
                }

                Some(_) => {
                    break;
                }

                None => {
                    break;
                }
            }
        }

        file.flush().await?;

        let hash = *hasher.finalize().as_bytes();

        Ok::<(u64, [u8; 32]), Box<dyn std::error::Error + Send + Sync>>((total, hash))
    })
    .await;

    let (total, hash) = match result {
        Ok(Ok(result)) => result,

        Ok(Err(error)) => {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(error);
        }

        Err(_) => {
            let _ = tokio::fs::remove_file(&temp_path).await;

            return Err(format!(
                "transfer timed out after {} seconds",
                TRANSFER_TIMEOUT.as_secs()
            )
            .into());
        }
    };

    if total != offer.file_size {
        let _ = tokio::fs::remove_file(&temp_path).await;

        return Err(format!(
            "transfer size mismatch: expected {}, received {}",
            offer.file_size, total
        )
        .into());
    }

    tokio::fs::rename(&temp_path, &output_path).await?;

    println!();
    println!("======================================");
    println!("       TRANSFER COMPLETE");
    println!("======================================");
    println!("File : {}", filename);
    println!("Size : {} bytes", total);
    println!("Saved: {}", output_path.display());
    println!("BLAKE3: {}", hash_hex(&hash));
    println!("======================================");

    Ok((total, hash))
}

async fn handle_connection(
    connection: quinn::Connection,
    receive_policy: ReceivePolicy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut send, mut recv) = connection.accept_bi().await?;

    let offer = read_offer(&mut recv).await?;

    let filename = safe_filename(&offer.file_name)?;

    println!();
    println!("======================================");
    println!("         INCOMING TAILSFER");
    println!("======================================");
    println!("File : {}", filename);
    println!("Size : {} bytes", offer.file_size);
    println!("Type : {}", offer.mime_type);
    println!("From : {}", connection.remote_address());
    println!("======================================");
    println!();

    let accepted = match receive_policy {
        ReceivePolicy::Auto => {
            println!("Receive policy: AUTO");
            println!("Automatically accepting transfer.");
            true
        }

        ReceivePolicy::Manual => {
            println!("Receive policy: MANUAL");
            println!("The file will NOT be saved unless you accept.");
            println!();

            print!("Accept [y/N]: ");
            io::stdout().flush()?;

            let mut answer = String::new();

            io::stdin().read_line(&mut answer)?;

            answer.trim().eq_ignore_ascii_case("y")
        }
    };

    let decision = if accepted {
        TransferDecision::Accept
    } else {
        TransferDecision::Reject
    };

    let decision_frame = decision.to_frame(offer.transfer_id)?;

    let encoded = decision_frame.encode()?;

    send.write_all(&encoded).await?;

    if !accepted {
        println!("Rejected.");

        send.finish()?;

        connection.close(0u32.into(), b"rejected");

        return Ok(());
    }

    println!();
    println!("Accepted. Receiving {}...", filename);

    let result = receive_file(recv, offer.clone()).await;

    match result {
        Ok((total, hash)) => {
            /*
             * VERIFIED now contains:
             *
             * transfer_id + receiver BLAKE3 hash
             *
             * The sender can compare this hash with its own
             * sender-side digest.
             */
            let verification = verified_frame(offer.transfer_id, hash)?;

            let encoded = verification.encode()?;

            send.write_all(&encoded).await?;

            send.finish()?;

            println!();
            println!("Verification sent to sender: {} bytes", total);
            println!("Receiver BLAKE3: {}", hash_hex(&hash));

            tokio::time::sleep(Duration::from_millis(100)).await;

            connection.close(0u32.into(), b"done");
        }

        Err(error) => {
            eprintln!("Transfer error: {}", error);

            let failure = TransferDecision::Reject.to_frame(offer.transfer_id)?;

            let encoded = failure.encode()?;

            let _ = send.write_all(&encoded).await;

            let _ = send.finish();

            connection.close(1u32.into(), b"transfer failed");

            return Err(error);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    println!("==============================");
    println!("        TAILSFER NODE");
    println!("==============================");

    let identity = load_device_identity()?;

    let receive_policy = ReceivePolicy::from_env();

    println!("Receive policy: {}", receive_policy.name());

    println!("Device ID: {}", identity.device_id_hex());

    println!();

    let cert = rcgen::generate_simple_self_signed(vec!["tailsfer.local".to_string()])?;

    let cert_der = cert.cert.der().clone();

    let key_der = cert.signing_key.serialize_der();

    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![quinn::rustls::pki_types::CertificateDer::from(cert_der)],
            quinn::rustls::pki_types::PrivateKeyDer::try_from(key_der)?,
        )?;

    tls_config.alpn_protocols = vec![ALPN.to_vec()];

    let quic_config = quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)?;

    let server_config = ServerConfig::with_crypto(Arc::new(quic_config));

    let addr: SocketAddr = format!("0.0.0.0:{}", TAILSFER_PORT).parse()?;

    let endpoint = Endpoint::server(server_config, addr)?;

    println!("Tailsfer QUIC listening on 0.0.0.0:{}", TAILSFER_PORT);

    println!("Local node ready for LAN transfers.");

    println!();

    while let Some(connecting) = endpoint.accept().await {
        let policy = receive_policy;

        tokio::spawn(async move {
            match connecting.await {
                Ok(connection) => {
                    println!("\nConnection from {}", connection.remote_address());

                    if let Err(error) = handle_connection(connection, policy).await {
                        eprintln!("Transfer error: {}", error);
                    }
                }

                Err(error) => {
                    eprintln!("QUIC connection error: {}", error);
                }
            }
        });
    }

    Ok(())
}
