use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, ping, request_response, StreamProtocol};

use super::protocol::{RemoteJobRequest, RemoteJobResponse, PROTOCOL};

pub type JobRr =
    request_response::json::Behaviour<RemoteJobRequest, RemoteJobResponse>;

#[derive(NetworkBehaviour)]
pub struct ClusterBehaviour {
    pub identify: identify::Behaviour,
    pub ping: ping::Behaviour,
    pub jobs: JobRr,
}

impl ClusterBehaviour {
    pub fn new(local_public_key: libp2p::identity::PublicKey) -> Self {
        let identify = identify::Behaviour::new(identify::Config::new(
            "cluster-runtime/1.0.0".into(),
            local_public_key,
        ));
        let ping = ping::Behaviour::default();
        let jobs = request_response::json::Behaviour::new(
            [(
                StreamProtocol::new(PROTOCOL),
                request_response::ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );
        Self {
            identify,
            ping,
            jobs,
        }
    }
}
