// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 权限分配服务
//!
//! 负责角色绑定、权限查询、鉴权入口。

use std::sync::Arc;

use crate::error::*;
use crate::event::EventBus;
use crate::rbac::*;
use crate::store::Store;

#[derive(Clone)]
pub struct PermissionService {
    pub store: Arc<Store>,
    pub bus: EventBus,
}

impl PermissionService {
    pub fn new(store: Arc<Store>, bus: EventBus) -> Self {
        Self { store, bus }
    }

    /// 授予/追加角色绑定（幂等合并）
    pub async fn assign_role(&self, binding: RoleBinding) {
        let member_id = binding.member_id.clone();
        let role = binding.role;
        let scope = binding.scope.clone();
        let mut bindings = self.store.get_bindings(&member_id).await;
        bindings.retain(|b| !(b.role == role && b.scope == scope));
        bindings.push(binding);
        self.store.set_bindings(&member_id, bindings).await;
    }

    pub async fn bindings_of(&self, member_id: &str) -> Vec<RoleBinding> {
        self.store.get_bindings(member_id).await
    }

    pub async fn effective_permissions(&self, member_id: &str) -> Vec<Permission> {
        let mut out = Vec::new();
        for b in self.store.get_bindings(member_id).await {
            for p in b.role.effective_permissions() {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        out
    }

    /// 鉴权入口：返回 AppError::Forbidden 表示拒绝
    pub async fn authorize(
        &self,
        member_id: &str,
        perm: Permission,
        ctx: &ResourceCtx,
    ) -> Result<()> {
        let bindings = self.store.get_bindings(member_id).await;
        match authorize(member_id, &bindings, perm, ctx) {
            Authz::Allowed => Ok(()),
            Authz::Denied(reason) => Err(AppError::Forbidden(reason)),
        }
    }
}
