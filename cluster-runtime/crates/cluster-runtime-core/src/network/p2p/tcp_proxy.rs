//! TCP proxy over iroh: dial by EndpointId, forward to a local loopback port.

use std::net::SocketAddr;
use std::sync::Arc;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh::{Endpoint, EndpointAddr, EndpointId};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// Accepts iroh streams and forwards each to `127.0.0.1:<port>` where `<port>`
/// is the first two bytes (big-endian u16) of the stream.
#[derive(Debug, Clone, Default)]
pub struct TcpProxyHandler;

impl ProtocolHandler for TcpProxyHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        log::info!("iroh tcp-proxy: accepted connection from {remote}");

        loop {
            let (send, recv) = match connection.accept_bi().await {
                Ok(s) => s,
                Err(_) => break,
            };

            tokio::spawn(async move {
                if let Err(e) = proxy_inbound(send, recv).await {
                    log::warn!("iroh tcp-proxy: inbound session error: {e}");
                }
            });
        }

        Ok(())
    }
}

async fn proxy_inbound(mut send: SendStream, mut recv: RecvStream) -> Result<(), String> {
    let mut port_buf = [0u8; 2];
    recv.read_exact(&mut port_buf)
        .await
        .map_err(|e| e.to_string())?;
    let port = u16::from_be_bytes(port_buf);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let tcp = TcpStream::connect(addr)
        .await
        .map_err(|e| format!("connect {addr}: {e}"))?;
    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let to_tcp = async {
        let n = tokio::io::copy(&mut recv, &mut tcp_write)
            .await
            .map_err(|e| e.to_string())?;
        let _ = tcp_write.shutdown().await;
        Ok::<u64, String>(n)
    };
    let to_iroh = async {
        let n = tokio::io::copy(&mut tcp_read, &mut send)
            .await
            .map_err(|e| e.to_string())?;
        let _ = send.finish();
        Ok::<u64, String>(n)
    };

    match tokio::try_join!(to_tcp, to_iroh) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// Local loopback listener that tunnels each accepted TCP connection over iroh
/// to `remote_id` targeting `remote_port` on the peer.
pub struct TcpTunnel {
    local_port: u16,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TcpTunnel {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn local_tcp_address(&self) -> String {
        format!("tcp://127.0.0.1:{}", self.local_port)
    }
}

impl Drop for TcpTunnel {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Bind `127.0.0.1:0` and forward accepted connections to `remote_id`:`remote_port`
/// over [`super::protocol::TCP_PROXY_ALPN`].
pub async fn open_tunnel(
    endpoint: Endpoint,
    remote_id: EndpointId,
    remote_port: u16,
) -> Result<TcpTunnel, String> {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .map_err(|e| e.to_string())?;
    let local_port = listener
        .local_addr()
        .map_err(|e| e.to_string())?
        .port();

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let endpoint = Arc::new(endpoint);
    let addr = EndpointAddr::new(remote_id);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    log::info!("iroh tcp-proxy: local tunnel on :{local_port} shutting down");
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((tcp, _)) => {
                            let ep = endpoint.clone();
                            let addr = addr.clone();
                            tokio::spawn(async move {
                                if let Err(e) = proxy_outbound(ep, addr, remote_port, tcp).await {
                                    log::warn!("iroh tcp-proxy: outbound session error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("iroh tcp-proxy: accept error: {e}");
                            break;
                        }
                    }
                }
            }
        }
    });

    Ok(TcpTunnel {
        local_port,
        shutdown_tx: Some(shutdown_tx),
    })
}

async fn proxy_outbound(
    endpoint: Arc<Endpoint>,
    addr: EndpointAddr,
    remote_port: u16,
    tcp: TcpStream,
) -> Result<(), String> {
    let conn = endpoint
        .connect(addr, super::protocol::TCP_PROXY_ALPN)
        .await
        .map_err(|e| format!("iroh connect: {e}"))?;
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|e| format!("open_bi: {e}"))?;

    send.write_all(&remote_port.to_be_bytes())
        .await
        .map_err(|e| e.to_string())?;

    let (mut tcp_read, mut tcp_write) = tcp.into_split();

    let to_remote = async {
        let n = tokio::io::copy(&mut tcp_read, &mut send)
            .await
            .map_err(|e| e.to_string())?;
        let _ = send.finish();
        Ok::<u64, String>(n)
    };
    let to_local = async {
        let n = tokio::io::copy(&mut recv, &mut tcp_write)
            .await
            .map_err(|e| e.to_string())?;
        let _ = tcp_write.shutdown().await;
        Ok::<u64, String>(n)
    };

    match tokio::try_join!(to_remote, to_local) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
