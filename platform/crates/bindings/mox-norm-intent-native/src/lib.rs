// napi build 存根（为 workspace 注册提供 lib 目标）
use napi_derive::napi;

#[napi]
pub fn mox_norm_intent_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
