// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 操作人身份：与 mox-system Role 枚举精确映射（5 角色 → 4 级 clearance）
//!
//! | mox-system::Role | 本模块 clearance | 说明 |
//! | ---------------- | ---------------- | ---- |
//! | Auditor          | L0 (0)           | 只读全局：进程列表、音量读取、文件存在、鼠标位置 |
//! | Member           | L1 (1)           | 非破坏性写：open_app、open_file_with_app、set_volume（非 0）、type_text 仅 ASCII |
//! | Expert           | L2 (2)           | 剪贴板、键鼠 L2 动作、中文 type_text（粘贴回退）、copy_to_clipboard |
//! | Coordinator      | L2 (2)           | 同 Expert（协调员与专家在桌面 L2 等价） |
//! | MoxAdmin         | L3 (3)           | 破坏性动作：close_app、move_to_trash、screenshot、set_volume 0 静音、mouse_drag |
//!
//! `is_owner` 为 true 时，`L2/L3` 的 Own 语义动作在 `Expert/Coordinator` 角色上额外放行（如删除自己创建的文件）。

use serde::{Deserialize, Serialize};

use crate::rbac::ClearanceLevel;
use crate::XiaobaiError;

/// 5 角色字符串枚举：**故意不用直接引用 mox_platform_system_core::Role**
///
/// 原因：mox-system 默认 feature 会拉 rusqlite bundled 编译链，cargo check 成本高；
/// 桌面端部署场景下身份来源是本地 BallWidget 的配置文件/环境变量，用字符串枚举即可。
/// 字符串值与 `mox-system/src/rbac.rs Role::label()` 的中文标签 + 英文枚举名双匹配：
/// "mox_admin" / "璇玑管理员" 都能映射到 Role::MoxAdmin。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RoleTag {
    MoxAdmin,
    Coordinator,
    Expert,
    Member,
    Auditor,
}

impl RoleTag {
    /// 从宽松字符串（含中文标签 / 枚举名大小写 / 空格）解析角色，失败返回 `XB-009 InvalidArgument`
    pub fn parse_loose(s: &str) -> Result<Self, XiaobaiError> {
        let t = s.trim().to_lowercase().replace([' ', '（', '）', '(', ')', '-', '_'], "");
        Ok(match t.as_str() {
            "moxadmin" | "璇玑管理员" | "admin" | "administrator" => RoleTag::MoxAdmin,
            "coordinator" | "协调员" | "coord" => RoleTag::Coordinator,
            "expert" | "专家" => RoleTag::Expert,
            "member" | "成员" | "普通成员" | "user" => RoleTag::Member,
            "auditor" | "审计员" | "只读" => RoleTag::Auditor,
            other => {
                return Err(XiaobaiError::InvalidArgument {
                    action: "parse_role".into(),
                    param: "role".into(),
                    value: other.into(),
                    hint: "合法取值：mox_admin/coordinator/expert/member/auditor 或中文标签".into(),
                });
            }
        })
    }

    /// 映射到 clearance level 整数（L0..L3，Coordinator 与 Expert 合并为 L2）
    pub fn to_clearance_level(self) -> ClearanceLevel {
        match self {
            RoleTag::Auditor => ClearanceLevel::L0,
            RoleTag::Member => ClearanceLevel::L1,
            RoleTag::Expert | RoleTag::Coordinator => ClearanceLevel::L2,
            RoleTag::MoxAdmin => ClearanceLevel::L3,
        }
    }

    pub fn label_zh(self) -> &'static str {
        use RoleTag::*;
        match self {
            MoxAdmin => "璇玑管理员",
            Coordinator => "协调员",
            Expert => "专家",
            Member => "成员",
            Auditor => "审计员",
        }
    }
}

/// 操作人身份（一次 dispatch_intent 的调用上下文）
///
/// - user_id 可为空（Guest 模式，默认为 Member 角色）
/// - role 为原始 mox-system 5 角色字符串标签；clearance 在 `check_clearance` 时动态计算
/// - is_owner 仅用于 TaskEditOwn / MoveToTrashOwn 等"自己资源"语义动作（桌面端：当前文件的 Owner SID == 当前登录用户 SID 时置 true）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorIdentity {
    pub user_id: String,
    pub role: RoleTag,
    pub is_owner: bool,
}

impl Default for OperatorIdentity {
    /// 默认：未登录本地用户 → Member 角色，非 Owner
    fn default() -> Self {
        Self {
            user_id: "local-guest".into(),
            role: RoleTag::Member,
            is_owner: false,
        }
    }
}

impl OperatorIdentity {
    pub fn new(user_id: impl Into<String>, role: RoleTag, is_owner: bool) -> Self {
        Self {
            user_id: user_id.into(),
            role,
            is_owner,
        }
    }

    /// 快捷构造 Auditor 只读身份（用于 selftest RBAC 矩阵）
    pub fn auditor() -> Self {
        Self::new("tester-audit", RoleTag::Auditor, false)
    }
    pub fn member() -> Self {
        Self::new("tester-member", RoleTag::Member, false)
    }
    pub fn expert() -> Self {
        Self::new("tester-expert", RoleTag::Expert, false)
    }
    pub fn coord() -> Self {
        Self::new("tester-coord", RoleTag::Coordinator, false)
    }
    pub fn admin() -> Self {
        Self::new("tester-admin", RoleTag::MoxAdmin, false)
    }

    /// 稳定标识串（用于 PermissionDenied 错误里 `identity:` 字段，可审计）
    pub fn stable_display(&self) -> String {
        format!(
            "uid={} role={} clearance={} owner={}",
            self.user_id,
            self.role.label_zh(),
            self.role.to_clearance_level().as_u8(),
            self.is_owner
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_tag_loose_parse_mox_admin_chinese() {
        let r = RoleTag::parse_loose("璇玑管理员").unwrap();
        assert_eq!(r, RoleTag::MoxAdmin);
        assert_eq!(r.to_clearance_level().as_u8(), 3);
    }

    #[test]
    fn role_tag_coordinator_and_expert_both_l2() {
        assert_eq!(
            RoleTag::parse_loose("Coordinator").unwrap().to_clearance_level().as_u8(),
            2
        );
        assert_eq!(
            RoleTag::parse_loose("专家").unwrap().to_clearance_level().as_u8(),
            2
        );
    }

    #[test]
    fn role_tag_parse_invalid_returns_xb_009() {
        let e = RoleTag::parse_loose("super_saiyan").unwrap_err();
        assert_eq!(e.as_error_code(), "XB-009");
    }

    #[test]
    fn default_identity_member_l1() {
        let id = OperatorIdentity::default();
        assert_eq!(id.role.to_clearance_level().as_u8(), 1);
    }
}
