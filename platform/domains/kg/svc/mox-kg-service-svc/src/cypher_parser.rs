// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! openCypher Parser：20 条语句模式匹配 → PlanNode（复用 ngql_parser::PlanNode 的 cypher 变体）。
//!
//! 识别优先级：更具体的子句优先于通用开头（如 MATCH…DELETE 需要识别 DELETE 非 CypherMatch）。

use crate::error::{GraphError, GraphResult};
use crate::ngql_parser::PlanNode;

pub struct CypherParser;

impl CypherParser {
    pub fn parse(sql: &str) -> GraphResult<PlanNode> {
        let s = sql.trim().trim_end_matches(';').trim();
        if s.is_empty() {
            return Err(GraphError::SyntaxError("empty cypher".into()));
        }
        let up = s.to_ascii_uppercase();

        // 具体/更长的关键字优先匹配（从高具体度到低）
        if up.starts_with("DETACH DELETE") {
            return Ok(PlanNode::CypherDetachDelete);
        }
        if up.contains("DETACH DELETE") {
            return Ok(PlanNode::CypherDetachDelete);
        }

        if up.starts_with("OPTIONAL MATCH") {
            return Ok(PlanNode::CypherOptionalMatch);
        }

        if up.starts_with("MERGE ") {
            if up.contains("ON CREATE") {
                return Ok(PlanNode::CypherMerge1);
            }
            return Ok(PlanNode::CypherMerge2);
        }

        if up.starts_with("UNWIND ") {
            return Ok(PlanNode::CypherUnwind);
        }

        if up.starts_with("CREATE ") {
            return Ok(PlanNode::CypherCreate);
        }

        if up.starts_with("WITH ") {
            return Ok(PlanNode::CypherWith);
        }
        if up.contains(" WITH ") {
            return Ok(PlanNode::CypherWith);
        }

        if up.starts_with("DELETE") {
            return Ok(PlanNode::CypherDelete);
        }
        if up.contains(" DELETE") {
            return Ok(PlanNode::CypherDelete);
        }

        if up.starts_with("SET ") {
            return Ok(PlanNode::CypherSet);
        }
        if up.contains(" SET ") {
            return Ok(PlanNode::CypherSet);
        }

        if up.starts_with("REMOVE ") {
            return Ok(PlanNode::CypherRemove);
        }
        if up.contains(" REMOVE ") {
            return Ok(PlanNode::CypherRemove);
        }

        // 聚合 count：RETURN count(…)
        if up.contains("RETURN ") && up.contains("COUNT(") {
            return Ok(PlanNode::CypherCount);
        }

        if up.starts_with("RETURN ") {
            if up.contains(',') {
                return Ok(PlanNode::CypherReturn2);
            }
            return Ok(PlanNode::CypherReturn1);
        }

        if up.contains("ORDER BY") {
            return Ok(PlanNode::CypherOrderBy);
        }
        if up.contains(" SKIP ") || up.ends_with(" SKIP") {
            return Ok(PlanNode::CypherSkip);
        }
        if up.contains(" LIMIT ") || up.ends_with(" LIMIT") {
            return Ok(PlanNode::CypherLimit);
        }

        // WHERE 3 条：用特征区分
        if up.contains("WHERE") {
            if up.contains(" AND ") {
                return Ok(PlanNode::CypherWhere2);
            }
            if up.contains(" IN ") {
                return Ok(PlanNode::CypherWhere3);
            }
            return Ok(PlanNode::CypherWhere1);
        }

        if up.starts_with("MATCH ") {
            return Ok(PlanNode::CypherMatch);
        }

        Err(GraphError::SyntaxError(format!("unrecognized cypher: {s}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_cypher_match() {
        let p = CypherParser::parse("MATCH (n) RETURN n").unwrap();
        assert_eq!(p, PlanNode::CypherMatch);
    }
}
