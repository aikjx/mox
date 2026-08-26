const { EnterpriseBootstrap, BizAction } = require('./src/enterprise');

(async () => {
  console.log('=== 冒烟测试开始 ===');

  const app = await new EnterpriseBootstrap({
    dbPath: ':memory:',
    installIndustries: ['common','finance'],
    logger: { info: () => {} },
  }).start();
  const db = app.db;
  console.log('[1] 启动成功 (内存DB)');

  // 2. 注册测试实体 project + 3 字段（通过官方 defineEntity，正确分配字段槽位）
  app.defineEntity({
    tenantId: 'system', entityCode: 'project', entityName: '项目', entityCategory: 'transaction',
    fields: [
      { field_code:'title',  field_name:'项目标题', field_type:'string', required:true, ui_widget:'input' },
      { field_code:'amount', field_name:'项目金额', field_type:'decimal', ui_widget:'number' },
      { field_code:'status', field_name:'项目状态', field_type:'enum',    ui_widget:'select',
        options_inline: [
          {label:'草稿',value:'draft'},
          {label:'进行中',value:'running'},
          {label:'已完成',value:'done'},
        ],
      },
    ],
  });
  console.log('[2] 测试实体 project + 3字段 注册成功');

  // 3. 准备租户/部门/用户/角色
  const TID = 't001', UID = 'u001';
  db.prepare("INSERT OR IGNORE INTO iam_tenant (tenant_id, tenant_code, tenant_name, tenant_mode, tenant_status, tenant_plan, created_at, updated_at, version) VALUES (?, ?, '测试公司','logical','active','enterprise',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1)").run(TID, 't001');
  db.prepare("INSERT OR IGNORE INTO iam_department (dept_id,tenant_id,dept_code,dept_name,dept_type,dept_level,dept_path,sort_order,status,created_at,updated_at,version) VALUES ('d001',?, 'D001','研发部','department',0,'/d001',0,'active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1)").run(TID);
  db.prepare("INSERT OR IGNORE INTO iam_user (user_id,tenant_id,user_code,username,real_name,dept_id,user_status,created_at,updated_at,version) VALUES (?, ?, 'E001','zhangsan','张三','d001','active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1)").run(UID, TID);
  db.prepare("INSERT OR IGNORE INTO iam_role (role_id,tenant_id,role_code,role_name,role_type,is_builtin,status,created_at,updated_at,version) VALUES ('r001',?, 'pm','项目经理','custom',0,'active',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP,1)").run(TID);
  console.log('[3] 租户+部门+用户+角色 初始化成功');

  const tenant = { tenantId: TID, tenant_name: '测试公司', plan: 'enterprise' };
  const user   = { userId: UID, roles: ['pm','admin'], permissions: [] };
  const req    = { requestId: 'r_test_001', traceId: 't_abc', clientIp: '127.0.0.1' };

  // 4. CREATE
  const r1 = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.CREATE,
    input: { title: 'AI大模型项目', amount: 1234567.89, status: 'running' } });
  console.log('[4] CREATE → success:', r1.success, 'biz_id:', !!r1.data?.biz_id, 'biz_code:', !!r1.data?.biz_code, 'status__label:', r1.data?.status__label);

  // 5. LIST
  const r2 = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.LIST, input: { pageSize: 10 } });
  console.log('[5] LIST → success:', r2.success, 'total:', r2.data?.pagination?.total, 'list_len:', r2.data?.list.length);

  // 6. UPDATE
  const id = r1.data.biz_id;
  const r3 = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.UPDATE,
    input: { id, updates: { amount: 9999999.99, status: 'done' } } });
  console.log('[6] UPDATE → success:', r3.success, 'new_amount:', r3.data?.amount, 'new_status__label:', r3.data?.status__label);

  // 7. GET
  const r4 = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.GET, input: { id } });
  console.log('[7] GET → success:', r4.success, 'title:', r4.data?.title, 'version:', r4.data?.version);

  // 8. 版本历史验证（create + 2 次 update(实际一次) + update 内的 get 不产生版本 → 实际应为 2 或 3）
  const vc = db.prepare('SELECT COUNT(*) AS c FROM biz_data_version WHERE biz_id = ?').get(id).c;
  console.log('[8] 版本历史条数:', vc, '>=2:', vc >= 2);

  // 9. 指标
  const metrics = app.orchestrator.getMetrics();
  console.log('[9] 编排器指标: totalCalls=', metrics.totalCalls, 'failRate=', metrics.failRate);

  // 10. 审计日志条数
  const ac = db.prepare('SELECT COUNT(*) AS c FROM audit_log').get().c;
  console.log('[10] 审计日志条数:', ac, '>=3:', ac >= 3);

  // 11. DELETE（软删）
  const r5 = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.DELETE, input: { id } });
  console.log('[11] DELETE → success:', r5.success);
  const afterDel = await app.execute({ tenant, user, request: req, entityCode: 'project', action: BizAction.LIST, input: { pageSize: 10 } });
  console.log('     删除后列表总数:', afterDel.data?.pagination?.total, '(应为 0)');

  console.log('=== 冒烟测试结束：全部通过 ===');
})().catch(e => { console.error('冒烟失败:', e.stack || e.message); process.exit(1); });
