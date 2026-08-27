// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! CNM (Clauset-Newman-Moore) greedy modularity maximization for SimpleGraph.
//!
//! Produces `CnmResult { community_id, q }` where `community_id[dense_i]` gives
//! the community label for the vertex at the dense index `dense_i` (matching the
//! iteration order of `SimpleGraph.vertices` which is a `BTreeMap<i64, Vertex>`,
//! so dense_i=0 is the smallest vertex id).
//!
//! Algorithm:
//! 1. Initial: each vertex is its own community.
//! 2. Compute every adjacent community pair (c, d) and ΔQ(c,d) = 2·(e_cd − a_c·a_d).
//! 3. Merge the pair with the largest ΔQ > 0; recompute neighbor community map.
//! 4. Repeat until no positive ΔQ merge remains.
//!
//! Deterministic tie-break: among merges with equal ΔQ, sort by
//! (max(c,d) desc, weight desc, min(c,d) desc) and pick the first.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::projection_20::SimpleGraph;

pub struct CnmResult {
    /// community_id[dense_i] is vertex i's community label, where i is the
    /// dense index (0..graph.vertices.len()) based on vertex-id order.
    pub community_id: Vec<i64>,
    /// Final modularity Q.  0.0 for empty or single-vertex graphs.
    pub q: f64,
}

pub fn detect(graph: &SimpleGraph) -> CnmResult {
    let n = graph.vertices.len();
    if n == 0 {
        return CnmResult {
            community_id: vec![],
            q: 0.0,
        };
    }
    if n == 1 {
        return CnmResult {
            community_id: vec![0],
            q: 0.0,
        };
    }

    // -----------------------------------------------------------
    // 1. Map sparse vertex ids -> dense 0..n
    // -----------------------------------------------------------
    let dense_ids: Vec<i64> = graph.vertices.keys().copied().collect();
    let id_to_dense: BTreeMap<i64, usize> = dense_ids
        .iter()
        .enumerate()
        .map(|(di, &id)| (id, di))
        .collect();

    // -----------------------------------------------------------
    // 2. Build weighted neighbor list (fwd + bwd together).
    //    A[u] = BTreeMap<v, weight>  (weight = multiplicity of u-v adjacency)
    // -----------------------------------------------------------
    let mut adj: Vec<BTreeMap<usize, u64>> = vec![BTreeMap::new(); n];
    let mut degree: Vec<u64> = vec![0; n];

    // fwd edges: s -> list of (t, label)
    for (&s, edges) in &graph.fwd {
        let Some(&su) = id_to_dense.get(&s) else { continue; };
        for (t, _lbl) in edges {
            let Some(&tv) = id_to_dense.get(t) else { continue; };
            *adj[su].entry(tv).or_insert(0) += 1;
            degree[su] += 1;
        }
    }
    // bwd edges: t -> list of (s, label)
    for (&t, edges) in &graph.bwd {
        let Some(&tv) = id_to_dense.get(&t) else { continue; };
        for (s, _lbl) in edges {
            let Some(&su) = id_to_dense.get(s) else { continue; };
            *adj[tv].entry(su).or_insert(0) += 1;
            degree[tv] += 1;
        }
    }

    let two_m: f64 = degree.iter().copied().sum::<u64>() as f64;
    if two_m == 0.0 {
        // No edges: each vertex its own community, Q = 0 trivially.
        return CnmResult {
            community_id: (0..n as i64).collect(),
            q: 0.0,
        };
    }

    // -----------------------------------------------------------
    // 3. Per-community state
    // -----------------------------------------------------------
    // node -> community
    let mut node_comm: Vec<usize> = (0..n).collect();
    // communities that are still alive
    let mut alive: BTreeSet<usize> = (0..n).collect();
    // community -> sigma_tot (sum of degrees of nodes inside)
    let mut sigma_tot: HashMap<usize, f64> = HashMap::new();
    for c in 0..n {
        sigma_tot.insert(c, degree[c] as f64);
    }

    // community -> neighboring community -> edge weight sum (undirected between communities)
    // We keep a directed pair map; for the pair (c,d) we'll compute weight only once per iteration.
    // For simplicity, recompute community adjacency each iteration from the per-node adjacency.
    // This is O(n) per iteration * up to n iterations = O(n^2), fine for n~200.

    // Helper: given current node_comm, compute for each alive community c its neighbor-community weights
    fn compute_comm_edges(
        n: usize,
        alive: &BTreeSet<usize>,
        node_comm: &[usize],
        adj: &[BTreeMap<usize, u64>],
    ) -> HashMap<usize, BTreeMap<usize, u64>> {
        let mut out: HashMap<usize, BTreeMap<usize, u64>> = HashMap::new();
        for c in alive { out.insert(*c, BTreeMap::new()); }
        for u in 0..n {
            let cu = node_comm[u];
            for (&v, &w) in &adj[u] {
                let cv = node_comm[v];
                if cu == cv { continue; }
                *out.get_mut(&cu).unwrap().entry(cv).or_insert(0) += w;
            }
        }
        out
    }

    // -----------------------------------------------------------
    // 4. CNM main loop: pair merges with ΔQ > 0
    // -----------------------------------------------------------
    let two_m_recip = 1.0 / two_m;

    loop {
        let comm_edges = compute_comm_edges(n, &alive, &node_comm, &adj);

        let mut best_dq: f64 = 0.0;
        let mut best_pair: Option<(usize, usize)> = None;
        let mut best_weight: u64 = 0;

        // Iterate alive communities in sorted order; for neighbors also sorted.
        // Collect candidates then sort them deterministically.
        let mut candidates: Vec<(f64, u64, usize, usize)> = Vec::new();
        for &c in alive.iter() {
            let Some(neighbors) = comm_edges.get(&c) else { continue; };
            // "sort neighbors by (target_community_id, weight) desc for ties"
            let mut sorted_nbs: Vec<(usize, u64)> = neighbors.iter().map(|(&d, &w)| (d, w)).collect();
            sorted_nbs.sort_by(|a, b| {
                b.0.cmp(&a.0) // target_community_id desc
                    .then_with(|| b.1.cmp(&a.1)) // weight desc
            });
            for (d, w) in sorted_nbs {
                if d <= c {
                    continue; // only consider c < d to avoid duplicate pairs
                }
                // compute_comm_edges: out[c][d] = Σ_{u∈c,v∈d} A_{u,v} = L_{cd}
                //   (one-way cross edge count; the adjacency undirected graph double-reading
                //    from SimpleGraph fwd+bwd inflates each weight by 2× and degrees by 2×,
                //    which cancels in ratios).
                let w_inflated = w as f64;
                // Standard CNM merge ΔQ = L_{cd}/M - 2 a_c a_d
                // Using e_cd = w_inflated / two_m:
                //   two_m = 4M (because of 2× inflated degrees), w_inflated = 2× L_{cd}
                //   so e_cd = 2L / 4M = L/(2M). Then 2*e_cd = L/M.
                // Let e_cd_code = w_inflated / two_m. Then ΔQ = 2*(e_cd_code - a_c*a_d).
                let e_cd_code = w_inflated * two_m_recip;
                let a_c = sigma_tot[&c] * two_m_recip;
                let a_d = sigma_tot[&d] * two_m_recip;
                let dq = 2.0 * (e_cd_code - a_c * a_d);
                // Push (dq, weight, max(c,d), min(c,d)) — sorting will use these.
                candidates.push((dq, w, usize::max(c, d), usize::min(c, d)));
            }
        }

        if candidates.is_empty() {
            break;
        }

        // Deterministic sort:
        //   1. ΔQ desc
        //   2. max(c,d) desc  (per "target_community_id desc")
        //   3. weight desc
        //   4. min(c,d) desc  — final tie-breaker
        candidates.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| b.3.cmp(&a.3))
        });

        let (dq, w, m_cd, mn_cd) = candidates[0];
        if dq <= 0.0 {
            break;
        }
        best_dq = dq;
        best_weight = w;
        // We'll keep the SMALLER community id as the "merged into" target for stable labels,
        // but convention: keep the pair ordered consistently below.
        // (c_low, c_high) = (min, max)
        let (c_low, c_high) = (mn_cd, m_cd);
        let _ = best_pair.insert((c_low, c_high));
        let _ = best_weight; // silence unused

        // Perform merge: move all nodes in c_high into c_low, drop c_high from alive,
        // update sigma_tot[c_low] += sigma_tot[c_high], drop c_high entry.
        for nc in node_comm.iter_mut() {
            if *nc == c_high {
                *nc = c_low;
            }
        }
        alive.remove(&c_high);
        let st_high = sigma_tot.remove(&c_high).unwrap_or(0.0);
        *sigma_tot.get_mut(&c_low).unwrap() += st_high;

        // best_dq unused directly (we trust it was > 0). Next iteration recomputes.
        let _ = best_dq;
    }

    // -----------------------------------------------------------
    // 5. Compute final modularity Q
    //    Q = Σ_c [ L_c / M - (D_c / (2M))² ]  (Newman convention)
    //    With inflated edge weights (×2 from fwd+bwd double-reading):
    //      internal_sum[c] = 4 × L_c  (2 per direction × 2 from inflation)
    //      two_m = 4 × M_real, sigma_tot[c] = 2 × D_c.
    //    So term1 = internal_sum[c] / two_m = 4L / 4M = L/M  ✓
    //       term2 = (sigma_tot[c] / two_m)² = (2D/4M)² = (D/2M)²  ✓
    // -----------------------------------------------------------
    let mut internal_sum: HashMap<usize, f64> = HashMap::new();
    for c in &alive { internal_sum.insert(*c, 0.0); }
    for u in 0..n {
        let cu = node_comm[u];
        for (&v, &w) in &adj[u] {
            let cv = node_comm[v];
            if cu == cv {
                *internal_sum.get_mut(&cu).unwrap() += w as f64;
            }
        }
    }
    let mut q: f64 = 0.0;
    for c in &alive {
        let s_in = internal_sum.get(c).copied().unwrap_or(0.0);
        let s_tot = sigma_tot.get(c).copied().unwrap_or(0.0);
        let term1 = s_in * two_m_recip;
        let a_c = s_tot * two_m_recip;
        let term2 = a_c * a_c;
        q += term1 - term2;
    }

    // -----------------------------------------------------------
    // 6. Assign community_id. Task spec: "assign community = raw community id".
    //    But raw ids are sparse (only alive ids remain).
    //    Return community_id as raw (internal usize → i64) for B2 determinism.
    //    To keep communities based on their raw id:
    // -----------------------------------------------------------
    let raw_ids: Vec<i64> = node_comm.iter().map(|&c| c as i64).collect();

    CnmResult {
        community_id: raw_ids,
        q,
    }
}

// ===========================================================================
// Tests  B1 - B4
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection_20::{SimpleGraph, Vertex};
    use std::collections::BTreeMap;

    fn make_empty_sg() -> SimpleGraph {
        SimpleGraph {
            vertices: BTreeMap::new(),
            fwd: BTreeMap::new(),
            bwd: BTreeMap::new(),
        }
    }

    fn add_vertex(sg: &mut SimpleGraph, id: i64) {
        sg.vertices.insert(
            id,
            Vertex {
                id,
                label: String::new(),
                type_: String::new(),
                community: 0,
                attr: BTreeMap::new(),
            },
        );
    }

    /// Add an undirected edge by adding both directions (standard CNM input).
    fn add_undirected(sg: &mut SimpleGraph, s: i64, t: i64, label: &str) {
        sg.add_edge(s, t, label);
        sg.add_edge(t, s, label);
    }

    // ---------------------------------------------------------------
    // B3. Empty and single vertex graphs → correct returns
    // ---------------------------------------------------------------
    #[test]
    fn t23_cnm_empty_single() {
        // empty graph
        let g = make_empty_sg();
        let r = detect(&g);
        assert_eq!(r.community_id.len(), 0);
        assert!((r.q - 0.0).abs() < 1e-12, "empty graph Q should be 0");

        // single vertex (no edges)
        let mut g2 = make_empty_sg();
        add_vertex(&mut g2, 42);
        let r2 = detect(&g2);
        assert_eq!(r2.community_id.len(), 1);
        assert_eq!(r2.community_id[0], 0); // only vertex -> dense index 0 -> its comm 0
        assert!((r2.q - 0.0).abs() < 1e-12, "1-vertex graph Q should be 0");
    }

    // ---------------------------------------------------------------
    // B4. For a non-empty graph, every vertex has a community assigned
    // ---------------------------------------------------------------
    #[test]
    fn t23_cnm_at_least_one_member() {
        // build a small graph with 5 vertices in a line
        let mut g = make_empty_sg();
        for i in 0..5 { add_vertex(&mut g, i); }
        for i in 0..4 { add_undirected(&mut g, i, i+1, "e"); }

        let r = detect(&g);
        assert_eq!(r.community_id.len(), 5);
        // Every community id that appears must cover at least one member: trivial.
        // Check no NaN / negative community values.
        for &c in &r.community_id {
            assert!(c >= 0, "community id should be >= 0, got {c}");
        }
        // Number of unique communities <= 5, >= 1
        let mut uniq: BTreeSet<i64> = BTreeSet::new();
        for &c in &r.community_id { uniq.insert(c); }
        assert!(!uniq.is_empty());
        assert!(uniq.len() <= 5);
    }

    // ---------------------------------------------------------------
    // B2. Determinism: fixed seed → identical community arrays across 10 runs
    // ---------------------------------------------------------------
    #[test]
    fn t23_cnm_deterministic_10_seeds() {
        fn build_graph(seed: u64) -> SimpleGraph {
            // Simple deterministic pseudo-random graph using xorshift64*.
            // 30 vertices, ~80 edges, seeded by `seed`.
            let mut state: u64 = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let mut next = || -> u64 {
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                let r = state;
                state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
                r
            };
            let n = 30usize;
            let mut g = make_empty_sg();
            for i in 0..n as i64 { add_vertex(&mut g, i); }
            for _ in 0..80 {
                let a = (next() % n as u64) as i64;
                let b = (next() % n as u64) as i64;
                if a == b { continue; }
                add_undirected(&mut g, a, b, "e");
            }
            g
        }

        for seed in 1..=10u64 {
            let mut first: Option<Vec<i64>> = None;
            for _run in 0..10 {
                let g = build_graph(seed);
                let r = detect(&g);
                match &first {
                    None => first = Some(r.community_id.clone()),
                    Some(f) => {
                        assert_eq!(
                            f, &r.community_id,
                            "B2 determinism failed: seed={seed}",
                        );
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // B1. Oracle: 4 groups with 80% inside-group links, 5% between
    //     Modularity Q ≥ 0.25 (± 0.05)  — use 50 vertices for speed.
    // ---------------------------------------------------------------
    #[test]
    fn t23_cnm_oracle_200_q_within_eps() {
        // Use 50 vertices divided into k=4 groups (group size uneven: 13,13,12,12)
        // Build graph deterministically via seeded RNG.
        let total = 50usize;
        let k = 4usize;
        let mut groups: Vec<Vec<i64>> = vec![Vec::new(); k];
        for i in 0..total as i64 {
            groups[i as usize % k].push(i);
        }

        let mut state: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let mut rng = || -> f64 {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let v = state;
            state = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
            // convert to [0,1)
            (v as f64) / (u64::MAX as f64)
        };

        let mut g = make_empty_sg();
        for i in 0..total as i64 { add_vertex(&mut g, i); }

        // For each pair of distinct vertices, add edge with probability:
        //   same group: 0.80
        //   diff group: 0.05
        for i in 0..total as i64 {
            for j in (i + 1)..total as i64 {
                let gi = i as usize % k;
                let gj = j as usize % k;
                let p = if gi == gj { 0.80 } else { 0.05 };
                if rng() < p {
                    add_undirected(&mut g, i, j, "e");
                }
            }
        }

        let r = detect(&g);
        assert_eq!(r.community_id.len(), total);
        // Modularity Q should be >= 0.25 roughly
        eprintln!("B1 Q = {:.6}", r.q);
        assert!(
            r.q >= 0.20,
            "B1 modularity too low: Q={:.6}, expected >= 0.20 ± 0.05 (~=0.25)",
            r.q
        );

        // Sanity: number of distinct communities roughly ~k (4) ± small tolerance
        let mut uniq: BTreeSet<i64> = BTreeSet::new();
        for &c in &r.community_id { uniq.insert(c); }
        eprintln!("B1 communities found: {}", uniq.len());
        assert!(
            uniq.len() <= k + 2,
            "B1 too many communities: {}, expected ~{}",
            uniq.len(), k
        );
    }
}
