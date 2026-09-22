// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! MOX API 传输加密归一层
//!
//! # 一键加密：全链路接口 data 压缩 + 加密传输
//!
//! 单一环境变量开关 [`config::ENV_MODE`]（`MOX_API_CRYPTO`）即可为**所有挂载
//! [`middleware::crypto_layer`] 的服务**（网关 :3080 / 调度 :3100 / 执行 :3200 /
//! 注册 :3400）启用统一 API 信封（`mox-api-protocol::ApiResponse` 的 `data` 字段）
//! 的 gzip 压缩 + SM4-GCM（GM/T 0002-2012 + NIST SP 800-38D，国密认证加密）传输：
//!
//! | 值 | 行为 |
//! |---|---|
//! | 未设置 / `off` | 完全直通（零开销，向后兼容：现网明文客户端无感） |
//! | `sm4` | 启用压缩+加密（客户端须按 [`codec`] 协商，见下） |
//!
//! # 协商语义（能力探测，不破坏存量客户端）
//!
//! 客户端在请求头携带 `x-mox-crypto: sm4-gcm+gzip` 表示"我支持解密"，服务端
//! （开关开启时）才对该响应的 `data` 做 压缩→加密；未携带该头的请求（浏览器前端、
//! 旧脚本）仍得到明文信封 —— 因此"一键添加加密"不会击穿任何既有链路，加密能力
//! 随客户端升级逐维度铺开。请求方向同理：客户端把 JSON 体编码为
//! `{"crypto":{…}}` 并带协商头，服务端中间件透明解密后再交给业务 handler。
//!
//! 密钥：`MOX_API_CRYPTO_KEY` 取 32 位 hex（128-bit SM4 密钥）；未配置时使用
//! 内置开发密钥并打 WARN（仅限本地/演示，生产必须显式注入）。

pub mod client;
pub mod codec;
pub mod config;
pub mod middleware;

/// 国密 SM4-GCM 原语（自 `mox-data-standards-core` 归一化上移至 foundation 层，
/// 供数据标准域与本传输层共同引用，一处实现全链复用）
pub mod sm4_gcm;
