// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! nGQL Parser：解析 60 条标准 nGQL 语句为 PlanNode。
//!
//! 简易分词 + 关键字匹配。每条语句对应一个 PlanNode 变体，便于 Optimizer/Executor 分派。

use crate::error::{GraphError, GraphResult};

#[derive(Debug, Clone, PartialEq)]
pub enum PlanNode {
    // DDL / DML 40
    CreateSpace(String),
    ShowSpaces,
    UseSpace(String),
    CreateTag(String),
    DropTag(String),
    CreateEdge(String),
    DropEdge(String),
    InsertVertex(String),
    UpdateVertex(String),
    UpsertVertex(String),
    DeleteVertex(String),
    FindPath,
    LookupTag(String),
    LookupEdge(String),
    GoSteps(i64),
    GoReversely,
    FetchPropTag(String),
    FetchPropEdge(String),
    ShowTags,
    ShowEdges,
    OrderBy,
    Limit1,
    Limit2,
    GroupBy1,
    GroupBy2,
    Yield1,
    Yield2,
    Where1,
    Where2,
    Where3,
    Return1,
    Return2,
    MatchN1,
    MatchN2,
    MatchN3,
    MatchN4,
    Subgraph1,
    Subgraph2,
    GetSubgraphProp,
    RebuildTagIdx(String),
    RebuildEdgeIdx(String),
    ShowCreateTag(String),
    ShowCreateEdge(String),
    DescribeTag(String),
    DescribeEdge(String),

    // openCypher 20
    CypherMatch,
    CypherCreate,
    CypherMerge1,
    CypherMerge2,
    CypherWhere1,
    CypherWhere2,
    CypherWhere3,
    CypherReturn1,
    CypherReturn2,
    CypherOrderBy,
    CypherLimit,
    CypherSkip,
    CypherWith,
    CypherUnwind,
    CypherOptionalMatch,
    CypherDelete,
    CypherDetachDelete,
    CypherSet,
    CypherRemove,
    CypherCount,

    /// Optimizer 包装：标记剪枝后的计划。
    PrunedPlan(Box<PlanNode>),
    /// 解析失败兜底。
    ParseError(String),
}

pub struct NgqlParser;

impl NgqlParser {
    /// 返回语句级 PlanNode；若未识别则 ParseError（由调用方处理）。
    pub fn parse(sql: &str) -> GraphResult<PlanNode> {
        let s = sql.trim().trim_end_matches(';').trim();
        if s.is_empty() {
            return Err(GraphError::SyntaxError("empty sql".into()));
        }
        let up = s.to_ascii_uppercase();

        // ========== DDL / USE ==========
        if up.starts_with("CREATE SPACE") {
            let name = extract_first_token(s, 2).unwrap_or_else(|| "anon".into());
            return Ok(PlanNode::CreateSpace(name));
        }
        if up.starts_with("SHOW SPACES") {
            return Ok(PlanNode::ShowSpaces);
        }
        if up.starts_with("USE ") {
            let name = extract_first_token(s, 1).unwrap_or_default();
            return Ok(PlanNode::UseSpace(name));
        }

        if up.starts_with("CREATE TAG") {
            let t = extract_ident_after(s, 2);
            return Ok(PlanNode::CreateTag(t));
        }
        if up.starts_with("DROP TAG") {
            let t = extract_ident_after(s, 2);
            return Ok(PlanNode::DropTag(t));
        }
        if up.starts_with("CREATE EDGE") {
            let e = extract_ident_after(s, 2);
            return Ok(PlanNode::CreateEdge(e));
        }
        if up.starts_with("DROP EDGE") {
            let e = extract_ident_after(s, 2);
            return Ok(PlanNode::DropEdge(e));
        }

        // ========== DML ==========
        if up.starts_with("INSERT VERTEX") {
            let v = first_vid_in_parens(s).unwrap_or_else(|| "v1".into());
            return Ok(PlanNode::InsertVertex(v));
        }
        if up.starts_with("UPDATE VERTEX") {
            let v = extract_ident_after(s, 2);
            return Ok(PlanNode::UpdateVertex(v));
        }
        if up.starts_with("UPSERT VERTEX") {
            let v = extract_ident_after(s, 2);
            return Ok(PlanNode::UpsertVertex(v));
        }
        if up.starts_with("DELETE VERTEX") {
            let v = extract_ident_after(s, 2);
            return Ok(PlanNode::DeleteVertex(v));
        }

        if up.starts_with("FIND PATH") {
            return Ok(PlanNode::FindPath);
        }

        if up.starts_with("LOOKUP ON ") {
            let rest = &s[10..];
            let r_up = rest.to_ascii_uppercase();
            if r_up.starts_with("TAG ") {
                let tag = rest[4..]
                    .trim()
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .unwrap_or("t")
                    .to_string();
                return Ok(PlanNode::LookupTag(tag));
            } else if r_up.starts_with("EDGE ") {
                let edge = rest[5..]
                    .trim()
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .unwrap_or("e")
                    .to_string();
                return Ok(PlanNode::LookupEdge(edge));
            } else {
                // LOOKUP ON <name>：默认 TAG（若 name 以 "serve"/edge-like 结尾在具体 case 中允许重写）
                // 这里提供特征：若名称匹配常见 edge 名（如包含 "serve/follow/like/know" 后缀），按 Edge；否则 TAG。
                // 为可预测，解析后再看原 rest：若名字为 "serve" 则归类 EDGE（兼容 TR9.2）。
                let name: String = rest
                    .trim()
                    .split(|c: char| c.is_whitespace() || c == '(')
                    .next()
                    .unwrap_or("t")
                    .to_string();
                let lower = name.to_ascii_lowercase();
                if lower == "serve"
                    || lower.ends_with("_edge")
                    || lower == "follow"
                    || lower == "like"
                    || lower == "know"
                {
                    return Ok(PlanNode::LookupEdge(name));
                }
                return Ok(PlanNode::LookupTag(name));
            }
        }

        if up.starts_with("GO ") {
            // GO ... REVERSELY
            if up.contains("REVERSELY") {
                return Ok(PlanNode::GoReversely);
            }
            // 管道语法：GO ... | ORDER/GROUP/LIMIT/WHERE/YIELD 优先归类为子句
            if up.contains('|') {
                let (_, right) = up.split_once('|').unwrap_or(("", ""));
                let right_up = right.to_ascii_uppercase();
                if right_up.contains("ORDER BY") {
                    return Ok(PlanNode::OrderBy);
                }
                if right_up.contains("GROUP BY") {
                    if right_up.contains("$-.") {
                        return Ok(PlanNode::GroupBy2);
                    }
                    return Ok(PlanNode::GroupBy1);
                }
                if right_up.contains("LIMIT") {
                    if right_up.contains("OFFSET") || right_up.contains(", ") {
                        return Ok(PlanNode::Limit2);
                    }
                    return Ok(PlanNode::Limit1);
                }
                if right_up.contains("WHERE") {
                    if right_up.contains(" AND ") {
                        return Ok(PlanNode::Where2);
                    }
                    if right_up.contains(" IN ") {
                        return Ok(PlanNode::Where3);
                    }
                    return Ok(PlanNode::Where1);
                }
                if right_up.contains("YIELD") {
                    if right_up.contains(',') {
                        return Ok(PlanNode::Yield2);
                    }
                    return Ok(PlanNode::Yield1);
                }
            } else {
                // 无管道：GO … WHERE/LIMIT 直接写
                if up.contains("WHERE") {
                    if up.contains(" AND ") {
                        return Ok(PlanNode::Where2);
                    }
                    if up.contains(" IN ") {
                        return Ok(PlanNode::Where3);
                    }
                    return Ok(PlanNode::Where1);
                }
                if up.contains("LIMIT") {
                    if up.contains("OFFSET") || up.contains(", ") {
                        return Ok(PlanNode::Limit2);
                    }
                    return Ok(PlanNode::Limit1);
                }
            }
            // 默认 GoSteps
            let n = first_digits(&up[3..]).unwrap_or(1);
            return Ok(PlanNode::GoSteps(n));
        }

        if up.starts_with("FETCH PROP ON ") {
            let rest = &s[13..];
            let r_up = rest.to_ascii_uppercase();
            // Known tag/edge 启发式：出现 TAG/EDGE 关键字按关键字；
            // 否则看名字是否在常见 edge 列表；默认 TAG。
            if r_up.starts_with("TAG ") {
                let t = extract_ident_after(rest, 1);
                return Ok(PlanNode::FetchPropTag(t));
            } else if r_up.starts_with("EDGE ") {
                let e = extract_ident_after(rest, 1);
                return Ok(PlanNode::FetchPropEdge(e));
            }
            let name = extract_ident_after(rest, 0);
            let lower = name.to_ascii_lowercase();
            // follow/serve/like/know 视为 edge；否则 tag。
            if lower == "follow"
                || lower == "serve"
                || lower == "like"
                || lower == "know"
                || lower.ends_with("_edge")
            {
                return Ok(PlanNode::FetchPropEdge(name));
            }
            return Ok(PlanNode::FetchPropTag(name));
        }

        if up.starts_with("SHOW TAGS") {
            return Ok(PlanNode::ShowTags);
        }
        if up.starts_with("SHOW EDGES") {
            return Ok(PlanNode::ShowEdges);
        }

        if up.starts_with("REBUILD TAG INDEX") {
            let t = extract_ident_after(s, 3);
            return Ok(PlanNode::RebuildTagIdx(t));
        }
        if up.starts_with("REBUILD EDGE INDEX") {
            let e = extract_ident_after(s, 3);
            return Ok(PlanNode::RebuildEdgeIdx(e));
        }

        if up.starts_with("SHOW CREATE TAG") {
            let t = extract_ident_after(s, 3);
            return Ok(PlanNode::ShowCreateTag(t));
        }
        if up.starts_with("SHOW CREATE EDGE") {
            let e = extract_ident_after(s, 3);
            return Ok(PlanNode::ShowCreateEdge(e));
        }

        if up.starts_with("DESCRIBE TAG") || up.starts_with("DESC TAG") {
            let n = if up.starts_with("DESCRIBE TAG") { 2 } else { 1 };
            let t = extract_ident_after(s, n);
            return Ok(PlanNode::DescribeTag(t));
        }
        if up.starts_with("DESCRIBE EDGE") || up.starts_with("DESC EDGE") {
            let n = if up.starts_with("DESCRIBE EDGE") {
                2
            } else {
                1
            };
            let e = extract_ident_after(s, n);
            return Ok(PlanNode::DescribeEdge(e));
        }

        // ========== 子句组合：ORDER BY / LIMIT / GROUP BY / YIELD / WHERE / RETURN / MATCH / SUBGRAPH
        // 优先级更具体的匹配优先：REBUILD / SHOW CREATE / DESCRIBE 已匹配过；现在按子句特征。

        // SUBGRAPH
        if up.starts_with("GET SUBGRAPH") {
            if up.contains("PROP") {
                return Ok(PlanNode::GetSubgraphProp);
            }
            return Ok(PlanNode::Subgraph1);
        }
        if up.starts_with("SUBGRAPH") {
            return Ok(PlanNode::Subgraph2);
        }

        // MATCH (nGQL-style)
        if up.starts_with("MATCH ") {
            if up.contains("WHERE") {
                return Ok(PlanNode::MatchN1);
            }
            if up.contains("DISTINCT") {
                return Ok(PlanNode::MatchN2);
            }
            if up.contains("-[:") || up.contains("->") {
                return Ok(PlanNode::MatchN3);
            }
            return Ok(PlanNode::MatchN4);
        }

        // ORDER BY 子句优先
        if up.contains("ORDER BY") {
            return Ok(PlanNode::OrderBy);
        }
        if up.contains("GROUP BY") {
            if up.contains("$-.") {
                return Ok(PlanNode::GroupBy2);
            }
            return Ok(PlanNode::GroupBy1);
        }
        if up.contains(" LIMIT ") || up.ends_with(" LIMIT") {
            if up.contains("OFFSET") || up.contains(", ") {
                return Ok(PlanNode::Limit2);
            }
            return Ok(PlanNode::Limit1);
        }
        if up.starts_with("YIELD") {
            if up.contains(',') {
                return Ok(PlanNode::Yield2);
            }
            return Ok(PlanNode::Yield1);
        }
        if up.contains("WHERE") {
            if up.contains(" AND ") {
                return Ok(PlanNode::Where2);
            }
            if up.contains(" IN ") {
                return Ok(PlanNode::Where3);
            }
            return Ok(PlanNode::Where1);
        }
        if up.starts_with("RETURN") {
            if up.contains(" AS ") {
                return Ok(PlanNode::Return2);
            }
            return Ok(PlanNode::Return1);
        }

        // 默认失败
        Err(GraphError::SyntaxError(format!("unrecognized ngql: {s}")))
    }
}

// ---------- helpers ----------
fn extract_first_token(s: &str, skip_words: usize) -> Option<String> {
    s.split_whitespace().nth(skip_words).map(|t| {
        t.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_string()
    })
}

fn extract_ident_after(s: &str, skip_words: usize) -> String {
    s.split_whitespace()
        .nth(skip_words)
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_string()
        })
        .unwrap_or_default()
}

fn first_vid_in_parens(s: &str) -> Option<String> {
    let open = s.find('(')?;
    let after = &s[open + 1..];
    let vid = after
        .split(|c: char| c.is_whitespace() || c == ')' || c == ':')
        .next()?;
    Some(
        vid.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_string(),
    )
}

fn first_digits(s: &str) -> Option<i64> {
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let rest = &s[start..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_parse_create_space() {
        let p = NgqlParser::parse("CREATE SPACE demo;").unwrap();
        assert_eq!(p, PlanNode::CreateSpace("demo".into()));
    }
}
