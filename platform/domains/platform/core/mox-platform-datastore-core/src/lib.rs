//! MOX 平台通用数据存储核心
//! 万能 biz_data 单表 + 槽位映射 DAO + 版本链 + 嵌套 SAVEPOINT 事务

pub mod port;
pub mod dao;
pub mod slot_allocator;
pub mod transaction;
pub mod audit_chain;

pub mod ddl {
    pub const SQL: &str = include_str!("ddl.sql");
}

pub use crate::dao::{UniversalBizDAO, Filter, SortSpec, ListResult, DDL_SQL};
pub use crate::slot_allocator::{FieldSlotAllocator, SlotInfo, SlotCategory};
pub use crate::transaction::TxManager;
pub use crate::audit_chain::compute_hash;
pub use crate::port::{FieldSpec, EntityWithFields, ValidationRule, EnumOption,
    MetaRepository, InMemoryMetaRepo,
    User, AuditLogEntry, IamRepository, InMemoryIamRepo};
