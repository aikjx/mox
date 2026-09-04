-- =============================================================================
-- mox_sys 外键强化层（单体强一致版 · 可选）  依赖：mox_sys-universal-template.sql
-- =============================================================================
-- 定位：可移植母版刻意零物理外键（跨库/分库/事件一致性）。本文件是【发布期可选层】，
--       仅在「单库 MySQL 8.0.16+ 单体部署」时执行，为实体图追加 81 条物理 FK，
--       让数据库替你兜底引用完整性。执行顺序：先灌母版，再执行本文件。
--       重复执行会报 duplicate FK 名——本脚本一次性使用。
--
-- 约定：
--   1) 全部 ON DELETE RESTRICT / ON UPDATE RESTRICT：全库软删除（deleted_at），
--      绝不级联物理删除，误删会立刻报错而不是静默断链。
--   2) 只引用 PK / UNIQUE 键列，且两侧类型/字符集完全一致（BINARY(16)↔BINARY(16)，
--      VARCHAR(96) ascii_bin↔ascii_bin），否则 MySQL 8 会拒绝建 FK。
--   3) 多态引用（owner_type/owner_id、subject/object、resource_type/resource_id、
--      aggregate_type/aggregate_id、entity_type/entity_id、scope_kind/scope_id、
--      source_module/source_id、secret_ref）天生无法加 FK，不在本层范围内。
--   4) 溯源列（created_by/updated_by/deleted_by/assigned_by/granted_by/actor_* 等
--      指向 sys_user 的操作者指针）不加 FK：操作者可能先于记录被物理清理，
--      加 FK 会把历史审计/日志表的写入卡死。
--
-- 执行失败排查：先清数据孤儿（孤儿行 SELECT 出来人工处理），再重跑本文件。
-- =============================================================================
USE `mox_v3`;

-- ---------------------------------------------------------------------------
-- 组织域（P02/P03）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_enterprise`
  ADD CONSTRAINT `fk_enterprise_tenant`      FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_org_unit`
  ADD CONSTRAINT `fk_org_unit_tenant`        FOREIGN KEY (`tenant_id`)     REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_org_unit_enterprise`    FOREIGN KEY (`enterprise_id`) REFERENCES `sys_enterprise` (`id`),
  ADD CONSTRAINT `fk_org_unit_parent`        FOREIGN KEY (`parent_id`)     REFERENCES `sys_org_unit` (`id`);

ALTER TABLE `sys_identity_provider`
  ADD CONSTRAINT `fk_idp_tenant`             FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_user_identity`
  ADD CONSTRAINT `fk_user_identity_user`     FOREIGN KEY (`user_id`)     REFERENCES `sys_user` (`id`),
  ADD CONSTRAINT `fk_user_identity_provider` FOREIGN KEY (`provider_id`) REFERENCES `sys_identity_provider` (`id`);

ALTER TABLE `sys_user_mfa`
  ADD CONSTRAINT `fk_user_mfa_user`          FOREIGN KEY (`user_id`) REFERENCES `sys_user` (`id`);

ALTER TABLE `sys_user_session`
  ADD CONSTRAINT `fk_user_session_user`      FOREIGN KEY (`user_id`) REFERENCES `sys_user` (`id`);

ALTER TABLE `sys_tenant_member`
  ADD CONSTRAINT `fk_tmember_tenant`         FOREIGN KEY (`tenant_id`)              REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_tmember_user`           FOREIGN KEY (`user_id`)                REFERENCES `sys_user` (`id`),
  ADD CONSTRAINT `fk_tmember_def_enterprise` FOREIGN KEY (`default_enterprise_id`)  REFERENCES `sys_enterprise` (`id`),
  ADD CONSTRAINT `fk_tmember_def_org`        FOREIGN KEY (`default_org_unit_id`)    REFERENCES `sys_org_unit` (`id`);

ALTER TABLE `sys_org_member`
  ADD CONSTRAINT `fk_omember_tenant`         FOREIGN KEY (`tenant_id`)     REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_omember_enterprise`     FOREIGN KEY (`enterprise_id`) REFERENCES `sys_enterprise` (`id`),
  ADD CONSTRAINT `fk_omember_org_unit`       FOREIGN KEY (`org_unit_id`)   REFERENCES `sys_org_unit` (`id`),
  ADD CONSTRAINT `fk_omember_user`           FOREIGN KEY (`member_id`)     REFERENCES `sys_user` (`id`);

-- ---------------------------------------------------------------------------
-- 授权域（P04/P05）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_permission`
  ADD CONSTRAINT `fk_permission_resource`    FOREIGN KEY (`resource_id`) REFERENCES `sys_resource` (`id`),
  ADD CONSTRAINT `fk_permission_tenant`      FOREIGN KEY (`tenant_id`)   REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_role`
  ADD CONSTRAINT `fk_role_tenant`            FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_role_permission`
  ADD CONSTRAINT `fk_role_perm_tenant`       FOREIGN KEY (`tenant_id`)      REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_role_perm_role`         FOREIGN KEY (`role_id`)        REFERENCES `sys_role` (`id`),
  ADD CONSTRAINT `fk_role_perm_permission`   FOREIGN KEY (`permission_id`)  REFERENCES `sys_permission` (`id`);

ALTER TABLE `sys_user_role`
  ADD CONSTRAINT `fk_user_role_tenant`       FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_user_role_user`         FOREIGN KEY (`member_id`) REFERENCES `sys_user` (`id`),
  ADD CONSTRAINT `fk_user_role_role`         FOREIGN KEY (`role_id`)   REFERENCES `sys_role` (`id`);

ALTER TABLE `sys_policy`
  ADD CONSTRAINT `fk_policy_tenant`          FOREIGN KEY (`tenant_id`)           REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_policy_target_resource` FOREIGN KEY (`target_resource_id`)  REFERENCES `sys_resource` (`id`);

ALTER TABLE `sys_relation`
  ADD CONSTRAINT `fk_relation_tenant`        FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_menu`
  ADD CONSTRAINT `fk_menu_tenant`            FOREIGN KEY (`tenant_id`)   REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_menu_parent`            FOREIGN KEY (`parent_id`)   REFERENCES `sys_menu` (`id`),
  ADD CONSTRAINT `fk_menu_resource`          FOREIGN KEY (`resource_id`) REFERENCES `sys_resource` (`id`);

-- ---------------------------------------------------------------------------
-- 事件 / 幂等（P08/P09）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_audit_change`
  ADD CONSTRAINT `fk_audit_change_event`     FOREIGN KEY (`audit_event_id`) REFERENCES `sys_audit_event` (`id`);

ALTER TABLE `sys_inbox_event`
  ADD CONSTRAINT `fk_inbox_event_source`     FOREIGN KEY (`event_id`) REFERENCES `sys_outbox_event` (`id`);

ALTER TABLE `sys_idempotency_key`
  ADD CONSTRAINT `fk_idem_tenant`            FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

-- ---------------------------------------------------------------------------
-- 文件 / 通知（P10/P11）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_file_object`
  ADD CONSTRAINT `fk_file_object_tenant`     FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_file_link`
  ADD CONSTRAINT `fk_file_link_tenant`       FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_file_link_file`         FOREIGN KEY (`file_id`)   REFERENCES `sys_file_object` (`id`);

ALTER TABLE `sys_notification_template`
  ADD CONSTRAINT `fk_ntf_tpl_tenant`         FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_notification_message`
  ADD CONSTRAINT `fk_ntf_msg_tenant`         FOREIGN KEY (`tenant_id`)   REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_ntf_msg_template`       FOREIGN KEY (`template_id`) REFERENCES `sys_notification_template` (`id`),
  ADD CONSTRAINT `fk_ntf_msg_recipient`      FOREIGN KEY (`recipient_id`) REFERENCES `sys_user` (`id`);

ALTER TABLE `sys_notification_pref`
  ADD CONSTRAINT `fk_ntf_pref_tenant`        FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_ntf_pref_user`          FOREIGN KEY (`user_id`)   REFERENCES `sys_user` (`id`);

-- ---------------------------------------------------------------------------
-- 调度 / 特性开关（P12/P13）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_job`
  ADD CONSTRAINT `fk_job_tenant`             FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_job_run`
  ADD CONSTRAINT `fk_job_run_tenant`         FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_job_run_job`            FOREIGN KEY (`job_id`)    REFERENCES `sys_job` (`id`);

ALTER TABLE `sys_feature_flag`
  ADD CONSTRAINT `fk_flag_tenant`            FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_feature_flag_rollout`
  ADD CONSTRAINT `fk_flag_rollout_flag`      FOREIGN KEY (`flag_id`) REFERENCES `sys_feature_flag` (`id`);

-- ---------------------------------------------------------------------------
-- 连接器 / 国际化（P14/P15）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_connector_endpoint`
  ADD CONSTRAINT `fk_conn_ep_tenant`         FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_connector_credential`
  ADD CONSTRAINT `fk_conn_cred_endpoint`     FOREIGN KEY (`endpoint_id`) REFERENCES `sys_connector_endpoint` (`id`);

ALTER TABLE `sys_connector_call`
  ADD CONSTRAINT `fk_conn_call_tenant`       FOREIGN KEY (`tenant_id`)   REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_conn_call_endpoint`     FOREIGN KEY (`endpoint_id`) REFERENCES `sys_connector_endpoint` (`id`);

ALTER TABLE `sys_i18n_bundle`
  ADD CONSTRAINT `fk_i18n_bundle_tenant`     FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_i18n_message`
  ADD CONSTRAINT `fk_i18n_msg_bundle`        FOREIGN KEY (`bundle_id`) REFERENCES `sys_i18n_bundle` (`id`);

-- ---------------------------------------------------------------------------
-- 套餐 / 扩展钩子（P16/P18）
-- ---------------------------------------------------------------------------
ALTER TABLE `sys_subscription`
  ADD CONSTRAINT `fk_sub_tenant`             FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_sub_plan`               FOREIGN KEY (`plan_id`)   REFERENCES `sys_plan` (`id`);

ALTER TABLE `sys_usage_meter`
  ADD CONSTRAINT `fk_usage_tenant`           FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `sys_custom_field_schema`
  ADD CONSTRAINT `fk_cfs_tenant`             FOREIGN KEY (`tenant_id`)     REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_cfs_enum_type`          FOREIGN KEY (`enum_type_id`)  REFERENCES `sys_enum_type` (`id`);

ALTER TABLE `sys_custom_field_value`
  ADD CONSTRAINT `fk_cfv_schema`             FOREIGN KEY (`schema_id`) REFERENCES `sys_custom_field_schema` (`id`);

ALTER TABLE `sys_webhook`
  ADD CONSTRAINT `fk_webhook_tenant`         FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

-- ---------------------------------------------------------------------------
-- 模块注册与知识图谱（P17）
-- ---------------------------------------------------------------------------
ALTER TABLE `mox_sys_module_version`
  ADD CONSTRAINT `fk_msv_module`             FOREIGN KEY (`module_id`) REFERENCES `mox_sys_module` (`id`);

ALTER TABLE `mox_sys_module_dependency`
  ADD CONSTRAINT `fk_msd_module`             FOREIGN KEY (`module_id`)          REFERENCES `mox_sys_module` (`id`),
  ADD CONSTRAINT `fk_msd_requires`           FOREIGN KEY (`requires_module_id`) REFERENCES `mox_sys_module` (`id`);

ALTER TABLE `mox_sys_schema_version`
  ADD CONSTRAINT `fk_schema_ver_module`      FOREIGN KEY (`module_code`) REFERENCES `mox_sys_module` (`module_code`);

ALTER TABLE `mox_sys_graph`
  ADD CONSTRAINT `fk_graph_tenant`           FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`);

ALTER TABLE `mox_sys_node_type`
  ADD CONSTRAINT `fk_node_type_graph`        FOREIGN KEY (`graph_id`) REFERENCES `mox_sys_graph` (`id`);

ALTER TABLE `mox_sys_relation_type`
  ADD CONSTRAINT `fk_rel_type_graph`         FOREIGN KEY (`graph_id`) REFERENCES `mox_sys_graph` (`id`);

ALTER TABLE `mox_sys_node`
  ADD CONSTRAINT `fk_node_tenant`            FOREIGN KEY (`tenant_id`)     REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_node_graph`             FOREIGN KEY (`graph_id`)      REFERENCES `mox_sys_graph` (`id`),
  ADD CONSTRAINT `fk_node_type`              FOREIGN KEY (`node_type_id`)  REFERENCES `mox_sys_node_type` (`id`);

ALTER TABLE `mox_sys_edge`
  ADD CONSTRAINT `fk_edge_tenant`            FOREIGN KEY (`tenant_id`)        REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_edge_graph`             FOREIGN KEY (`graph_id`)         REFERENCES `mox_sys_graph` (`id`),
  ADD CONSTRAINT `fk_edge_rel_type`          FOREIGN KEY (`relation_type_id`) REFERENCES `mox_sys_relation_type` (`id`),
  ADD CONSTRAINT `fk_edge_from_node`         FOREIGN KEY (`from_node_id`)     REFERENCES `mox_sys_node` (`id`),
  ADD CONSTRAINT `fk_edge_to_node`           FOREIGN KEY (`to_node_id`)       REFERENCES `mox_sys_node` (`id`);

ALTER TABLE `mox_sys_evidence`
  ADD CONSTRAINT `fk_evidence_tenant`        FOREIGN KEY (`tenant_id`) REFERENCES `sys_tenant` (`id`),
  ADD CONSTRAINT `fk_evidence_graph`         FOREIGN KEY (`graph_id`)  REFERENCES `mox_sys_graph` (`id`),
  ADD CONSTRAINT `fk_evidence_edge`          FOREIGN KEY (`edge_id`)   REFERENCES `mox_sys_edge` (`id`),
  ADD CONSTRAINT `fk_evidence_node`          FOREIGN KEY (`node_id`)   REFERENCES `mox_sys_node` (`id`);

-- =============================================================================
-- 验证：应返回 81 条外键
--   SELECT COUNT(*) FROM information_schema.REFERENTIAL_CONSTRAINTS
--   WHERE CONSTRAINT_SCHEMA = 'mox_v3';
-- =============================================================================
