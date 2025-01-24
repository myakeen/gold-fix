use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::message::Message;
use crate::Result;

pub struct Transport {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Transport {
    pub fn new(stream: TcpStream) -> Self {
        Transport {
            stream,
            buffer: Vec::with_capacity(4096),
        }
    }

    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let msg_str = message.to_string()?;
        self.stream.write_all(msg_str.as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<Option<Message>> {
        let mut temp_buf = [0u8; 1024];

        match self.stream.read(&mut temp_buf).await? {
            0 => Ok(None), // Connection closed
            n => {
                self.buffer.extend_from_slice(&temp_buf[..n]);

                // Process complete messages
                if let Some(msg_str) = self.extract_message() {
                    let message = crate::message::parser::MessageParser::parse(&msg_str)?;
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn extract_message(&mut self) -> Option<String> {
        if let Some(pos) = self.buffer.iter().position(|&b| b == b'\x01') {
            let msg_data = self.buffer.drain(..=pos).collect::<Vec<_>>();
            String::from_utf8(msg_data).ok()
        } else {
            None
        }
    }
}