use std::net::SocketAddr;

use quinn::{Endpoint, ServerConfig};
use rustls::ServerConfig as RustlsServerConfig;

use crate::transport::protocol::ALPN;

pub const TAILSFER_PORT: u16 = 47691;

pub fn server_config() -> ServerConfig {
    let cert = rcgen::generate_simple_self_signed(vec!["tailsfer.local".to_string()])
        .expect("certificate generation failed");

    let cert_der = cert.cert.der().clone();
    let key_der = cert.signing_key.serialize_der();

    let certificate = rustls::pki_types::CertificateDer::from(cert_der);

    let key = rustls::pki_types::PrivateKeyDer::try_from(key_der).expect("invalid private key");

    let mut tls_config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], key)
        .expect("failed to create TLS server config");

    /*
     * The sender advertises the Tailsfer ALPN.
     * The receiver must advertise the exact same protocol.
     */
    tls_config.alpn_protocols = vec![ALPN.to_vec()];

    let quic_config = quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)
        .expect("failed to create QUIC TLS config");

    ServerConfig::with_crypto(std::sync::Arc::new(quic_config))
}

pub async fn listen(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let config = server_config();

    let endpoint = Endpoint::server(config, addr)?;

    println!("Tailsfer QUIC listening on {addr}");

    while let Some(connecting) = endpoint.accept().await {
        match connecting.await {
            Ok(connection) => {
                println!("Connection from {}", connection.remote_address());
            }

            Err(error) => {
                eprintln!("QUIC connection error: {error}");
            }
        }
    }

    Ok(())
}
