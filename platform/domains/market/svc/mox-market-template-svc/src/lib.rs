// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 草莓多平台 · 系统模板市场（Template Market）
//!
//! 这是"对话驱动全栈生成式开发平台"的**资产中枢**：
//!
//! 1. **模板 = 一个完整可复用系统的蓝图**：包含
//!    - `graph`：由对话/璇玑生成的 FlowGraph（业务功能 + 关联关系 + 流程图）
//!    - `artifacts`：由 codegen 生成的代码包（后端/DB/前端），可留空待生成
//!    - `tags`：业务域标签（商城 / 小说 / 论文 / 产品设计 / 影视 …），支持通用模块归类
//!    - `derived_from`：引用链（"引用下载"他人模板后二开）
//! 2. **四类核心操作**（对应你的诉求）：
//!    - `publish`   —— 上传/发布一个系统模板（也可从对话实时生成后落盘）
//!    - `list`      —— 浏览所有模板（按标签/关键词检索，支持"通用模块"复用）
//!    - `load`      —— 下载/加载模板到本地工程
//!    - `fork`      —— 引用他人模板生成派生模板（"引用下载后快速开发"）
//! 3. **持续学习**：`record_feedback` 把"某模板被复用/评分"的反馈沉淀，供后续生成优化。
//!
//! 所有模板以 JSON 持久化到 `templates/` 目录，幂等、可版本化、可走 Git 协作。

// ============================================================================
// 模块声明
// ============================================================================

pub mod constants;
mod template_market;

#[cfg(test)]
mod tests;

// ============================================================================
// 公开 API 重导出（保持向后兼容）
// ============================================================================

pub use constants::{CRATE_ID, CRATE_META, ENGINE_NAME};
pub use template_market::market::TemplateMarket;
pub use template_market::types::{Domain, MarketError, SystemTemplate};
