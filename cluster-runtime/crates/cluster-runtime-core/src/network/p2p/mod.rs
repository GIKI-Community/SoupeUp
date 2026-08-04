//! iroh WAN mesh for peer discovery and remote job relay.
//!
//! Dial peers by EndpointId (n0 relays + hole punching). Local clients still
//! use the loopback axum API on 8129.

mod framing;
mod identity;
mod job_handler;
mod peer_record;
mod protocol;
mod service;
mod tcp_proxy;

pub use protocol::{RemoteJobRequest, RemoteJobResponse, JOB_ALPN, TCP_PROXY_ALPN};
pub use service::P2pService;
#[allow(unused_imports)]
pub use service::PeerSnapshot;
