//! RBAC 4 级 clearance 与三策略调度模式（替代 Python require_level 装饰器）
//!
//! 注意：本模块是 **过程宏前的函数式实现**，不使用 proc_macro crate——
//! 原因：跨 workspace proc_macro 需要 1 个独立 crate + 稳定 syn/quote 版本冻结，
//! 最小交付路径先用函数式 `check_clearance(required, identity)`，后续 P3 长尾再补 proc-macro
//! `#[require_level(L2)]` 语法糖，功能等价。

use serde::{Deserialize, Serialize};

use crate::errors::{ClearanceLevelRepr, XiaobaiError, XiaobaiResult};
use crate::identity::OperatorIdentity;

/// 4 级 clearance：数字越大权限越高，高等级自动继承低等级
///
/// L0 Auditor（只读全局）
/// L1 Member（非破坏写：打开应用、设置音量、播放 TTS）
/// L2 Expert/Coordinator（剪贴板、键鼠 L2、中文粘贴）
/// L3 MoxAdmin（破坏性：关应用、丢回收站、截图、拖拽）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ClearanceLevel {
    L0 = 0,
    L1 = 1,
    L2 = 2,
    L3 = 3,
}

impl ClearanceLevel {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(v: u8) -> XiaobaiResult<Self> {
        Ok(match v {
            0 => ClearanceLevel::L0,
            1 => ClearanceLevel::L1,
            2 => ClearanceLevel::L2,
            3 => ClearanceLevel::L3,
            other => {
                return Err(XiaobaiError::InvalidArgument {
                    action: "ClearanceLevel::from_u8".into(),
                    param: "clearance_u8".into(),
                    value: other.to_string(),
                    hint: "合法范围 0..=3（L0 Auditor ~ L3 MoxAdmin）".into(),
                });
            }
        })
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            ClearanceLevel::L0 => "只读 (Auditor)",
            ClearanceLevel::L1 => "非破坏 (Member)",
            ClearanceLevel::L2 => "剪贴/键鼠 (Expert/Coordinator)",
            ClearanceLevel::L3 => "破坏性 (MoxAdmin)",
        }
    }
}

/// 三策略调度模式（与 Python operator.base DispatchMode 1:1 对齐）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DispatchMode {
    /// 默认：本地算子直干，异步发审计消息到联盟；桥断不阻塞
    LocalFirst,
    /// 先问联盟裁决 800ms 超时窗口；本地 `OPERATOR_UNSUPPORTED` 才转远程算子执行
    CloudFallback,
    /// 强制联盟裁决，桥断立即返回 BRIDGE_DISCONNECTED
    CloudOnly,
}

impl Default for DispatchMode {
    fn default() -> Self {
        DispatchMode::LocalFirst
    }
}

impl DispatchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DispatchMode::LocalFirst => "local_first",
            DispatchMode::CloudFallback => "cloud_fallback",
            DispatchMode::CloudOnly => "cloud_only",
        }
    }

    pub fn parse_loose(s: &str) -> XiaobaiResult<Self> {
        let t = s.trim().to_lowercase().replace(['-', '_', ' '], "");
        Ok(match t.as_str() {
            "localfirst" | "local" | "default" => DispatchMode::LocalFirst,
            "cloudfallback" | "fallback" | "degrade" => DispatchMode::CloudFallback,
            "cloudonly" | "cloud" | "remote" => DispatchMode::CloudOnly,
            other => {
                return Err(XiaobaiError::InvalidArgument {
                    action: "DispatchMode::parse_loose".into(),
                    param: "mode".into(),
                    value: other.into(),
                    hint: "合法取值：local_first / cloud_fallback / cloud_only".into(),
                });
            }
        })
    }
}

/// 鉴权核心函数：替代 Python `@require_level(Lx)` 装饰器
///
/// 逻辑：`identity.role → clearance`；若 `required` ≤ 身份 clearance → PERMIT。
/// 若动作支持 Own 语义（`own_qualified=true` 且 `identity.is_owner=true`）则 **额外 -1 级宽容**：
/// 例如 L2 Expert + Owner 可执行 L3 `move_to_trash(自己桌面上的文件)`。
pub fn check_clearance(
    action: &str,
    required: ClearanceLevel,
    identity: &OperatorIdentity,
    own_qualified: bool,
) -> XiaobaiResult<()> {
    let actual = identity.role.to_clearance_level();
    let effective_actual = if own_qualified && identity.is_owner {
        // Owner 宽容：升 1 级（L2 + Owner → 视为 L3；但 L0 Auditor + Owner 最多升到 L1）
        ClearanceLevel::from_u8(std::cmp::min(3, actual.as_u8().saturating_add(1)))?
    } else {
        actual
    };
    if effective_actual >= required {
        Ok(())
    } else {
        let reason = if own_qualified && !identity.is_owner {
            "该动作要求 L3 破坏性权限或资源所有者；当前身份均不满足"
        } else {
            match required {
                ClearanceLevel::L3 => "动作属于破坏性（关应用/删文件/截图/拖拽），需要 MoxAdmin 角色或 Owner",
                ClearanceLevel::L2 => "动作涉及剪贴板读写/键鼠输入，需要 Expert 或 Coordinator 角色",
                ClearanceLevel::L1 => "动作会改变系统状态（开应用/调音量/打字），需要至少 Member 角色",
                ClearanceLevel::L0 => "该动作即使 Auditor 也可调用——此处报错是内部 bug，请上报",
            }
        };
        Err(XiaobaiError::PermissionDenied {
            action: action.into(),
            required: ClearanceLevelRepr(required.as_u8()),
            identity: identity.stable_display(),
            reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{OperatorIdentity, RoleTag};

    /// 4 × 5 级 RBAC 矩阵（与 Python selftest fr13_rbac_4level_auth_matrix 对齐）
    #[test]
    fn rbac_matrix_l3_admin_only() {
        // close_app 需要 L3
        for (id, expect_ok) in [
            (OperatorIdentity::auditor(), false),
            (OperatorIdentity::member(), false),
            (OperatorIdentity::expert(), false),
            (OperatorIdentity::coord(), false),
            (OperatorIdentity::admin(), true),
        ] {
            let r = check_clearance("close_app", ClearanceLevel::L3, &id, false);
            assert_eq!(r.is_ok(), expect_ok, "identity={id:?}");
        }
    }

    #[test]
    fn rbac_owner_promotion_l2_to_l3() {
        // 专家是自己文件的 Owner：move_to_trash(L3, own_qualified=true) 放行
        let mut owner_expert = OperatorIdentity::expert();
        owner_expert.is_owner = true;
        let r = check_clearance("move_to_trash", ClearanceLevel::L3, &owner_expert, true);
        assert!(r.is_ok(), "专家+Owner 执行自己资源的 L3 动作应放行");
    }

    #[test]
    fn rbac_l0_cant_get_past_l1() {
        let r = check_clearance("open_app", ClearanceLevel::L1, &OperatorIdentity::auditor(), false);
        assert!(matches!(r.unwrap_err(), XiaobaiError::PermissionDenied { .. }));
    }

    #[test]
    fn dispatch_mode_case_and_dash_insensitive() {
        assert_eq!(DispatchMode::parse_loose(" CLOUD-Fallback ").unwrap(), DispatchMode::CloudFallback);
        assert_eq!(DispatchMode::parse_loose("LocalFirst").unwrap(), DispatchMode::LocalFirst);
    }
}
