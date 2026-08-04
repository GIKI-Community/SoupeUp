//! Application messages for remote job control over iroh.

use serde::{Deserialize, Serialize};

use crate::jobs::models::{JobResult, JobSpec, JobStatus, SubmitAck};

/// ALPN for the remote job request/response protocol.
pub const JOB_ALPN: &[u8] = b"cluster-runtime/job/1";

/// ALPN for TCP proxy tunnels (Dask scheduler ports, etc.).
pub const TCP_PROXY_ALPN: &[u8] = b"cluster-runtime/tcp-proxy/1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RemoteJobRequest {
    Hello { node_name: String },
    Submit { owner: String, spec: JobSpec },
    Status { job_id: String },
    Cancel { job_id: String },
    Result { job_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RemoteJobResponse {
    Hello {
        peer_id: String,
        node_name: String,
    },
    SubmitAck(SubmitAck),
    Status { status: JobStatus },
    Cancelled,
    Result(JobResult),
    Error { message: String },
}
