//! Mox V5 §3 10 Standards Matrix Test Crate.

pub mod dengbao_hash_chain;
pub mod etag_crc32c;
pub mod sm3_hash;
pub mod sm2_sign;
pub mod fips_hmac;
pub mod rfc5424;
pub mod sigv4;

#[cfg(feature = "gm-sm")]
pub mod sm4_gcm;
#[cfg(feature = "gm-sm")]
pub mod sts_sm2;

pub use sm3_hash::{hmac_sm3, hmac_sm3_hex, sm3, sm3_hex};
pub use dengbao_hash_chain::{
    verify_json_file, ChainVerifyResult, HashChain, HashChainBlock, Outcome,
};

#[cfg(feature = "gm-sm")]
pub use sm2_sign::*;

#[cfg(test)]
mod _std_tests_touch {
    use crate::dengbao_hash_chain as _dbc;
    #[allow(dead_code)]
    fn _t() { let _ = _dbc::HashChain::new(b"k"); }
}

fn split_parent(path: &str) -> (&str, &str) {
    if path == "/" { return ("/", ""); }
    let p = path.trim_end_matches('/');
    match p.rfind('/') {
        Some(0) => ("/", &p[1..]),
        Some(i) => (&p[..i], &p[i + 1..]),
        None => (".", p),
    }
}

pub mod posix_skeleton {
    use super::split_parent;
    use async_trait::async_trait;
    use mox_cloud_foundation::{FileStat, MetaStorageProvider, MockMetaStorageProvider};
    #[async_trait]
    pub trait PosixFiler: Send + Sync {
        async fn mkdir(&self, path: &str, mode: u32) -> std::io::Result<()>;
        async fn stat(&self, path: &str) -> std::io::Result<FileStat>;
        async fn symlink(&self, target: &str, link_path: &str) -> std::io::Result<()>;
    }
    pub struct MockPosixFiler(pub MockMetaStorageProvider);
    fn to_io<E: std::fmt::Display>(e: E) -> std::io::Error { std::io::Error::other(e.to_string()) }
    #[async_trait]
    impl PosixFiler for MockPosixFiler {
        async fn mkdir(&self, path: &str, mode: u32) -> std::io::Result<()> {
            let (pp, name) = split_parent(path);
            if name.is_empty() { return Ok(()); }
            self.0.mkdir(pp, name, mode).await.map_err(to_io)?; Ok(())
        }
        async fn stat(&self, path: &str) -> std::io::Result<FileStat> { self.0.stat(path).await.map_err(to_io) }
        async fn symlink(&self, target: &str, link_path: &str) -> std::io::Result<()> {
            self.0.symlink(target, link_path).await.map_err(to_io)
        }
    }
}

pub mod ngql_skeleton {
    use async_trait::async_trait;
    use mox_cloud_foundation::{GraphQueryProvider, MockGraphQueryProvider, QueryResultSet};
    #[async_trait]
    pub trait NgqlRunner: Send + Sync {
        async fn execute_ngql(&self, space: &str, ngql: &str) -> anyhow::Result<QueryResultSet>;
    }
    pub struct MockNgqlRunner(pub MockGraphQueryProvider);
    #[async_trait]
    impl NgqlRunner for MockNgqlRunner {
        async fn execute_ngql(&self, space: &str, ngql: &str) -> anyhow::Result<QueryResultSet> {
            self.0.execute_ngql(space, ngql).await.map_err(|e| anyhow::anyhow!(e.to_string()))
        }
    }
}

pub mod cypher_skeleton {
    use async_trait::async_trait;
    use mox_cloud_foundation::{GraphQueryProvider, MockGraphQueryProvider, QueryResultSet};
    #[async_trait]
    pub trait CypherRunner: Send + Sync {
        async fn execute_cypher(&self, cypher: &str) -> anyhow::Result<QueryResultSet>;
    }
    pub struct MockCypherRunner(pub MockGraphQueryProvider);
    const DEFAULT_SPACE: &str = "default_space";
    #[async_trait]
    impl CypherRunner for MockCypherRunner {
        async fn execute_cypher(&self, cypher: &str) -> anyhow::Result<QueryResultSet> {
            self.0.execute_cypher(DEFAULT_SPACE, cypher).await.map_err(|e| anyhow::anyhow!(e.to_string()))
        }
    }
}

pub mod gql_skeleton {
    use async_trait::async_trait;
    use mox_cloud_foundation::{GraphQueryProvider, MockGraphQueryProvider, QueryResultSet};
    #[async_trait]
    pub trait GqlRunner: Send + Sync {
        async fn execute_gql(&self, gql: &str) -> anyhow::Result<QueryResultSet>;
    }
    pub struct MockGqlRunner(pub MockGraphQueryProvider);
    #[async_trait]
    impl GqlRunner for MockGqlRunner {
        async fn execute_gql(&self, gql: &str) -> anyhow::Result<QueryResultSet> {
            self.0.execute_cypher("gql_space", gql).await.map_err(|e| anyhow::anyhow!(e.to_string()))
        }
    }
}

pub mod ais_skeleton {
    use async_trait::async_trait;
    use bytes::Bytes;
    use mox_cloud_foundation::{MockGraphMetaProvider, MockIamProvider, MockObjectStorageProvider};
    pub struct AisLayeredBundle {
        pub storage: MockObjectStorageProvider,
        pub iam: MockIamProvider,
        pub graph_meta: MockGraphMetaProvider,
    }
    const DEFAULT_BUCKET: &str = "ais-default";
    impl AisLayeredBundle {
        pub fn new() -> Self { Self { storage: MockObjectStorageProvider::default(), iam: MockIamProvider::default(), graph_meta: MockGraphMetaProvider::default() } }
    }
    impl Default for AisLayeredBundle { fn default() -> Self { Self::new() } }
    #[async_trait]
    pub trait AisStorageGate: Send + Sync {
        async fn put(&self, key: &str, data: Vec<u8>) -> anyhow::Result<()>;
        async fn get(&self, key: &str) -> anyhow::Result<Vec<u8>>;
    }
    #[async_trait]
    impl AisStorageGate for AisLayeredBundle {
        async fn put(&self, _k: &str, _d: Vec<u8>) -> anyhow::Result<()> { Ok(()) }
        async fn get(&self, _k: &str) -> anyhow::Result<Vec<u8>> { Ok(vec![]) }
    }
    struct AisStore(parking_lot::Mutex<std::collections::BTreeMap<String, Vec<u8>>>);
    impl std::fmt::Debug for AisStore { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "AisStore({})", self.0.lock().len()) } }
    pub struct AisLayeredBundleReal {
        pub storage: MockObjectStorageProvider,
        pub iam: MockIamProvider,
        pub graph_meta: MockGraphMetaProvider,
        store: AisStore,
    }
    impl Default for AisLayeredBundleReal { fn default() -> Self { Self::new() } }
    impl AisLayeredBundleReal {
        pub fn new() -> Self { Self { storage: MockObjectStorageProvider::default(), iam: MockIamProvider::default(), graph_meta: MockGraphMetaProvider::default(), store: AisStore(parking_lot::Mutex::new(std::collections::BTreeMap::new())) } }
    }
    #[async_trait]
    impl AisStorageGate for AisLayeredBundleReal {
        async fn put(&self, key: &str, data: Vec<u8>) -> anyhow::Result<()> { self.store.0.lock().insert(key.into(), data); Ok(()) }
        async fn get(&self, key: &str) -> anyhow::Result<Vec<u8>> {
            self.store.0.lock().get(key).cloned().ok_or_else(|| anyhow::anyhow!("nf: {key}"))
        }
    }
    pub use AisLayeredBundleReal as AisLayeredBundle2;
    pub fn ais_bundle_real() -> AisLayeredBundleReal { AisLayeredBundleReal::new() }
    #[allow(dead_code)] pub(crate) fn _default_bucket() -> &'static str { DEFAULT_BUCKET }
    #[allow(dead_code)] pub(crate) fn _bytes_from_vec(v: Vec<u8>) -> Bytes { Bytes::from(v) }
}

pub mod dengbao_skeleton {
    pub use crate::dengbao_hash_chain::{verify_json_file, ChainVerifyResult, HashChain, HashChainBlock, Outcome};
    use sha2::{Digest, Sha256};
    #[derive(Debug, Clone)]
    pub struct AuditEvent { pub seq: u64, pub ts_ms: u64, pub actor: String, pub action: String, pub resource: String, pub prev_hash: String }
    impl AuditEvent {
        pub fn hash(&self) -> String {
            let mut h = Sha256::new();
            h.update(self.seq.to_le_bytes()); h.update(self.ts_ms.to_le_bytes());
            h.update(&self.actor); h.update(&self.action); h.update(&self.resource); h.update(&self.prev_hash);
            hex::encode(h.finalize())
        }
    }
    pub fn validate_chain(events: &[AuditEvent]) -> bool {
        if events.is_empty() { return true; }
        if events[0].prev_hash != "GENESIS" { return false; }
        let mut p = events[0].hash();
        for ev in &events[1..] { if ev.prev_hash != p { return false } p = ev.hash(); }
        true
    }
}