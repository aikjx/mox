//! 数据设计 DDL · 由关联图谱自动生成（primiflow::assoc::primiflow_seed）
//! 对应 primiflow/SPEC.md §4 数据模型（PostgreSQL + pgvector）
//! 真实部署时在本文件基础上补 `pgvector` 扩展与 embedding 列即可。
pub const SCHEMA_DDL: &str = include_str!("ddl.sql");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ddl_has_six_tables() {
        let count = SCHEMA_DDL.matches("CREATE TABLE").count();
        assert_eq!(count, 6, "应覆盖 6 张核心表");
    }
}
