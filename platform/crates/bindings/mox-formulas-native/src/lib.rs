//! napi-rs 最小存根（真实实现会在下一个任务切片填入）。
//! 此文件必须存在以便 workspace 注册能解析 crate。
use napi_derive::napi;

#[napi]
pub fn mox_formulas_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
