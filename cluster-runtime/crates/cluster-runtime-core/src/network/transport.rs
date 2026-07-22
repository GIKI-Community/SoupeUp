//! Transport layer for cluster communication
//! 
//! Provides TCP transport with binary message framing using length-prefixed encoding

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};
use bytes::{Bytes, BytesMut};
use crate::network::messages::Message;
use crate::network::NetworkError;

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub connection_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub max_message_size: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            connection_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            max_message_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Transport layer for sending/receiving messages
pub struct Transport {
    stream: TcpStream,
    config: TransportConfig,
    read_buf: BytesMut,
}

impl Transport {
    /// Create a new transport from an existing TCP stream
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            config: TransportConfig::default(),
            read_buf: BytesMut::with_capacity(4096),
        }
    }

    /// Create a transport with custom configuration
    pub fn with_config(stream: TcpStream, config: TransportConfig) -> Self {
        Self {
            stream,
            config,
            read_buf: BytesMut::with_capacity(4096),
        }
    }

    /// Connect to a peer
    pub async fn connect(addr: &str, config: TransportConfig) -> Result<Self, NetworkError> {
        let stream = timeout(
            config.connection_timeout,
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| NetworkError::ConnectionTimeout)?
        .map_err(|e| NetworkError::Io(e))?;

        Ok(Self::with_config(stream, config))
    }

    /// Send a message
    pub async fn send(&mut self, message: &Message) -> Result<(), NetworkError> {
        let data = serde_json::to_vec(message)?;
        
        // Check message size
        if data.len() > self.config.max_message_size {
            return Err(NetworkError::Serialization(
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "message too large"
                ))
            ));
        }

        // Write length prefix (4 bytes) + message data
        let len = data.len() as u32;
        let mut write_buf = Vec::with_capacity(4 + data.len());
        write_buf.extend_from_slice(&len.to_be_bytes());
        write_buf.extend_from_slice(&data);

        timeout(
            self.config.write_timeout,
            self.stream.write_all(&write_buf),
        )
        .await
        .map_err(|_| NetworkError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "write timeout"
        )))?
        .map_err(|e| NetworkError::Io(e))?;

        self.stream.flush().await?;
        Ok(())
    }

    /// Receive a message
    pub async fn receive(&mut self) -> Result<Option<Message>, NetworkError> {
        // Read message length (4 bytes)
        let len_bytes = self.read_exact(4).await?;
        if len_bytes.is_none() {
            return Ok(None);
        }

        let len = u32::from_be_bytes([
            len_bytes.as_ref().unwrap()[0],
            len_bytes.as_ref().unwrap()[1],
            len_bytes.as_ref().unwrap()[2],
            len_bytes.as_ref().unwrap()[3],
        ]) as usize;

        // Validate message size
        if len > self.config.max_message_size {
            return Err(NetworkError::Serialization(
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "message exceeds maximum size"
                ))
            ));
        }

        // Read message data
        let data = self.read_exact(len).await?;
        if data.is_none() {
            return Ok(None);
        }

        let message: Message = serde_json::from_slice(data.as_ref().unwrap())?;
        Ok(Some(message))
    }

    /// Read exact number of bytes with timeout
    async fn read_exact(&mut self, n: usize) -> Result<Option<Bytes>, NetworkError> {
        let mut buf = vec![0u8; n];
        let read = 0;

        while read < n {
            // Try to read from buffer first
            if self.read_buf.len() >= n - read {
                let data = self.read_buf.split_to(n - read).freeze();
                buf[read..].copy_from_slice(&data);
                return Ok(Some(Bytes::from(buf)));
            }

            // Read more from socket
            let mut chunk = vec![0u8; 4096];
            let result = timeout(
                self.config.read_timeout,
                self.stream.read(&mut chunk),
            ).await;

            match result {
                Ok(Ok(0)) => {
                    // Connection closed
                    return if read > 0 {
                        Ok(Some(Bytes::from(buf[..read].to_vec())))
                    } else {
                        Ok(None)
                    };
                }
                Ok(Ok(n_read)) => {
                    self.read_buf.extend_from_slice(&chunk[..n_read]);
                }
                Ok(Err(e)) => return Err(NetworkError::Io(e)),
                Err(_) => return Err(NetworkError::Io(
                    std::io::Error::new(std::io::ErrorKind::TimedOut, "read timeout")
                )),
            }
        }

        Ok(Some(Bytes::from(buf)))
    }

    /// Get the peer address
    pub fn peer_addr(&self) -> Result<std::net::SocketAddr, NetworkError> {
        Ok(self.stream.peer_addr()?)
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, NetworkError> {
        Ok(self.stream.local_addr()?)
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<(), NetworkError> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

/// Server for accepting incoming connections
pub struct TransportServer {
    listener: TcpListener,
    config: TransportConfig,
}

impl TransportServer {
    /// Bind to an address and create a server
    pub async fn bind(addr: &str, config: TransportConfig) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener, config })
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> Result<Transport, NetworkError> {
        let (stream, _) = self.listener.accept().await?;
        Ok(Transport::with_config(stream, self.config.clone()))
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr, NetworkError> {
        Ok(self.listener.local_addr()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::messages::*;

    #[tokio::test]
    async fn test_transport_send_receive() {
        let config = TransportConfig::default();
        let server = TransportServer::bind("127.0.0.1:0", config.clone()).await.unwrap();
        let addr = server.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut transport = server.accept().await.unwrap();
            let msg = transport.receive().await.unwrap().unwrap();
            match msg {
                Message::Ping(ping) => {
                    assert_eq!(ping.node_id, "test-node");
                    let pong = Message::pong("server-node".to_string(), ping.timestamp);
                    transport.send(&pong).await.unwrap();
                }
                _ => panic!("Expected Ping message"),
            }
        });

        let mut client = Transport::connect(&format!("127.0.0.1:{}", addr.port()), config).await.unwrap();
        let ping = Message::ping("test-node".to_string());
        client.send(&ping).await.unwrap();
        let response = client.receive().await.unwrap().unwrap();
        match response {
            Message::Pong(pong) => {
                assert_eq!(pong.node_id, "server-node");
            }
            _ => panic!("Expected Pong message"),
        }

        server_task.await.unwrap();
    }
}
