// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use crate::types::KnowledgeEdge;

// ============================================================================
// 7 核心算法·第 7 条：RAW 边双向展开（对齐 Node _expandRawEdges）
// ============================================================================
/// RAW 边双向展开：每条 `{u,v,w}` 展开为 `[(u→v,w), (v→u,w)]`，
/// 用于无向语义算法（度/介数/紧密/社区）在 DiGraph 上的统一实现，
/// 使入度出度对称，对齐 Node 端 `_expandRawEdges(edges, {directed:false})` 行为。
///
/// - 跳过 source/target 为空的边
/// - 自环：只保留一份（u→u，不重复）
/// - 保留原 weight（默认 1.0）
pub fn raw_bidirectional_expand(edges: &[KnowledgeEdge]) -> Vec<KnowledgeEdge> {
    let mut out = Vec::with_capacity(edges.len() * 2);
    for e in edges {
        let s = &e.source;
        let t = &e.target;
        if s.is_empty() || t.is_empty() {
            continue;
        }
        let w = if e.weight == 0.0 { 1.0 } else { e.weight };
        let rt = if e.relation_type.is_empty() {
            "related".to_string()
        } else {
            e.relation_type.clone()
        };
        let props = e.properties.clone();
        // u -> v
        out.push(KnowledgeEdge {
            source: s.clone(),
            target: t.clone(),
            weight: w,
            relation_type: rt.clone(),
            properties: props.clone(),
        });
        if s != t {
            // v -> u（自环不重复）
            out.push(KnowledgeEdge {
                source: t.clone(),
                target: s.clone(),
                weight: w,
                relation_type: rt,
                properties: props,
            });
        }
    }
    out
}
