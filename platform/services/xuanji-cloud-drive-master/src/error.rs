use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasterError {
    VolumeNotFound(String),
    NoCapacity(String),
    ReplicaQuorum(String),
    HeartbeatTimeout(String),
    SnapshotInvalid(String),
    InvalidReplicaCount(String),
    Internal(String),
}

impl fmt::Display for MasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MasterError::VolumeNotFound(id) => write!(f, "Volume not found: {}", id),
            MasterError::NoCapacity(msg) => write!(f, "No capacity: {}", msg),
            MasterError::ReplicaQuorum(msg) => write!(f, "Replica quorum failed: {}", msg),
            MasterError::HeartbeatTimeout(id) => write!(f, "Heartbeat timeout for volume: {}", id),
            MasterError::SnapshotInvalid(id) => write!(f, "Invalid snapshot: {}", id),
            MasterError::InvalidReplicaCount(msg) => write!(f, "Invalid replica count: {}", msg),
            MasterError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl Error for MasterError {}

pub type MasterResult<T> = Result<T, MasterError>;
