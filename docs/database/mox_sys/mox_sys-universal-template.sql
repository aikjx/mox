-- =============================================================================
-- mox_sys UNIVERSAL TEMPLATE  —  mox 模块化系统架构 · 归一化 · 企业级 · 可扩展 通用数据库内核
-- =============================================================================
-- 目标后端 : MySQL 8.3+ / InnoDB / utf8mb4（PostgreSQL / SQLite 经 adapter，见 cross-database.md）
-- ID 策略  : 应用层生成 UUID v7，统一存 BINARY(16)。禁止自增 / UUID_SHORT() / 触发器发号。
-- 时间策略 : UTC DATETIME(3)。API 暴露 RFC3339；epoch 仅作传输格式。
-- 租户策略 : 每个租户业务表必有 tenant_id；tenant_id IS NULL 仅表示平台级共享资源（scope=G）。
-- 删除策略 : 业务软删除 deleted_at；不可变审计/凭证/迁移记录不得 UPDATE 覆盖。
-- 外键策略 : 默认无跨服务物理外键；单体部署可在发布期追加；跨模块一致性由 outbox + 事件保证。
--
-- 【单一归一化母版 · 一键安装】
--   本文件是 mox_sys 的【唯一权威 DDL】，包含：系统内核 19 维度包（P01–P16,P18,P19）
--   + 模块注册与知识图谱（P17，mox_sys_* 10 表）。不再有第二个并行母版，不存在重复定义。
--   安装：一条命令把本文件灌入目标库即可（见同目录 install.ps1 / install.sh）。
--     1) 新建产品库：直接执行本文件（自动 CREATE DATABASE IF NOT EXISTS mox_v3）。
--     2) 已装 mox_v3 且有同名旧表：先跑 mox-v3.0-migration-plan.md 的迁移脚本对齐，
--        再用本文件补齐缺失表（本文件对已有表不会重复定义）。
--     3) 单库 MySQL 8.0.16+ 单体强一致部署：可选执行 mox_sys-fk.sql 追加 81 条物理外键
--        （多态/溯源列除外，全部 RESTRICT；分库/事件一致性部署跳过）。
--   ⚠ 本文件与 mox-v3.0-baseline.sql 的区别：baseline 是当前 mox 产品的“现状落库”，
--     本文件是“归一化目标母版”。二者不是安装关系，不要叠加执行。
--
-- 【归一化级别】BCNF 基线，详见《mox 模块化系统架构企业级数据库模板.md》第 2 章
--   1NF 原子列 + 明确主键 / 2NF 无部分依赖 / 3NF 无传递依赖（字典·配置外提）
--   BCNF 决定因子均为候选键（短码字典化） / 4NF 多值依赖拆表 / 5NF ReBAC 三元组
--
-- 【NULL 安全唯一性 · 强制约定】
--   MySQL 的 UNIQUE 允许多个 NULL，因此“可空 tenant_id + UNIQUE(tenant_id, code)”会让
--   平台级共享行重复插入。凡 tenant_id 可为 NULL 且参与唯一键的表，一律增加生成列：
--     tenant_scope_id BINARY(16) GENERATED ALWAYS AS (COALESCE(tenant_id, UNHEX('00'*16))) STORED
--   并用 tenant_scope_id 替代 tenant_id 进入 UNIQUE。（该模式与 baseline 的
--   sys_setting.scope_key / sys_identity_provider.tenant_scope_id 保持一致。）
-- =============================================================================

SET NAMES utf8mb4 COLLATE utf8mb4_0900_ai_ci;
CREATE DATABASE IF NOT EXISTS `mox_v3`
  CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci;
USE `mox_v3`;
SET sql_mode = 'STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION';

-- 通用基类字段（每表手写以保证可移植）：
--   id BINARY(16) / tenant_id BINARY(16) / created_at / updated_at
--   deleted_at / row_version BIGINT UNSIGNED DEFAULT 1 / created_by / updated_by / deleted_by

-- =============================================================================
-- P06 · 代码字典（归一化核心：所有短码只定义一次，消灭散落 CHECK 约束与魔法串）
-- =============================================================================

CREATE TABLE `sys_enum_type` (
  `id` BINARY(16) NOT NULL,
  `enum_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `enum_name` VARCHAR(160) NOT NULL,
  `owner_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'mox_sys',
  `is_system` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N' COMMENT 'Y=平台保留不可删',
  `description` VARCHAR(500) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL,
  `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_enum_type_code` (`enum_code`),
  KEY `idx_enum_type_status` (`status`, `enum_code`)
) ENGINE=InnoDB COMMENT='代码字典类型（归一化：替代各表散落 CHECK 约束）';

CREATE TABLE `sys_enum_item` (
  `id` BINARY(16) NOT NULL,
  `enum_type_id` BINARY(16) NOT NULL,
  `item_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `item_name` VARCHAR(160) NOT NULL,
  `sort_no` INT NOT NULL DEFAULT 0,
  `is_default` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `ext_attrs` JSON DEFAULT NULL COMMENT '可演进展示/语义属性，不用于强过滤',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL,
  `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_enum_item` (`enum_type_id`, `item_code`),
  KEY `idx_enum_item_status` (`enum_type_id`, `status`, `sort_no`)
) ENGINE=InnoDB COMMENT='代码字典取值（4NF：type/code/label/attr 各成一列）';

-- =============================================================================
-- P02 · 租户 / 企业 / 组织（隔离三键：tenant_id / enterprise_id / org_unit_id 不混用）
-- =============================================================================

CREATE TABLE `sys_tenant` (
  `id` BINARY(16) NOT NULL COMMENT 'UUID v7',
  `tenant_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `tenant_name` VARCHAR(160) NOT NULL,
  `tenant_mode` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'logical' COMMENT 'logical/physical/hybrid，引用 sys_enum_type[tenant_mode]',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `plan_code` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'free',
  `data_region` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `timezone` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'UTC',
  `locale` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'zh-CN',
  `created_by` BINARY(16) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_by` BINARY(16) DEFAULT NULL,
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL,
  `deleted_by` BINARY(16) DEFAULT NULL,
  `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_tenant_code` (`tenant_code`),
  KEY `idx_tenant_status` (`status`, `updated_at`)
) ENGINE=InnoDB COMMENT='租户主表（隔离根）；配置走 sys_setting(scope=T)，不再冗余 settings_ref';

CREATE TABLE `sys_enterprise` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) NOT NULL,
  `enterprise_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `enterprise_name` VARCHAR(200) NOT NULL,
  `legal_name` VARCHAR(200) DEFAULT NULL,
  `registration_no` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `enterprise_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'company',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `is_default` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_by` BINARY(16) DEFAULT NULL, `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_enterprise_code` (`tenant_id`,`enterprise_code`),
  KEY `idx_enterprise_status` (`tenant_id`,`status`,`updated_at`),
  KEY `idx_enterprise_reg_no` (`tenant_id`,`registration_no`)
) ENGINE=InnoDB COMMENT='租户内企业主体';

CREATE TABLE `sys_org_unit` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) NOT NULL,
  `enterprise_id` BINARY(16) NOT NULL,
  `parent_id` BINARY(16) DEFAULT NULL,
  `org_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `org_name` VARCHAR(160) NOT NULL,
  `org_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'dept' COMMENT 'root/company/dept/team/virtual',
  `path_key` VARCHAR(2000) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT '物化路径，便于子树查询',
  `level_no` INT UNSIGNED NOT NULL DEFAULT 0,
  `sort_no` INT NOT NULL DEFAULT 0,
  `manager_id` BINARY(16) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_by` BINARY(16) DEFAULT NULL, `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_org_code` (`tenant_id`,`enterprise_id`,`org_code`),
  KEY `idx_org_parent` (`tenant_id`,`enterprise_id`,`parent_id`,`sort_no`),
  KEY `idx_org_path` (`tenant_id`,`enterprise_id`,`path_key`(191))
) ENGINE=InnoDB COMMENT='企业组织树节点';

-- =============================================================================
-- P01 · 身份主体（全局唯一，与租户/组织解耦）
-- =============================================================================

CREATE TABLE `sys_user` (
  `id` BINARY(16) NOT NULL,
  `login_name` VARCHAR(128) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
  `display_name` VARCHAR(160) NOT NULL,
  `email` VARCHAR(254) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin DEFAULT NULL,
  `phone` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `password_hash` VARBINARY(255) DEFAULT NULL COMMENT '只存 Argon2id/bcrypt 摘要',
  `user_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'person',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `locale` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'zh-CN',
  `timezone` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'UTC',
  `avatar_object_id` BINARY(16) DEFAULT NULL COMMENT '指向 sys_file_object',
  `last_login_at` DATETIME(3) DEFAULT NULL, `last_login_ip` VARBINARY(16) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_user_login` (`login_name`),
  KEY `idx_user_email` (`email`), KEY `idx_user_status` (`status`,`updated_at`)
) ENGINE=InnoDB COMMENT='全局身份主体';

-- 身份提供方（IdP）与身份绑定分离：3NF，避免 provider 属性在每条绑定上重复
CREATE TABLE `sys_identity_provider` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `provider_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `provider_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'oidc/saml/ldap/wechat/dingtalk',
  `display_name` VARCHAR(120) NOT NULL,
  `config_ciphertext` VARBINARY(8192) DEFAULT NULL COMMENT '密文配置，不落地明文',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_idp_code` (`tenant_scope_id`,`provider_code`),
  KEY `idx_idp_type` (`tenant_id`,`provider_type`,`status`)
) ENGINE=InnoDB COMMENT='SSO/LDAP 身份提供商';

CREATE TABLE `sys_user_identity` (
  `id` BINARY(16) NOT NULL,
  `user_id` BINARY(16) NOT NULL,
  `provider_id` BINARY(16) NOT NULL,
  `external_subject` VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
  `claims` JSON DEFAULT NULL,
  `last_seen_at` DATETIME(3) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_identity_subject` (`provider_id`, `external_subject`),
  KEY `idx_identity_user` (`user_id`, `status`)
) ENGINE=InnoDB COMMENT='外部身份到本地用户映射（多值依赖→独立表，4NF）';

CREATE TABLE `sys_user_mfa` (
  `id` BINARY(16) NOT NULL, `user_id` BINARY(16) NOT NULL,
  `factor_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'totp/webauthn/sms/email',
  `secret_ref` BINARY(16) DEFAULT NULL COMMENT '密文引用，不落地明文',
  `last_used_at` DATETIME(3) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_user_mfa` (`user_id`,`factor_type`),
  KEY `idx_user_mfa_user` (`user_id`,`status`)
) ENGINE=InnoDB COMMENT='用户 MFA 因子';

CREATE TABLE `sys_user_session` (
  `id` BINARY(16) NOT NULL, `user_id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL,
  `session_hash` BINARY(32) NOT NULL COMMENT '只存摘要，不存 token 明文',
  `ip_addr` VARBINARY(16) DEFAULT NULL, `user_agent` VARCHAR(1000) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `last_seen_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `expires_at` DATETIME(3) NOT NULL, `revoked_at` DATETIME(3) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_session_hash` (`session_hash`),
  KEY `idx_session_user` (`user_id`,`revoked_at`,`expires_at`)
) ENGINE=InnoDB COMMENT='登录会话（只存摘要，高吞吐独立归档）';

-- =============================================================================
-- P03 · 成员关系（身份↔租户、身份↔组织 拆开，避免传递依赖）
-- =============================================================================

CREATE TABLE `sys_tenant_member` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `user_id` BINARY(16) NOT NULL,
  `member_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'user',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `is_owner` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `default_enterprise_id` BINARY(16) DEFAULT NULL, `default_org_unit_id` BINARY(16) DEFAULT NULL,
  `joined_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `expires_at` DATETIME(3) DEFAULT NULL,
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_member_tenant_user` (`tenant_id`,`user_id`),
  KEY `idx_member_user` (`user_id`,`status`), KEY `idx_member_tenant_status` (`tenant_id`,`status`)
) ENGINE=InnoDB COMMENT='用户与租户成员关系';

CREATE TABLE `sys_org_member` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `enterprise_id` BINARY(16) NOT NULL,
  `org_unit_id` BINARY(16) NOT NULL, `member_id` BINARY(16) NOT NULL,
  `relation_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'member' COMMENT 'member/manager/owner',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_org_member` (`org_unit_id`,`member_id`,`relation_type`),
  KEY `idx_org_member_user` (`tenant_id`,`member_id`,`status`), KEY `idx_org_member_unit` (`org_unit_id`,`status`)
) ENGINE=InnoDB COMMENT='用户与企业组织节点关系';

-- =============================================================================
-- P04 · 授权（RBAC + ABAC + ReBAC 三正交模型，互不混入）
-- =============================================================================

CREATE TABLE `sys_resource` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL COMMENT 'NULL=平台共享资源',
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `resource_code` VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `resource_name` VARCHAR(200) NOT NULL,
  `resource_kind` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'api' COMMENT 'api/menu/data/field',
  `module_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_resource_code` (`tenant_scope_id`, `resource_code`),
  KEY `idx_resource_module` (`module_code`,`status`)
) ENGINE=InnoDB COMMENT='受保护资源（api/menu/data/field）';

CREATE TABLE `sys_permission` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL,
  `resource_id` BINARY(16) NOT NULL,
  `action_code` VARCHAR(48) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'view/create/edit/delete/export/approve',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_permission` (`resource_id`,`action_code`),
  KEY `idx_permission_tenant` (`tenant_id`,`status`)
) ENGINE=InnoDB COMMENT='资源+动作 权限原语';

CREATE TABLE `sys_role` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `role_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `role_name` VARCHAR(160) NOT NULL,
  `role_kind` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'business' COMMENT 'system/business',
  `data_scope` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'self',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `deleted_by` BINARY(16) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_role_code` (`tenant_id`,`role_code`),
  KEY `idx_role_tenant` (`tenant_id`,`status`)
) ENGINE=InnoDB COMMENT='角色（RBAC）';

CREATE TABLE `sys_role_permission` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `role_id` BINARY(16) NOT NULL, `permission_id` BINARY(16) NOT NULL,
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `deleted_at` DATETIME(3) DEFAULT NULL,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_role_perm` (`role_id`,`permission_id`),
  KEY `idx_role_perm_role` (`tenant_id`,`role_id`), KEY `idx_role_perm_perm` (`permission_id`)
) ENGINE=InnoDB COMMENT='角色-权限分配（RBAC 关联）';

CREATE TABLE `sys_user_role` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `member_id` BINARY(16) NOT NULL, `role_id` BINARY(16) NOT NULL,
  `assigned_by` BINARY(16) DEFAULT NULL, `assigned_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `expires_at` DATETIME(3) DEFAULT NULL, `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  PRIMARY KEY (`id`), UNIQUE KEY `uk_user_role` (`member_id`,`role_id`),
  KEY `idx_user_role_member` (`tenant_id`,`member_id`,`status`), KEY `idx_user_role_role` (`role_id`)
) ENGINE=InnoDB COMMENT='用户-角色分配';

-- ABAC 策略：条件表达式与角色解耦，独立求值
CREATE TABLE `sys_policy` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `policy_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `policy_name` VARCHAR(200) NOT NULL,
  `effect` VARCHAR(8) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'allow' COMMENT 'allow/deny',
  `target_resource_id` BINARY(16) DEFAULT NULL,
  `condition_expr` JSON NOT NULL COMMENT '结构化条件（tenant/org/status/time/context），不写原生 SQL',
  `priority` INT NOT NULL DEFAULT 50,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_policy_code` (`tenant_id`,`policy_code`),
  KEY `idx_policy_target` (`target_resource_id`,`status`),
  CONSTRAINT `chk_policy_effect` CHECK (`effect` IN ('allow','deny'))
) ENGINE=InnoDB COMMENT='ABAC 策略（与 RBAC 正交）';

-- ReBAC 关系三元组：subject - relation - object（5NF 投影）
CREATE TABLE `sys_relation` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `subject_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'user/group',
  `subject_id` BINARY(16) NOT NULL,
  `relation_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'owner/editor/viewer/member',
  `object_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'doc/dataset/workspace',
  `object_id` BINARY(16) NOT NULL,
  `granted_by` BINARY(16) DEFAULT NULL, `expires_at` DATETIME(3) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_relation_triple` (`tenant_id`,`subject_type`,`subject_id`,`relation_code`,`object_type`,`object_id`),
  KEY `idx_relation_object` (`tenant_id`,`object_type`,`object_id`,`status`),
  KEY `idx_relation_subject` (`tenant_id`,`subject_type`,`subject_id`,`status`)
) ENGINE=InnoDB COMMENT='ReBAC 关系授权（三元组，不与 RBAC 混）';

-- =============================================================================
-- P05 · 菜单 / 导航（菜单是资源投影，不承载权限语义）
-- =============================================================================

CREATE TABLE `sys_menu` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `parent_id` BINARY(16) DEFAULT NULL,
  `menu_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `menu_name` VARCHAR(160) NOT NULL, `path` VARCHAR(255) DEFAULT NULL, `icon` VARCHAR(64) DEFAULT NULL,
  `sort_no` INT NOT NULL DEFAULT 0,
  `menu_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'menu' COMMENT 'catalog/menu/button',
  `resource_id` BINARY(16) DEFAULT NULL COMMENT '按钮级权限关联 sys_resource',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_menu_code` (`tenant_scope_id`, `menu_code`),
  KEY `idx_menu_parent` (`tenant_id`,`parent_id`,`sort_no`)
) ENGINE=InnoDB COMMENT='菜单/导航（资源投影）';

-- =============================================================================
-- P07 · 配置（按 scope 归一化为行；密文走 secret ref）
--      采用 scope_key 生成列保证 G 级（scope_id IS NULL）唯一性不被 NULL 绕过
-- =============================================================================

CREATE TABLE `sys_setting` (
  `id` BINARY(16) NOT NULL,
  `scope_kind` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'G全局/T租户/E企业/U用户',
  `scope_id` BINARY(16) DEFAULT NULL,
  `scope_key` BINARY(16) GENERATED ALWAYS AS (COALESCE(`scope_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `setting_key` VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `value_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'TEXT' COMMENT 'TEXT/JSON/NUMBER/BOOLEAN',
  `value_text` LONGTEXT DEFAULT NULL,
  `value_json` JSON DEFAULT NULL,
  `is_secret` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `updated_by` BINARY(16) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_setting_scope_key` (`scope_kind`,`scope_key`,`setting_key`),
  KEY `idx_setting_key` (`setting_key`),
  CONSTRAINT `chk_setting_scope` CHECK (
    (`scope_kind` = 'G' AND `scope_id` IS NULL) OR
    (`scope_kind` IN ('T','E','U') AND `scope_id` IS NOT NULL)
  )
) ENGINE=InnoDB COMMENT='配置中心（按 scope 归一化行，G 级唯一性由 scope_key 生成列保证）';

-- =============================================================================
-- P08 · 审计（事件头 + 逐字段变更，hash 链防篡改；不复制业务快照表）
-- =============================================================================

CREATE TABLE `sys_audit_event` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `request_id` BINARY(16) DEFAULT NULL,
  `trace_id` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `actor_user_id` BINARY(16) DEFAULT NULL,
  `actor_member_id` BINARY(16) DEFAULT NULL,
  `action_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'module.entity.action',
  `resource_type` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `resource_id` BINARY(16) DEFAULT NULL,
  `result_code` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'success',
  `ip_addr` VARBINARY(16) DEFAULT NULL,
  `user_agent` VARCHAR(1000) DEFAULT NULL,
  `before_data` JSON DEFAULT NULL,
  `after_data` JSON DEFAULT NULL,
  `prev_hash` BINARY(32) DEFAULT NULL COMMENT 'hash 链前驱',
  `event_hash` BINARY(32) NOT NULL COMMENT '本事件 sha256',
  `occurred_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_audit_hash` (`event_hash`) COMMENT '防篡改：事件哈希全局唯一',
  KEY `idx_audit_tenant_time` (`tenant_id`,`occurred_at`),
  KEY `idx_audit_resource` (`resource_type`,`resource_id`,`occurred_at`),
  KEY `idx_audit_actor` (`actor_user_id`,`occurred_at`),
  KEY `idx_audit_action` (`action_code`,`occurred_at`)
) ENGINE=InnoDB COMMENT='统一不可变审计事件（hash 链）';

-- 逐字段变更：3NF，把 JSON 里的 changed_fields 提升为可列查询的独立表
CREATE TABLE `sys_audit_change` (
  `id` BINARY(16) NOT NULL, `audit_event_id` BINARY(16) NOT NULL,
  `field_name` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `before_value` TEXT DEFAULT NULL, `after_value` TEXT DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_audit_change` (`audit_event_id`,`field_name`),
  KEY `idx_audit_change_event` (`audit_event_id`)
) ENGINE=InnoDB COMMENT='审计逐字段变更（3NF：可从 JSON 提升为列查询）';

-- =============================================================================
-- P09 · 事件总线 / 幂等（outbox 同库提交，消费端幂等）
-- =============================================================================

CREATE TABLE `sys_outbox_event` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL,
  `aggregate_type` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `aggregate_id` BINARY(16) NOT NULL,
  `event_type` VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `schema_version` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT '1.0.0',
  `event_key` VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT '业务去重键，防止重复投递',
  `payload` JSON NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'pending' COMMENT 'pending/retrying/sent/dead',
  `attempts` INT UNSIGNED NOT NULL DEFAULT 0,
  `available_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `sent_at` DATETIME(3) DEFAULT NULL,
  `last_error` VARCHAR(2000) DEFAULT NULL,
  `trace_id` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_outbox_event_key` (`event_key`),
  KEY `idx_outbox_dispatch` (`status`,`available_at`,`created_at`),
  KEY `idx_outbox_agg` (`aggregate_type`,`aggregate_id`)
) ENGINE=InnoDB COMMENT='事务消息 outbox（与写事务同库，event_key 去重）';

CREATE TABLE `sys_inbox_event` (
  `id` BINARY(16) NOT NULL, `event_id` BINARY(16) NOT NULL COMMENT '来源 outbox id，幂等去重',
  `consumer_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'processed' COMMENT 'processed/skipped/failed',
  `processed_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_inbox_event` (`event_id`,`consumer_code`)
) ENGINE=InnoDB COMMENT='收件箱（消费幂等）';

CREATE TABLE `sys_idempotency_key` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `actor_id` BINARY(16) DEFAULT NULL,
  `idempotency_key` VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `request_hash` BINARY(32) NOT NULL,
  `response_code` INT DEFAULT NULL, `response_body` JSON DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'pending',
  `expires_at` DATETIME(3) NOT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_idempotency_scope` (`tenant_scope_id`,`actor_id`,`idempotency_key`),
  KEY `idx_idempotency_expire` (`expires_at`)
) ENGINE=InnoDB COMMENT='幂等请求记录（按租户+操作者+键唯一）';

-- =============================================================================
-- P10 · 文件（对象与关联拆开；大文件走对象存储，DB 只存元数据）
-- =============================================================================

CREATE TABLE `sys_file_object` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `bucket_name` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `object_key` VARCHAR(1024) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
  `file_name` VARCHAR(255) NOT NULL,
  `content_type` VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `size_bytes` BIGINT UNSIGNED NOT NULL,
  `sha256` BINARY(32) NOT NULL,
  `storage_class` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'STANDARD',
  `encryption_mode` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'SSE',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_file_hash` (`tenant_id`,`sha256`,`size_bytes`) COMMENT '同租户内容去重',
  KEY `idx_file_object` (`bucket_name`,`object_key`(191)),
  KEY `idx_file_tenant_time` (`tenant_id`,`created_at`)
) ENGINE=InnoDB COMMENT='对象存储文件元数据';

CREATE TABLE `sys_file_link` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `file_id` BINARY(16) NOT NULL,
  `owner_type` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `owner_id` BINARY(16) NOT NULL,
  `usage_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'attachment' COMMENT 'avatar/attachment/cover',
  `sort_no` INT NOT NULL DEFAULT 0,
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_file_link` (`file_id`,`owner_type`,`owner_id`,`usage_code`),
  KEY `idx_link_resource` (`tenant_id`,`owner_type`,`owner_id`,`sort_no`)
) ENGINE=InnoDB COMMENT='文件多态关联（独立主键，无主键关系表已消灭）';

-- =============================================================================
-- P11 · 通知（模板 / 消息 / 偏好 三实体归一化）
-- =============================================================================

CREATE TABLE `sys_notification_template` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `template_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `channel` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'email/sms/push/inapp/webhook',
  `locale` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'zh-CN',
  `title_tpl` VARCHAR(512) DEFAULT NULL, `body_tpl` MEDIUMTEXT NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_ntf_tpl` (`tenant_scope_id`,`template_code`,`channel`,`locale`),
  KEY `idx_ntf_tpl_status` (`status`)
) ENGINE=InnoDB COMMENT='通知模板（按渠道+语言归一化）';

CREATE TABLE `sys_notification_message` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL,
  `template_id` BINARY(16) DEFAULT NULL, `recipient_id` BINARY(16) DEFAULT NULL,
  `channel` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'inapp',
  `title` VARCHAR(255) DEFAULT NULL, `body` MEDIUMTEXT DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'pending' COMMENT 'pending/sent/failed/read',
  `sent_at` DATETIME(3) DEFAULT NULL, `read_at` DATETIME(3) DEFAULT NULL,
  `trace_id` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  KEY `idx_ntf_msg_recipient` (`tenant_id`,`recipient_id`,`status`,`created_at`),
  KEY `idx_ntf_msg_status` (`status`,`created_at`)
) ENGINE=InnoDB COMMENT='通知消息实例（高吞吐，按时间归档）';

CREATE TABLE `sys_notification_pref` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `user_id` BINARY(16) NOT NULL,
  `channel` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `topic_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'system/marketing/security',
  `enabled` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'Y',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_ntf_pref` (`tenant_id`,`user_id`,`channel`,`topic_code`)
) ENGINE=InnoDB COMMENT='用户通知偏好（多值依赖→独立表）';

-- =============================================================================
-- P12 · 调度（与 Quartz 协议解耦的可移植作业表；集群锁走独立 worker）
-- =============================================================================

CREATE TABLE `sys_job` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `job_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `job_name` VARCHAR(200) NOT NULL,
  `invoke_target` VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'Fqcn.method() 或 module.job',
  `cron_expr` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `trigger_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'cron' COMMENT 'cron/interval/once',
  `interval_sec` INT UNSIGNED DEFAULT NULL,
  `misfire_policy` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'fire_once',
  `concurrency` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N' COMMENT 'Y=允许并发',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `last_run_at` DATETIME(3) DEFAULT NULL, `next_run_at` DATETIME(3) DEFAULT NULL,
  `created_by` BINARY(16) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_job_code` (`tenant_scope_id`, `job_code`),
  KEY `idx_job_schedule` (`status`,`next_run_at`),
  KEY `idx_job_tenant` (`tenant_id`,`status`)
) ENGINE=InnoDB COMMENT='作业定义（可移植，不绑定 QRTZ_*）';

CREATE TABLE `sys_job_run` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL, `job_id` BINARY(16) NOT NULL,
  `run_no` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'running' COMMENT 'running/success/failed/cancelled',
  `trigger_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'schedule',
  `attempt_no` INT UNSIGNED NOT NULL DEFAULT 0,
  `started_at` DATETIME(3) DEFAULT NULL, `ended_at` DATETIME(3) DEFAULT NULL,
  `duration_ms` INT UNSIGNED DEFAULT NULL,
  `error_msg` VARCHAR(2000) DEFAULT NULL, `trace_id` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_job_run_no` (`job_id`,`run_no`),
  KEY `idx_job_run_dispatch` (`tenant_id`,`status`,`created_at`),
  KEY `idx_job_run_job_time` (`job_id`,`created_at`)
) ENGINE=InnoDB COMMENT='作业执行记录（高吞吐归档）';

-- =============================================================================
-- P13 · 特性开关（灰度 / 租户分级发布）
-- =============================================================================

CREATE TABLE `sys_feature_flag` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `flag_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `flag_name` VARCHAR(200) NOT NULL,
  `rollout_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'global' COMMENT 'global/tenant/user/percent',
  `default_on` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_flag_code` (`tenant_scope_id`, `flag_code`),
  KEY `idx_flag_status` (`status`)
) ENGINE=InnoDB COMMENT='特性开关';

CREATE TABLE `sys_feature_flag_rollout` (
  `id` BINARY(16) NOT NULL, `flag_id` BINARY(16) NOT NULL,
  `target_type` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'tenant/user/percent',
  `target_id` BINARY(16) DEFAULT NULL, `percent` TINYINT UNSIGNED DEFAULT NULL,
  `enabled` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'Y',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_flag_rollout` (`flag_id`,`target_type`,`target_id`),
  KEY `idx_flag_rollout_flag` (`flag_id`),
  CONSTRAINT `chk_flag_percent` CHECK (`percent` IS NULL OR `percent` <= 100)
) ENGINE=InnoDB COMMENT='特性开关分级投放';

-- =============================================================================
-- P14 · 连接器（端点 / 凭证 / 调用 三实体；密文不进调用日志）
-- =============================================================================

CREATE TABLE `sys_connector_endpoint` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `endpoint_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `endpoint_name` VARCHAR(200) NOT NULL,
  `protocol_code` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'http/sftp/s3/sql/kafka',
  `address` VARCHAR(1000) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL,
  `config` JSON DEFAULT NULL COMMENT '非密配置',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_conn_ep` (`tenant_id`,`endpoint_code`),
  KEY `idx_conn_ep_status` (`tenant_id`,`status`)
) ENGINE=InnoDB COMMENT='连接器端点';

CREATE TABLE `sys_connector_credential` (
  `id` BINARY(16) NOT NULL, `endpoint_id` BINARY(16) NOT NULL,
  `cred_kind` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'apikey/token/oauth/basic',
  `secret_ref` BINARY(16) NOT NULL COMMENT '密文引用，不落地明文',
  `expires_at` DATETIME(3) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), KEY `idx_conn_cred_ep` (`endpoint_id`,`status`)
) ENGINE=InnoDB COMMENT='连接器凭证（密文外置）';

CREATE TABLE `sys_connector_call` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `endpoint_id` BINARY(16) NOT NULL,
  `call_type` VARCHAR(48) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'ok' COMMENT 'ok/failed/timeout',
  `requested_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `duration_ms` INT UNSIGNED DEFAULT NULL,
  `error_msg` VARCHAR(1000) DEFAULT NULL, `trace_id` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `idx_conn_call_ep` (`endpoint_id`,`requested_at`),
  KEY `idx_conn_call_status` (`tenant_id`,`status`,`requested_at`)
) ENGINE=InnoDB COMMENT='连接器调用日志（密文不落此表）';

-- =============================================================================
-- P15 · 国际化（bundle / message 两实体，按 locale 归一化）
-- =============================================================================

CREATE TABLE `sys_i18n_bundle` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `bundle_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `bundle_name` VARCHAR(200) NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_i18n_bundle` (`tenant_scope_id`, `bundle_code`)
) ENGINE=InnoDB COMMENT='国际化资源束';

CREATE TABLE `sys_i18n_message` (
  `id` BINARY(16) NOT NULL, `bundle_id` BINARY(16) NOT NULL,
  `locale` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `msg_key` VARCHAR(160) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `msg_value` TEXT NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_i18n_msg` (`bundle_id`,`locale`,`msg_key`),
  KEY `idx_i18n_msg_locale` (`locale`,`msg_key`)
) ENGINE=InnoDB COMMENT='国际化消息（locale 维度归一化行）';

-- =============================================================================
-- P16 · 计量 / 套餐（租户用量与订阅，支撑 SaaS 计费）
-- =============================================================================

CREATE TABLE `sys_plan` (
  `id` BINARY(16) NOT NULL, `plan_code` VARCHAR(48) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `plan_name` VARCHAR(160) NOT NULL, `quota_json` JSON NOT NULL COMMENT '各维度配额（可演进）',
  `price_json` JSON DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_plan_code` (`plan_code`)
) ENGINE=InnoDB COMMENT='套餐定义';

CREATE TABLE `sys_subscription` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `plan_id` BINARY(16) NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/trialing/cancelled/past_due',
  `started_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `renews_at` DATETIME(3) DEFAULT NULL, `cancelled_at` DATETIME(3) DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_sub_tenant` (`tenant_id`), KEY `idx_sub_status` (`status`,`renews_at`)
) ENGINE=InnoDB COMMENT='租户订阅';

CREATE TABLE `sys_usage_meter` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `meter_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'api_call/storage_mb/seat',
  `period_no` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'YYYYMM',
  `quantity` DECIMAL(24,6) NOT NULL DEFAULT 0, `unit` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'count',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_usage_meter` (`tenant_id`,`meter_code`,`period_no`),
  KEY `idx_usage_period` (`period_no`,`meter_code`)
) ENGINE=InnoDB COMMENT='租户用量计量（按月聚合）';

-- =============================================================================
-- P18 · 扩展钩子（自定义字段 / Webhook：业务零改表即可扩展）
-- =============================================================================

CREATE TABLE `sys_custom_field_schema` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL,
  `entity_type` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT '可扩展的业务实体，如 ea_expert',
  `field_code` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `field_name` VARCHAR(160) NOT NULL,
  `field_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL COMMENT 'string/number/date/enum/ref',
  `enum_type_id` BINARY(16) DEFAULT NULL COMMENT 'field_type=enum 时引用 sys_enum_type',
  `is_required` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N',
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`), UNIQUE KEY `uk_custom_field` (`tenant_id`,`entity_type`,`field_code`)
) ENGINE=InnoDB COMMENT='自定义字段 schema（EAV 模式开关）';

CREATE TABLE `sys_custom_field_value` (
  `id` BINARY(16) NOT NULL, `schema_id` BINARY(16) NOT NULL, `entity_id` BINARY(16) NOT NULL,
  `field_value` TEXT DEFAULT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_custom_value` (`schema_id`,`entity_id`),
  KEY `idx_custom_value_entity` (`entity_id`)
) ENGINE=InnoDB COMMENT='自定义字段值（EAV；高频查询字段建议提为生成列）';

CREATE TABLE `sys_webhook` (
  `id` BINARY(16) NOT NULL,
  `tenant_id` BINARY(16) DEFAULT NULL,
  `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED,
  `webhook_code` VARCHAR(120) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `endpoint_url` VARCHAR(512) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `event_filter` JSON NOT NULL COMMENT '订阅的事件类型列表',
  `secret_ref` BINARY(16) DEFAULT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active',
  `created_by` BINARY(16) DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  `deleted_at` DATETIME(3) DEFAULT NULL, `row_version` BIGINT UNSIGNED NOT NULL DEFAULT 1,
  PRIMARY KEY (`id`),
  UNIQUE KEY `uk_webhook_code` (`tenant_scope_id`, `webhook_code`)
) ENGINE=InnoDB COMMENT='Webhook 订阅（事件驱动的扩展出口）';

-- =============================================================================
-- P19 · 视图：归一化后的可读性投影（可选）
-- =============================================================================

CREATE OR REPLACE VIEW `v_tenant_user` AS
SELECT t.tenant_code, u.login_name, u.display_name, tm.member_type, tm.is_owner, e.enterprise_name, ou.org_name
FROM sys_tenant t
  JOIN sys_tenant_member tm ON tm.tenant_id = t.id AND tm.deleted_at IS NULL
  JOIN sys_user u ON u.id = tm.user_id AND u.deleted_at IS NULL
  LEFT JOIN sys_enterprise e ON e.id = tm.default_enterprise_id
  LEFT JOIN sys_org_unit ou ON ou.id = tm.default_org_unit_id;

CREATE OR REPLACE VIEW `v_user_effective_permission` AS
SELECT DISTINCT ur.member_id, p.resource_id, p.action_code, r.data_scope
FROM sys_user_role ur
  JOIN sys_role_permission rp ON rp.role_id = ur.role_id
  JOIN sys_permission p ON p.id = rp.permission_id
  JOIN sys_role r ON r.id = ur.role_id
WHERE ur.status = 'active' AND r.status = 'active';

-- =============================================================================
-- P17 · 模块注册与通用知识图谱（在本文件内一并定义，不再另起文件）
-- =============================================================================

CREATE TABLE `mox_sys_module` (
  `id` BINARY(16) NOT NULL,
  `module_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `module_name` VARCHAR(160) NOT NULL,
  `module_kind` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `owner_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,
  `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/deprecated/archived',
  `manifest` JSON NOT NULL,
  `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_module_code` (`module_code`), KEY `idx_mox_sys_module_status` (`status`,`module_code`),
  CONSTRAINT `chk_mox_sys_module_status` CHECK (`status` IN ('active','deprecated','archived'))
) ENGINE=InnoDB COMMENT='mox_sys 模块注册';

CREATE TABLE `mox_sys_module_version` (
  `id` BINARY(16) NOT NULL, `module_id` BINARY(16) NOT NULL, `module_version` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `migration_version` BIGINT UNSIGNED NOT NULL, `migration_checksum` BINARY(32) NOT NULL, `manifest` JSON NOT NULL, `released_at` DATETIME(3) DEFAULT NULL, `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'draft' COMMENT 'draft/released/yanked', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_module_ver` (`module_id`,`module_version`), UNIQUE KEY `uk_mox_sys_migration_ver` (`module_id`,`migration_version`), KEY `idx_mox_sys_module_ver_status` (`module_id`,`status`)
, CONSTRAINT `chk_mox_sys_module_version_status` CHECK (`status` IN ('draft','released','yanked'))
) ENGINE=InnoDB COMMENT='mox_sys 模块版本';

CREATE TABLE `mox_sys_module_dependency` (
  `id` BINARY(16) NOT NULL, `module_id` BINARY(16) NOT NULL, `requires_module_id` BINARY(16) NOT NULL, `version_range` VARCHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `is_optional` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_module_dep` (`module_id`,`requires_module_id`), KEY `idx_mox_sys_module_dep_required` (`requires_module_id`)
) ENGINE=InnoDB COMMENT='mox_sys 模块依赖';

CREATE TABLE `mox_sys_schema_version` (
  `id` BINARY(16) NOT NULL, `module_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `version_no` BIGINT UNSIGNED NOT NULL, `migration_name` VARCHAR(255) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `checksum` BINARY(32) NOT NULL, `applied_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `applied_by` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL, `execution_ms` INT UNSIGNED DEFAULT NULL, `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'applied' COMMENT 'applied/failed/rolled_back', PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_schema_ver` (`module_code`,`version_no`), KEY `idx_mox_sys_schema_status` (`module_code`,`status`,`version_no`)
, CONSTRAINT `chk_mox_sys_schema_version_status` CHECK (`status` IN ('applied','failed','rolled_back'))
) ENGINE=InnoDB COMMENT='mox_sys 迁移账本';

CREATE TABLE `mox_sys_graph` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) DEFAULT NULL, `tenant_scope_id` BINARY(16) GENERATED ALWAYS AS (COALESCE(`tenant_id`,UNHEX('00000000000000000000000000000000'))) STORED, `graph_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `graph_name` VARCHAR(160) NOT NULL, `graph_kind` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'PROPERTY', `backend_code` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'SQL', `ontology_version` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL,   `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/deprecated/archived', `config` JSON DEFAULT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3), `deleted_at` DATETIME(3) DEFAULT NULL, PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_graph_code` (`tenant_scope_id`,`graph_code`), KEY `idx_mox_sys_graph_status` (`tenant_id`,`status`),
CONSTRAINT `chk_mox_sys_graph_status` CHECK (`status` IN ('active','deprecated','archived'))
) ENGINE=InnoDB COMMENT='通用知识图谱命名空间';

CREATE TABLE `mox_sys_node_type` (
  `id` BINARY(16) NOT NULL, `graph_id` BINARY(16) NOT NULL, `type_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `type_name` VARCHAR(160) NOT NULL, `owner_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `schema_json` JSON DEFAULT NULL,   `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/deprecated', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3), PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_node_type` (`graph_id`,`type_code`), KEY `idx_mox_sys_node_type_module` (`owner_module`,`status`),
CONSTRAINT `chk_mox_sys_node_type_status` CHECK (`status` IN ('active','deprecated'))
) ENGINE=InnoDB COMMENT='图谱节点类型';

CREATE TABLE `mox_sys_relation_type` (
  `id` BINARY(16) NOT NULL, `graph_id` BINARY(16) NOT NULL, `relation_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `relation_name` VARCHAR(160) NOT NULL, `inverse_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL, `cardinality_code` VARCHAR(16) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N:N', `is_symmetric` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N', `is_transitive` CHAR(1) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'N', `owner_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `schema_json` JSON DEFAULT NULL,   `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/deprecated', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3), PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_relation_type` (`graph_id`,`relation_code`), KEY `idx_mox_sys_relation_type_module` (`owner_module`,`status`), CONSTRAINT `chk_mox_sys_relation_cardinality` CHECK (`cardinality_code` IN ('1:1','1:N','N:1','N:N')),
CONSTRAINT `chk_mox_sys_relation_type_status` CHECK (`status` IN ('active','deprecated'))
) ENGINE=InnoDB COMMENT='图谱关系类型';

CREATE TABLE `mox_sys_node` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `graph_id` BINARY(16) NOT NULL, `node_type_id` BINARY(16) NOT NULL, `entity_key` VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_bin NOT NULL, `label` VARCHAR(255) NOT NULL, `owner_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `owner_id` BINARY(16) DEFAULT NULL, `properties` JSON DEFAULT NULL, `sensitivity_code` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL, `valid_from` DATETIME(3) DEFAULT NULL, `valid_to` DATETIME(3) DEFAULT NULL,   `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/deprecated/archived', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `updated_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3), `deleted_at` DATETIME(3) DEFAULT NULL, PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_node_key` (`graph_id`,`node_type_id`,`entity_key`), KEY `idx_mox_sys_node_tenant` (`tenant_id`,`graph_id`,`node_type_id`,`updated_at`), KEY `idx_mox_sys_node_owner` (`owner_module`,`owner_id`),
CONSTRAINT `chk_mox_sys_node_status` CHECK (`status` IN ('active','deprecated','archived'))
) ENGINE=InnoDB COMMENT='通用知识图谱节点';

CREATE TABLE `mox_sys_edge` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `graph_id` BINARY(16) NOT NULL, `relation_type_id` BINARY(16) NOT NULL, `from_node_id` BINARY(16) NOT NULL, `to_node_id` BINARY(16) NOT NULL, `revision_no` INT UNSIGNED NOT NULL DEFAULT 1, `properties` JSON DEFAULT NULL, `confidence` DECIMAL(8,6) DEFAULT NULL, `source_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `source_id` BINARY(16) DEFAULT NULL, `valid_from` DATETIME(3) DEFAULT NULL, `valid_to` DATETIME(3) DEFAULT NULL,   `status` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'active' COMMENT 'active/superseded', `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), `deleted_at` DATETIME(3) DEFAULT NULL, PRIMARY KEY (`id`), UNIQUE KEY `uk_mox_sys_edge_revision` (`graph_id`,`relation_type_id`,`from_node_id`,`to_node_id`,`revision_no`), KEY `idx_mox_sys_edge_from` (`tenant_id`,`graph_id`,`from_node_id`,`relation_type_id`,`status`), KEY `idx_mox_sys_edge_to` (`tenant_id`,`graph_id`,`to_node_id`,`relation_type_id`,`status`), KEY `idx_mox_sys_edge_source` (`source_module`,`source_id`), CONSTRAINT `chk_mox_sys_edge_confidence` CHECK (`confidence` IS NULL OR (`confidence` >= 0 AND `confidence` <= 1)),
CONSTRAINT `chk_mox_sys_edge_status` CHECK (`status` IN ('active','superseded'))
) ENGINE=InnoDB COMMENT='通用知识图谱关系事实';

CREATE TABLE `mox_sys_evidence` (
  `id` BINARY(16) NOT NULL, `tenant_id` BINARY(16) NOT NULL, `graph_id` BINARY(16) NOT NULL, `edge_id` BINARY(16) DEFAULT NULL, `node_id` BINARY(16) DEFAULT NULL, `evidence_type` VARCHAR(24) CHARACTER SET ascii COLLATE ascii_bin NOT NULL DEFAULT 'EVENT', `source_module` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin NOT NULL, `source_id` BINARY(16) DEFAULT NULL, `extractor_code` VARCHAR(96) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL, `extractor_version` VARCHAR(32) CHARACTER SET ascii COLLATE ascii_bin DEFAULT NULL, `confidence` DECIMAL(8,6) DEFAULT NULL, `evidence` JSON NOT NULL, `created_at` DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3), PRIMARY KEY (`id`), KEY `idx_mox_sys_evidence_edge` (`tenant_id`,`edge_id`,`created_at`), KEY `idx_mox_sys_evidence_node` (`tenant_id`,`node_id`,`created_at`), KEY `idx_mox_sys_evidence_source` (`source_module`,`source_id`,`created_at`), CONSTRAINT `chk_mox_sys_evidence_confidence` CHECK (`confidence` IS NULL OR (`confidence` >= 0 AND `confidence` <= 1))
) ENGINE=InnoDB COMMENT='图谱关系证据与来源';
