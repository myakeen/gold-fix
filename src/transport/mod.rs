use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::message::Message;
use crate::message::parser::MessageParser;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use std::time::Duration;
    use crate::message::{Field, field};

    #[tokio::test]
    async fn test_transport_send_receive() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Client connection
        let client = tokio::spawn(async move {
            let stream = TcpStream::connect(addr).await.unwrap();
            let mut transport = Transport::new(stream);

            // Create and send a heartbeat message
            let mut msg = Message::new(field::values::HEARTBEAT);
            msg.set_field(Field::new(field::SENDER_COMP_ID, "CLIENT"));
            msg.set_field(Field::new(field::TARGET_COMP_ID, "SERVER"));
            msg.set_field(Field::new(field::MSG_SEQ_NUM, "1"));

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
        response.set_field(Field::new(field::SENDER_COMP_ID, "SERVER"));
        response.set_field(Field::new(field::TARGET_COMP_ID, "CLIENT"));
        response.set_field(Field::new(field::MSG_SEQ_NUM, "1"));
        transport.send(&response).await.unwrap();

        // Wait for client
        let client_result = client.await.unwrap();
        assert!(client_result.is_some());
    }
}