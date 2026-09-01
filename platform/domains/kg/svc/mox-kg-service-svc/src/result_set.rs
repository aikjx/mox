// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 标准 ResultSet（列名 / 行值数组）。
//!
//! Spec 要求：columns: Vec<String>, rows: Vec<Vec<PropValue>>，并提供 Display、ok_or_err。
//! LPA helper 在此文件提供 `#[deprecated]` 占位实现。

use serde::{Deserialize, Serialize};
use std::fmt;

/// PropValue：单元格枚举。对应 SPEC-V4 类型矩阵。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    F64(f64),
    Str(String),
    List(Vec<PropValue>),
    Map(Vec<(String, PropValue)>),
}

// ── 手写 Eq/Hash/PartialEq ─────────────────────────────────────────────
// f64 无法 derive `Eq`/`Hash`，故手写并保持三者语义一致：
//   · F64 按位级比较（`f64::to_bits`）——确保 `相等 ⇒ 同哈希` 契约成立，
//     消除 HashMap 分组/连接键（-0.0/0.0、NaN）的歧义；
//   · Map 按 key 排序后比较/哈希——Map 语义上无序，保证 `相等 ⇒ 同哈希`。
impl PartialEq for PropValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PropValue::Null, PropValue::Null) => true,
            (PropValue::Bool(a), PropValue::Bool(b)) => a == b,
            (PropValue::Int(a), PropValue::Int(b)) => a == b,
            (PropValue::F64(a), PropValue::F64(b)) => a.to_bits() == b.to_bits(),
            (PropValue::Str(a), PropValue::Str(b)) => a == b,
            (PropValue::List(a), PropValue::List(b)) => a == b,
            (PropValue::Map(a), PropValue::Map(b)) => {
                let mut x = a.clone();
                let mut y = b.clone();
                x.sort_by(|p, q| p.0.cmp(&q.0));
                y.sort_by(|p, q| p.0.cmp(&q.0));
                x == y
            }
            _ => false,
        }
    }
}

impl Eq for PropValue {}

impl std::hash::Hash for PropValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash as _;
        std::mem::discriminant(self).hash(state);
        match self {
            PropValue::Null => {}
            PropValue::Bool(b) => b.hash(state),
            PropValue::Int(i) => i.hash(state),
            PropValue::F64(d) => d.to_bits().hash(state),
            PropValue::Str(s) => s.hash(state),
            PropValue::List(l) => l.hash(state),
            PropValue::Map(m) => {
                let mut pairs = m.clone();
                pairs.sort_by(|a, b| a.0.cmp(&b.0));
                pairs.hash(state);
            }
        }
    }
}

impl fmt::Display for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropValue::Null => write!(f, "NULL"),
            PropValue::Bool(b) => write!(f, "{b}"),
            PropValue::Int(i) => write!(f, "{i}"),
            PropValue::F64(d) => write!(f, "{d}"),
            PropValue::Str(s) => write!(f, "\"{s}\""),
            PropValue::List(l) => {
                let items: Vec<String> = l.iter().map(|x| format!("{x}")).collect();
                write!(f, "[{}]", items.join(", "))
            }
            PropValue::Map(m) => {
                let items: Vec<String> = m.iter().map(|(k, v)| format!("{k}:{v}")).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
        }
    }
}

/// ResultSet：标准输出集合。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<PropValue>>,
    /// 可选：语句类型标签，便于测试断言
    #[serde(default)]
    pub kind_label: String,
    /// Optimizer 是否对本执行计划进行了剪枝。
    #[serde(default)]
    pub pruned: bool,
    /// 执行是否成功；false 且空列 = err。
    #[serde(default)]
    pub ok: bool,
    /// 错误消息（ok=false 时可能非空）
    #[serde(default)]
    pub error: String,
}

impl fmt::Display for ResultSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "ResultSet cols={} rows={}",
            self.columns.len(),
            self.rows.len()
        )?;
        if !self.columns.is_empty() {
            let head: Vec<String> = self.columns.iter().cloned().collect();
            writeln!(f, "| {} |", head.join(" | "))?;
        }
        for r in &self.rows {
            let cells: Vec<String> = r.iter().map(|x| format!("{x}")).collect();
            writeln!(f, "| {} |", cells.join(" | "))?;
        }
        Ok(())
    }
}

impl ResultSet {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<PropValue>>) -> Self {
        Self {
            columns,
            rows,
            ok: true,
            ..Default::default()
        }
    }

    /// 包装 Result<ResultSet> 语义：成功 ok=true，失败 error=msg + ok=false。
    pub fn ok_or_err<E: std::fmt::Display>(r: Result<Self, E>) -> Self {
        match r {
            Ok(mut ok) => {
                ok.ok = true;
                ok
            }
            Err(e) => ResultSet {
                columns: vec!["error".into()],
                rows: vec![vec![PropValue::Str(format!("{e}"))]],
                error: format!("{e}"),
                ok: false,
                ..Default::default()
            },
        }
    }
}

/// LPA helper（公共 API 已弃用，仅保留桩实现：返回空 communities）。
#[deprecated(
    since = "3.0.0",
    note = "LPA public API deprecated; use CNM (module degree greedy) via AlgoBridge::cnm for communities."
)]
pub fn lpa_communities_deprecated() -> Vec<Vec<String>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_pv_null_display() {
        assert_eq!(format!("{}", PropValue::Null), "NULL");
    }
    #[test]
    fn t_pv_bool() {
        assert_eq!(format!("{}", PropValue::Bool(true)), "true");
    }
    #[test]
    fn t_pv_int() {
        assert_eq!(format!("{}", PropValue::Int(7)), "7");
    }
    #[test]
    fn t_pv_f64() {
        let s = format!("{}", PropValue::F64(3.1415));
        assert!(s.contains("3.1415"));
    }
    #[test]
    fn t_pv_str() {
        assert_eq!(format!("{}", PropValue::Str("hi".into())), "\"hi\"");
    }
    #[test]
    fn t_pv_list() {
        let l = PropValue::List(vec![PropValue::Int(1), PropValue::Int(2)]);
        assert!(format!("{}", l).contains("1"));
    }
    #[test]
    fn t_pv_map() {
        let m = PropValue::Map(vec![("a".into(), PropValue::Int(1))]);
        assert!(format!("{}", m).contains("a:1"));
    }

    #[test]
    fn t_rs_new_display() {
        let r = ResultSet::new(vec!["name".into()], vec![vec![PropValue::Str("a".into())]]);
        let s = format!("{r}");
        assert!(s.contains("name"));
        assert!(s.contains("\"a\""));
    }

    #[test]
    fn t_rs_ok_or_err_ok() {
        let r = ResultSet::ok_or_err::<String>(Ok(ResultSet::new(
            vec!["c".into()],
            vec![vec![PropValue::Int(1)]],
        )));
        assert!(r.ok);
    }
    #[test]
    fn t_rs_ok_or_err_err() {
        let r = ResultSet::ok_or_err::<&str>(Err("boom"));
        assert!(!r.ok);
        assert_eq!(r.rows.len(), 1);
    }
}
