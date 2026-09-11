//! OA/ERP 系统集成适配器模块
//!
//! 支持 SAP / Oracle / Workday / 北森 / 用友 / 金蝶 等主流 OA/ERP 系统
//! 提供统一的连接器管理、数据同步、字段映射、同步日志等能力

pub mod model;
pub mod connector;
pub mod api;

pub use model::*;
pub use connector::*;
