use async_trait::async_trait;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphQueryError {
    NotFound(String),
    InvalidQuery(String),
    ExecutionFailed(String),
    Timeout,
    Internal(String),
}
impl fmt::Display for GraphQueryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GraphQueryError::NotFound(s) => write!(f, "NF {s}"),
            GraphQueryError::InvalidQuery(s) => write!(f, "IQ {s}"),
            GraphQueryError::ExecutionFailed(s) => write!(f, "EF {s}"),
            GraphQueryError::Timeout => write!(f, "TO"),
            GraphQueryError::Internal(s) => write!(f, "IN {s}"),
        }
    }
}
impl Error for GraphQueryError {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vertex {
    pub vid: String,
    pub tags: Vec<String>,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Edge {
    pub src_vid: String,
    pub dst_vid: String,
    pub edge_type: String,
    pub rank: i64,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subgraph {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryResultSet {
    pub column_names: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlgoSingleResult {
    pub scores: Vec<(String, f64)>,
    pub extra_json: String,
}

#[async_trait]
pub trait GraphQueryProvider: Send + Sync {
    async fn get_vertex(
        &self,
        space: &str,
        vid: &str,
        tags: &[String],
    ) -> Result<Option<Vertex>, Box<dyn Error + Send + Sync>>;
    async fn get_edge(
        &self,
        space: &str,
        src: &str,
        dst: &str,
        et: &str,
        rank: i64,
    ) -> Result<Option<Edge>, Box<dyn Error + Send + Sync>>;
    async fn get_neighbors(
        &self,
        space: &str,
        vid: &str,
        dir: &str,
        ets: &[String],
    ) -> Result<Vec<(Edge, Vertex)>, Box<dyn Error + Send + Sync>>;
    async fn k_hop_neighbors(
        &self,
        space: &str,
        vid: &str,
        k: u32,
        dir: &str,
        ets: &[String],
    ) -> Result<Vec<Vertex>, Box<dyn Error + Send + Sync>>;
    async fn subgraph_by_vids(
        &self,
        space: &str,
        vids: &[String],
        step: u32,
    ) -> Result<Subgraph, Box<dyn Error + Send + Sync>>;
    async fn execute_ngql(
        &self,
        space: &str,
        ngql: &str,
    ) -> Result<QueryResultSet, Box<dyn Error + Send + Sync>>;
    async fn execute_cypher(
        &self,
        space: &str,
        cypher: &str,
    ) -> Result<QueryResultSet, Box<dyn Error + Send + Sync>>;
    async fn run_single_algo(
        &self,
        space: &str,
        name: &str,
        params: BTreeMap<String, String>,
    ) -> Result<AlgoSingleResult, Box<dyn Error + Send + Sync>>;
}

/// space → BTreeMap<(src, dst, edge_type, rank), Edge>
type SpaceEdgeMap = BTreeMap<String, BTreeMap<(String, String, String, i64), Edge>>;

pub struct MockGraphQueryProvider {
    v: parking_lot::Mutex<BTreeMap<String, BTreeMap<String, Vertex>>>,
    e: parking_lot::Mutex<SpaceEdgeMap>,
}
impl Default for MockGraphQueryProvider {
    fn default() -> Self {
        Self {
            v: parking_lot::Mutex::new(BTreeMap::new()),
            e: parking_lot::Mutex::new(BTreeMap::new()),
        }
    }
}

#[async_trait]
impl GraphQueryProvider for MockGraphQueryProvider {
    async fn get_vertex(
        &self,
        space: &str,
        vid: &str,
        tags: &[String],
    ) -> Result<Option<Vertex>, Box<dyn Error + Send + Sync>> {
        let vs = self.v.lock();
        let Some(space_map) = vs.get(space) else {
            return Ok(None);
        };
        let Some(v) = space_map.get(vid) else {
            return Ok(None);
        };
        if tags.is_empty() {
            return Ok(Some(v.clone()));
        }
        let filtered: Vec<String> = v
            .tags
            .iter()
            .filter(|t| tags.contains(t))
            .cloned()
            .collect();
        Ok(Some(Vertex {
            vid: v.vid.clone(),
            tags: filtered,
            properties: v.properties.clone(),
        }))
    }
    async fn get_edge(
        &self,
        space: &str,
        src: &str,
        dst: &str,
        et: &str,
        rank: i64,
    ) -> Result<Option<Edge>, Box<dyn Error + Send + Sync>> {
        let es = self.e.lock();
        let Some(space_map) = es.get(space) else {
            return Ok(None);
        };
        Ok(space_map
            .get(&(src.into(), dst.into(), et.into(), rank))
            .cloned())
    }
    async fn get_neighbors(
        &self,
        space: &str,
        vid: &str,
        _dir: &str,
        _ets: &[String],
    ) -> Result<Vec<(Edge, Vertex)>, Box<dyn Error + Send + Sync>> {
        // Mock: empty unless inserted (we don't have insert API in this trait)
        let vs = self.v.lock();
        let es = self.e.lock();
        let space_es = es.get(space);
        let space_vs = vs.get(space);
        let mut out = vec![];
        if let (Some(se), Some(sv)) = (space_es, space_vs) {
            for ((s, d, _, _), e) in se {
                if s == vid {
                    if let Some(dst_v) = sv.get(d) {
                        out.push((e.clone(), dst_v.clone()));
                    }
                }
            }
        }
        Ok(out)
    }
    async fn k_hop_neighbors(
        &self,
        space: &str,
        vid: &str,
        k: u32,
        dir: &str,
        ets: &[String],
    ) -> Result<Vec<Vertex>, Box<dyn Error + Send + Sync>> {
        let mut result = vec![];
        let mut frontier = vec![vid.to_string()];
        for _ in 0..k {
            let mut next = Vec::new();
            for v in frontier.drain(..) {
                let nb = self.get_neighbors(space, &v, dir, ets).await?;
                for (_, dst) in nb {
                    if !result.iter().any(|x: &Vertex| x.vid == dst.vid) {
                        next.push(dst.vid.clone());
                        result.push(dst);
                    }
                }
            }
            frontier = next;
        }
        Ok(result)
    }
    async fn subgraph_by_vids(
        &self,
        space: &str,
        vids: &[String],
        step: u32,
    ) -> Result<Subgraph, Box<dyn Error + Send + Sync>> {
        let mut vertices = Vec::new();
        let mut seen = BTreeMap::new();
        for v in vids {
            if let Some(vtx) = self.get_vertex(space, v, &[]).await? {
                seen.insert(vtx.vid.clone(), ());
                vertices.push(vtx);
            }
        }
        if step > 1 {
            for v in vids {
                let n = self.k_hop_neighbors(space, v, step - 1, "out", &[]).await?;
                for nv in n {
                    if !seen.contains_key(&nv.vid) {
                        seen.insert(nv.vid.clone(), ());
                        vertices.push(nv);
                    }
                }
            }
        }
        Ok(Subgraph {
            vertices,
            edges: vec![],
        })
    }
    async fn execute_ngql(
        &self,
        _space: &str,
        _ngql: &str,
    ) -> Result<QueryResultSet, Box<dyn Error + Send + Sync>> {
        Ok(QueryResultSet::default())
    }
    async fn execute_cypher(
        &self,
        _space: &str,
        _cypher: &str,
    ) -> Result<QueryResultSet, Box<dyn Error + Send + Sync>> {
        Ok(QueryResultSet::default())
    }
    async fn run_single_algo(
        &self,
        _space: &str,
        _name: &str,
        _params: BTreeMap<String, String>,
    ) -> Result<AlgoSingleResult, Box<dyn Error + Send + Sync>> {
        Ok(AlgoSingleResult::default())
    }
}
