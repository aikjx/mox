//! 插件宏 — Plugin Macros
//!
//! 提供插件入口函数标记和生命周期钩子定义。
//!
//! 注意：实际的过程宏（#[plugin_entry]）需要单独的proc-macro crate实现。
//! 这里提供文档说明和生命周期常量定义。

/// 插件入口函数标记（文档说明）
///
/// # 示例
///
/// ```ignore
/// use mox_plugin_sdk::prelude::*;
///
/// // 插件主入口函数（由宿主运行时调用）
/// // 函数签名必须是: async fn plugin_main(ctx: PluginContext) -> PluginResult<()>
/// async fn plugin_main(ctx: PluginContext) -> PluginResult<()> {
///     ctx.log_info("Plugin started");
///     Ok(())
/// }
/// ```
///
/// 实际的#[plugin_entry]过程宏应在独立的proc-macro crate中实现，
/// 它会：
/// 1. 生成WASM导出函数 `plugin_main`
/// 2. 自动初始化日志和panic处理
/// 3. 包装错误处理
pub mod plugin_entry {
    /// 入口函数名
    pub const FN_NAME: &str = "plugin_main";
    /// 入口函数签名说明
    pub const FN_SIGNATURE: &str = "async fn plugin_main(ctx: PluginContext) -> PluginResult<()>";
}

/// 插件生命周期钩子
pub mod lifecycle {
    /// 插件初始化钩子（在plugin_main之前调用）
    pub const INIT_FN: &str = "plugin_init";
    /// 插件关闭钩子（在插件卸载前调用）
    pub const SHUTDOWN_FN: &str = "plugin_shutdown";
    /// 插件主入口
    pub const MAIN_FN: &str = "plugin_main";
    /// 插件健康检查钩子
    pub const HEALTH_CHECK_FN: &str = "plugin_health_check";
    /// 插件配置更新钩子
    pub const CONFIG_UPDATE_FN: &str = "plugin_config_update";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_constants() {
        assert_eq!(lifecycle::INIT_FN, "plugin_init");
        assert_eq!(lifecycle::SHUTDOWN_FN, "plugin_shutdown");
        assert_eq!(lifecycle::MAIN_FN, "plugin_main");
        assert_eq!(lifecycle::HEALTH_CHECK_FN, "plugin_health_check");
        assert_eq!(lifecycle::CONFIG_UPDATE_FN, "plugin_config_update");
    }

    #[test]
    fn test_plugin_entry_constants() {
        assert_eq!(plugin_entry::FN_NAME, "plugin_main");
        assert!(plugin_entry::FN_SIGNATURE.contains("PluginContext"));
    }
}
