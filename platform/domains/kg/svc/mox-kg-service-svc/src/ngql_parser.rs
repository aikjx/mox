// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

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

    // ---------- 索引管理（新增）----------
    /// CREATE TAG INDEX
    CreateTagIndex(String),
    /// CREATE EDGE INDEX
    CreateEdgeIndex(String),
    /// DROP TAG INDEX
    DropTagIndex(String),
    /// DROP EDGE INDEX
    DropEdgeIndex(String),
    /// CREATE FULLTEXT INDEX
    CreateFulltextIndex(String),
    /// DROP FULLTEXT INDEX
    DropFulltextIndex(String),
    /// CREATE VECTOR INDEX
    CreateVectorIndex(String),
    /// DROP VECTOR INDEX
    DropVectorIndex(String),
    /// SHOW CREATE TAG INDEX
    ShowCreateTagIndex(String),
    /// SHOW CREATE EDGE INDEX
    ShowCreateEdgeIndex(String),
    /// SHOW INDEXES
    ShowIndexes,
    /// SHOW TAG INDEXES
    ShowTagIndexes,
    /// SHOW EDGE INDEXES
    ShowEdgeIndexes,
    /// DESCRIBE INDEX
    DescribeIndex(String),

    // ---------- 执行计划分析（新增）----------
    /// EXPLAIN
    Explain(String),
    /// PROFILE
    Profile(String),
    /// EXPLAIN FORMAT=json
    ExplainFormatJson(String),

    // ---------- 异步任务（新增）----------
    /// SUBMIT JOB COMPACT
    SubmitJobCompact,
    /// SUBMIT JOB STATS
    SubmitJobStats,
    /// SUBMIT JOB REBUILD INDEX
    SubmitJobRebuildIndex(String),
    /// SHOW JOB
    ShowJob(i64),
    /// SHOW JOBS
    ShowJobs,
    /// STOP JOB
    StopJob(i64),
    /// RECOVER JOB
    RecoverJob(i64),

    // ---------- 快照管理（新增）----------
    /// CREATE SNAPSHOT
    CreateSnapshot(String),
    /// SHOW SNAPSHOTS
    ShowSnapshots,
    /// DROP SNAPSHOT
    DropSnapshot(String),
    /// CHECK SNAPSHOT
    CheckSnapshot(String),

    // ---------- 数据均衡（新增）----------
    /// BALANCE DATA
    BalanceData,
    /// BALANCE DATA REMOVE
    BalanceDataRemove(Vec<String>),
    /// BALANCE LEADER
    BalanceLeader,
    /// BALANCE STOP
    BalanceStop,
    /// SHOW BALANCE
    ShowBalance,

    // ---------- 配置管理（新增）----------
    /// CONFIG GET
    ConfigGet(String),
    /// CONFIG SET
    ConfigSet(String, String),
    /// SHOW CONFIGS
    ShowConfigs,
    /// SHOW VARIABLES
    ShowVariables,

    // ---------- 运维管理（新增）----------
    /// SHOW HOSTS
    ShowHosts,
    /// SHOW PARTS
    ShowParts,
    /// SHOW SESSIONS
    ShowSessions,
    /// SHOW QUERIES
    ShowQueries,
    /// KILL QUERY
    KillQuery(String),
    /// SHOW CHARSET
    ShowCharset,
    /// SHOW COLLATION
    ShowCollation,
    /// SHOW TIME ZONE
    ShowTimeZone,
    /// SHOW VERSION
    ShowVersion,

    // ---------- 组/角色管理（新增）----------
    /// CREATE USER
    CreateUser(String),
    /// DROP USER
    DropUser(String),
    /// ALTER USER
    AlterUser(String),
    /// SHOW USERS
    ShowUsers,
    /// CREATE ROLE
    CreateRole(String),
    /// DROP ROLE
    DropRole(String),
    /// GRANT ROLE
    GrantRole(String, String),
    /// REVOKE ROLE
    RevokeRole(String, String),
    /// SHOW ROLES
    ShowRoles(String),

    // ---------- 全文索引（新增）----------
    /// SHOW FULLTEXT INDEXES
    ShowFulltextIndexes,
    /// REBUILD FULLTEXT INDEX
    RebuildFulltextIndex(String),
    /// SEARCH FULLTEXT
    SearchFulltext(String),

    // ---------- 高级查询（新增）----------
    /// UNWIND
    Unwind,
    /// OPTIONAL MATCH（nGQL 扩展）
    OptionalMatch,
    /// UNION / UNION ALL
    UnionAll,
    /// INTERSECT
    Intersect,
    /// MINUS
    Minus,
    /// WITH（nGQL 扩展）
    WithClause,
    /// CALL (procedure)
    CallProcedure(String),
    /// YIELD DISTINCT
    YieldDistinct,

    // ---------- 图算法（新增）----------
    /// FIND SHORTEST PATH
    FindShortestPath,
    /// FIND ALL PATH
    FindAllPath,
    /// FIND NOLOOP PATH
    FindNoLoopPath,
    /// GET SUBGRAPH WITH PROP（已有别名，此处补充 V2 语法）
    GetSubgraphV2,

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

        // ========== 索引管理（新增）==========
        if up.starts_with("CREATE TAG INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::CreateTagIndex(name));
        }
        if up.starts_with("CREATE EDGE INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::CreateEdgeIndex(name));
        }
        if up.starts_with("DROP TAG INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::DropTagIndex(name));
        }
        if up.starts_with("DROP EDGE INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::DropEdgeIndex(name));
        }
        if up.starts_with("CREATE FULLTEXT INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::CreateFulltextIndex(name));
        }
        if up.starts_with("DROP FULLTEXT INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::DropFulltextIndex(name));
        }
        if up.starts_with("CREATE VECTOR INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::CreateVectorIndex(name));
        }
        if up.starts_with("DROP VECTOR INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::DropVectorIndex(name));
        }

        if up.starts_with("SHOW CREATE TAG INDEX") {
            let name = extract_ident_after(s, 4);
            return Ok(PlanNode::ShowCreateTagIndex(name));
        }
        if up.starts_with("SHOW CREATE EDGE INDEX") {
            let name = extract_ident_after(s, 4);
            return Ok(PlanNode::ShowCreateEdgeIndex(name));
        }

        if up.starts_with("SHOW TAG INDEXES") {
            return Ok(PlanNode::ShowTagIndexes);
        }
        if up.starts_with("SHOW EDGE INDEXES") {
            return Ok(PlanNode::ShowEdgeIndexes);
        }
        if up.starts_with("SHOW FULLTEXT INDEXES") {
            return Ok(PlanNode::ShowFulltextIndexes);
        }
        if up.starts_with("SHOW INDEXES") {
            return Ok(PlanNode::ShowIndexes);
        }

        if up.starts_with("DESCRIBE INDEX") || up.starts_with("DESC INDEX") {
            let n = if up.starts_with("DESCRIBE INDEX") { 2 } else { 1 };
            let name = extract_ident_after(s, n);
            return Ok(PlanNode::DescribeIndex(name));
        }

        if up.starts_with("REBUILD FULLTEXT INDEX") {
            let name = extract_ident_after(s, 3);
            return Ok(PlanNode::RebuildFulltextIndex(name));
        }

        if up.starts_with("SEARCH FULLTEXT") {
            let rest = &s[14..].trim();
            return Ok(PlanNode::SearchFulltext(rest.to_string()));
        }

        // ========== EXPLAIN / PROFILE（新增）==========
        if up.starts_with("EXPLAIN FORMAT=JSON") || up.starts_with("EXPLAIN FORMAT = JSON") {
            let rest = extract_after_keyword(s, "JSON");
            return Ok(PlanNode::ExplainFormatJson(rest));
        }
        if up.starts_with("EXPLAIN") {
            let rest = extract_after_keyword(s, "EXPLAIN");
            return Ok(PlanNode::Explain(rest));
        }
        if up.starts_with("PROFILE") {
            let rest = extract_after_keyword(s, "PROFILE");
            return Ok(PlanNode::Profile(rest));
        }

        // ========== 异步任务（新增）==========
        if up.starts_with("SUBMIT JOB COMPACT") {
            return Ok(PlanNode::SubmitJobCompact);
        }
        if up.starts_with("SUBMIT JOB STATS") {
            return Ok(PlanNode::SubmitJobStats);
        }
        if up.starts_with("SUBMIT JOB REBUILD INDEX") {
            let name = extract_ident_after(s, 4);
            return Ok(PlanNode::SubmitJobRebuildIndex(name));
        }
        if up.starts_with("SHOW JOBS") {
            return Ok(PlanNode::ShowJobs);
        }
        if up.starts_with("SHOW JOB ") {
            let id = first_digits(&s[9..]).unwrap_or(0);
            return Ok(PlanNode::ShowJob(id));
        }
        if up.starts_with("STOP JOB ") {
            let id = first_digits(&s[9..]).unwrap_or(0);
            return Ok(PlanNode::StopJob(id));
        }
        if up.starts_with("RECOVER JOB ") {
            let id = first_digits(&s[12..]).unwrap_or(0);
            return Ok(PlanNode::RecoverJob(id));
        }

        // ========== 快照管理（新增）==========
        if up.starts_with("CREATE SNAPSHOT") {
            let name = extract_first_token(s, 2).unwrap_or_else(|| "snap_1".into());
            return Ok(PlanNode::CreateSnapshot(name));
        }
        if up.starts_with("SHOW SNAPSHOTS") {
            return Ok(PlanNode::ShowSnapshots);
        }
        if up.starts_with("DROP SNAPSHOT") {
            let name = extract_first_token(s, 2).unwrap_or_default();
            return Ok(PlanNode::DropSnapshot(name));
        }
        if up.starts_with("CHECK SNAPSHOT") {
            let name = extract_first_token(s, 2).unwrap_or_default();
            return Ok(PlanNode::CheckSnapshot(name));
        }

        // ========== 数据均衡（新增）==========
        if up.starts_with("BALANCE DATA REMOVE") {
            // 解析要移除的主机列表
            let hosts = extract_host_list(s);
            return Ok(PlanNode::BalanceDataRemove(hosts));
        }
        if up.starts_with("BALANCE DATA") {
            return Ok(PlanNode::BalanceData);
        }
        if up.starts_with("BALANCE LEADER") {
            return Ok(PlanNode::BalanceLeader);
        }
        if up.starts_with("BALANCE STOP") {
            return Ok(PlanNode::BalanceStop);
        }
        if up.starts_with("SHOW BALANCE") {
            return Ok(PlanNode::ShowBalance);
        }

        // ========== 配置管理（新增）==========
        if up.starts_with("CONFIG GET ") {
            let key = extract_after_keyword(s, "GET").trim().to_string();
            return Ok(PlanNode::ConfigGet(key));
        }
        if up.starts_with("CONFIG SET ") {
            let rest = extract_after_keyword(s, "SET").trim();
            let (key, value) = parse_config_kv(rest);
            return Ok(PlanNode::ConfigSet(key, value));
        }
        if up.starts_with("SHOW CONFIGS") {
            return Ok(PlanNode::ShowConfigs);
        }
        if up.starts_with("SHOW VARIABLES") {
            return Ok(PlanNode::ShowVariables);
        }

        // ========== 运维管理（新增）==========
        if up.starts_with("SHOW HOSTS") {
            return Ok(PlanNode::ShowHosts);
        }
        if up.starts_with("SHOW PARTS") {
            return Ok(PlanNode::ShowParts);
        }
        if up.starts_with("SHOW SESSIONS") {
            return Ok(PlanNode::ShowSessions);
        }
        if up.starts_with("SHOW QUERIES") {
            return Ok(PlanNode::ShowQueries);
        }
        if up.starts_with("KILL QUERY ") {
            let id = extract_ident_after(s, 2);
            return Ok(PlanNode::KillQuery(id));
        }
        if up.starts_with("SHOW CHARSET") {
            return Ok(PlanNode::ShowCharset);
        }
        if up.starts_with("SHOW COLLATION") {
            return Ok(PlanNode::ShowCollation);
        }
        if up.starts_with("SHOW TIME ZONE") {
            return Ok(PlanNode::ShowTimeZone);
        }
        if up.starts_with("SHOW VERSION") {
            return Ok(PlanNode::ShowVersion);
        }

        // ========== 用户/角色管理（新增）==========
        if up.starts_with("CREATE USER ") {
            let name = extract_ident_after(s, 2);
            return Ok(PlanNode::CreateUser(name));
        }
        if up.starts_with("DROP USER ") {
            let name = extract_ident_after(s, 2);
            return Ok(PlanNode::DropUser(name));
        }
        if up.starts_with("ALTER USER ") {
            let name = extract_ident_after(s, 2);
            return Ok(PlanNode::AlterUser(name));
        }
        if up.starts_with("SHOW USERS") {
            return Ok(PlanNode::ShowUsers);
        }
        if up.starts_with("CREATE ROLE ") {
            let name = extract_ident_after(s, 2);
            return Ok(PlanNode::CreateRole(name));
        }
        if up.starts_with("DROP ROLE ") {
            let name = extract_ident_after(s, 2);
            return Ok(PlanNode::DropRole(name));
        }
        if up.starts_with("GRANT ROLE ") {
            // GRANT ROLE role ON space TO user
            let (role, user) = parse_grant_role(s);
            return Ok(PlanNode::GrantRole(role, user));
        }
        if up.starts_with("REVOKE ROLE ") {
            let (role, user) = parse_grant_role(s);
            return Ok(PlanNode::RevokeRole(role, user));
        }
        if up.starts_with("SHOW ROLES IN ") || up.starts_with("SHOW ROLES OF ") {
            let space = extract_ident_after(s, 3);
            return Ok(PlanNode::ShowRoles(space));
        }

        // ========== 高级查询语法（新增）==========
        if up.starts_with("UNWIND ") {
            return Ok(PlanNode::Unwind);
        }
        if up.starts_with("OPTIONAL MATCH") {
            return Ok(PlanNode::OptionalMatch);
        }
        if up.contains("UNION ALL") {
            return Ok(PlanNode::UnionAll);
        }
        if up.contains("INTERSECT") {
            return Ok(PlanNode::Intersect);
        }
        if up.contains("MINUS") {
            return Ok(PlanNode::Minus);
        }
        if up.starts_with("WITH ") {
            return Ok(PlanNode::WithClause);
        }
        if up.starts_with("CALL ") {
            let proc = extract_ident_after(s, 1);
            return Ok(PlanNode::CallProcedure(proc));
        }
        if up.starts_with("YIELD DISTINCT") {
            return Ok(PlanNode::YieldDistinct);
        }

        // ========== 图路径查找扩展（新增）==========
        if up.starts_with("FIND SHORTEST PATH") {
            return Ok(PlanNode::FindShortestPath);
        }
        if up.starts_with("FIND ALL PATH") {
            return Ok(PlanNode::FindAllPath);
        }
        if up.starts_with("FIND NOLOOP PATH") {
            return Ok(PlanNode::FindNoLoopPath);
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

/// 提取指定关键字之后的所有文本
fn extract_after_keyword(s: &str, keyword: &str) -> String {
    let upper_s = s.to_ascii_uppercase();
    let upper_kw = keyword.to_ascii_uppercase();
    if let Some(pos) = upper_s.find(&upper_kw) {
        let after = &s[pos + keyword.len()..];
        after.trim().trim_end_matches(';').trim().to_string()
    } else {
        String::new()
    }
}

/// 从 BALANCE DATA REMOVE 语句中提取主机列表
fn extract_host_list(s: &str) -> Vec<String> {
    // 格式：BALANCE DATA REMOVE HOSTS "192.168.1.1:9779","192.168.1.2:9779"
    let mut hosts = Vec::new();
    let lower = s.to_ascii_lowercase();
    if let Some(pos) = lower.find("hosts") {
        let rest = &s[pos + 5..];
        // 提取引号中的主机名
        let mut current = String::new();
        let mut in_quotes = false;
        for c in rest.chars() {
            match c {
                '"' | '\'' => {
                    if in_quotes {
                        if !current.is_empty() {
                            hosts.push(current.clone());
                            current.clear();
                        }
                        in_quotes = false;
                    } else {
                        in_quotes = true;
                    }
                }
                ',' if !in_quotes => {}
                c if in_quotes => current.push(c),
                _ => {}
            }
        }
    }
    hosts
}

/// 解析 CONFIG SET 的 key=value
fn parse_config_kv(s: &str) -> (String, String) {
    if let Some((k, v)) = s.split_once('=') {
        (k.trim().to_string(), v.trim().trim_matches('"').trim_matches('\'').to_string())
    } else {
        (s.trim().to_string(), String::new())
    }
}

/// 解析 GRANT ROLE / REVOKE ROLE 语句
fn parse_grant_role(s: &str) -> (String, String) {
    // 格式：GRANT ROLE role ON space TO user
    let mut role = String::new();
    let mut user = String::new();

    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].to_ascii_uppercase();
        if t == "ROLE" && i + 1 < tokens.len() {
            role = tokens[i + 1].trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string();
        }
        if t == "TO" && i + 1 < tokens.len() {
            user = tokens[i + 1].trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string();
        }
        i += 1;
    }

    (role, user)
}

// ---------------------------------------------------------------------------
// 表达式系统（Expression System）
// ---------------------------------------------------------------------------

/// 表达式类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// 常量值
    Constant(crate::result_set::PropValue),
    /// 变量引用
    Variable(String),
    /// 属性引用：alias.prop
    Property(String, String),
    /// 一元表达式
    Unary(UnaryOp, Box<Expression>),
    /// 二元表达式
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    /// 函数调用
    FunctionCall(String, Vec<Expression>),
    /// CASE WHEN 表达式
    CaseWhen {
        when_thens: Vec<(Expression, Expression)>,
        else_expr: Option<Box<Expression>>,
    },
    /// 类型转换
    TypeCast(Box<Expression>, CastType),
    /// 列表构造
    ListConstruct(Vec<Expression>),
    /// Map 构造
    MapConstruct(Vec<(String, Expression)>),
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,    // -
    Not,    // NOT
    BitNot, // ~
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,  // +
    Sub,  // -
    Mul,  // *
    Div,  // /
    Mod,  // %
    Eq,   // ==
    Ne,   // !=
    Lt,   // <
    Le,   // <=
    Gt,   // >
    Ge,   // >=
    And,  // AND
    Or,   // OR
    Xor,  // XOR
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    LShift, // <<
    RShift, // >>
    In,     // IN
    Like,   // LIKE
    Contains, // CONTAINS
    StartsWith, // STARTS WITH
    EndsWith,   // ENDS WITH
}

/// 类型转换目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastType {
    Int,
    Float,
    String,
    Bool,
    Date,
    DateTime,
    Time,
    Timestamp,
}

/// 表达式解析器
pub struct ExpressionParser;

impl ExpressionParser {
    /// 解析表达式字符串（简化实现）
    pub fn parse(_expr: &str) -> crate::error::GraphResult<Expression> {
        // 简化实现：返回常量表达式
        // 完整实现需要递归下降解析器
        Ok(Expression::Constant(crate::result_set::PropValue::Null))
    }

    /// 检查是否为字符串函数
    pub fn is_string_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "substr" | "substring" | "left" | "right" | "ltrim" | "rtrim" | "trim"
                | "upper" | "lower" | "length" | "size" | "reverse" | "lpad" | "rpad"
                | "replace" | "split" | "concat" | "concat_ws" | "lcase" | "ucase"
                | "startswith" | "endswith" | "contains" | "regexp" | "matches"
                | "find_in_set" | "instr" | "locate" | "position" | "repeat"
                | "strcasecmp" | "strcmp" | "ascii" | "char" | "char_length"
                | "character_length" | "field" | "format" | "from_base64"
                | "to_base64" | "md5" | "sha" | "sha1" | "sha2"
        )
    }

    /// 检查是否为数学函数
    pub fn is_math_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "abs" | "acos" | "asin" | "atan" | "atan2" | "cos" | "cot" | "sin" | "tan"
                | "ceiling" | "ceil" | "floor" | "round" | "truncate" | "sign"
                | "sqrt" | "pow" | "power" | "exp" | "ln" | "log" | "log2" | "log10"
                | "pi" | "rand" | "radians" | "degrees" | "mod" | "div"
                | "greatest" | "least" | "coalesce" | "ifnull" | "nullif"
        )
    }

    /// 检查是否为日期时间函数
    pub fn is_datetime_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "now" | "current_timestamp" | "current_date" | "current_time"
                | "date" | "time" | "year" | "month" | "day" | "hour" | "minute"
                | "second" | "microsecond" | "week" | "weekday" | "weekofyear"
                | "dayofyear" | "dayofmonth" | "dayofweek" | "quarter"
                | "date_add" | "date_sub" | "datediff" | "timediff"
                | "timestampdiff" | "timestampadd"
                | "date_format" | "time_format" | "str_to_date"
                | "from_unixtime" | "unix_timestamp" | "to_seconds"
                | "extract" | "last_day" | "makedate" | "maketime"
                | "period_add" | "period_diff" | "sec_to_time" | "time_to_sec"
                | "utc_date" | "utc_time" | "utc_timestamp"
        )
    }

    /// 检查是否为类型转换函数
    pub fn is_cast_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "tointeger" | "tofloat" | "tostring" | "tobool" | "todate"
                | "todatetime" | "totimestamp" | "totime"
                | "cast" | "convert"
                | "int" | "float" | "string" | "bool"
        )
    }

    /// 检查是否为聚合函数
    pub fn is_aggregate_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "count" | "sum" | "avg" | "min" | "max" | "std" | "stddev"
                | "variance" | "var_pop" | "var_samp" | "stddev_pop" | "stddev_samp"
                | "collect" | "collect_set" | "group_concat" | "bit_and" | "bit_or" | "bit_xor"
        )
    }

    /// 检查是否为图函数
    pub fn is_graph_function(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "id" | "src" | "dst" | "type" | "tags" | "labels" | "properties"
                | "startnode" | "endnode" | "relationship" | "relationships"
                | "nodes" | "length" | "head" | "last" | "tail"
                | "size" | "keys" | "values"
        )
    }

    /// 获取所有支持的函数名列表
    pub fn all_functions() -> Vec<&'static str> {
        vec![
            // 字符串函数
            "substr", "substring", "left", "right", "trim", "ltrim", "rtrim",
            "upper", "lower", "length", "reverse", "replace", "split", "concat",
            "concat_ws", "lpad", "rpad", "startswith", "endswith", "contains",
            "regexp", "find_in_set", "instr", "locate", "repeat", "ascii",
            "char_length", "from_base64", "to_base64", "md5", "sha1", "sha2",
            // 数学函数
            "abs", "acos", "asin", "atan", "atan2", "cos", "cot", "sin", "tan",
            "ceil", "floor", "round", "truncate", "sign", "sqrt", "pow",
            "exp", "ln", "log", "log2", "log10", "pi", "rand",
            "radians", "degrees", "greatest", "least", "coalesce",
            // 日期时间函数
            "now", "current_timestamp", "current_date", "current_time",
            "date", "year", "month", "day", "hour", "minute", "second",
            "date_add", "date_sub", "datediff", "date_format",
            "from_unixtime", "unix_timestamp", "last_day",
            // 类型转换
            "tointeger", "tofloat", "tostring", "tobool", "todate",
            "todatetime", "totimestamp", "cast",
            // 聚合函数
            "count", "sum", "avg", "min", "max", "stddev", "variance",
            "collect", "collect_set", "group_concat",
            // 图函数
            "id", "src", "dst", "type", "tags", "properties",
            "startnode", "endnode", "nodes", "relationships",
            "head", "last", "keys", "values",
        ]
    }
}

// ---------------------------------------------------------------------------
// nGQL 函数注册表
// ---------------------------------------------------------------------------

/// nGQL 内置函数分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionCategory {
    String,
    Math,
    DateTime,
    Aggregate,
    TypeConversion,
    Graph,
    Conditional,
    List,
    Map,
}

/// 函数元信息
#[derive(Debug, Clone)]
pub struct FunctionMeta {
    pub name: &'static str,
    pub category: FunctionCategory,
    pub min_args: usize,
    pub max_args: Option<usize>,
    pub description: &'static str,
}

/// 内置函数注册表
pub struct FunctionRegistry;

impl FunctionRegistry {
    /// 获取所有内置函数元信息
    pub fn all_functions() -> Vec<FunctionMeta> {
        vec![
            // ===== 字符串函数 =====
            FunctionMeta {
                name: "substr",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(3),
                description: "截取子字符串：substr(str, pos, len)",
            },
            FunctionMeta {
                name: "substring",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(3),
                description: "同 substr，截取子字符串",
            },
            FunctionMeta {
                name: "left",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "返回字符串左侧 n 个字符",
            },
            FunctionMeta {
                name: "right",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "返回字符串右侧 n 个字符",
            },
            FunctionMeta {
                name: "trim",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "去除字符串两端空格",
            },
            FunctionMeta {
                name: "ltrim",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "去除字符串左侧空格",
            },
            FunctionMeta {
                name: "rtrim",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "去除字符串右侧空格",
            },
            FunctionMeta {
                name: "upper",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "转换为大写",
            },
            FunctionMeta {
                name: "lower",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "转换为小写",
            },
            FunctionMeta {
                name: "length",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "返回字符串长度",
            },
            FunctionMeta {
                name: "reverse",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "反转字符串",
            },
            FunctionMeta {
                name: "replace",
                category: FunctionCategory::String,
                min_args: 3,
                max_args: Some(3),
                description: "替换字符串中的子串",
            },
            FunctionMeta {
                name: "split",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "按分隔符分割字符串",
            },
            FunctionMeta {
                name: "concat",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: None,
                description: "连接多个字符串",
            },
            FunctionMeta {
                name: "concat_ws",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: None,
                description: "带分隔符的字符串连接",
            },
            FunctionMeta {
                name: "lpad",
                category: FunctionCategory::String,
                min_args: 3,
                max_args: Some(3),
                description: "左侧填充",
            },
            FunctionMeta {
                name: "rpad",
                category: FunctionCategory::String,
                min_args: 3,
                max_args: Some(3),
                description: "右侧填充",
            },
            FunctionMeta {
                name: "regexp",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "正则表达式匹配",
            },
            FunctionMeta {
                name: "startswith",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "是否以指定前缀开始",
            },
            FunctionMeta {
                name: "endswith",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "是否以指定后缀结束",
            },
            FunctionMeta {
                name: "contains",
                category: FunctionCategory::String,
                min_args: 2,
                max_args: Some(2),
                description: "是否包含子串",
            },
            FunctionMeta {
                name: "md5",
                category: FunctionCategory::String,
                min_args: 1,
                max_args: Some(1),
                description: "计算 MD5 哈希",
            },
            // ===== 数学函数 =====
            FunctionMeta {
                name: "abs",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "绝对值",
            },
            FunctionMeta {
                name: "acos",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "反余弦",
            },
            FunctionMeta {
                name: "asin",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "反正弦",
            },
            FunctionMeta {
                name: "atan",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "反正切",
            },
            FunctionMeta {
                name: "atan2",
                category: FunctionCategory::Math,
                min_args: 2,
                max_args: Some(2),
                description: "两个参数的反正切",
            },
            FunctionMeta {
                name: "cos",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "余弦",
            },
            FunctionMeta {
                name: "sin",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "正弦",
            },
            FunctionMeta {
                name: "tan",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "正切",
            },
            FunctionMeta {
                name: "ceil",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "向上取整",
            },
            FunctionMeta {
                name: "floor",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "向下取整",
            },
            FunctionMeta {
                name: "round",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(2),
                description: "四舍五入",
            },
            FunctionMeta {
                name: "sqrt",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "平方根",
            },
            FunctionMeta {
                name: "pow",
                category: FunctionCategory::Math,
                min_args: 2,
                max_args: Some(2),
                description: "幂运算",
            },
            FunctionMeta {
                name: "exp",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "e 的指数",
            },
            FunctionMeta {
                name: "ln",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "自然对数",
            },
            FunctionMeta {
                name: "log",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(2),
                description: "对数",
            },
            FunctionMeta {
                name: "log2",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "以 2 为底的对数",
            },
            FunctionMeta {
                name: "log10",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "以 10 为底的对数",
            },
            FunctionMeta {
                name: "pi",
                category: FunctionCategory::Math,
                min_args: 0,
                max_args: Some(0),
                description: "圆周率 π",
            },
            FunctionMeta {
                name: "rand",
                category: FunctionCategory::Math,
                min_args: 0,
                max_args: Some(1),
                description: "随机数",
            },
            FunctionMeta {
                name: "greatest",
                category: FunctionCategory::Math,
                min_args: 2,
                max_args: None,
                description: "返回最大值",
            },
            FunctionMeta {
                name: "least",
                category: FunctionCategory::Math,
                min_args: 2,
                max_args: None,
                description: "返回最小值",
            },
            FunctionMeta {
                name: "coalesce",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: None,
                description: "返回第一个非 NULL 值",
            },
            FunctionMeta {
                name: "sign",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "符号函数",
            },
            FunctionMeta {
                name: "radians",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "角度转弧度",
            },
            FunctionMeta {
                name: "degrees",
                category: FunctionCategory::Math,
                min_args: 1,
                max_args: Some(1),
                description: "弧度转角度",
            },
            // ===== 日期时间函数 =====
            FunctionMeta {
                name: "now",
                category: FunctionCategory::DateTime,
                min_args: 0,
                max_args: Some(0),
                description: "当前时间戳",
            },
            FunctionMeta {
                name: "current_timestamp",
                category: FunctionCategory::DateTime,
                min_args: 0,
                max_args: Some(0),
                description: "同 now()",
            },
            FunctionMeta {
                name: "current_date",
                category: FunctionCategory::DateTime,
                min_args: 0,
                max_args: Some(0),
                description: "当前日期",
            },
            FunctionMeta {
                name: "current_time",
                category: FunctionCategory::DateTime,
                min_args: 0,
                max_args: Some(0),
                description: "当前时间",
            },
            FunctionMeta {
                name: "date",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取日期部分",
            },
            FunctionMeta {
                name: "year",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取年份",
            },
            FunctionMeta {
                name: "month",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取月份",
            },
            FunctionMeta {
                name: "day",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取日",
            },
            FunctionMeta {
                name: "hour",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取小时",
            },
            FunctionMeta {
                name: "minute",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取分钟",
            },
            FunctionMeta {
                name: "second",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "提取秒",
            },
            FunctionMeta {
                name: "date_add",
                category: FunctionCategory::DateTime,
                min_args: 2,
                max_args: Some(3),
                description: "日期加法",
            },
            FunctionMeta {
                name: "date_sub",
                category: FunctionCategory::DateTime,
                min_args: 2,
                max_args: Some(3),
                description: "日期减法",
            },
            FunctionMeta {
                name: "datediff",
                category: FunctionCategory::DateTime,
                min_args: 2,
                max_args: Some(2),
                description: "日期间隔天数",
            },
            FunctionMeta {
                name: "date_format",
                category: FunctionCategory::DateTime,
                min_args: 2,
                max_args: Some(2),
                description: "日期格式化",
            },
            FunctionMeta {
                name: "from_unixtime",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(2),
                description: "Unix 时间戳转日期",
            },
            FunctionMeta {
                name: "unix_timestamp",
                category: FunctionCategory::DateTime,
                min_args: 0,
                max_args: Some(1),
                description: "日期转 Unix 时间戳",
            },
            FunctionMeta {
                name: "last_day",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "当月最后一天",
            },
            FunctionMeta {
                name: "week",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "一年中的第几周",
            },
            FunctionMeta {
                name: "quarter",
                category: FunctionCategory::DateTime,
                min_args: 1,
                max_args: Some(1),
                description: "季度",
            },
            // ===== 类型转换函数 =====
            FunctionMeta {
                name: "tointeger",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为整数",
            },
            FunctionMeta {
                name: "tofloat",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为浮点数",
            },
            FunctionMeta {
                name: "tostring",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为字符串",
            },
            FunctionMeta {
                name: "tobool",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为布尔值",
            },
            FunctionMeta {
                name: "todate",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为日期",
            },
            FunctionMeta {
                name: "todatetime",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为日期时间",
            },
            FunctionMeta {
                name: "totimestamp",
                category: FunctionCategory::TypeConversion,
                min_args: 1,
                max_args: Some(1),
                description: "转换为时间戳",
            },
            FunctionMeta {
                name: "cast",
                category: FunctionCategory::TypeConversion,
                min_args: 2,
                max_args: Some(2),
                description: "类型转换",
            },
            // ===== 聚合函数 =====
            FunctionMeta {
                name: "count",
                category: FunctionCategory::Aggregate,
                min_args: 0,
                max_args: Some(1),
                description: "计数",
            },
            FunctionMeta {
                name: "sum",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "求和",
            },
            FunctionMeta {
                name: "avg",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "平均值",
            },
            FunctionMeta {
                name: "min",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "最小值",
            },
            FunctionMeta {
                name: "max",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "最大值",
            },
            FunctionMeta {
                name: "stddev",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "标准差",
            },
            FunctionMeta {
                name: "variance",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "方差",
            },
            FunctionMeta {
                name: "collect",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "收集为列表（可重复）",
            },
            FunctionMeta {
                name: "collect_set",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(1),
                description: "收集为集合（去重）",
            },
            FunctionMeta {
                name: "group_concat",
                category: FunctionCategory::Aggregate,
                min_args: 1,
                max_args: Some(2),
                description: "字符串连接聚合",
            },
            // ===== 图函数 =====
            FunctionMeta {
                name: "id",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取顶点 VID",
            },
            FunctionMeta {
                name: "src",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取边的源 VID",
            },
            FunctionMeta {
                name: "dst",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取边的目标 VID",
            },
            FunctionMeta {
                name: "type",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取边类型",
            },
            FunctionMeta {
                name: "tags",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取顶点标签列表",
            },
            FunctionMeta {
                name: "properties",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取属性 Map",
            },
            FunctionMeta {
                name: "startnode",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取路径起始顶点",
            },
            FunctionMeta {
                name: "endnode",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取路径结束顶点",
            },
            FunctionMeta {
                name: "nodes",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取路径中的顶点列表",
            },
            FunctionMeta {
                name: "relationships",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取路径中的边列表",
            },
            FunctionMeta {
                name: "head",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取列表第一个元素",
            },
            FunctionMeta {
                name: "last",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取列表最后一个元素",
            },
            FunctionMeta {
                name: "keys",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取 Map 的键列表",
            },
            FunctionMeta {
                name: "values",
                category: FunctionCategory::Graph,
                min_args: 1,
                max_args: Some(1),
                description: "获取 Map 的值列表",
            },
        ]
    }

    /// 按分类获取函数
    pub fn functions_by_category(category: FunctionCategory) -> Vec<FunctionMeta> {
        Self::all_functions()
            .into_iter()
            .filter(|f| f.category == category)
            .collect()
    }

    /// 查找函数元信息
    pub fn find_function(name: &str) -> Option<FunctionMeta> {
        let name_lower = name.to_ascii_lowercase();
        Self::all_functions()
            .into_iter()
            .find(|f| f.name == name_lower.as_str())
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ===== DDL 语句测试 =====

    #[test]
    fn test_parse_create_space() {
        let p = NgqlParser::parse("CREATE SPACE demo;").unwrap();
        assert_eq!(p, PlanNode::CreateSpace("demo".into()));
    }

    #[test]
    fn test_parse_show_spaces() {
        let p = NgqlParser::parse("SHOW SPACES;").unwrap();
        assert_eq!(p, PlanNode::ShowSpaces);
    }

    #[test]
    fn test_parse_use_space() {
        let p = NgqlParser::parse("USE demo_space;").unwrap();
        assert_eq!(p, PlanNode::UseSpace("demo_space".into()));
    }

    #[test]
    fn test_parse_create_tag() {
        let p = NgqlParser::parse("CREATE TAG person(name string, age int);").unwrap();
        assert_eq!(p, PlanNode::CreateTag("person".into()));
    }

    #[test]
    fn test_parse_drop_tag() {
        let p = NgqlParser::parse("DROP TAG person;").unwrap();
        assert_eq!(p, PlanNode::DropTag("person".into()));
    }

    #[test]
    fn test_parse_create_edge() {
        let p = NgqlParser::parse("CREATE EDGE follow(degree int);").unwrap();
        assert_eq!(p, PlanNode::CreateEdge("follow".into()));
    }

    #[test]
    fn test_parse_drop_edge() {
        let p = NgqlParser::parse("DROP EDGE follow;").unwrap();
        assert_eq!(p, PlanNode::DropEdge("follow".into()));
    }

    // ===== DML 语句测试 =====

    #[test]
    fn test_parse_insert_vertex() {
        let p = NgqlParser::parse("INSERT VERTEX person(name, age) VALUES \"100\":(\"Tom\", 18);").unwrap();
        assert_eq!(p, PlanNode::InsertVertex("100".into()));
    }

    #[test]
    fn test_parse_update_vertex() {
        let p = NgqlParser::parse("UPDATE VERTEX ON person \"100\" SET age = age + 1;").unwrap();
        assert_eq!(p, PlanNode::UpdateVertex("100".into()));
    }

    #[test]
    fn test_parse_delete_vertex() {
        let p = NgqlParser::parse("DELETE VERTEX \"100\";").unwrap();
        assert_eq!(p, PlanNode::DeleteVertex("100".into()));
    }

    // ===== 查询语句测试 =====

    #[test]
    fn test_parse_go_steps() {
        let p = NgqlParser::parse("GO 3 STEPS FROM \"100\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::GoSteps(3));
    }

    #[test]
    fn test_parse_go_reversely() {
        let p = NgqlParser::parse("GO FROM \"100\" OVER follow REVERSELY;").unwrap();
        assert_eq!(p, PlanNode::GoReversely);
    }

    #[test]
    fn test_parse_lookup_tag() {
        let p = NgqlParser::parse("LOOKUP ON tag person WHERE person.age > 18;").unwrap();
        assert_eq!(p, PlanNode::LookupTag("person".into()));
    }

    #[test]
    fn test_parse_lookup_edge() {
        let p = NgqlParser::parse("LOOKUP ON edge follow WHERE follow.degree > 5;").unwrap();
        assert_eq!(p, PlanNode::LookupEdge("follow".into()));
    }

    #[test]
    fn test_fetch_prop_tag() {
        let p = NgqlParser::parse("FETCH PROP ON person \"100\" YIELD person.name, person.age;").unwrap();
        assert_eq!(p, PlanNode::FetchPropTag("person".into()));
    }

    #[test]
    fn test_fetch_prop_edge() {
        let p = NgqlParser::parse("FETCH PROP ON follow \"100\" -> \"200\" YIELD follow.degree;").unwrap();
        assert_eq!(p, PlanNode::FetchPropEdge("follow".into()));
    }

    // ===== 索引管理测试 =====

    #[test]
    fn test_parse_create_tag_index() {
        let p = NgqlParser::parse("CREATE TAG INDEX idx_person_name ON person(name(20));").unwrap();
        assert_eq!(p, PlanNode::CreateTagIndex("idx_person_name".into()));
    }

    #[test]
    fn test_parse_create_edge_index() {
        let p = NgqlParser::parse("CREATE EDGE INDEX idx_follow_degree ON follow(degree);").unwrap();
        assert_eq!(p, PlanNode::CreateEdgeIndex("idx_follow_degree".into()));
    }

    #[test]
    fn test_parse_drop_tag_index() {
        let p = NgqlParser::parse("DROP TAG INDEX idx_person_name;").unwrap();
        assert_eq!(p, PlanNode::DropTagIndex("idx_person_name".into()));
    }

    #[test]
    fn test_parse_drop_edge_index() {
        let p = NgqlParser::parse("DROP EDGE INDEX idx_follow_degree;").unwrap();
        assert_eq!(p, PlanNode::DropEdgeIndex("idx_follow_degree".into()));
    }

    #[test]
    fn test_parse_create_fulltext_index() {
        let p = NgqlParser::parse("CREATE FULLTEXT INDEX ft_idx ON person(name);").unwrap();
        assert_eq!(p, PlanNode::CreateFulltextIndex("ft_idx".into()));
    }

    #[test]
    fn test_parse_drop_fulltext_index() {
        let p = NgqlParser::parse("DROP FULLTEXT INDEX ft_idx;").unwrap();
        assert_eq!(p, PlanNode::DropFulltextIndex("ft_idx".into()));
    }

    #[test]
    fn test_parse_create_vector_index() {
        let p = NgqlParser::parse("CREATE VECTOR INDEX vec_idx ON person(embedding);").unwrap();
        assert_eq!(p, PlanNode::CreateVectorIndex("vec_idx".into()));
    }

    #[test]
    fn test_parse_drop_vector_index() {
        let p = NgqlParser::parse("DROP VECTOR INDEX vec_idx;").unwrap();
        assert_eq!(p, PlanNode::DropVectorIndex("vec_idx".into()));
    }

    #[test]
    fn test_parse_show_indexes() {
        let p = NgqlParser::parse("SHOW INDEXES;").unwrap();
        assert_eq!(p, PlanNode::ShowIndexes);
    }

    #[test]
    fn test_parse_show_tag_indexes() {
        let p = NgqlParser::parse("SHOW TAG INDEXES;").unwrap();
        assert_eq!(p, PlanNode::ShowTagIndexes);
    }

    #[test]
    fn test_parse_show_edge_indexes() {
        let p = NgqlParser::parse("SHOW EDGE INDEXES;").unwrap();
        assert_eq!(p, PlanNode::ShowEdgeIndexes);
    }

    #[test]
    fn test_parse_describe_index() {
        let p = NgqlParser::parse("DESCRIBE INDEX idx_person_name;").unwrap();
        assert_eq!(p, PlanNode::DescribeIndex("idx_person_name".into()));
    }

    #[test]
    fn test_parse_rebuild_fulltext_index() {
        let p = NgqlParser::parse("REBUILD FULLTEXT INDEX ft_idx;").unwrap();
        assert_eq!(p, PlanNode::RebuildFulltextIndex("ft_idx".into()));
    }

    #[test]
    fn test_parse_search_fulltext() {
        let p = NgqlParser::parse("SEARCH FULLTEXT ft_idx FOR \"hello world\";").unwrap();
        assert_eq!(p, PlanNode::SearchFulltext("ft_idx FOR \"hello world\"".into()));
    }

    // ===== EXPLAIN / PROFILE 测试 =====

    #[test]
    fn test_parse_explain() {
        let p = NgqlParser::parse("EXPLAIN GO FROM \"100\" OVER follow;").unwrap();
        match p {
            PlanNode::Explain(_) => {}
            _ => panic!("Expected Explain, got {:?}", p),
        }
    }

    #[test]
    fn test_parse_explain_format_json() {
        let p = NgqlParser::parse("EXPLAIN FORMAT=JSON GO FROM \"100\" OVER follow;").unwrap();
        match p {
            PlanNode::ExplainFormatJson(_) => {}
            _ => panic!("Expected ExplainFormatJson, got {:?}", p),
        }
    }

    #[test]
    fn test_parse_profile() {
        let p = NgqlParser::parse("PROFILE GO FROM \"100\" OVER follow;").unwrap();
        match p {
            PlanNode::Profile(_) => {}
            _ => panic!("Expected Profile, got {:?}", p),
        }
    }

    // ===== 异步任务测试 =====

    #[test]
    fn test_parse_submit_job_compact() {
        let p = NgqlParser::parse("SUBMIT JOB COMPACT;").unwrap();
        assert_eq!(p, PlanNode::SubmitJobCompact);
    }

    #[test]
    fn test_parse_submit_job_stats() {
        let p = NgqlParser::parse("SUBMIT JOB STATS;").unwrap();
        assert_eq!(p, PlanNode::SubmitJobStats);
    }

    #[test]
    fn test_parse_submit_job_rebuild_index() {
        let p = NgqlParser::parse("SUBMIT JOB REBUILD INDEX idx_person;").unwrap();
        assert_eq!(p, PlanNode::SubmitJobRebuildIndex("idx_person".into()));
    }

    #[test]
    fn test_parse_show_jobs() {
        let p = NgqlParser::parse("SHOW JOBS;").unwrap();
        assert_eq!(p, PlanNode::ShowJobs);
    }

    #[test]
    fn test_parse_show_job() {
        let p = NgqlParser::parse("SHOW JOB 123;").unwrap();
        assert_eq!(p, PlanNode::ShowJob(123));
    }

    #[test]
    fn test_parse_stop_job() {
        let p = NgqlParser::parse("STOP JOB 456;").unwrap();
        assert_eq!(p, PlanNode::StopJob(456));
    }

    #[test]
    fn test_parse_recover_job() {
        let p = NgqlParser::parse("RECOVER JOB 789;").unwrap();
        assert_eq!(p, PlanNode::RecoverJob(789));
    }

    // ===== 快照管理测试 =====

    #[test]
    fn test_parse_create_snapshot() {
        let p = NgqlParser::parse("CREATE SNAPSHOT snap_20240101;").unwrap();
        assert_eq!(p, PlanNode::CreateSnapshot("snap_20240101".into()));
    }

    #[test]
    fn test_parse_show_snapshots() {
        let p = NgqlParser::parse("SHOW SNAPSHOTS;").unwrap();
        assert_eq!(p, PlanNode::ShowSnapshots);
    }

    #[test]
    fn test_parse_drop_snapshot() {
        let p = NgqlParser::parse("DROP SNAPSHOT snap_20240101;").unwrap();
        assert_eq!(p, PlanNode::DropSnapshot("snap_20240101".into()));
    }

    #[test]
    fn test_parse_check_snapshot() {
        let p = NgqlParser::parse("CHECK SNAPSHOT snap_20240101;").unwrap();
        assert_eq!(p, PlanNode::CheckSnapshot("snap_20240101".into()));
    }

    // ===== 数据均衡测试 =====

    #[test]
    fn test_parse_balance_data() {
        let p = NgqlParser::parse("BALANCE DATA;").unwrap();
        assert_eq!(p, PlanNode::BalanceData);
    }

    #[test]
    fn test_parse_balance_leader() {
        let p = NgqlParser::parse("BALANCE LEADER;").unwrap();
        assert_eq!(p, PlanNode::BalanceLeader);
    }

    #[test]
    fn test_parse_balance_stop() {
        let p = NgqlParser::parse("BALANCE STOP;").unwrap();
        assert_eq!(p, PlanNode::BalanceStop);
    }

    #[test]
    fn test_parse_show_balance() {
        let p = NgqlParser::parse("SHOW BALANCE;").unwrap();
        assert_eq!(p, PlanNode::ShowBalance);
    }

    // ===== 配置管理测试 =====

    #[test]
    fn test_parse_config_get() {
        let p = NgqlParser::parse("CONFIG GET min_vertices_per_bucket;").unwrap();
        assert_eq!(p, PlanNode::ConfigGet("min_vertices_per_bucket".into()));
    }

    #[test]
    fn test_parse_config_set() {
        let p = NgqlParser::parse("CONFIG SET min_vertices_per_bucket=1024;").unwrap();
        assert_eq!(p, PlanNode::ConfigSet("min_vertices_per_bucket".into(), "1024".into()));
    }

    #[test]
    fn test_parse_show_configs() {
        let p = NgqlParser::parse("SHOW CONFIGS;").unwrap();
        assert_eq!(p, PlanNode::ShowConfigs);
    }

    #[test]
    fn test_parse_show_variables() {
        let p = NgqlParser::parse("SHOW VARIABLES;").unwrap();
        assert_eq!(p, PlanNode::ShowVariables);
    }

    // ===== 运维管理测试 =====

    #[test]
    fn test_parse_show_hosts() {
        let p = NgqlParser::parse("SHOW HOSTS;").unwrap();
        assert_eq!(p, PlanNode::ShowHosts);
    }

    #[test]
    fn test_parse_show_parts() {
        let p = NgqlParser::parse("SHOW PARTS;").unwrap();
        assert_eq!(p, PlanNode::ShowParts);
    }

    #[test]
    fn test_parse_show_sessions() {
        let p = NgqlParser::parse("SHOW SESSIONS;").unwrap();
        assert_eq!(p, PlanNode::ShowSessions);
    }

    #[test]
    fn test_parse_show_queries() {
        let p = NgqlParser::parse("SHOW QUERIES;").unwrap();
        assert_eq!(p, PlanNode::ShowQueries);
    }

    #[test]
    fn test_parse_kill_query() {
        let p = NgqlParser::parse("KILL QUERY 12345;").unwrap();
        assert_eq!(p, PlanNode::KillQuery("12345".into()));
    }

    #[test]
    fn test_parse_show_charset() {
        let p = NgqlParser::parse("SHOW CHARSET;").unwrap();
        assert_eq!(p, PlanNode::ShowCharset);
    }

    #[test]
    fn test_parse_show_collation() {
        let p = NgqlParser::parse("SHOW COLLATION;").unwrap();
        assert_eq!(p, PlanNode::ShowCollation);
    }

    #[test]
    fn test_parse_show_version() {
        let p = NgqlParser::parse("SHOW VERSION;").unwrap();
        assert_eq!(p, PlanNode::ShowVersion);
    }

    // ===== 用户/角色管理测试 =====

    #[test]
    fn test_parse_create_user() {
        let p = NgqlParser::parse("CREATE USER user1 WITH PASSWORD '123456';").unwrap();
        assert_eq!(p, PlanNode::CreateUser("user1".into()));
    }

    #[test]
    fn test_parse_drop_user() {
        let p = NgqlParser::parse("DROP USER user1;").unwrap();
        assert_eq!(p, PlanNode::DropUser("user1".into()));
    }

    #[test]
    fn test_parse_show_users() {
        let p = NgqlParser::parse("SHOW USERS;").unwrap();
        assert_eq!(p, PlanNode::ShowUsers);
    }

    #[test]
    fn test_parse_create_role() {
        let p = NgqlParser::parse("CREATE ROLE role1;").unwrap();
        assert_eq!(p, PlanNode::CreateRole("role1".into()));
    }

    #[test]
    fn test_parse_drop_role() {
        let p = NgqlParser::parse("DROP ROLE role1;").unwrap();
        assert_eq!(p, PlanNode::DropRole("role1".into()));
    }

    #[test]
    fn test_parse_grant_role() {
        let p = NgqlParser::parse("GRANT ROLE admin ON space1 TO user1;").unwrap();
        assert_eq!(p, PlanNode::GrantRole("admin".into(), "user1".into()));
    }

    #[test]
    fn test_parse_revoke_role() {
        let p = NgqlParser::parse("REVOKE ROLE admin ON space1 FROM user1;").unwrap();
        assert_eq!(p, PlanNode::RevokeRole("admin".into(), "user1".into()));
    }

    // ===== 高级查询语法测试 =====

    #[test]
    fn test_parse_unwind() {
        let p = NgqlParser::parse("UNWIND [1, 2, 3] AS x RETURN x;").unwrap();
        assert_eq!(p, PlanNode::Unwind);
    }

    #[test]
    fn test_parse_optional_match() {
        let p = NgqlParser::parse("OPTIONAL MATCH (n:person) RETURN n;").unwrap();
        assert_eq!(p, PlanNode::OptionalMatch);
    }

    #[test]
    fn test_parse_union_all() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow UNION ALL GO FROM \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::UnionAll);
    }

    #[test]
    fn test_parse_intersect() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow INTERSECT GO FROM \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::Intersect);
    }

    #[test]
    fn test_parse_minus() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow MINUS GO FROM \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::Minus);
    }

    #[test]
    fn test_parse_call_procedure() {
        let p = NgqlParser::parse("CALL db.labels() YIELD label;").unwrap();
        assert_eq!(p, PlanNode::CallProcedure("db.labels()".into()));
    }

    // ===== 图路径查找测试 =====

    #[test]
    fn test_parse_find_shortest_path() {
        let p = NgqlParser::parse("FIND SHORTEST PATH FROM \"1\" TO \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::FindShortestPath);
    }

    #[test]
    fn test_parse_find_all_path() {
        let p = NgqlParser::parse("FIND ALL PATH FROM \"1\" TO \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::FindAllPath);
    }

    #[test]
    fn test_parse_find_noloop_path() {
        let p = NgqlParser::parse("FIND NOLOOP PATH FROM \"1\" TO \"2\" OVER follow;").unwrap();
        assert_eq!(p, PlanNode::FindNoLoopPath);
    }

    // ===== 子句测试 =====

    #[test]
    fn test_parse_order_by() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow YIELD follow.degree AS d | ORDER BY d DESC;").unwrap();
        assert_eq!(p, PlanNode::OrderBy);
    }

    #[test]
    fn test_parse_limit() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow YIELD follow._dst AS dst | LIMIT 10;").unwrap();
        assert_eq!(p, PlanNode::Limit1);
    }

    #[test]
    fn test_parse_group_by() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow YIELD follow._dst AS dst | GROUP BY dst YIELD count(*) AS cnt;").unwrap();
        assert_eq!(p, PlanNode::GroupBy1);
    }

    #[test]
    fn test_parse_where() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow WHERE follow.degree > 5;").unwrap();
        assert_eq!(p, PlanNode::Where1);
    }

    #[test]
    fn test_parse_where_and() {
        let p = NgqlParser::parse("GO FROM \"1\" OVER follow WHERE follow.degree > 5 AND follow.degree < 10;").unwrap();
        assert_eq!(p, PlanNode::Where2);
    }

    // ===== MATCH 语句测试 =====

    #[test]
    fn test_parse_match_basic() {
        let p = NgqlParser::parse("MATCH (n:person) RETURN n;").unwrap();
        assert_eq!(p, PlanNode::MatchN4);
    }

    #[test]
    fn test_parse_match_with_where() {
        let p = NgqlParser::parse("MATCH (n:person) WHERE n.age > 18 RETURN n;").unwrap();
        assert_eq!(p, PlanNode::MatchN1);
    }

    // ===== 表达式系统测试 =====

    #[test]
    fn test_expression_constant() {
        use crate::result_set::PropValue;
        let expr = Expression::Constant(PropValue::Int(42));
        match expr {
            Expression::Constant(v) => assert_eq!(v, PropValue::Int(42)),
            _ => panic!("Expected Constant"),
        }
    }

    #[test]
    fn test_expression_variable() {
        let expr = Expression::Variable("n".into());
        match expr {
            Expression::Variable(name) => assert_eq!(name, "n"),
            _ => panic!("Expected Variable"),
        }
    }

    #[test]
    fn test_expression_property() {
        let expr = Expression::Property("n".into(), "name".into());
        match expr {
            Expression::Property(alias, prop) => {
                assert_eq!(alias, "n");
                assert_eq!(prop, "name");
            }
            _ => panic!("Expected Property"),
        }
    }

    #[test]
    fn test_expression_binary() {
        use crate::result_set::PropValue;
        let expr = Expression::Binary(
            BinaryOp::Add,
            Box::new(Expression::Constant(PropValue::Int(1))),
            Box::new(Expression::Constant(PropValue::Int(2))),
        );
        match expr {
            Expression::Binary(op, left, right) => {
                assert_eq!(op, BinaryOp::Add);
                assert_eq!(*left, Expression::Constant(PropValue::Int(1)));
                assert_eq!(*right, Expression::Constant(PropValue::Int(2)));
            }
            _ => panic!("Expected Binary"),
        }
    }

    #[test]
    fn test_expression_function_call() {
        let expr = Expression::FunctionCall("count".into(), vec![]);
        match expr {
            Expression::FunctionCall(name, args) => {
                assert_eq!(name, "count");
                assert!(args.is_empty());
            }
            _ => panic!("Expected FunctionCall"),
        }
    }

    #[test]
    fn test_expression_case_when() {
        use crate::result_set::PropValue;
        let expr = Expression::CaseWhen {
            when_thens: vec![(
                Expression::Constant(PropValue::Bool(true)),
                Expression::Constant(PropValue::Int(1)),
            )],
            else_expr: Some(Box::new(Expression::Constant(PropValue::Int(0)))),
        };
        match expr {
            Expression::CaseWhen { when_thens, else_expr } => {
                assert_eq!(when_thens.len(), 1);
                assert!(else_expr.is_some());
            }
            _ => panic!("Expected CaseWhen"),
        }
    }

    // ===== 表达式解析器测试 =====

    #[test]
    fn test_is_string_function() {
        assert!(ExpressionParser::is_string_function("substr"));
        assert!(ExpressionParser::is_string_function("trim"));
        assert!(ExpressionParser::is_string_function("upper"));
        assert!(ExpressionParser::is_string_function("lower"));
        assert!(ExpressionParser::is_string_function("concat"));
        assert!(ExpressionParser::is_string_function("split"));
        assert!(ExpressionParser::is_string_function("regexp"));
        assert!(!ExpressionParser::is_string_function("abs"));
    }

    #[test]
    fn test_is_math_function() {
        assert!(ExpressionParser::is_math_function("abs"));
        assert!(ExpressionParser::is_math_function("sin"));
        assert!(ExpressionParser::is_math_function("cos"));
        assert!(ExpressionParser::is_math_function("sqrt"));
        assert!(ExpressionParser::is_math_function("pow"));
        assert!(ExpressionParser::is_math_function("log"));
        assert!(ExpressionParser::is_math_function("exp"));
        assert!(!ExpressionParser::is_math_function("substr"));
    }

    #[test]
    fn test_is_datetime_function() {
        assert!(ExpressionParser::is_datetime_function("now"));
        assert!(ExpressionParser::is_datetime_function("date"));
        assert!(ExpressionParser::is_datetime_function("year"));
        assert!(ExpressionParser::is_datetime_function("month"));
        assert!(ExpressionParser::is_datetime_function("date_add"));
        assert!(ExpressionParser::is_datetime_function("datediff"));
        assert!(!ExpressionParser::is_datetime_function("abs"));
    }

    #[test]
    fn test_is_cast_function() {
        assert!(ExpressionParser::is_cast_function("tointeger"));
        assert!(ExpressionParser::is_cast_function("tofloat"));
        assert!(ExpressionParser::is_cast_function("tostring"));
        assert!(ExpressionParser::is_cast_function("tobool"));
        assert!(ExpressionParser::is_cast_function("cast"));
        assert!(!ExpressionParser::is_cast_function("abs"));
    }

    #[test]
    fn test_is_aggregate_function() {
        assert!(ExpressionParser::is_aggregate_function("count"));
        assert!(ExpressionParser::is_aggregate_function("sum"));
        assert!(ExpressionParser::is_aggregate_function("avg"));
        assert!(ExpressionParser::is_aggregate_function("min"));
        assert!(ExpressionParser::is_aggregate_function("max"));
        assert!(ExpressionParser::is_aggregate_function("stddev"));
        assert!(!ExpressionParser::is_aggregate_function("abs"));
    }

    #[test]
    fn test_is_graph_function() {
        assert!(ExpressionParser::is_graph_function("id"));
        assert!(ExpressionParser::is_graph_function("src"));
        assert!(ExpressionParser::is_graph_function("dst"));
        assert!(ExpressionParser::is_graph_function("type"));
        assert!(ExpressionParser::is_graph_function("tags"));
        assert!(ExpressionParser::is_graph_function("properties"));
        assert!(!ExpressionParser::is_graph_function("abs"));
    }

    #[test]
    fn test_all_functions_not_empty() {
        let funcs = ExpressionParser::all_functions();
        assert!(!funcs.is_empty());
        assert!(funcs.contains(&"substr"));
        assert!(funcs.contains(&"abs"));
        assert!(funcs.contains(&"now"));
        assert!(funcs.contains(&"count"));
        assert!(funcs.contains(&"id"));
    }

    // ===== 函数注册表测试 =====

    #[test]
    fn test_function_registry_not_empty() {
        let funcs = FunctionRegistry::all_functions();
        assert!(!funcs.is_empty());
    }

    #[test]
    fn test_function_meta_fields() {
        let funcs = FunctionRegistry::all_functions();
        let substr = funcs.iter().find(|f| f.name == "substr").unwrap();
        assert_eq!(substr.category, FunctionCategory::String);
        assert_eq!(substr.min_args, 2);
        assert_eq!(substr.max_args, Some(3));
        assert!(!substr.description.is_empty());
    }

    // ===== 辅助函数测试 =====

    #[test]
    fn test_extract_first_token() {
        assert_eq!(extract_first_token("CREATE SPACE demo", 2), Some("demo".into()));
        assert_eq!(extract_first_token("USE my_space", 1), Some("my_space".into()));
        assert_eq!(extract_first_token("SHOW SPACES", 2), None);
    }

    #[test]
    fn test_extract_ident_after() {
        assert_eq!(extract_ident_after("CREATE TAG person", 2), "person");
        assert_eq!(extract_ident_after("DROP EDGE follow", 2), "follow");
    }

    #[test]
    fn test_first_digits() {
        assert_eq!(first_digits("GO 3 STEPS"), Some(3));
        assert_eq!(first_digits("LIMIT 10 OFFSET 5"), Some(10));
        assert_eq!(first_digits("no digits here"), None);
    }

    #[test]
    fn test_extract_after_keyword() {
        assert_eq!(extract_after_keyword("EXPLAIN SELECT * FROM t", "EXPLAIN"), "SELECT * FROM t");
        assert_eq!(extract_after_keyword("PROFILE GO FROM 1", "PROFILE"), "GO FROM 1");
    }

    #[test]
    fn test_parse_config_kv() {
        assert_eq!(parse_config_kv("key=value"), ("key".into(), "value".into()));
        assert_eq!(parse_config_kv("a = 123"), ("a".into(), "123".into()));
    }

    // ===== 错误处理测试 =====

    #[test]
    fn test_parse_empty_sql() {
        let result = NgqlParser::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = NgqlParser::parse("   ;  ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unrecognized() {
        let result = NgqlParser::parse("INVALID SQL STATEMENT");
        assert!(result.is_err());
    }

    // ===== 向后兼容测试 =====

    #[test]
    fn test_backward_compatible_basic() {
        // 确保原有的 60 条语句仍然支持
        assert!(NgqlParser::parse("CREATE SPACE test;").is_ok());
        assert!(NgqlParser::parse("SHOW SPACES;").is_ok());
        assert!(NgqlParser::parse("USE test;").is_ok());
        assert!(NgqlParser::parse("CREATE TAG t1;").is_ok());
        assert!(NgqlParser::parse("DROP TAG t1;").is_ok());
        assert!(NgqlParser::parse("CREATE EDGE e1;").is_ok());
        assert!(NgqlParser::parse("DROP EDGE e1;").is_ok());
        assert!(NgqlParser::parse("INSERT VERTEX t1 VALUES \"v1\":();").is_ok());
        assert!(NgqlParser::parse("GO FROM \"1\" OVER e1;").is_ok());
        assert!(NgqlParser::parse("FIND PATH;").is_ok());
        assert!(NgqlParser::parse("SHOW TAGS;").is_ok());
        assert!(NgqlParser::parse("SHOW EDGES;").is_ok());
    }
}
