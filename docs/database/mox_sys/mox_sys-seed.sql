-- =============================================================================
-- mox_sys 平台引导种子（可选）  依赖：mox_sys-universal-template.sql
-- =============================================================================
-- 定位：母版只建结构；本文件灌入【平台内建字典】(B 级类别字段的取值登记，
--       与 DDL 内 COMMENT/CHECK 逐字一致) 与【平台引导数据】(platform 租户/
--       admin 用户/sys_admin 角色)，实现“装完即用、字典可查询”。
-- 约定：
--   1) 全部行使用【保留低位段】确定性 ID（0x…00000001 起，共 255 个），
--      与业务 UUID v7（应用层随机）不相交；禁止业务复用本段。
--   2) 全部 INSERT IGNORE：依赖 UNIQUE 键天然可重复执行，重复安装不报错。
--   3) 本文件是 MySQL 专属（UNHEX）；与 fk.sql 一样为可选层，可移植母版不含。
--   4) admin 用户 password_hash 置 NULL：首次登录必须由应用按 Argon2id/bcrypt
--      写入初始密码或走 IdP，禁止在本文件落地任何明文/固定摘要。
-- 执行顺序：母版 → (可选 fk.sql) → 本种子 → 业务模块迁移。
-- 校验：SELECT COUNT(*) FROM sys_enum_type;   -- 期望 30
--       SELECT COUNT(*) FROM sys_enum_item;    -- 期望 108
-- =============================================================================
SET NAMES utf8mb4 COLLATE utf8mb4_0900_ai_ci;
USE `mox_v3`;

-- =============================================================================
-- 一、平台引导对象（ID 保留段 0x…00000001–0x…00000007）
-- =============================================================================
INSERT IGNORE INTO `sys_tenant` (`id`,`tenant_code`,`tenant_name`,`tenant_mode`,`plan_code`,`data_region`,`timezone`,`locale`)
VALUES (UNHEX('00000000000000000000000000000001'),'platform','平台租户','logical','free',NULL,'UTC','zh-CN');

INSERT IGNORE INTO `sys_user` (`id`,`login_name`,`display_name`,`email`,`phone`,`password_hash`,`user_type`,`status`,`locale`,`timezone`,`last_login_at`)
VALUES (UNHEX('00000000000000000000000000000002'),'admin','平台管理员','admin@mox.local',NULL,NULL,'person','active','zh-CN','UTC',NULL);

INSERT IGNORE INTO `sys_enterprise` (`id`,`tenant_id`,`enterprise_code`,`enterprise_name`,`enterprise_type`,`status`,`is_default`)
VALUES (UNHEX('00000000000000000000000000000004'),UNHEX('00000000000000000000000000000001'),'platform','平台企业','company','active','Y');

INSERT IGNORE INTO `sys_org_unit` (`id`,`tenant_id`,`enterprise_id`,`parent_id`,`org_code`,`org_name`,`org_type`,`path_key`,`level_no`,`sort_no`,`manager_id`,`status`)
VALUES (UNHEX('00000000000000000000000000000005'),UNHEX('00000000000000000000000000000001'),UNHEX('00000000000000000000000000000004'),NULL,'HQ','总部','root','/HQ/',0,0,UNHEX('00000000000000000000000000000002'),'active');

INSERT IGNORE INTO `sys_tenant_member` (`id`,`tenant_id`,`user_id`,`member_type`,`status`,`is_owner`,`default_enterprise_id`,`default_org_unit_id`)
VALUES (UNHEX('00000000000000000000000000000003'),UNHEX('00000000000000000000000000000001'),UNHEX('00000000000000000000000000000002'),'user','active','Y',UNHEX('00000000000000000000000000000004'),UNHEX('00000000000000000000000000000005'));

INSERT IGNORE INTO `sys_role` (`id`,`tenant_id`,`role_code`,`role_name`,`role_kind`,`data_scope`,`status`)
VALUES (UNHEX('00000000000000000000000000000006'),UNHEX('00000000000000000000000000000001'),'sys_admin','系统管理员','system','tenant','active');

INSERT IGNORE INTO `sys_user_role` (`id`,`tenant_id`,`member_id`,`role_id`,`status`)
VALUES (UNHEX('00000000000000000000000000000007'),UNHEX('00000000000000000000000000000001'),UNHEX('00000000000000000000000000000002'),UNHEX('00000000000000000000000000000006'),'active');

-- =============================================================================
-- 二、平台内建字典（30 类 · 111 项；取值与母版 DDL 的 COMMENT/CHECK 逐字一致）
--     ID 段：类型 = 0x…00E0xxxx；取值 = 0x…0100xxxx
-- =============================================================================
-- 01 tenant_mode 租户隔离模式
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00001'),'tenant_mode','租户隔离模式','mox_sys','Y','sys_tenant.tenant_mode：logical/physical/hybrid');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='tenant_mode');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000001'),@et,'logical','逻辑隔离（共享实例）',1,'Y','active'),
(UNHEX('00000000000000000000000001000002'),@et,'physical','物理隔离（独立库）',2,'N','active'),
(UNHEX('00000000000000000000000001000003'),@et,'hybrid','混合隔离',3,'N','active');

-- 02 org_type 组织节点类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00002'),'org_type','组织节点类型','mox_sys','Y','sys_org_unit.org_type：root/company/dept/team/virtual');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='org_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000004'),@et,'root','根节点',1,'N','active'),
(UNHEX('00000000000000000000000001000005'),@et,'company','公司',2,'N','active'),
(UNHEX('00000000000000000000000001000006'),@et,'dept','部门',3,'Y','active'),
(UNHEX('00000000000000000000000001000007'),@et,'team','团队',4,'N','active'),
(UNHEX('00000000000000000000000001000008'),@et,'virtual','虚拟组织',5,'N','active');

-- 03 provider_type 身份提供方类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00003'),'provider_type','身份提供方类型','mox_sys','Y','sys_identity_provider.provider_type：oidc/saml/ldap/wechat/dingtalk');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='provider_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000009'),@et,'oidc','OIDC',1,'N','active'),
(UNHEX('0000000000000000000000000100000a'),@et,'saml','SAML',2,'N','active'),
(UNHEX('0000000000000000000000000100000b'),@et,'ldap','LDAP/AD',3,'N','active'),
(UNHEX('0000000000000000000000000100000c'),@et,'wechat','微信',4,'N','active'),
(UNHEX('0000000000000000000000000100000d'),@et,'dingtalk','钉钉',5,'N','active');

-- 04 factor_type MFA 因子类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00004'),'factor_type','MFA 因子类型','mox_sys','Y','sys_user_mfa.factor_type：totp/webauthn/sms/email');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='factor_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100000e'),@et,'totp','TOTP 动态码',1,'N','active'),
(UNHEX('0000000000000000000000000100000f'),@et,'webauthn','WebAuthn 通行密钥',2,'N','active'),
(UNHEX('00000000000000000000000001000010'),@et,'sms','短信验证码',3,'N','active'),
(UNHEX('00000000000000000000000001000011'),@et,'email','邮件验证码',4,'N','active');

-- 05 org_relation 组织成员关系
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00005'),'org_relation','组织成员关系','mox_sys','Y','sys_org_member.relation_type：member/manager/owner');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='org_relation');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000012'),@et,'member','成员',1,'Y','active'),
(UNHEX('00000000000000000000000001000013'),@et,'manager','主管',2,'N','active'),
(UNHEX('00000000000000000000000001000014'),@et,'owner','负责人',3,'N','active');

-- 06 role_kind 角色类别
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00006'),'role_kind','角色类别','mox_sys','Y','sys_role.role_kind：system/business');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='role_kind');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000015'),@et,'system','系统内置',1,'N','active'),
(UNHEX('00000000000000000000000001000016'),@et,'business','业务角色',2,'Y','active');

-- 07 data_scope 数据范围
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00007'),'data_scope','数据范围','mox_sys','Y','sys_role.data_scope：self/department/enterprise/tenant/custom');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='data_scope');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000017'),@et,'self','本人',1,'Y','active'),
(UNHEX('00000000000000000000000001000018'),@et,'department','本部门',2,'N','active'),
(UNHEX('00000000000000000000000001000019'),@et,'enterprise','本企业',3,'N','active'),
(UNHEX('0000000000000000000000000100001a'),@et,'tenant','本租户',4,'N','active'),
(UNHEX('0000000000000000000000000100001b'),@et,'custom','自定义',5,'N','active');

-- 08 resource_kind 资源类别
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00008'),'resource_kind','资源类别','mox_sys','Y','sys_resource.resource_kind：api/menu/data/field');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='resource_kind');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100001c'),@et,'api','接口',1,'Y','active'),
(UNHEX('0000000000000000000000000100001d'),@et,'menu','菜单',2,'N','active'),
(UNHEX('0000000000000000000000000100001e'),@et,'data','数据',3,'N','active'),
(UNHEX('0000000000000000000000000100001f'),@et,'field','字段',4,'N','active');

-- 09 menu_type 菜单类别
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00009'),'menu_type','菜单类别','mox_sys','Y','sys_menu.menu_type：catalog/menu/button');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='menu_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000020'),@et,'catalog','目录',1,'N','active'),
(UNHEX('00000000000000000000000001000021'),@et,'menu','菜单',2,'Y','active'),
(UNHEX('00000000000000000000000001000022'),@et,'button','按钮',3,'N','active');

-- 10 scope_kind 配置范围（A 级 CHECK：G/T/E/U）
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000a'),'scope_kind','配置范围','mox_sys','Y','sys_setting.scope_kind：G全局/T租户/E企业/U用户');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='scope_kind');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000023'),@et,'G','全局',1,'N','active'),
(UNHEX('00000000000000000000000001000024'),@et,'T','租户',2,'N','active'),
(UNHEX('00000000000000000000000001000025'),@et,'E','企业',3,'N','active'),
(UNHEX('00000000000000000000000001000026'),@et,'U','用户',4,'N','active');

-- 11 value_type 配置值类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000b'),'value_type','配置值类型','mox_sys','Y','sys_setting.value_type：TEXT/JSON/NUMBER/BOOLEAN');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='value_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000027'),@et,'TEXT','文本',1,'Y','active'),
(UNHEX('00000000000000000000000001000028'),@et,'JSON','JSON',2,'N','active'),
(UNHEX('00000000000000000000000001000029'),@et,'NUMBER','数值',3,'N','active'),
(UNHEX('0000000000000000000000000100002a'),@et,'BOOLEAN','布尔',4,'N','active');

-- 12 rollout_type 特性开关投放
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000c'),'rollout_type','特性开关投放','mox_sys','Y','sys_feature_flag.rollout_type：global/tenant/user/percent');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='rollout_type');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100002b'),@et,'global','全局',1,'Y','active'),
(UNHEX('0000000000000000000000000100002c'),@et,'tenant','按租户',2,'N','active'),
(UNHEX('0000000000000000000000000100002d'),@et,'user','按用户',3,'N','active'),
(UNHEX('0000000000000000000000000100002e'),@et,'percent','按百分比',4,'N','active');

-- 13 rollout_target 投放目标类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000d'),'rollout_target','投放目标类型','mox_sys','Y','sys_feature_flag_rollout.target_type：tenant/user/percent');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='rollout_target');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100002f'),@et,'tenant','按租户',1,'N','active'),
(UNHEX('00000000000000000000000001000030'),@et,'user','按用户',2,'N','active'),
(UNHEX('00000000000000000000000001000031'),@et,'percent','按百分比',3,'N','active');

-- 14 job_trigger 作业触发类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000e'),'job_trigger','作业触发类型','mox_sys','Y','sys_job.trigger_type：cron/interval/once');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='job_trigger');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000032'),@et,'cron','Cron 表达式',1,'Y','active'),
(UNHEX('00000000000000000000000001000033'),@et,'interval','固定间隔',2,'N','active'),
(UNHEX('00000000000000000000000001000034'),@et,'once','单次',3,'N','active');

-- 15 outbox_status 事件投递状态
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0000f'),'outbox_status','事件投递状态','mox_sys','Y','sys_outbox_event.status：pending/retrying/sent/dead');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='outbox_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000035'),@et,'pending','待投递',1,'Y','active'),
(UNHEX('00000000000000000000000001000036'),@et,'retrying','重试中',2,'N','active'),
(UNHEX('00000000000000000000000001000037'),@et,'sent','已投递',3,'N','active'),
(UNHEX('00000000000000000000000001000038'),@et,'dead','死信',4,'N','active');

-- 16 inbox_status 消费状态
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00010'),'inbox_status','消费状态','mox_sys','Y','sys_inbox_event.status：processed/skipped/failed');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='inbox_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000039'),@et,'processed','已处理',1,'Y','active'),
(UNHEX('0000000000000000000000000100003a'),@et,'skipped','已跳过',2,'N','active'),
(UNHEX('0000000000000000000000000100003b'),@et,'failed','失败',3,'N','active');

-- 17 job_run_status 作业运行状态
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00011'),'job_run_status','作业运行状态','mox_sys','Y','sys_job_run.status：running/success/failed/cancelled');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='job_run_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100003c'),@et,'running','运行中',1,'Y','active'),
(UNHEX('0000000000000000000000000100003d'),@et,'success','成功',2,'N','active'),
(UNHEX('0000000000000000000000000100003e'),@et,'failed','失败',3,'N','active'),
(UNHEX('0000000000000000000000000100003f'),@et,'cancelled','已取消',4,'N','active');

-- 18 notify_channel 通知渠道
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00012'),'notify_channel','通知渠道','mox_sys','Y','sys_notification_*.channel：email/sms/push/inapp/webhook');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='notify_channel');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000040'),@et,'email','邮件',1,'N','active'),
(UNHEX('00000000000000000000000001000041'),@et,'sms','短信',2,'N','active'),
(UNHEX('00000000000000000000000001000042'),@et,'push','推送',3,'N','active'),
(UNHEX('00000000000000000000000001000043'),@et,'inapp','站内信',4,'Y','active'),
(UNHEX('00000000000000000000000001000044'),@et,'webhook','Webhook',5,'N','active');

-- 19 notify_topic 通知主题
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00013'),'notify_topic','通知主题','mox_sys','Y','sys_notification_pref.topic_code：system/marketing/security');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='notify_topic');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000045'),@et,'system','系统通知',1,'N','active'),
(UNHEX('00000000000000000000000001000046'),@et,'marketing','营销',2,'N','active'),
(UNHEX('00000000000000000000000001000047'),@et,'security','安全',3,'N','active');

-- 20 notify_status 通知状态
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00014'),'notify_status','通知状态','mox_sys','Y','sys_notification_message.status：pending/sent/failed/read');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='notify_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000048'),@et,'pending','待发送',1,'Y','active'),
(UNHEX('00000000000000000000000001000049'),@et,'sent','已发送',2,'N','active'),
(UNHEX('0000000000000000000000000100004a'),@et,'failed','失败',3,'N','active'),
(UNHEX('0000000000000000000000000100004b'),@et,'read','已读',4,'N','active');

-- 21 file_usage 文件用途
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00015'),'file_usage','文件用途','mox_sys','Y','sys_file_link.usage_code：avatar/attachment/cover');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='file_usage');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100004c'),@et,'avatar','头像',1,'N','active'),
(UNHEX('0000000000000000000000000100004d'),@et,'attachment','附件',2,'Y','active'),
(UNHEX('0000000000000000000000000100004e'),@et,'cover','封面',3,'N','active');

-- 22 protocol_code 连接协议
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00016'),'protocol_code','连接协议','mox_sys','Y','sys_connector_endpoint.protocol_code：http/sftp/s3/sql/kafka');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='protocol_code');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100004f'),@et,'http','HTTP(S)',1,'N','active'),
(UNHEX('00000000000000000000000001000050'),@et,'sftp','SFTP',2,'N','active'),
(UNHEX('00000000000000000000000001000051'),@et,'s3','S3 对象存储',3,'N','active'),
(UNHEX('00000000000000000000000001000052'),@et,'sql','SQL',4,'N','active'),
(UNHEX('00000000000000000000000001000053'),@et,'kafka','Kafka',5,'N','active');

-- 23 cred_kind 凭证类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00017'),'cred_kind','凭证类型','mox_sys','Y','sys_connector_credential.cred_kind：apikey/token/oauth/basic');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='cred_kind');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000054'),@et,'apikey','API Key',1,'N','active'),
(UNHEX('00000000000000000000000001000055'),@et,'token','Token',2,'N','active'),
(UNHEX('00000000000000000000000001000056'),@et,'oauth','OAuth',3,'N','active'),
(UNHEX('00000000000000000000000001000057'),@et,'basic','Basic 账号密码',4,'N','active');

-- 24 conn_call_status 连接器调用结果
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00018'),'conn_call_status','连接器调用结果','mox_sys','Y','sys_connector_call.status：ok/failed/timeout');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='conn_call_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000058'),@et,'ok','成功',1,'Y','active'),
(UNHEX('00000000000000000000000001000059'),@et,'failed','失败',2,'N','active'),
(UNHEX('0000000000000000000000000100005a'),@et,'timeout','超时',3,'N','active');

-- 25 meter_code 计量项
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E00019'),'meter_code','计量项','mox_sys','Y','sys_usage_meter.meter_code：api_call/storage_mb/seat');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='meter_code');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100005b'),@et,'api_call','API 调用',1,'N','active'),
(UNHEX('0000000000000000000000000100005c'),@et,'storage_mb','存储 MB',2,'N','active'),
(UNHEX('0000000000000000000000000100005d'),@et,'seat','席位',3,'N','active');

-- 26 subscription_status 订阅状态
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0001a'),'subscription_status','订阅状态','mox_sys','Y','sys_subscription.status：active/trialing/cancelled/past_due');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='subscription_status');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('0000000000000000000000000100005e'),@et,'active','生效',1,'Y','active'),
(UNHEX('0000000000000000000000000100005f'),@et,'trialing','试用',2,'N','active'),
(UNHEX('00000000000000000000000001000060'),@et,'cancelled','已取消',3,'N','active'),
(UNHEX('00000000000000000000000001000061'),@et,'past_due','欠费',4,'N','active');

-- 27 effect 策略效果（A 级 CHECK：allow/deny）
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0001b'),'effect','策略效果','mox_sys','Y','sys_policy.effect：allow/deny');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='effect');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000062'),@et,'allow','允许',1,'Y','active'),
(UNHEX('00000000000000000000000001000063'),@et,'deny','拒绝',2,'N','active');

-- 28 rebac_subject 关系授权主体类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0001c'),'rebac_subject','关系授权主体','mox_sys','Y','sys_relation.subject_type：user/group');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='rebac_subject');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000064'),@et,'user','用户',1,'N','active'),
(UNHEX('00000000000000000000000001000065'),@et,'group','用户组',2,'N','active');

-- 29 rebac_object 关系授权客体类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0001d'),'rebac_object','关系授权客体','mox_sys','Y','sys_relation.object_type：doc/dataset/workspace');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='rebac_object');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000066'),@et,'doc','文档',1,'N','active'),
(UNHEX('00000000000000000000000001000067'),@et,'dataset','数据集',2,'N','active'),
(UNHEX('00000000000000000000000001000068'),@et,'workspace','工作空间',3,'N','active');

-- 30 rebac_relation 关系授权类型
INSERT IGNORE INTO `sys_enum_type` (`id`,`enum_code`,`enum_name`,`owner_module`,`is_system`,`description`)
VALUES (UNHEX('00000000000000000000000000E0001e'),'rebac_relation','关系授权类型','mox_sys','Y','sys_relation.relation_code：owner/editor/viewer/member');
SET @et := (SELECT `id` FROM `sys_enum_type` WHERE `enum_code`='rebac_relation');
INSERT IGNORE INTO `sys_enum_item` (`id`,`enum_type_id`,`item_code`,`item_name`,`sort_no`,`is_default`,`status`) VALUES
(UNHEX('00000000000000000000000001000069'),@et,'owner','所有者',1,'N','active'),
(UNHEX('0000000000000000000000000100006a'),@et,'editor','可编辑',2,'N','active'),
(UNHEX('0000000000000000000000000100006b'),@et,'viewer','只读',3,'N','active'),
(UNHEX('0000000000000000000000000100006c'),@et,'member','成员',4,'N','active');
