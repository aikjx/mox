// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KG 连接器抽象 Trait
//!
//! 定义统一的图谱连接器接口，所有实现（HTTP / SDK / Mock）均遵循此契约。
//! 设计原则：
//!   - 面向 intent 分类与专家匹配场景，而非全量图谱操作
//!   - 同步接口（blocking），适配 intent.rs 的同步调用路径
//!   - 错误用 String 简化，便于在上层直接标记 degraded

use std::collections::BTreeMap;

use super::types::GraphSearchHit;

/// KG 连接器抽象 Trait（统一接口）
///
/// 所有图谱连接器实现均遵循此契约，便于：
///   - mock 测试
///   - 多实现切换（HTTP / SDK / 内存）
///   - 降级策略统一处理
pub trait KgConnector: Send + Sync {
    /// 激活扩散：从 seeds 出发，按 damping 衰减、rounds 轮扩散
    /// 返回 {node_label: score} 映射（intent.rs 会归一到 7 类）
    fn spread(
        &self,
        seeds: &[String],
        damping: f64,
        rounds: u32,
    ) -> Result<BTreeMap<String, f64>, String>;

    /// 混合检索：返回 top_k 条命中
    fn search(&self, query: &str, top_k: usize) -> Result<Vec<GraphSearchHit>, String>;

    /// 图谱是否可用（健康检查）
    fn available(&self) -> bool;

    /// 连接器名称（用于日志/trace）
    fn name(&self) -> &str;
}

// ================== 向后兼容别名 ==================

/// 旧版 trait 名称，保留用于向后兼容
///
/// 重构前代码使用 `KgHubConnector`，重构后统一为 `KgConnector`。
/// 此处保留 type alias 确保外部引用不报错。
pub use KgConnector as KgHubConnector;

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 KgHubConnector 是 KgConnector 的别名
    #[test]
    fn kg_hub_connector_is_alias_of_kg_connector() {
        // 编译期验证：trait alias 可用
        fn accepts_kg_hub<C: KgHubConnector>(_c: &C) {}
        fn accepts_kg<C: KgConnector>(c: &C) {
            accepts_kg_hub(c); // 必须能互相转换
        }

        // 用 Mock 验证（实际在 mock.rs 中实现，这里仅验证类型别名编译通过）
        struct Dummy;
        impl KgConnector for Dummy {
            fn spread(&self, _: &[String], _: f64, _: u32) -> Result<BTreeMap<String, f64>, String> {
                Ok(BTreeMap::new())
            }
            fn search(&self, _: &str, _: usize) -> Result<Vec<GraphSearchHit>, String> {
                Ok(Vec::new())
            }
            fn available(&self) -> bool { true }
            fn name(&self) -> &str { "dummy" }
        }

        let d = Dummy;
        accepts_kg(&d);
        assert_eq!(d.name(), "dummy");
    }
}
