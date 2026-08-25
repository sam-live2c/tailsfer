use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Debug)]
pub enum RendezvousError {
    Io(io::Error),
    Protocol(String),
    InvalidEndpoint(String),
}

impl std::fmt::Display for RendezvousError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::InvalidEndpoint(error) => write!(f, "invalid endpoint: {error}"),
        }
    }
}

impl std::error::Error for RendezvousError {}

impl From<io::Error> for RendezvousError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct RendezvousClient {
    server: SocketAddr,
}

impl RendezvousClient {
    pub fn new(server: SocketAddr) -> Self {
        Self { server }
    }

    pub async fn register(
        &self,
        device_id: &str,
        endpoint: SocketAddr,
    ) -> Result<(), RendezvousError> {
        let mut connection = self.connect().await?;

        connection
            .send_line(&format!("REGISTER {device_id} {endpoint}"))
            .await?;

        match connection.read_line().await?.as_str() {
            "OK registered" => Ok(()),
            response => Err(RendezvousError::Protocol(response.to_string())),
        }
    }

    pub async fn heartbeat(&self, device_id: &str) -> Result<(), RendezvousError> {
        let mut connection = self.connect().await?;

        connection
            .send_line(&format!("HEARTBEAT {device_id}"))
            .await?;

        match connection.read_line().await?.as_str() {
            "OK heartbeat" => Ok(()),
            response => Err(RendezvousError::Protocol(response.to_string())),
        }
    }

    pub async fn lookup(&self, device_id: &str) -> Result<Option<SocketAddr>, RendezvousError> {
        let mut connection = self.connect().await?;

        connection.send_line(&format!("LOOKUP {device_id}")).await?;

        let response = connection.read_line().await?;

        if response == "NOT_FOUND" {
            return Ok(None);
        }

        let endpoint = response
            .strip_prefix("FOUND ")
            .ok_or_else(|| RendezvousError::Protocol(response.clone()))?;

        endpoint
            .parse()
            .map(Some)
            .map_err(|_| RendezvousError::InvalidEndpoint(endpoint.to_string()))
    }

    async fn connect(&self) -> Result<RendezvousConnection, RendezvousError> {
        let stream = TcpStream::connect(self.server).await?;

        let (reader, writer) = stream.into_split();

        Ok(RendezvousConnection {
            reader: BufReader::new(reader),
            writer,
        })
    }
}

struct RendezvousConnection {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl RendezvousConnection {
    async fn send_line(&mut self, line: &str) -> Result<(), RendezvousError> {
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_line(&mut self) -> Result<String, RendezvousError> {
        let mut line = String::new();

        self.reader.read_line(&mut line).await?;

        if line.is_empty() {
            return Err(RendezvousError::Protocol(
                "rendezvous server closed connection".into(),
            ));
        }

        Ok(line.trim_end().to_string())
    }
}
