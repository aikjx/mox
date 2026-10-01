use async_trait::async_trait;
use bytes::Bytes;
use mox_base_store_core::{BlobObject, ObjectStore, StoreResult};
use mox_cloud_sdk::{create_backend, BackendKind, StoreBackend, StoreConfig};
use mox_kb_svc::{access::KnowledgeAccess, KbState};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
struct CountedObjects {
    inner: Arc<dyn ObjectStore>,
    gets: AtomicUsize,
    heads: AtomicUsize,
    lists: AtomicUsize,
}
#[async_trait]
impl ObjectStore for CountedObjects {
    async fn put(&self, p: &str, t: &str, b: Bytes) -> StoreResult<BlobObject> {
        self.inner.put(p, t, b).await
    }
    async fn get(&self, p: &str) -> StoreResult<Bytes> {
        self.gets.fetch_add(1, Ordering::Relaxed);
        self.inner.get(p).await
    }
    async fn get_range(&self, p: &str, o: u64, n: u64) -> StoreResult<Bytes> {
        self.inner.get_range(p, o, n).await
    }
    async fn delete(&self, p: &str) -> StoreResult<()> {
        self.inner.delete(p).await
    }
    async fn head(&self, p: &str) -> StoreResult<BlobObject> {
        self.heads.fetch_add(1, Ordering::Relaxed);
        self.inner.head(p).await
    }
    async fn list_keys(&self, p: &str) -> StoreResult<Vec<String>> {
        self.lists.fetch_add(1, Ordering::Relaxed);
        self.inner.list_keys(p).await
    }
    async fn exists(&self, p: &str) -> StoreResult<bool> {
        self.inner.exists(p).await
    }
}
impl CountedObjects {
    fn reset(&self) {
        self.gets.store(0, Ordering::Relaxed);
        self.heads.store(0, Ordering::Relaxed);
        self.lists.store(0, Ordering::Relaxed);
    }
}
#[tokio::test]
async fn stats_reads_each_document_once_without_head_requests_or_cached_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let original = create_backend(&StoreConfig {
        kind: BackendKind::Fs,
        data_dir: dir.path().into(),
        verify_checksum: true,
        s3: None,
    })
    .unwrap();
    let counted = Arc::new(CountedObjects {
        inner: original.object.clone(),
        gets: AtomicUsize::new(0),
        heads: AtomicUsize::new(0),
        lists: AtomicUsize::new(0),
    });
    let backend = Arc::new(StoreBackend { object: counted.clone(), ..original });
    let state = KbState::new(backend.clone());
    let owner = state.scoped(KnowledgeAccess {
        tenant_id: "a".into(),
        owner_id: "owner".into(),
        administrator: false,
        readonly: false,
    });
    for i in 0..12 {
        let mut doc = owner.docs.create(&format!("doc-{i}"), "body", None).await.unwrap();
        doc.tags = vec!["shared-tag".into()];
        owner.docs.save(&doc).await.unwrap();
    }
    counted.reset();
    let mut micros = Vec::new();
    for _ in 0..15 {
        let start = std::time::Instant::now();
        let stats = owner.docs.stats().await.unwrap();
        assert_eq!(stats["documents"], 12);
        assert_eq!(stats["tags"], 1);
        assert!(stats["storage_bytes"].as_u64().unwrap() > 0);
        micros.push(start.elapsed().as_micros());
    }
    micros.sort();
    println!("QUERY_COST {{\"documents\":12,\"iterations\":15,\"get_calls\":{},\"head_calls\":{},\"list_calls\":{},\"median_us\":{}}}",counted.gets.load(Ordering::Relaxed),counted.heads.load(Ordering::Relaxed),counted.lists.load(Ordering::Relaxed),micros[7]);
    assert_eq!(counted.gets.load(Ordering::Relaxed), 12 * 15);
    assert_eq!(counted.heads.load(Ordering::Relaxed), 0);
    assert_eq!(counted.lists.load(Ordering::Relaxed), 15);
    let reader = state.scoped(KnowledgeAccess {
        tenant_id: "a".into(),
        owner_id: "reader".into(),
        administrator: false,
        readonly: false,
    });
    let id = owner.docs.list().await.unwrap()[0]["id"].as_str().unwrap().to_owned();
    assert_eq!(reader.docs.stats().await.unwrap()["documents"], 0);
    owner.docs.change_reader(&id, "reader", true, 0).await.unwrap();
    assert_eq!(reader.docs.stats().await.unwrap()["documents"], 1);
    // A separately assembled writer changes the same source, without a shared cache/lock.
    let external = KbState::new(backend).scoped(KnowledgeAccess {
        tenant_id: "a".into(),
        owner_id: "owner".into(),
        administrator: false,
        readonly: false,
    });
    external.docs.change_reader(&id, "reader", false, 1).await.unwrap();
    assert_eq!(reader.docs.stats().await.unwrap()["documents"], 0);
}
