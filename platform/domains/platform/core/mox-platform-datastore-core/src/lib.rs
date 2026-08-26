//! MOX 平台通用数据存储核心
//! 万能 biz_data 单表 + 槽位映射 DAO + 版本链 + 嵌套 SAVEPOINT 事务

pub mod audit_chain;
pub mod dao;
pub mod port;
pub mod slot_allocator;
pub mod transaction;

pub mod ddl {
    pub const SQL: &str = include_str!("ddl.sql");
}

pub use crate::audit_chain::compute_hash;
pub use crate::dao::{Filter, ListResult, SortSpec, UniversalBizDAO, DDL_SQL};
pub use crate::port::{
    AuditLogEntry, EntityWithFields, EnumOption, FieldSpec, IamRepository, InMemoryIamRepo,
    InMemoryMetaRepo, MetaRepository, User, ValidationRule,
};
pub use crate::slot_allocator::{FieldSlotAllocator, SlotCategory, SlotInfo};
pub use crate::transaction::TxManager;
