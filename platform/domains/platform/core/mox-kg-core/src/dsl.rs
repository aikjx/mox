// mox-kg-core 查询DSL解析器：类SQL语法的图谱查询语言

use crate::error::{KgError, KgResult};
use crate::model::TraverseDirection;

/// DSL查询类型
#[derive(Debug, Clone, PartialEq)]
pub enum DslQueryType {
    /// GET：查询顶点
    Get,
    /// MATCH：路径匹配
    Match,
    /// SEARCH：全文搜索
    Search,
    /// COUNT：统计
    Count,
    /// DELETE：删除
    Delete,
}

/// WHERE条件
#[derive(Debug, Clone)]
pub struct WhereCondition {
    pub field: String,
    pub operator: String, // =, !=, >, <, >=, <=, CONTAINS, IN
    pub value: String,
}

/// DSL查询解析结果
#[derive(Debug, Clone)]
pub struct DslQuery {
    pub query_type: DslQueryType,
    /// 目标顶点类型（如 "product"、"case"，"*" 表示所有类型）
    pub target_type: String,
    /// 路径段（多跳查询）
    pub path_segments: Vec<PathSegment>,
    /// WHERE条件
    pub conditions: Vec<WhereCondition>,
    /// ORDER BY字段
    pub order_by: Option<String>,
    /// 排序方向
    pub order_desc: bool,
    /// LIMIT
    pub limit: Option<usize>,
    /// OFFSET
    pub offset: Option<usize>,
    /// GROUP BY字段
    pub group_by: Option<String>,
    /// 聚合函数
    pub aggregate: Option<String>, // COUNT, SUM, AVG, MIN, MAX
    /// 搜索关键词（SEARCH类型）
    pub search_keyword: Option<String>,
    /// 搜索字段（SEARCH类型）
    pub search_field: Option<String>,
}

/// 路径段（一跳）
#[derive(Debug, Clone)]
pub struct PathSegment {
    /// 边类型（如 "uses"、"belongs_to"，"*" 表示所有类型）
    pub edge_type: String,
    /// 遍历方向
    pub direction: TraverseDirection,
    /// 目标顶点类型
    pub target_type: String,
}

/// DSL解析器
pub struct DslParser;

impl DslParser {
    /// 解析DSL查询语句
    pub fn parse(dsl: &str) -> KgResult<DslQuery> {
        let dsl = dsl.trim();
        if dsl.is_empty() {
            return Err(KgError::DslParseError("empty query".to_string()));
        }

        // 识别查询类型
        let upper = dsl.to_uppercase();
        let query_type = if upper.starts_with("GET ") {
            DslQueryType::Get
        } else if upper.starts_with("MATCH ") {
            DslQueryType::Match
        } else if upper.starts_with("SEARCH ") {
            DslQueryType::Search
        } else if upper.starts_with("COUNT ") {
            DslQueryType::Count
        } else if upper.starts_with("DELETE ") {
            DslQueryType::Delete
        } else {
            return Err(KgError::DslParseError(format!(
                "unknown query type, must start with GET/MATCH/SEARCH/COUNT/DELETE: {}",
                &dsl[..dsl.len().min(30)]
            )));
        };

        // 去掉查询类型前缀
        let rest = match query_type {
            DslQueryType::Get => &dsl[4..],
            DslQueryType::Match => &dsl[6..],
            DslQueryType::Search => &dsl[7..],
            DslQueryType::Count => &dsl[6..],
            DslQueryType::Delete => &dsl[7..],
        }
        .trim();

        let mut query = DslQuery {
            query_type: query_type.clone(),
            target_type: "*".to_string(),
            path_segments: vec![],
            conditions: vec![],
            order_by: None,
            order_desc: false,
            limit: None,
            offset: None,
            group_by: None,
            aggregate: None,
            search_keyword: None,
            search_field: None,
        };

        // 处理SEARCH类型
        if query_type == DslQueryType::Search {
            // SEARCH product WHERE name CONTAINS 'iPhone'
            Self::parse_search(rest, &mut query)?;
            return Ok(query);
        }

        // 解析路径和目标类型
        let after_path = Self::parse_path(rest, &mut query)?;

        // 解析WHERE
        let after_where = if after_path.to_uppercase().contains("WHERE ") {
            Self::parse_where(&after_path, &mut query)?
        } else {
            after_path.to_string()
        };

        // 解析ORDER BY
        let after_order = if after_where.to_uppercase().contains("ORDER BY ") {
            Self::parse_order_by(&after_where, &mut query)?
        } else {
            after_where
        };

        // 解析GROUP BY
        let after_group = if after_order.to_uppercase().contains("GROUP BY ") {
            Self::parse_group_by(&after_order, &mut query)?
        } else {
            after_order
        };

        // 解析LIMIT/OFFSET
        Self::parse_limit_offset(&after_group, &mut query)?;

        Ok(query)
    }

    fn parse_search(rest: &str, query: &mut DslQuery) -> KgResult<String> {
        // SEARCH product WHERE name CONTAINS 'iPhone'
        let parts: Vec<&str> = rest.splitn(2, |c| c == ' ' || c == '\t').collect();
        query.target_type = parts[0].trim().to_string();

        if parts.len() > 1 {
            let where_part = parts[1].trim();
            if where_part.to_uppercase().starts_with("WHERE ") {
                let cond_str = &where_part[6..];
                // 解析 field CONTAINS 'value'
                let cond_str = cond_str.trim();
                if let Some(pos) = cond_str.to_uppercase().find("CONTAINS") {
                    let field = cond_str[..pos].trim().to_string();
                    let value_str = cond_str[pos + 8..].trim().trim_matches('\'').trim_matches('"').to_string();
                    query.search_field = Some(field);
                    query.search_keyword = Some(value_str);
                }
            }
        }

        Ok(String::new())
    }

    fn parse_path(rest: &str, query: &mut DslQuery) -> KgResult<String> {
        // 格式1: GET product WHERE ...
        // 格式2: GET case -[uses]-> product WHERE ...
        // 格式3: GET customer -[belongs_to_industry]-> case -[uses]-> product WHERE ...
        // 格式4: GET product <-[uses]- case WHERE ... (入边)

        let rest = rest.trim();

        // 检查是否包含路径箭头（出边 -[...]-> 或入边 <-[...]-）
        let has_out_arrow = rest.contains("-[") && rest.contains("]->");
        let has_in_arrow = rest.contains("<-[") && rest.contains("]-");

        if has_out_arrow || has_in_arrow {
            // 多跳路径查询
            let mut remaining = rest;
            let mut first = true;

            while (remaining.contains("-[") && remaining.contains("]->")) ||
                  (remaining.contains("<-[") && remaining.contains("]-")) {

                // 检测是出边还是入边
                let is_inbound = remaining.starts_with("<-[") ||
                    (remaining.find("<-[").is_some() &&
                     remaining.find("<-[").unwrap() < remaining.find("-[").unwrap_or(usize::MAX));

                let (arrow_start, arrow_end, edge_def, before_arrow) = if is_inbound {
                    // 入边格式: <-[type]-
                    let start = remaining.find("<-[").unwrap();
                    let end_marker = remaining[start..].find("]-").unwrap() + start + 2;
                    let before = remaining[..start].trim();
                    let edge = &remaining[start + 3..end_marker - 2]; // 去掉 <-[ 和 ]-
                    (start, end_marker, edge.to_string(), before.to_string())
                } else {
                    // 出边格式: -[type]->
                    let start = remaining.find("-[").unwrap();
                    let end_marker = remaining.find("]->").unwrap() + 3;
                    let before = remaining[..start].trim();
                    let edge = &remaining[start + 2..end_marker - 3]; // 去掉 -[ 和 ]->
                    (start, end_marker, edge.to_string(), before.to_string())
                };

                if first {
                    // 第一个是起始顶点类型
                    query.target_type = before_arrow.trim().to_string();
                    first = false;
                }

                // 边类型和方向
                let direction = if is_inbound { TraverseDirection::In } else { TraverseDirection::Out };
                let edge_type = edge_def.trim().to_string();

                // 解析箭头后的目标类型
                let after_arrow = &remaining[arrow_end..];
                let next_out = after_arrow.find("-[");
                let next_in = after_arrow.find("<-[");
                let next_arrow = match (next_out, next_in) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                let target_end = if let Some(pos) = next_arrow {
                    pos
                } else {
                    // 找WHERE/ORDER/LIMIT
                    let upper = after_arrow.to_uppercase();
                    let mut end = after_arrow.len();
                    for keyword in &[" WHERE ", " ORDER BY ", " LIMIT ", " GROUP BY "] {
                        if let Some(pos) = upper.find(keyword) {
                            end = end.min(pos);
                        }
                    }
                    end
                };

                let target_type = after_arrow[..target_end].trim().to_string();

                query.path_segments.push(PathSegment {
                    edge_type,
                    direction,
                    target_type,
                });

                remaining = &after_arrow[target_end..];
            }

            Ok(remaining.trim().to_string())
        } else {
            // 简单查询：GET product WHERE ...
            let parts: Vec<&str> = rest.splitn(2, |c| c == ' ' || c == '\t').collect();
            query.target_type = parts[0].trim().to_string();
            if parts.len() > 1 {
                Ok(parts[1].trim().to_string())
            } else {
                Ok(String::new())
            }
        }
    }

    fn parse_where(rest: &str, query: &mut DslQuery) -> KgResult<String> {
        let upper = rest.to_uppercase();
        let where_pos = upper.find("WHERE ").unwrap();
        let after_where = &rest[where_pos + 6..];

        // 找WHERE子句的结束位置（ORDER BY / GROUP BY / LIMIT）
        let upper_after = after_where.to_uppercase();
        let mut end = after_where.len();
        for keyword in &[" ORDER BY ", " GROUP BY ", " LIMIT "] {
            if let Some(pos) = upper_after.find(keyword) {
                end = end.min(pos);
            }
        }

        let where_clause = &after_where[..end];

        // 解析条件（支持AND连接）
        let conditions: Vec<&str> = where_clause
            .split(" AND ")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for cond in conditions {
            if let Some(c) = Self::parse_single_condition(cond) {
                query.conditions.push(c);
            }
        }

        Ok(after_where[end..].trim().to_string())
    }

    fn parse_single_condition(cond: &str) -> Option<WhereCondition> {
        let cond = cond.trim();
        // 支持的操作符：>=, <=, !=, =, >, <, CONTAINS, IN
        let operators = [">=", "<=", "!=", "CONTAINS", " IN ", "=", ">", "<"];

        for op in &operators {
            let upper = cond.to_uppercase();
            if let Some(pos) = upper.find(op) {
                let field = cond[..pos].trim().to_string();
                let value = cond[pos + op.len()..].trim().trim_matches('\'').trim_matches('"').to_string();
                let operator = if op.trim() == "IN" { "IN".to_string() } else { op.trim().to_string() };
                return Some(WhereCondition { field, operator, value });
            }
        }

        None
    }

    fn parse_order_by(rest: &str, query: &mut DslQuery) -> KgResult<String> {
        let upper = rest.to_uppercase();
        let pos = upper.find("ORDER BY ").unwrap();
        let after = &rest[pos + 9..];

        let upper_after = after.to_uppercase();
        let mut end = after.len();
        for keyword in &[" GROUP BY ", " LIMIT "] {
            if let Some(p) = upper_after.find(keyword) {
                end = end.min(p);
            }
        }

        let order_clause = after[..end].trim();
        let parts: Vec<&str> = order_clause.split_whitespace().collect();
        if !parts.is_empty() {
            query.order_by = Some(parts[0].to_string());
            if parts.len() > 1 && parts[1].to_uppercase() == "DESC" {
                query.order_desc = true;
            }
        }

        Ok(after[end..].trim().to_string())
    }

    fn parse_group_by(rest: &str, query: &mut DslQuery) -> KgResult<String> {
        let upper = rest.to_uppercase();
        let pos = upper.find("GROUP BY ").unwrap();
        let after = &rest[pos + 9..];

        let upper_after = after.to_uppercase();
        let end = if let Some(p) = upper_after.find(" LIMIT ") {
            p
        } else {
            after.len()
        };

        query.group_by = Some(after[..end].trim().to_string());
        Ok(after[end..].trim().to_string())
    }

    fn parse_limit_offset(rest: &str, query: &mut DslQuery) -> KgResult<()> {
        let upper = rest.to_uppercase();
        if let Some(pos) = upper.find("LIMIT ") {
            let after = &rest[pos + 6..];
            let parts: Vec<&str> = after.split(',').collect();
            if parts.len() == 1 {
                query.limit = parts[0].trim().parse().ok();
            } else if parts.len() >= 2 {
                query.offset = parts[0].trim().parse().ok();
                query.limit = parts[1].trim().parse().ok();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let q = DslParser::parse("GET product WHERE status = 'ACTIVE' ORDER BY created_at DESC LIMIT 20").unwrap();
        assert_eq!(q.query_type, DslQueryType::Get);
        assert_eq!(q.target_type, "product");
        assert_eq!(q.conditions.len(), 1);
        assert_eq!(q.conditions[0].field, "status");
        assert_eq!(q.conditions[0].value, "ACTIVE");
        assert_eq!(q.order_by.as_deref(), Some("created_at"));
        assert!(q.order_desc);
        assert_eq!(q.limit, Some(20));
    }

    #[test]
    fn test_parse_one_hop() {
        let q = DslParser::parse("GET case -[uses]-> product WHERE product.id = 'product:1'").unwrap();
        assert_eq!(q.target_type, "case");
        assert_eq!(q.path_segments.len(), 1);
        assert_eq!(q.path_segments[0].edge_type, "uses");
        assert_eq!(q.path_segments[0].target_type, "product");
        assert_eq!(q.path_segments[0].direction, TraverseDirection::Out);
    }

    #[test]
    fn test_parse_two_hop() {
        let q = DslParser::parse("GET customer -[belongs_to_industry]-> case -[uses]-> product WHERE customer.industry = '金融'").unwrap();
        assert_eq!(q.target_type, "customer");
        assert_eq!(q.path_segments.len(), 2);
        assert_eq!(q.path_segments[0].edge_type, "belongs_to_industry");
        assert_eq!(q.path_segments[0].target_type, "case");
        assert_eq!(q.path_segments[1].edge_type, "uses");
        assert_eq!(q.path_segments[1].target_type, "product");
    }

    #[test]
    fn test_parse_search() {
        let q = DslParser::parse("SEARCH product WHERE name CONTAINS 'iPhone'").unwrap();
        assert_eq!(q.query_type, DslQueryType::Search);
        assert_eq!(q.target_type, "product");
        assert_eq!(q.search_field.as_deref(), Some("name"));
        assert_eq!(q.search_keyword.as_deref(), Some("iPhone"));
    }

    #[test]
    fn test_parse_inbound_edge() {
        let q = DslParser::parse("GET product <-[uses]- case WHERE product.id = '1'").unwrap();
        assert_eq!(q.path_segments.len(), 1);
        assert_eq!(q.path_segments[0].direction, TraverseDirection::In);
        assert_eq!(q.path_segments[0].edge_type, "uses");
    }
}
