//! Application messages for remote job control over libp2p.

use serde::{Deserialize, Serialize};

use crate::jobs::models::{JobResult, JobSpec, JobStatus, SubmitAck};

pub const PROTOCOL: &str = "/cluster-runtime/job/1.0.0";

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
