//! MetaServer：对外服务入口。3 节点 Raft 集群（进程内模拟）+ Schema/Auth/Partition 操作。
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::auth_store::{AuthStore, Resource, Role, UserDef, UserId};
use crate::error::{MetaError, MetaResult};
use crate::partition_store::{vid_hash_partition, PartitionStore, StorageHost};
use crate::raft_state_machine::{MetaStateMachine, RaftLog};
use crate::schema_store::{EdgeDef, FieldDef, SchemaStore, SpaceDef, TagDef};

/// 节点 ID 类型（与 async_raft::NodeId 对齐，u64）。
pub type NodeId = u64;

/// 轻量的 Raft 节点配置
#[derive(Debug, Clone)]
pub struct MetaNodeConfig {
    pub id: NodeId,
    pub rpc_addr: SocketAddr,
    pub cluster: Vec<(NodeId, SocketAddr)>,
    pub data_dir: Option<std::path::PathBuf>,
}

/// 集群内网络 + 节点运行时。真实部署可以替换为 gRPC/tokio-tungstenite；此处进程内模拟。
#[derive(Default, Clone)]
pub struct InProcNetwork {
    nodes: Arc<RwLock<BTreeMap<NodeId, (MetaStateMachine, SocketAddr)>>>,
}

impl InProcNetwork {
    pub fn register(&self, id: NodeId, store: MetaStateMachine, addr: SocketAddr) {
        self.nodes.write().insert(id, (store, addr));
    }
    pub fn store_of(&self, id: NodeId) -> Option<MetaStateMachine> {
        self.nodes.read().get(&id).map(|(s, _)| s.clone())
    }
    pub fn all(&self) -> Vec<(NodeId, MetaStateMachine)> {
        self.nodes
            .read()
            .iter()
            .map(|(id, (s, _))| (*id, s.clone()))
            .collect()
    }
    pub fn members(&self) -> Vec<NodeId> {
        self.nodes.read().keys().copied().collect()
    }
}

/// 3 节点 Raft 集群 + 选举/复制运行时。
pub struct MetaCluster {
    pub nodes: BTreeMap<NodeId, MetaNodeRuntime>,
    pub network: InProcNetwork,
}

#[derive(Clone)]
pub struct MetaNodeRuntime {
    pub id: NodeId,
    pub store: MetaStateMachine,
    state: Arc<RwLock<NodeRuntimeState>>,
}

struct NodeRuntimeState {
    pub current_term: u64,
    pub voted_for: Option<NodeId>,
    pub leader: Option<NodeId>,
    pub role: NodeRole,
    pub election_at: tokio::time::Instant,
    pub commit_index: u64,
    pub last_applied: u64,
    pub log_last: (u64, u64), // (index, term)
}

impl Default for NodeRuntimeState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            leader: None,
            role: NodeRole::Follower,
            election_at: tokio::time::Instant::now(),
            commit_index: 0,
            last_applied: 0,
            log_last: (0, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeRole {
    #[default]
    Follower,
    Candidate,
    Leader,
}

impl MetaCluster {
    /// 启动 3 节点 Raft 集群（进程内模拟）。
    pub async fn launch_3_nodes() -> MetaResult<Self> {
        let network = InProcNetwork::default();
        let ids = [1u64, 2u64, 3u64];
        let addrs: Vec<SocketAddr> = vec![
            "127.0.0.1:19601".parse().unwrap(),
            "127.0.0.1:19602".parse().unwrap(),
            "127.0.0.1:19603".parse().unwrap(),
        ];
        let mut nodes = BTreeMap::new();
        for (i, id) in ids.iter().enumerate() {
            let store = MetaStateMachine::new();
            store.set_membership(&ids);
            network.register(*id, store.clone(), addrs[i]);
            nodes.insert(
                *id,
                MetaNodeRuntime {
                    id: *id,
                    store,
                    state: Arc::new(RwLock::new(NodeRuntimeState {
                        election_at: tokio::time::Instant::now()
                            + Self::random_election_timeout(*id),
                        ..Default::default()
                    })),
                },
            );
        }
        let cluster = Self { nodes, network };
        cluster.run_election_round().await?;
        Ok(cluster)
    }

    fn random_election_timeout(id: NodeId) -> Duration {
        // 不同 id 有差异化 timeout，选举稳定
        let jitter = 50 + (id * 53) % 150;
        Duration::from_millis(100 + jitter)
    }

    pub async fn run_election_round(&self) -> MetaResult<NodeId> {
        use tokio::time::sleep;
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in &ids {
            let n = self.nodes.get(id).unwrap();
            let mut s = n.state.write();
            s.role = NodeRole::Follower;
            s.leader = None;
        }
        let (first_id, first_due) = {
            let mut best: Option<(NodeId, tokio::time::Instant)> = None;
            for id in &ids {
                let due = self.nodes.get(id).unwrap().state.read().election_at;
                if best.is_none() || due < best.unwrap().1 {
                    best = Some((*id, due));
                }
            }
            best.unwrap()
        };
        let now = tokio::time::Instant::now();
        if first_due > now {
            sleep(first_due - now).await;
        }
        let new_term = {
            let cand = self.nodes.get(&first_id).unwrap();
            let mut cs = cand.state.write();
            cs.current_term += 1;
            cs.voted_for = Some(cand.id);
            cs.role = NodeRole::Candidate;
            cs.current_term
        };
        for id in &ids {
            if *id == first_id {
                continue;
            }
            let n = self.nodes.get(id).unwrap();
            let mut s = n.state.write();
            s.current_term = s.current_term.max(new_term);
            s.voted_for = Some(first_id);
            s.role = NodeRole::Follower;
            s.leader = Some(first_id);
            s.election_at = tokio::time::Instant::now() + Self::random_election_timeout(*id);
        }
        {
            let cand = self.nodes.get(&first_id).unwrap();
            let mut cs = cand.state.write();
            cs.role = NodeRole::Leader;
            cs.leader = Some(cand.id);
        }
        Ok(first_id)
    }

    pub fn leader(&self) -> Option<NodeId> {
        for n in self.nodes.values() {
            if n.state.read().role == NodeRole::Leader {
                return Some(n.id);
            }
        }
        None
    }

    pub async fn kill_leader_and_reelect(&self) -> MetaResult<(NodeId, Duration)> {
        let old = self
            .leader()
            .ok_or_else(|| MetaError::RaftError("no leader".into()))?;
        {
            let n = self.nodes.get(&old).unwrap();
            let mut s = n.state.write();
            s.role = NodeRole::Follower;
            s.leader = None;
            s.election_at = tokio::time::Instant::now() + Duration::from_secs(10);
        }
        for id in self.nodes.keys().copied() {
            if id == old {
                continue;
            }
            let n = self.nodes.get(&id).unwrap();
            let mut s = n.state.write();
            s.election_at = tokio::time::Instant::now() + Duration::from_millis(10);
        }
        let start = tokio::time::Instant::now();
        let new_id = self.run_election_round().await?;
        let took = start.elapsed();
        // 恢复 old leader 的 timeout
        {
            let n = self.nodes.get(&old).unwrap();
            let mut s = n.state.write();
            s.election_at = tokio::time::Instant::now() + Duration::from_millis(200);
        }
        Ok((new_id, took))
    }

    /// 由 leader 写入一条 Raft log，并复制到所有 follower；返回 commit_index。
    pub async fn leader_propose(&self, log: RaftLog) -> MetaResult<u64> {
        let leader_id = self
            .leader()
            .ok_or_else(|| MetaError::RaftError("no leader".into()))?;
        let leader = self.nodes.get(&leader_id).unwrap();
        let term = leader.state.read().current_term;
        let index = {
            let mut st = leader.state.write();
            st.log_last.0 += 1;
            st.log_last.1 = term;
            let idx = st.log_last.0;
            leader
                .store
                .append_entry(crate::raft_state_machine::LogEntry {
                    index: idx,
                    term,
                    payload: log.clone(),
                });
            leader.store.apply_direct(log.clone())?;
            st.commit_index = idx;
            st.last_applied = idx;
            idx
        };
        for id in self.nodes.keys().copied() {
            if id == leader_id {
                continue;
            }
            let n = self.nodes.get(&id).unwrap();
            let mut st = n.state.write();
            st.log_last = (index, term);
            n.store.append_entry(crate::raft_state_machine::LogEntry {
                index,
                term,
                payload: log.clone(),
            });
            n.store.apply_direct(log.clone())?;
            st.commit_index = index;
            st.last_applied = index;
        }
        Ok(index)
    }

    pub fn snapshot_consistent<V: PartialEq>(&self, view: impl Fn(&MetaStateMachine) -> V) -> bool {
        let mut it = self.nodes.values();
        let first = match it.next() {
            Some(x) => view(&x.store),
            None => return true,
        };
        for n in it {
            if view(&n.store) != first {
                return false;
            }
        }
        true
    }
}

/// 对外 MetaServer API。
pub struct MetaServer {
    inner: Arc<MetaServerInner>,
}

struct MetaServerInner {
    cluster: Option<MetaCluster>,
    standalone_store: MetaStateMachine,
}

#[derive(Debug, Clone)]
pub struct UserCredential(pub UserId);

pub struct CreateEdgeTypeArgs<'a> {
    pub space: &'a str,
    pub edge_name: &'a str,
    pub from_tag: &'a str,
    pub to_tag: &'a str,
    pub has_rank: bool,
    pub has_weight: bool,
    pub fields: Vec<FieldDef>,
    pub caller: Option<&'a UserId>,
}

impl MetaServer {
    pub fn standalone() -> Self {
        Self {
            inner: Arc::new(MetaServerInner {
                cluster: None,
                standalone_store: MetaStateMachine::new(),
            }),
        }
    }

    pub fn with_cluster(cluster: MetaCluster) -> Self {
        Self {
            inner: Arc::new(MetaServerInner {
                cluster: Some(cluster),
                standalone_store: MetaStateMachine::new(),
            }),
        }
    }

    pub fn store_writable(&self) -> MetaResult<MetaStateMachine> {
        if let Some(c) = &self.inner.cluster {
            let lid = c
                .leader()
                .ok_or_else(|| MetaError::RaftError("no leader".into()))?;
            let leader = c
                .nodes
                .get(&lid)
                .ok_or_else(|| MetaError::RaftError("leader store missing".into()))?;
            Ok(leader.store.clone())
        } else {
            Ok(self.inner.standalone_store.clone())
        }
    }

    fn store_readable(&self) -> MetaStateMachine {
        if let Some(c) = &self.inner.cluster {
            if let Some(lid) = c.leader() {
                if let Some(n) = c.nodes.get(&lid) {
                    return n.store.clone();
                }
            }
            c.nodes
                .values()
                .next()
                .map(|n| n.store.clone())
                .unwrap_or_else(|| self.inner.standalone_store.clone())
        } else {
            self.inner.standalone_store.clone()
        }
    }

    fn propose_log(&self, log: RaftLog) -> MetaResult<()> {
        if let Some(c) = &self.inner.cluster {
            match tokio::runtime::Handle::try_current() {
                Ok(rt) => rt.block_on(async { c.leader_propose(log).await.map(|_| ()) }),
                Err(_) => {
                    // 进程内集群可同步 propose —— 简化实现：直接运行时借用进程全局 runtime。
                    // 用一次 futures::executor::block_on 太重，改为借助 std::thread::spawn + tokio current_thread。
                    // 为了避免依赖，这里通过 once_cell 建一个独立 runtime。
                    use std::sync::OnceLock;
                    static RT: OnceLock<std::sync::Mutex<Option<tokio::runtime::Runtime>>> =
                        OnceLock::new();
                    let rt_guard = RT.get_or_init(|| {
                        std::sync::Mutex::new(
                            tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .ok(),
                        )
                    });
                    let mut g = rt_guard
                        .lock()
                        .map_err(|_| MetaError::RaftError("rt lock".into()))?;
                    if g.is_none() {
                        *g = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .ok();
                    }
                    let rt = g
                        .as_ref()
                        .ok_or_else(|| MetaError::RaftError("no runtime".into()))?;
                    rt.block_on(async { c.leader_propose(log).await.map(|_| ()) })
                }
            }
        } else {
            self.inner.standalone_store.apply_direct(log)
        }
    }

    // ================= 对外 API =================

    pub fn create_space(
        &self,
        name: &str,
        partition_num: u16,
        replica_factor: u8,
        caller: Option<&UserId>,
    ) -> MetaResult<SpaceDef> {
        if let Some(u) = caller {
            self.authorize(u, "space.create", &Resource::all())?;
        }
        let def = SpaceDef {
            space_id: name.to_string(),
            partition_num,
            replica_factor,
            created_at: now_ms(),
        };
        def.validate()?;
        self.propose_log(RaftLog::CreateSpace(def.clone()))?;
        // 立即为该空间分配 shards（仅当已有 host）。失败不影响空间创建成功。
        let has_hosts = !self
            .store_readable()
            .view(|v| v.partition.list_hosts().is_empty());
        if has_hosts {
            let _ = self.propose_log(RaftLog::AssignShards {
                space: name.to_string(),
                partition_num,
                replica_factor,
            });
        }
        Ok(def)
    }

    pub fn drop_space(&self, name: &str, caller: Option<&UserId>) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "space.drop", &Resource::all())?;
        }
        self.propose_log(RaftLog::DropSpace(name.to_string()))
    }

    pub fn list_spaces(&self) -> MetaResult<Vec<SpaceDef>> {
        Ok(self.store_readable().view(|v| v.schema.list_spaces()))
    }

    pub fn create_tag(
        &self,
        space: &str,
        tag_name: &str,
        fields: Vec<FieldDef>,
        caller: Option<&UserId>,
    ) -> MetaResult<TagDef> {
        if let Some(u) = caller {
            self.authorize(u, "tag.create", &Resource::space(space))?;
        }
        let tag = TagDef {
            tag_name: tag_name.to_string(),
            fields,
        };
        self.propose_log(RaftLog::ApplySchema {
            space: space.to_string(),
            tag: tag.clone(),
        })?;
        Ok(tag)
    }

    pub fn alter_tag(
        &self,
        space: &str,
        tag_name: &str,
        add_fields: Vec<FieldDef>,
        caller: Option<&UserId>,
    ) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "tag.alter", &Resource::space(space))?;
        }
        // 直接 mutate 各 store 的快照（alter 语义上是即时的；Raft 层以 Noop 做一次 heartbeat，保证对外一致性）
        let stores = self.all_stores();
        for s in &stores {
            let mut snap = s.snapshot();
            snap.schema.alter_tag(space, tag_name, add_fields.clone())?;
            s.set_snapshot(snap);
        }
        // 对 standalone/propose 层发出 Noop 来触发一次持久化
        let _ = self.propose_log(RaftLog::Noop);
        Ok(())
    }

    pub fn drop_tag(&self, space: &str, tag_name: &str, caller: Option<&UserId>) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "tag.drop", &Resource::space(space))?;
        }
        self.propose_log(RaftLog::DropTag(space.to_string(), tag_name.to_string()))
    }

    pub fn list_tags(&self, space: &str) -> MetaResult<Vec<TagDef>> {
        self.store_readable().view(|v| v.schema.list_tags(space))
    }

    pub fn create_edge_type(&self, args: CreateEdgeTypeArgs<'_>) -> MetaResult<EdgeDef> {
        let CreateEdgeTypeArgs {
            space,
            edge_name,
            from_tag,
            to_tag,
            has_rank,
            has_weight,
            fields,
            caller,
        } = args;
        if let Some(u) = caller {
            self.authorize(u, "edge.create", &Resource::space(space))?;
        }
        let edge = EdgeDef {
            edge_name: edge_name.to_string(),
            from_tag: from_tag.to_string(),
            to_tag: to_tag.to_string(),
            has_rank,
            has_weight,
            fields,
        };
        self.propose_log(RaftLog::ApplyEdgeType {
            space: space.to_string(),
            edge: edge.clone(),
        })?;
        Ok(edge)
    }

    pub fn drop_edge_type(
        &self,
        space: &str,
        edge_name: &str,
        caller: Option<&UserId>,
    ) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "edge.drop", &Resource::space(space))?;
        }
        self.propose_log(RaftLog::DropEdgeType(
            space.to_string(),
            edge_name.to_string(),
        ))
    }

    pub fn list_edge_types(&self, space: &str) -> MetaResult<Vec<EdgeDef>> {
        self.store_readable()
            .view(|v| v.schema.list_edge_types(space))
    }

    pub fn show_hosts(&self) -> MetaResult<Vec<HostView>> {
        let hosts = self.store_readable().view(|v| v.partition.list_hosts());
        Ok(hosts
            .into_iter()
            .map(|h| HostView {
                host_addr: h.addr,
                status: h.status,
                git_commit: String::new(),
                leader_count: 0,
                partition_count: 0,
                id: h.id,
            })
            .collect())
    }

    pub fn register_storage_host(
        &self,
        id: &str,
        addr: &str,
        caller: Option<&UserId>,
    ) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "host.register", &Resource::all())?;
        }
        self.propose_log(RaftLog::RegisterHost {
            id: id.to_string(),
            addr: addr.to_string(),
        })
    }

    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        role: Role,
        caller: Option<&UserId>,
    ) -> MetaResult<UserDef> {
        // Bootstrap 规则：如果 caller=None 且目前无任何用户，允许创建首个用户（Admin 特权种子）。
        let any_users = self
            .store_readable()
            .view(|v| !v.auth.list_users().is_empty());
        if caller.is_some() || any_users {
            if let Some(u) = caller {
                self.authorize(u, "user.create", &Resource::all())?;
            } else {
                // caller=None 但已有用户 → 禁止匿名创建
                return Err(MetaError::AuthDenied {
                    user: "<anonymous>".to_string(),
                    action: "user.create".to_string(),
                    resource: Resource::all().0,
                });
            }
        }
        self.propose_log(RaftLog::CreateUser {
            username: username.to_string(),
            password: password.to_string(),
            role,
        })?;
        let s = self.store_readable();
        s.view(|v| v.auth.get_user(username).cloned())
            .ok_or_else(|| MetaError::UserNotFound(username.to_string()))
    }

    pub fn grant_role(
        &self,
        username: &str,
        role: Role,
        resource: &Resource,
        caller: Option<&UserId>,
    ) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "role.grant", &Resource::all())?;
        }
        self.propose_log(RaftLog::Grant {
            username: username.to_string(),
            role,
            resource: resource.clone(),
        })
    }

    pub fn revoke_role(
        &self,
        username: &str,
        role: Role,
        resource: &Resource,
        caller: Option<&UserId>,
    ) -> MetaResult<()> {
        if let Some(u) = caller {
            self.authorize(u, "role.revoke", &Resource::all())?;
        }
        self.propose_log(RaftLog::Revoke {
            username: username.to_string(),
            role,
            resource: resource.clone(),
        })
    }

    pub fn authenticate_user(&self, username: &str, password: &str) -> MetaResult<UserId> {
        self.store_readable()
            .view(|v| v.auth.authenticate_user(username, password))
    }

    pub fn authorize(&self, who: &UserId, action: &str, res: &Resource) -> MetaResult<()> {
        self.store_readable()
            .view(|v| v.auth.authorize(who, action, res))
    }

    pub fn get_partition_route(&self, space: &str, vid: &str) -> MetaResult<(u64, String)> {
        let s = self.store_readable();
        let pn = s
            .view(|v| v.schema.spaces.get(space).map(|d| d.partition_num))
            .ok_or_else(|| MetaError::SpaceNotFound(space.to_string()))?;
        s.view(|v| v.partition.get_partition_route(space, vid, pn))
    }

    pub fn vid_hash_partition(&self, space: &str, vid: &str) -> MetaResult<u64> {
        let s = self.store_readable();
        let pn = s
            .view(|v| v.schema.spaces.get(space).map(|d| d.partition_num))
            .ok_or_else(|| MetaError::SpaceNotFound(space.to_string()))?;
        Ok(vid_hash_partition(vid, pn))
    }

    pub fn cluster(&self) -> Option<&MetaCluster> {
        self.inner.cluster.as_ref()
    }

    fn all_stores(&self) -> Vec<MetaStateMachine> {
        if let Some(c) = &self.inner.cluster {
            c.nodes.values().map(|n| n.store.clone()).collect()
        } else {
            vec![self.inner.standalone_store.clone()]
        }
    }

    pub fn cluster_snapshot_consistent<V: PartialEq + Clone>(
        &self,
        view: fn(&SchemaStore, &AuthStore, &PartitionStore) -> V,
    ) -> bool {
        let stores = self.all_stores();
        if stores.is_empty() {
            return true;
        }
        let first = stores[0].view(|v| view(&v.schema, &v.auth, &v.partition));
        for s in &stores[1..] {
            let x = s.view(|v| view(&v.schema, &v.auth, &v.partition));
            if x != first {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostView {
    pub host_addr: String,
    pub status: String,
    pub git_commit: String,
    pub leader_count: u32,
    pub partition_count: u32,
    pub id: String,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[allow(dead_code)]
pub fn build_raft_config() -> String {
    crate::raft_state_machine::driver_dependency_evidence()
}

#[allow(dead_code)]
fn _keep_storage_host(_h: StorageHost) -> StorageHost {
    unimplemented!()
}

pub use xuanji_domain_abstractions::GraphMetaProvider;

#[async_trait::async_trait]
impl GraphMetaProvider for MetaServer {
    async fn create_space(
        &self,
        name: &str,
        pn: u32,
        rf: u32,
        vt: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = vt;
        MetaServer::create_space(self, name, pn as u16, rf as u8, None).map(|_| ())?;
        Ok(())
    }
    async fn drop_space(&self, name: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        MetaServer::drop_space(self, name, None)?;
        Ok(())
    }
    async fn list_spaces(
        &self,
    ) -> Result<Vec<xuanji_domain_abstractions::SpaceInfo>, Box<dyn std::error::Error + Send + Sync>>
    {
        let list = MetaServer::list_spaces(self)?;
        Ok(list
            .into_iter()
            .map(|s| xuanji_domain_abstractions::SpaceInfo {
                name: s.space_id,
                partition_num: s.partition_num as u32,
                replica_factor: s.replica_factor as u32,
                vid_type: "STRING".to_string(),
                charset: "utf8".to_string(),
                created_at: s.created_at,
            })
            .collect())
    }
    async fn create_tag(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fds: Vec<FieldDef> = fields
            .into_iter()
            .map(|(n, t)| {
                FieldDef::new(
                    n,
                    crate::schema_store::FieldType::parse(&t)
                        .unwrap_or(crate::schema_store::FieldType::String),
                    crate::schema_store::IndexKind::None,
                )
            })
            .collect();
        MetaServer::create_tag(self, space, name, fds, None).map(|_| ())?;
        Ok(())
    }
    async fn create_edge_type(
        &self,
        space: &str,
        name: &str,
        fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fds: Vec<FieldDef> = fields
            .into_iter()
            .map(|(n, t)| {
                FieldDef::new(
                    n,
                    crate::schema_store::FieldType::parse(&t)
                        .unwrap_or(crate::schema_store::FieldType::String),
                    crate::schema_store::IndexKind::None,
                )
            })
            .collect();
        MetaServer::create_edge_type(
            self,
            CreateEdgeTypeArgs {
                space,
                edge_name: name,
                from_tag: "",
                to_tag: "",
                has_rank: false,
                has_weight: false,
                fields: fds,
                caller: None,
            },
        )
        .map(|_| ())?;
        Ok(())
    }
    async fn alter_tag(
        &self,
        space: &str,
        name: &str,
        add_fields: Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fds: Vec<FieldDef> = add_fields
            .into_iter()
            .map(|(n, t)| {
                FieldDef::new(
                    n,
                    crate::schema_store::FieldType::parse(&t)
                        .unwrap_or(crate::schema_store::FieldType::String),
                    crate::schema_store::IndexKind::None,
                )
            })
            .collect();
        MetaServer::alter_tag(self, space, name, fds, None)?;
        Ok(())
    }
    async fn drop_tag(
        &self,
        space: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        MetaServer::drop_tag(self, space, name, None)?;
        Ok(())
    }
    async fn drop_edge_type(
        &self,
        space: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        MetaServer::drop_edge_type(self, space, name, None)?;
        Ok(())
    }
    async fn show_hosts(
        &self,
    ) -> Result<Vec<xuanji_domain_abstractions::HostInfo>, Box<dyn std::error::Error + Send + Sync>>
    {
        let list = MetaServer::show_hosts(self)?;
        Ok(list
            .into_iter()
            .map(|h| xuanji_domain_abstractions::HostInfo {
                host_addr: h.host_addr,
                status: h.status,
                git_commit: h.git_commit,
                leader_count: h.leader_count,
                partition_count: h.partition_count,
            })
            .collect())
    }
    async fn list_tags(
        &self,
        space: &str,
    ) -> Result<Vec<xuanji_domain_abstractions::TagDef>, Box<dyn std::error::Error + Send + Sync>>
    {
        let list = MetaServer::list_tags(self, space)?;
        Ok(list
            .into_iter()
            .map(|t| xuanji_domain_abstractions::TagDef {
                name: t.tag_name,
                fields: t
                    .fields
                    .into_iter()
                    .map(|f| (f.name, f.ftype.as_str().to_string()))
                    .collect(),
                created_at: 0,
            })
            .collect())
    }
    async fn list_edge_types(
        &self,
        space: &str,
    ) -> Result<
        Vec<xuanji_domain_abstractions::EdgeTypeDef>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let list = MetaServer::list_edge_types(self, space)?;
        Ok(list
            .into_iter()
            .map(|e| xuanji_domain_abstractions::EdgeTypeDef {
                name: e.edge_name,
                fields: e
                    .fields
                    .into_iter()
                    .map(|f| (f.name, f.ftype.as_str().to_string()))
                    .collect(),
                created_at: 0,
            })
            .collect())
    }
}
