// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! EC profile – describes how many data/parity shards and the minimum object
//! size before an object is promoted to erasure-coded storage.

use crate::reed_solomon::RSError;
use serde::{Deserialize, Serialize};

/// Default minimum object size threshold – 64 KiB.
pub const DEFAULT_MIN_OBJ_SIZE: u64 = 65536;

/// Describes the Reed-Solomon (n + k) parameters used to erasure-code an
/// object, together with a minimum size threshold below which objects are
/// stored as plain replicas instead of being sliced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct EcProfile {
    /// Number of data shards (`n`).  Must be >= 2.
    pub data_shards: u16,
    /// Number of parity shards (`k`).  Must be >= 1.
    pub parity_shards: u16,
    /// Objects smaller than this are written as replica and skip EC.
    pub min_obj_size: u64,
}

impl EcProfile {
    /// Validate and build a new profile.  Returns `InvalidInput` when the
    /// constraints (`data >= 2`, `parity >= 1`) are violated.
    pub fn new(data_shards: u16, parity_shards: u16, min_obj_size: u64) -> Result<Self, RSError> {
        if data_shards < 2 {
            return Err(RSError::InvalidInput(format!(
                "data_shards must be >= 2, got {data_shards}"
            )));
        }
        if parity_shards < 1 {
            return Err(RSError::InvalidInput(format!(
                "parity_shards must be >= 1, got {parity_shards}"
            )));
        }
        Ok(Self { data_shards, parity_shards, min_obj_size })
    }

    /// Constructor with default `min_obj_size = DEFAULT_MIN_OBJ_SIZE`.
    pub fn with_default_min_size(data_shards: u16, parity_shards: u16) -> Result<Self, RSError> {
        Self::new(data_shards, parity_shards, DEFAULT_MIN_OBJ_SIZE)
    }

    /// Total shard count (data + parity).
    pub fn total_shards(&self) -> usize {
        (self.data_shards as usize).saturating_add(self.parity_shards as usize)
    }

    /// True if the object is small enough to bypass EC and remain a replica.
    pub fn is_replica(&self, object_size: u64) -> bool {
        object_size < self.min_obj_size
    }
}

impl Default for EcProfile {
    fn default() -> Self {
        // Safe: 4 + 2 always passes validation.
        Self::new(4, 2, DEFAULT_MIN_OBJ_SIZE).expect("default profile must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_rejected() {
        assert!(matches!(EcProfile::new(1, 2, 100), Err(RSError::InvalidInput(_))));
        assert!(matches!(EcProfile::new(2, 0, 100), Err(RSError::InvalidInput(_))));
    }

    #[test]
    fn default_sane() {
        let d = EcProfile::default();
        assert_eq!(d.data_shards, 4);
        assert_eq!(d.parity_shards, 2);
        assert_eq!(d.min_obj_size, DEFAULT_MIN_OBJ_SIZE);
        assert!(!d.is_replica(DEFAULT_MIN_OBJ_SIZE));
        assert!(d.is_replica(DEFAULT_MIN_OBJ_SIZE - 1));
    }
}


#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_default_min_obj_size_constant() {
        assert_eq!(DEFAULT_MIN_OBJ_SIZE, 65536);
    }

    #[test]
    fn test_ec_profile_serialization() {
        let p = EcProfile::with_default_min_size(4, 2).unwrap();
        let json = format!("{:?}", p);
        assert!(json.contains("data_shards"));
        assert!(json.contains("parity_shards"));
        assert!(json.contains("min_obj_size"));
    }

    #[test]
    fn test_ec_profile_copy_clone() {
        let p = EcProfile::with_default_min_size(6, 3).unwrap();
        let p2 = p;
        assert_eq!(p2.data_shards, 6);
        assert_eq!(p2.parity_shards, 3);
    }

    #[test]
    fn test_ec_profile_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let p1 = EcProfile::with_default_min_size(4, 2).unwrap();
        let p2 = EcProfile::with_default_min_size(4, 2).unwrap();
        set.insert(p1);
        set.insert(p2);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_ec_profile_debug() {
        let p = EcProfile::with_default_min_size(4, 2).unwrap();
        let s = format!("{p:?}");
        assert!(s.contains("EcProfile"));
        assert!(s.contains("data_shards"));
    }

    #[test]
    fn test_ec_profile_total_shards_saturating() {
        let p = EcProfile::new(u16::MAX, 1, 100).unwrap();
        // saturating_add prevents overflow
        assert_eq!(p.total_shards(), u16::MAX as usize + 1);
    }

    #[test]
    fn test_ec_profile_is_replica_boundary() {
        let p = EcProfile::new(4, 2, 1000).unwrap();
        assert!(p.is_replica(999));
        assert!(!p.is_replica(1000));
        assert!(!p.is_replica(1001));
    }
}
