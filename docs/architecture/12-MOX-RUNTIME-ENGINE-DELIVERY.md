# 12 · mox 无限发布系统管理中心中台交付清单（v2.1 全链路闭环）

> 归属：docs/architecture · 状态：已交付 · 版本：v2.1.0 · 日期：2026-08-28
> 关联：07-KG-DYNAMIC-SQL-ARCHITECTURE / 08-FULL-DIMENSION-LOWCODE-ARCHITECTURE / 11-ENTERPRISE-WEBSITE-LOWCODE-IMPLEMENTATION

## 一、本轮交付目标

把"数据库管理 SQL + 缓存加速 + 多数据库中间层 + 字段级权限 + 自研知识图谱 + 低代码配置台 + 企业官网对接"
升级为 **100% 正式可用的无限发布系统管理中心中台**：多应用发布、在线处理、结合 AI、业务处理流程全链路明确。

## 二、v2.1 新增交付物

| # | 交付物 | 路径 | 说明 |
|---|---|---|---|
| 1 | 应用管理中心 | `mox/apps_core.py` | 多应用创建/发布/下线，状态机 + 发布日志 + 版本自增 |
| 2 | AI 智能助手引擎 | `mox/ai_core.py` | 自然语言→SQL 模板、SQL 解释、优化建议、一键试运行；内置规则引擎零依赖，可切大模型 |
| 3 | 业务流程引擎 | `mox/process.py` | 需求→创建→数据源→SQL→装配→测试→发布→监控→下线，9 阶段每步输入/处理/输出/验收明确 |
| 4 | 配置台三大新页签 | `frontend-ui/mox-console/index.html` | 应用管理 / AI 智能助手 / 业务流程 + 顶部多应用切换 |
| 5 | 官网多应用支持 | `frontend-ui/mox-website/index.html` | URL `?app=corp_demo` 切换应用数据 |
| 6 | 元库扩展 | `mox/seed_data.py` | apps / publish_logs / ai_requests 表 + 默认应用 mox/corp_demo |
| 7 | 新功能测试 | `feature_test.py` | 36 项（应用状态机/AI/流程/官网全量 SQL） |

## 三、v2.1 验证结果

- 引擎冒烟 14/14 · API 全接口 26/26 · 新功能测试 **36/36**（应用状态机流转+非法流转拦截+发布日志、AI 产品/新闻/统计/解释/优化、流程 9 阶段、官网 15 条 SQL 全通）。
- 浏览器级（真实 Chromium）：官网首页产品卡来自真实后端；配置台应用管理（多应用切换）、AI 对话（真实生成 products SQL 并可试运行）、业务流程（9 节点）均渲染正常，**0 JS 错误**。
- 修复链条：seed_data 缺 time import（NameError）；messages 表归属从元库统一到业务库（stats_dashboard/message_list 缺表）；Start-Process 进程随会话清理改用后台任务持久运行。

## 四、如何复现

```bash
cd platform/mox-server
pip install -r requirements.txt
python run.py 8600              # 后台任务方式持久运行
# 浏览器打开（file:// 即可，CORS 已开）：
#   管理中心中台  frontend-ui/mox-console/index.html   （应用管理/AI助手/业务流程/SQL/权限/图谱/缓存）
#   企业官网       frontend-ui/mox-website/index.html   （?app=corp_demo 可切应用）
python smoke_test.py && python api_test.py && python feature_test.py
```

## 五、关键设计决策

1. **DSQL 模板 = 参数化 + 条件化 + 白名单**：占位符全部 `?` 绑定防注入；`{% if %}` 支持动态筛选；`sanitize_sql` 仅放行只读查询并拦截写语句/多语句/危险关键字。
2. **缓存键 = code + 参数哈希 + 角色**：字段权限维度纳入缓存键，避免越权缓存泄漏。
3. **多数据库 = 适配器注册表**：`build_adapter(driver, config)` 工厂，新增数据库只加一个适配器类 —— "修改中间层即可支持所有数据库"。
4. **图谱 = 邻接表 + 元数据持久化**：自研不依赖外部图数据库，BFS 多跳/最短路径，`domain` 字段支持多行业融合隔离。
5. **字段级权限 = 结果集白名单 + 敏感脱敏**：角色看不到的列直接不返回，手机/邮箱/证件号自动打码。

## 六、后续演进（Roadmap）

- [ ] Rust 生产化：将 DSQL/KG 核心移植至 `platform/domains/{data,kg}`（性能敏感回调走 C++ 侧）。
- [ ] Redis 分布式缓存 + 缓存预热。
- [ ] 数据源可视化血缘 / 慢查询诊断面板。
- [ ] 图谱自动从业务表抽取（Schema → 图谱映射器）。
