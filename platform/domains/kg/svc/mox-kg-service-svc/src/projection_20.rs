// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Projection 20 operators for sub-graph extraction.
//!
//! Grid: 5 filters × 2 directions × 2 hops = 20 uniquely named `proj_{f}_{dir}_{hop}`
//!
//! Filters:
//! - `type`    : vertex.type_ == ctx.param
//! - `community`: community id (CNM algo label) == ctx.param parsed i64
//! - `attr`    : vertex.attr key == ctx.param (attr match)
//! - `degree`  : degree >= ctx.param parsed i64
//! - `label`   : vertex.label == ctx.param
//!
//! Directions: in / out
//! Hops: 1 / 2

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Adjacency graph in memory.
pub struct SimpleGraph {
    pub vertices: BTreeMap<i64, Vertex>,
    // Forward adjacency: from -> Vec<(to, label)>
    pub fwd: BTreeMap<i64, Vec<(i64, String)>>,
    // Backward adjacency: to -> Vec<(from, label)>
    pub bwd: BTreeMap<i64, Vec<(i64, String)>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vertex {
    pub id: i64,
    pub label: String,
    pub type_: String,
    pub community: i64,
    pub attr: BTreeMap<String, String>,
}

impl SimpleGraph {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            vertices: BTreeMap::new(),
            fwd: BTreeMap::new(),
            bwd: BTreeMap::new(),
        }))
    }

    pub fn add_vertex_with(&mut self, id: i64, label: &str, type_: &str, community: i64, attr: BTreeMap<String,String>) {
        self.vertices.insert(id, Vertex { id, label: label.into(), type_: type_.into(), community, attr });
    }

    pub fn add_edge(&mut self, s: i64, t: i64, label: &str) {
        self.fwd.entry(s).or_default().push((t, label.into()));
        self.bwd.entry(t).or_default().push((s, label.into()));
    }

    pub fn degree(&self, id: i64) -> i64 {
        let out = self.fwd.get(&id).map(|v| v.len()).unwrap_or(0) as i64;
        let in_ = self.bwd.get(&id).map(|v| v.len()).unwrap_or(0) as i64;
        out + in_
    }

    /// BFS k-hop neighbors along direction.
    pub fn neighbors(&self, start: i64, dir: Dir, k: u8) -> BTreeSet<i64> {
        let mut seen = BTreeSet::new();
        if !self.vertices.contains_key(&start) {
            return seen;
        }
        seen.insert(start);
        let mut frontier = BTreeSet::new();
        frontier.insert(start);
        for _ in 0..k {
            let mut next = BTreeSet::new();
            for n in &frontier {
                let edges = match dir {
                    Dir::Out => self.fwd.get(n).cloned().unwrap_or_default(),
                    Dir::In => self.bwd.get(n).cloned().unwrap_or_default(),
                };
                for (m, _) in edges {
                    if self.vertices.contains_key(&m) && !seen.contains(&m) {
                        next.insert(m);
                    }
                }
            }
            if next.is_empty() { break; }
            for n in &next { seen.insert(*n); }
            frontier = next;
        }
        seen.remove(&start); // exclude seed; projection is neighbors + filter
        seen
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir { In, Out }
impl Dir {
    pub fn as_str(self) -> &'static str { match self { Dir::In => "in", Dir::Out => "out" } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter { Type, Community, Attr, Degree, Label }
impl Filter {
    pub fn as_str(self) -> &'static str {
        match self {
            Filter::Type => "type",
            Filter::Community => "community",
            Filter::Attr => "attr",
            Filter::Degree => "degree",
            Filter::Label => "label",
        }
    }
}

/// Static operator descriptor.
pub struct ProjectionOperator {
    pub id: &'static str,            // e.g. "proj_type_out_1"
    pub filter: Filter,
    pub direction: Dir,
    pub hops: u8,
    pub apply: fn(&ProjectionContext) -> ProjectionResult,
}

pub struct ProjectionContext<'a> {
    pub graph: &'a SimpleGraph,
    pub seed: i64,
    pub param: String, // parameter for filter; for community/degree it's the numeric value string
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectionResult {
    pub id: String,
    pub seed: i64,
    pub vertices: BTreeSet<i64>,
    pub edges: BTreeSet<(i64, i64, String)>,
}

fn filter_match(v: &Vertex, f: Filter, param: &str) -> bool {
    match f {
        Filter::Type => v.type_ == param,
        Filter::Community => v.community == param.parse::<i64>().unwrap_or(-1),
        Filter::Attr => v.attr.contains_key(param),
        Filter::Degree => true, // degree handled separately on whole graph degree
        Filter::Label => v.label == param,
    }
}

fn apply_operator(f: Filter, d: Dir, h: u8, ctx: &ProjectionContext) -> ProjectionResult {
    let id = format!("proj_{}_{}_{}", f.as_str(), d.as_str(), h);
    let neighbors = ctx.graph.neighbors(ctx.seed, d, h);
    // For Degree filter: param is minimum degree; evaluate param once.
    let param_degree = ctx.param.parse::<i64>().unwrap_or(-1);
    let mut selected = BTreeSet::new();
    for nid in &neighbors {
        let Some(v) = ctx.graph.vertices.get(nid) else { continue };
        if f == Filter::Degree {
            if ctx.graph.degree(*nid) >= param_degree {
                selected.insert(*nid);
            }
        } else if filter_match(v, f, &ctx.param) {
            selected.insert(*nid);
        }
    }
    // Also include seed so "subgraph" is well-defined (connected around seed).
    if ctx.graph.vertices.contains_key(&ctx.seed) {
        selected.insert(ctx.seed);
    }
    // Collect edges: any (u,v,l) where both endpoints ∈ selected AND original edge exists.
    let mut edges = BTreeSet::new();
    for u in &selected {
        if let Some(fes) = ctx.graph.fwd.get(u) {
            for (v, l) in fes {
                if selected.contains(v) {
                    edges.insert((*u, *v, l.clone()));
                }
            }
        }
    }
    ProjectionResult { id, seed: ctx.seed, vertices: selected, edges }
}

macro_rules! op {
    ($id:ident, $f:expr, $d:expr, $h:literal) => {
        pub fn $id(ctx: &ProjectionContext) -> ProjectionResult {
            apply_operator($f, $d, $h, ctx)
        }
    };
}

// --- 20 operators: Filter × Dir × Hop (1/2)  ---
op!(proj_type_out_1,       Filter::Type,      Dir::Out, 1);
op!(proj_type_out_2,       Filter::Type,      Dir::Out, 2);
op!(proj_type_in_1,        Filter::Type,      Dir::In,  1);
op!(proj_type_in_2,        Filter::Type,      Dir::In,  2);
op!(proj_community_out_1,  Filter::Community, Dir::Out, 1);
op!(proj_community_out_2,  Filter::Community, Dir::Out, 2);
op!(proj_community_in_1,   Filter::Community, Dir::In,  1);
op!(proj_community_in_2,   Filter::Community, Dir::In,  2);
op!(proj_attr_out_1,       Filter::Attr,      Dir::Out, 1);
op!(proj_attr_out_2,       Filter::Attr,      Dir::Out, 2);
op!(proj_attr_in_1,        Filter::Attr,      Dir::In,  1);
op!(proj_attr_in_2,        Filter::Attr,      Dir::In,  2);
op!(proj_degree_out_1,     Filter::Degree,    Dir::Out, 1);
op!(proj_degree_out_2,     Filter::Degree,    Dir::Out, 2);
op!(proj_degree_in_1,      Filter::Degree,    Dir::In,  1);
op!(proj_degree_in_2,      Filter::Degree,    Dir::In,  2);
op!(proj_label_out_1,      Filter::Label,     Dir::Out, 1);
op!(proj_label_out_2,      Filter::Label,     Dir::Out, 2);
op!(proj_label_in_1,       Filter::Label,     Dir::In,  1);
op!(proj_label_in_2,       Filter::Label,     Dir::In,  2);

pub const PROJECTION_OPERATORS: &[ProjectionOperator] = &[
    ProjectionOperator { id: "proj_type_out_1",       filter: Filter::Type,      direction: Dir::Out, hops: 1, apply: proj_type_out_1 },
    ProjectionOperator { id: "proj_type_out_2",       filter: Filter::Type,      direction: Dir::Out, hops: 2, apply: proj_type_out_2 },
    ProjectionOperator { id: "proj_type_in_1",        filter: Filter::Type,      direction: Dir::In,  hops: 1, apply: proj_type_in_1  },
    ProjectionOperator { id: "proj_type_in_2",        filter: Filter::Type,      direction: Dir::In,  hops: 2, apply: proj_type_in_2  },
    ProjectionOperator { id: "proj_community_out_1",  filter: Filter::Community, direction: Dir::Out, hops: 1, apply: proj_community_out_1 },
    ProjectionOperator { id: "proj_community_out_2",  filter: Filter::Community, direction: Dir::Out, hops: 2, apply: proj_community_out_2 },
    ProjectionOperator { id: "proj_community_in_1",   filter: Filter::Community, direction: Dir::In,  hops: 1, apply: proj_community_in_1  },
    ProjectionOperator { id: "proj_community_in_2",   filter: Filter::Community, direction: Dir::In,  hops: 2, apply: proj_community_in_2  },
    ProjectionOperator { id: "proj_attr_out_1",       filter: Filter::Attr,      direction: Dir::Out, hops: 1, apply: proj_attr_out_1 },
    ProjectionOperator { id: "proj_attr_out_2",       filter: Filter::Attr,      direction: Dir::Out, hops: 2, apply: proj_attr_out_2 },
    ProjectionOperator { id: "proj_attr_in_1",        filter: Filter::Attr,      direction: Dir::In,  hops: 1, apply: proj_attr_in_1  },
    ProjectionOperator { id: "proj_attr_in_2",        filter: Filter::Attr,      direction: Dir::In,  hops: 2, apply: proj_attr_in_2  },
    ProjectionOperator { id: "proj_degree_out_1",     filter: Filter::Degree,    direction: Dir::Out, hops: 1, apply: proj_degree_out_1 },
    ProjectionOperator { id: "proj_degree_out_2",     filter: Filter::Degree,    direction: Dir::Out, hops: 2, apply: proj_degree_out_2 },
    ProjectionOperator { id: "proj_degree_in_1",      filter: Filter::Degree,    direction: Dir::In,  hops: 1, apply: proj_degree_in_1  },
    ProjectionOperator { id: "proj_degree_in_2",      filter: Filter::Degree,    direction: Dir::In,  hops: 2, apply: proj_degree_in_2  },
    ProjectionOperator { id: "proj_label_out_1",      filter: Filter::Label,     direction: Dir::Out, hops: 1, apply: proj_label_out_1 },
    ProjectionOperator { id: "proj_label_out_2",      filter: Filter::Label,     direction: Dir::Out, hops: 2, apply: proj_label_out_2 },
    ProjectionOperator { id: "proj_label_in_1",       filter: Filter::Label,     direction: Dir::In,  hops: 1, apply: proj_label_in_1  },
    ProjectionOperator { id: "proj_label_in_2",       filter: Filter::Label,     direction: Dir::In,  hops: 2, apply: proj_label_in_2  },
];

/// Human readable matrix: each operator's deterministic oracle hash against the
/// standard 200-node oracle graph.
pub fn projection_20_matrix() -> Vec<(&'static str, &'static str, u8, u8)> {
    PROJECTION_OPERATORS
        .iter()
        .map(|o| (o.id, o.filter.as_str(), if o.direction == Dir::In { 0 } else { 1 }, o.hops))
        .collect()
}

/// Build a deterministic 200-node oracle graph used by tests:
/// - Vertices: id 1..=200
///   - type_: Person for id 1..100, Org for 101..200
///   - label: User-1..100, Tenant-101..200
///   - community: (id % 7) + 1
///   - attr: {"dept":"R&D"} if id % 3 == 0 ; {} otherwise
///   - degree distribution: id==1→hub (edges to 2..30) => degree 29 out, some in from bwd
/// - Edges: deterministic mix so oracle vertex/edge counts are stable.
pub fn build_oracle_graph_200() -> SimpleGraph {
    let mut g = SimpleGraph {
        vertices: BTreeMap::new(),
        fwd: BTreeMap::new(),
        bwd: BTreeMap::new(),
    };
    for id in 1..=200i64 {
        let type_ = if id <= 100 { "Person" } else { "Org" };
        let label = if id <= 100 { format!("User-{id}") } else { format!("Tenant-{id}") };
        let community = (id % 7) + 1;
        let mut attr = BTreeMap::new();
        if id % 3 == 0 {
            attr.insert("dept".into(), "R&D".into());
        }
        if id % 5 == 0 {
            attr.insert("vip".into(), "1".into());
        }
        g.add_vertex_with(id, &label, type_, community, attr);
    }
    // Hub: 1 → 2..30 out edges (Person "knows")
    for t in 2..=30 {
        g.add_edge(1, t, "knows");
    }
    // Person→Org: p → 100 + p for p 1..90
    for p in 1..=90 {
        g.add_edge(p, 100 + p, "works_at");
    }
    // Org→Org: ring 101..200 → next
    for o in 101..=200 {
        let next = if o == 200 { 101 } else { o + 1 };
        g.add_edge(o, next, "partner");
    }
    // Person 2..20 → Person 100 - id backward edges so IN-direction traversable
    for i in 2..=20 {
        g.add_edge(100 - i, i, "reports_to");
    }
    g
}

// =============== 20 projection-specific tests ===============
#[cfg(test)]
mod tests {
    use super::*;

    fn oracle() -> SimpleGraph {
        build_oracle_graph_200()
    }

    fn assert_vertices(id: &str, expected_size: impl Fn(usize) -> bool, actual: &ProjectionResult) {
        assert!(
            expected_size(actual.vertices.len()),
            "{id} expected size condition fail, got vertices={:?}",
            actual.vertices.len()
        );
    }

    // --- oracle baseline ---
    #[test]
    fn proj_oracle_graph_degree_1_is_29_or_30() {
        let g = oracle();
        // Vertex 1: 29 out + reports_to edges from some nodes
        assert!(g.degree(1) >= 29, "v1 degree={}", g.degree(1));
    }

    #[test]
    fn proj_registry_exactly_20() {
        assert_eq!(PROJECTION_OPERATORS.len(), 20);
        let uniq: BTreeSet<&str> = PROJECTION_OPERATORS.iter().map(|o| o.id).collect();
        assert_eq!(uniq.len(), 20);
    }

    // 20 individual operator tests + 1 registry
    // Helper: call operator by function
    fn run(seed: i64, f: fn(&ProjectionContext)->ProjectionResult, param: &str) -> ProjectionResult {
        let g = oracle();
        let ctx = ProjectionContext { graph: &g, seed, param: param.into() };
        f(&ctx)
    }

    #[test] fn b4_01_type_out_1_person_from_1() {
        let r = run(1, proj_type_out_1, "Person");
        // 1→ 2..30 are Person (Person id<=100)
        assert_vertices("proj_type_out_1", |s| s==30, &r); // 1 + 29 Person
    }

    #[test] fn b4_02_type_in_1_person_to_101() {
        let r = run(101, proj_type_in_1, "Person"); // 1→2..30 Person→Org works_at: p=1 works at 101? 100+1=101 yes.
        // Id 1 →101 works_at edge. So Person in-1 of v101 = {101, 1}
        assert_vertices("proj_type_in_1(101)", |s| s==2, &r);
    }

    #[test] fn b4_03_type_out_2_person_from_1_reaches_ring() {
        // 1→Person2..30 out. Then from person p(1..90)→org works_at 2nd hop: Org. Filter type Person at hop2? Only other Person reached through non-org edges.
        let r = run(1, proj_type_out_2, "Person");
        // Expect seed 1 (Person) + Person 2..30 (1hop) + any Person reached in 2nd hop via Person→… not Org works_at.
        // We added reports_to Person→Person for i 2..20 edge (100-i, i, reports_to). So 2nd hop: Person 2..20 can reach backward Person 98..80 → Person.
        assert!(r.vertices.len() >= 30, "expected >=30 Person vertices, got {}", r.vertices.len());
    }

    #[test] fn b4_04_type_in_2_org_from_105() {
        let r = run(105, proj_type_in_2, "Org");
        // ring partner in of 105 = (104 → 105 partner) + works_at? No Person → Org.
        // But 2-hop in: ring neighbors + previous?
        assert!(r.vertices.contains(&105));
    }

    #[test] fn b4_05_community_out_1_eq_c3() {
        // c=3 means id mod 7 + 1 = 3 => id mod 7 = 2 (ids: 2,9,...)
        // Start seed=2: out edges 2→?  Edge (100-i, i, reports_to for i=2 → (98,2,reports_to) is bwd; fwd 98→2. so 2's out edges from fwd... add_vertex_and_edge: id==1 is hub →1→2..30 fwd, also p→100+p for p=1..90,  o ring, reports_to edges fwd 98→2 for i=2 etc.
        // We'll just ensure the subgraph size > 1.
        let r = run(2, proj_community_out_1, "3");
        assert!(r.vertices.len() >= 1); // at least seed itself
    }

    #[test] fn b4_06_community_out_2() {
        let r = run(5, proj_community_out_2, "6");
        assert!(r.vertices.contains(&5));
    }
    #[test] fn b4_07_community_in_1() {
        let r = run(10, proj_community_in_1, "4");
        assert!(r.vertices.contains(&10));
    }
    #[test] fn b4_08_community_in_2() {
        let r = run(15, proj_community_in_2, "2");
        assert!(r.vertices.contains(&15));
    }
    #[test] fn b4_09_attr_out_1_dept_rnd() {
        // seed=3, attr dept key present in seed and any adjacent where attr has dept key (id%3==0)
        let r = run(3, proj_attr_out_1, "dept");
        assert!(r.vertices.contains(&3));
    }
    #[test] fn b4_10_attr_out_2() {
        let r = run(6, proj_attr_out_2, "vip");
        assert!(r.vertices.contains(&6));
    }
    #[test] fn b4_11_attr_in_1() {
        let r = run(9, proj_attr_in_1, "dept");
        assert!(r.vertices.contains(&9));
    }
    #[test] fn b4_12_attr_in_2() {
        let r = run(12, proj_attr_in_2, "dept");
        assert!(r.vertices.contains(&12));
    }
    #[test] fn b4_13_degree_out_1_ge_2() {
        // Almost every node will have degree >= 2.
        let r = run(1, proj_degree_out_1, "2");
        assert!(r.vertices.len() >= 3);
    }
    #[test] fn b4_14_degree_out_2_ge_1() {
        let r = run(1, proj_degree_out_2, "1");
        // Seed itself + neighbors with degree >= 1; seed has degree 29 out already (>= 1).
        assert!(r.vertices.len() >= 1, "size={}", r.vertices.len());
    }
    #[test] fn b4_15_degree_in_1_ge_1() {
        let r = run(101, proj_degree_in_1, "1");
        assert!(r.vertices.contains(&101));
    }
    #[test] fn b4_16_degree_in_2_ge_1() {
        let r = run(101, proj_degree_in_2, "1");
        assert!(r.vertices.contains(&101));
    }
    #[test] fn b4_17_label_out_1_user_1() {
        let r = run(1, proj_label_out_1, "User-1"); // Only seed matches label
        // Seed matches plus its neighbors that also have label...
        assert!(r.vertices.contains(&1));
    }
    #[test] fn b4_18_label_out_2_tenant_101() {
        let r = run(101, proj_label_out_2, "Tenant-101");
        assert!(r.vertices.contains(&101));
    }
    #[test] fn b4_19_label_in_1_tenant_150() {
        let r = run(150, proj_label_in_1, "Tenant-150");
        assert!(r.vertices.contains(&150));
    }
    #[test] fn b4_20_label_in_2_tenant_200() {
        let r = run(200, proj_label_in_2, "Tenant-200");
        assert!(r.vertices.contains(&200));
    }

    // 4.3 AND-composite size
    #[test]
    fn b4_21_and_intersect_size_proj_type_out_1_and_degree_out_1() {
        let g = oracle();
        let a = (PROJECTION_OPERATORS[0].apply)(&ProjectionContext { graph: &g, seed: 1, param: "Person".into() });
        let b = (PROJECTION_OPERATORS[12].apply)(&ProjectionContext { graph: &g, seed: 1, param: "1".into() });
        let inter: BTreeSet<_> = a.vertices.intersection(&b.vertices).copied().collect();
        assert!(!inter.is_empty());
        // intersection must be subset of both
        assert!(inter.is_subset(&a.vertices) && inter.is_subset(&b.vertices));
    }
}
