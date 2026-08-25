use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

const LISTEN_ADDR: &str = "0.0.0.0:47690";
const DEVICE_ID_HEX_LEN: usize = 32;
const REGISTRATION_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
struct Registration {
    endpoint: SocketAddr,
    last_seen: Instant,
}

type Registry = std::sync::Arc<RwLock<HashMap<String, Registration>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry: Registry = std::sync::Arc::new(RwLock::new(HashMap::new()));

    let cleanup_registry = registry.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;

            let mut registry = cleanup_registry.write().await;

            registry.retain(|_, registration| registration.last_seen.elapsed() < REGISTRATION_TTL);
        }
    });

    let listener = TcpListener::bind(LISTEN_ADDR).await?;

    println!("==============================");
    println!("     TAILSFER RENDEZVOUS");
    println!("==============================");
    println!("Listening on {LISTEN_ADDR}");

    loop {
        let (stream, address) = listener.accept().await?;
        let registry = registry.clone();

        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, registry).await {
                eprintln!("Connection {address} error: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    registry: Registry,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes = reader.read_line(&mut line).await?;

        if bytes == 0 {
            break;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "REGISTER" if parts.len() == 3 => {
                let device_id = parts[1];
                let endpoint: SocketAddr = parts[2].parse()?;

                if !valid_device_id(device_id) {
                    writer.write_all(b"ERROR invalid-device-id\n").await?;
                    continue;
                }

                registry.write().await.insert(
                    device_id.to_string(),
                    Registration {
                        endpoint,
                        last_seen: Instant::now(),
                    },
                );

                writer.write_all(b"OK registered\n").await?;
            }

            "HEARTBEAT" if parts.len() == 2 => {
                let device_id = parts[1];

                let mut registry = registry.write().await;

                if let Some(registration) = registry.get_mut(device_id) {
                    registration.last_seen = Instant::now();
                    writer.write_all(b"OK heartbeat\n").await?;
                } else {
                    writer.write_all(b"ERROR not-registered\n").await?;
                }
            }

            "LOOKUP" if parts.len() == 2 => {
                let device_id = parts[1];

                let registry = registry.read().await;

                match registry.get(device_id) {
                    Some(registration) if registration.last_seen.elapsed() < REGISTRATION_TTL => {
                        writer
                            .write_all(format!("FOUND {}\n", registration.endpoint).as_bytes())
                            .await?;
                    }

                    _ => {
                        writer.write_all(b"NOT_FOUND\n").await?;
                    }
                }
            }

            "PING" => {
                writer.write_all(b"PONG\n").await?;
            }

            _ => {
                writer.write_all(b"ERROR invalid-command\n").await?;
            }
        }
    }

    Ok(())
}

fn valid_device_id(device_id: &str) -> bool {
    device_id.len() == DEVICE_ID_HEX_LEN && device_id.bytes().all(|byte| byte.is_ascii_hexdigit())
}
