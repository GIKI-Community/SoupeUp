//! libp2p WAN mesh for peer discovery and remote job relay.
//!
//! Listens on firewall-friendly ports (default WebSocket TCP 8080; optionally
//! 80/443). Local clients still use the loopback axum API on 8129.

mod behaviour;
mod identity;
mod protocol;
mod service;

pub use service::P2pService;
#[allow(unused_imports)]
pub use service::PeerSnapshot;
#[allow(unused_imports)]
pub use protocol::{RemoteJobRequest, RemoteJobResponse};
