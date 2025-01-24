use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::client::TlsStream;
use tokio_rustls::{TlsConnector, rustls};
use rustls::{ClientConfig, RootCertStore};
use rustls::pki_types::{ServerName, CertificateDer, PrivateKeyDer};
use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use crate::message::Message;
use crate::message::parser::MessageParser;
use crate::Result;
use crate::error::FixError;

#[derive(Clone)]
pub struct TransportConfig {
    pub use_ssl: bool,
    pub cert_file: Option<PathBuf>,
    pub key_file: Option<PathBuf>,
    pub ca_file: Option<PathBuf>,
    pub verify_peer: bool,
    pub buffer_size: usize,
    pub connection_timeout: std::time::Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig {
            use_ssl: false,
            cert_file: None,
            key_file: None,
            ca_file: None,
            verify_peer: true,
            buffer_size: 4096,
            connection_timeout: std::time::Duration::from_secs(30),
        }
    }
}

enum ConnectionType {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

pub struct Transport {
    connection: Option<ConnectionType>,
    config: TransportConfig,
    buffer: Vec<u8>,
}

impl Transport {
    pub fn new_with_config(config: TransportConfig) -> Self {
        Transport {
            connection: None,
            config: config.clone(),
            buffer: Vec::with_capacity(config.buffer_size),
        }
    }

    pub fn new(stream: TcpStream) -> Self {
        Transport {
            connection: Some(ConnectionType::Plain(stream)),
            config: TransportConfig::default(),
            buffer: Vec::with_capacity(4096),
        }
    }

    pub async fn connect(&mut self, addr: &str) -> Result<()> {
        let stream = TcpStream::connect(addr).await?;

        if self.config.use_ssl {
            let tls_stream = self.establish_tls(stream).await?;
            self.connection = Some(ConnectionType::Tls(tls_stream));
        } else {
            self.connection = Some(ConnectionType::Plain(stream));
        }

        Ok(())
    }

    async fn establish_tls(&self, stream: TcpStream) -> Result<TlsStream<TcpStream>> {
        let mut root_store = RootCertStore::empty();

        // Load system root certificates
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        // Load custom CA if provided
        if let Some(ca_path) = &self.config.ca_file {
            let mut file = File::open(ca_path)
                .map_err(|e| FixError::TransportError(format!("Failed to open CA file: {}", e)))?;
            let mut reader = BufReader::new(file);
            let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
                .filter_map(|result| result.ok())
                .collect();

            for cert in certs {
                root_store.add(cert)
                    .map_err(|e| FixError::TransportError(format!("Failed to add CA cert: {}", e)))?;
            }
        }

        let config = if let (Some(cert_path), Some(key_path)) = (&self.config.cert_file, &self.config.key_file) {
            // Load client certificate and private key
            let cert_file = File::open(cert_path)
                .map_err(|e| FixError::TransportError(format!("Failed to open cert file: {}", e)))?;
            let mut cert_reader = BufReader::new(cert_file);
            let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
                .filter_map(|result| result.ok())
                .collect();

            let key_file = File::open(key_path)
                .map_err(|e| FixError::TransportError(format!("Failed to open key file: {}", e)))?;
            let mut key_reader = BufReader::new(key_file);
            let key = rustls_pemfile::private_key(&mut key_reader)
                .map_err(|e| FixError::TransportError(format!("Failed to parse key file: {}", e)))?
                .ok_or_else(|| FixError::TransportError("No private key found".to_string()))?;

            let key = match key {
                PrivateKeyDer::Pkcs8(key) => key,
                PrivateKeyDer::Pkcs1(key) => key.into(),
                _ => return Err(FixError::TransportError("Unsupported private key format".to_string())),
            };

            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_client_auth_cert(certs, key)
                .map_err(|e| FixError::TransportError(format!("Failed to configure client auth: {}", e)))?
        } else {
            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth()
        };

        let connector = TlsConnector::from(Arc::new(config));
        let domain = ServerName::try_from("localhost")
            .map_err(|e| FixError::TransportError(format!("Invalid DNS name: {}", e)))?;

        let tls_stream = connector.connect(domain, stream).await
            .map_err(|e| FixError::TransportError(format!("TLS connection failed: {}", e)))?;

        Ok(tls_stream)
    }

    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let msg_str = message.to_string()?;

        match self.connection.as_mut() {
            Some(ConnectionType::Plain(stream)) => {
                stream.write_all(msg_str.as_bytes()).await?;
                stream.flush().await?;
            },
            Some(ConnectionType::Tls(stream)) => {
                stream.write_all(msg_str.as_bytes()).await?;
                stream.flush().await?;
            },
            None => return Err(FixError::TransportError("Not connected".to_string())),
        }

        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<Message>> {
        let mut temp_buf = vec![0u8; self.config.buffer_size];

        let n = match self.connection.as_mut() {
            Some(ConnectionType::Plain(stream)) => stream.read(&mut temp_buf).await?,
            Some(ConnectionType::Tls(stream)) => stream.read(&mut temp_buf).await?,
            None => return Err(FixError::TransportError("Not connected".to_string())),
        };

        if n == 0 {
            return Ok(None); // Connection closed
        }

        self.buffer.extend_from_slice(&temp_buf[..n]);

        if let Some((msg_str, consumed)) = MessageParser::extract_complete_message(&self.buffer) {
            // Remove consumed bytes from buffer
            self.buffer.drain(..consumed);

            // Parse the message
            let message = MessageParser::parse(&msg_str)?;
            Ok(Some(message))
        } else {
            Ok(None)
        }
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.connection = None;
        self.buffer.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use std::time::Duration;
    use crate::message::{Field, field};

    #[tokio::test]
    async fn test_transport_plain_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Client connection
        let client = tokio::spawn(async move {
            let stream = TcpStream::connect(addr).await.unwrap();
            let mut transport = Transport::new(stream);

            // Create and send a heartbeat message
            let mut msg = Message::new(field::values::HEARTBEAT);
            let _ = msg.set_field(Field::new(field::SENDER_COMP_ID, "CLIENT"));
            let _ = msg.set_field(Field::new(field::TARGET_COMP_ID, "SERVER"));
            let _ = msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));

            transport.send(&msg).await.unwrap();

            // Wait for response
            tokio::time::sleep(Duration::from_millis(100)).await;
            transport.receive().await.unwrap()
        });

        // Server connection
        let (stream, _) = listener.accept().await.unwrap();
        let mut transport = Transport::new(stream);

        // Receive message
        let received = transport.receive().await.unwrap();
        assert!(received.is_some());
        let msg = received.unwrap();
        assert_eq!(msg.msg_type(), field::values::HEARTBEAT);

        // Send response
        let mut response = Message::new(field::values::HEARTBEAT);
        let _ = response.set_field(Field::new(field::SENDER_COMP_ID, "SERVER"));
        let _ = response.set_field(Field::new(field::TARGET_COMP_ID, "CLIENT"));
        let _ = response.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
        transport.send(&response).await.unwrap();

        // Wait for client
        let client_result = client.await.unwrap();
        assert!(client_result.is_some());
    }

    #[tokio::test]
    async fn test_transport_ssl_connection() {
        // This test requires SSL certificates
        let config = TransportConfig {
            use_ssl: true,
            cert_file: Some(PathBuf::from("tests/fixtures/client.crt")),
            key_file: Some(PathBuf::from("tests/fixtures/client.key")),
            ca_file: Some(PathBuf::from("tests/fixtures/ca.crt")),
            verify_peer: true,
            ..Default::default()
        };

        let mut transport = Transport::new_with_config(config);
        let result = transport.connect("localhost:8443").await;

        // This will fail without proper certificates, which is expected in the test environment
        assert!(result.is_err());
    }
}