// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Input 算子通用工具函数

use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::operator::ActionParam;

pub(crate) fn require_int(p: &ActionParam, action: &str, k: &str) -> Result<i64, XiaobaiError> {
    p.get_i64(k).ok_or_else(|| XiaobaiError::InvalidArgument {
        action: action.into(),
        param: k.to_string(),
        value: "<missing>".into(),
        hint: "需要整数参数".into(),
    })
}

pub(crate) fn enigo_check_ok() -> bool {
    // enigo 在无头 CI 里初始化会失败；这里提前探测（先 new 一下立即 drop）
    use enigo::*;
    Enigo::new(&Settings::default()).is_ok()
}
