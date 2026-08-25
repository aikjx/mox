use async_trait::async_trait;
use std::collections::BTreeMap;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpaceInfo {
    pub name: String,
    pub partition_num: u32,
    pub replica_factor: u32,
    pub vid_type: String,
    pub charset: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagDef {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdgeTypeDef {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostInfo {
    pub host_addr: String,
    pub status: String,
    pub git_commit: String,
    pub leader_count: u32,
    pub partition_count: u32,
}

#[derive(Debug, Clone, Default)]
struct SpaceMeta {
    info: SpaceInfo,
    tags: BTreeMap<String, TagDef>,
    edge_types: BTreeMap<String, EdgeTypeDef>,
}
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[async_trait]
pub trait GraphMetaProvider: Send + Sync {
    async fn create_space(
        &self,
        name: &str,
        pn: u32,
        rf: u32,
        vt: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn drop_space(&self, name: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn list_spaces(&self) -> Result<Vec<SpaceInfo>, Box<dyn Error + Send + Sync>>;
    async fn create_tag(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn create_edge_type(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn alter_tag(
        &self,
        space: &str,
        name: &str,
        add_fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn drop_tag(&self, space: &str, name: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn drop_edge_type(
        &self,
        space: &str,
        name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn show_hosts(&self) -> Result<Vec<HostInfo>, Box<dyn Error + Send + Sync>>;
    async fn list_tags(&self, space: &str) -> Result<Vec<TagDef>, Box<dyn Error + Send + Sync>>;
    async fn list_edge_types(
        &self,
        space: &str,
    ) -> Result<Vec<EdgeTypeDef>, Box<dyn Error + Send + Sync>>;
}

pub struct MockGraphMetaProvider {
    sp: parking_lot::Mutex<BTreeMap<String, SpaceMeta>>,
    hs: parking_lot::Mutex<Vec<HostInfo>>,
}
impl Default for MockGraphMetaProvider {
    fn default() -> Self {
        Self {
            sp: parking_lot::Mutex::new(BTreeMap::new()),
            hs: parking_lot::Mutex::new(vec![HostInfo {
                host_addr: "127.0.0.1:9669".into(),
                status: "ONLINE".into(),
                git_commit: "mock".into(),
                ..Default::default()
            }]),
        }
    }
}

#[async_trait]
impl GraphMetaProvider for MockGraphMetaProvider {
    async fn create_space(
        &self,
        name: &str,
        pn: u32,
        rf: u32,
        vt: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        if sp.contains_key(name) {
            return Err("space exists".into());
        }
        sp.insert(
            name.into(),
            SpaceMeta {
                info: SpaceInfo {
                    name: name.into(),
                    partition_num: pn,
                    replica_factor: rf,
                    vid_type: vt.into(),
                    charset: "utf8".into(),
                    created_at: now_ms(),
                },
                ..Default::default()
            },
        );
        Ok(())
    }
    async fn drop_space(&self, name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sp.lock().remove(name);
        Ok(())
    }
    async fn list_spaces(&self) -> Result<Vec<SpaceInfo>, Box<dyn Error + Send + Sync>> {
        Ok(self.sp.lock().values().map(|m| m.info.clone()).collect())
    }
    async fn create_tag(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        let sm = sp.get_mut(space).ok_or("space missing")?;
        sm.tags.insert(
            name.into(),
            TagDef {
                name: name.into(),
                fields,
                created_at: now_ms(),
            },
        );
        Ok(())
    }
    async fn create_edge_type(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        let sm = sp.get_mut(space).ok_or("space missing")?;
        sm.edge_types.insert(
            name.into(),
            EdgeTypeDef {
                name: name.into(),
                fields,
                created_at: now_ms(),
            },
        );
        Ok(())
    }
    async fn alter_tag(
        &self,
        space: &str,
        name: &str,
        add_fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        let sm = sp.get_mut(space).ok_or("space missing")?;
        let t = sm.tags.get_mut(name).ok_or("tag missing")?;
        t.fields.extend(add_fields);
        Ok(())
    }
    async fn drop_tag(&self, space: &str, name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        let sm = sp.get_mut(space).ok_or("space missing")?;
        sm.tags.remove(name);
        Ok(())
    }
    async fn drop_edge_type(
        &self,
        space: &str,
        name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut sp = self.sp.lock();
        let sm = sp.get_mut(space).ok_or("space missing")?;
        sm.edge_types.remove(name);
        Ok(())
    }
    async fn show_hosts(&self) -> Result<Vec<HostInfo>, Box<dyn Error + Send + Sync>> {
        Ok(self.hs.lock().clone())
    }
    async fn list_tags(&self, space: &str) -> Result<Vec<TagDef>, Box<dyn Error + Send + Sync>> {
        let sp = self.sp.lock();
        let sm = sp.get(space).ok_or("space missing")?;
        Ok(sm.tags.values().cloned().collect())
    }
    async fn list_edge_types(
        &self,
        space: &str,
    ) -> Result<Vec<EdgeTypeDef>, Box<dyn Error + Send + Sync>> {
        let sp = self.sp.lock();
        let sm = sp.get(space).ok_or("space missing")?;
        Ok(sm.edge_types.values().cloned().collect())
    }
}
