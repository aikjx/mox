# -*- coding: utf-8 -*-
import hashlib
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

folder = Path(__file__).resolve().parent
root = folder.parents[2]
names = ['paths_selftest', 'paths', 'docs', 'ports', 'frontend_module',
         'api_bindings', 'feedback_imports', 'framework_imports', 'theme', 'locale',
         'catalog', 'formulas', 'api_surface', 'diagrams', 'node_transport', 'diff',
         'format', 'rust_final', 'workspace_clippy', 'frontend_build_final',
         'frontend_runtime_final', 'real_runtime_final', 'navigation_runtime', 'docs_final', 'diff_final']
checks = {name: json.loads((folder / (name + '.json')).read_text(encoding='utf-8')) for name in names}
assert all(value['exit_code'] == 0 for value in checks.values()), checks
for log, files in {
    'real_runtime_final.log': ['reports/data/20261003-event-page-recovery/verify-browser.mjs',
                             'reports/data/20261003-event-page-recovery/verify-recovery.mjs',
                             'reports/html/20261003-event-page-recovery/assets/harness.js'],
    'frontend_runtime_final.log': ['frontend-ui/src/modules/admin-lowcode/composables/useCrudPage.js',
                                   'frontend-ui/src/modules/admin-lowcode/pages/access.page.js',
                                   'frontend-ui/src/modules/admin-lowcode/pages/audit.page.js'],
}.items():
    assert all((folder / log).stat().st_mtime >= (root / rel).stat().st_mtime for rel in files), 'Check output predates changed source: ' + log
rust = (folder / 'rust_final.log').read_text(encoding='utf-8', errors='replace')
groups = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', rust)
front = (folder / 'frontend_runtime_final.log').read_text(encoding='utf-8', errors='replace')
assert '85 passed (85)' in front and '1072 passed (1072)' in front
real = (folder / 'real_runtime_final.log').read_text(encoding='utf-8', errors='replace')
markers = ['REAL_RECOVERY:', 'REAL_BROWSER_RECOVERY:']
assert all(marker in real for marker in markers)
diagram_text = (folder / 'diagrams.log').read_text(encoding='utf-8', errors='replace')
diagrams = int(re.search(r'validated (\d+)', diagram_text).group(1))
sources = [
    'frontend-ui/src/modules/expert-alliance/contract/event-stream.js',
    'frontend-ui/src/modules/expert-alliance/contract/event-recovery.js',
    'frontend-ui/src/modules/expert-alliance/contract/favorite-request.js',
    'frontend-ui/src/modules/expert-alliance/contract/orchestration.js',
    'frontend-ui/src/modules/expert-alliance/model/request-fence.js',
    'frontend-ui/src/modules/expert-alliance/composables/useAllianceEventStream.js',
    'frontend-ui/src/modules/expert-alliance/components/EventConnectionStatus.vue',
    'frontend-ui/src/modules/expert-alliance/store/alliance-experts.store.js',
    'frontend-ui/src/modules/expert-alliance/store/alliance-orch.store.js',
    'frontend-ui/src/modules/expert-alliance/views/AllianceExpertsView.vue',
    'frontend-ui/src/modules/expert-alliance/views/AllianceOrchestrationView.vue',
    'frontend-ui/src/modules/admin-lowcode/composables/useCrudPage.js',
    'frontend-ui/src/modules/admin-lowcode/pages/access.page.js',
    'frontend-ui/src/modules/admin-lowcode/pages/audit.page.js',
    'frontend-ui/src/constants/nav.config.js',
    'platform/gateway/mox-platform-gateway-svc/tests/event_page_recovery.rs',
    'reports/data/20261003-event-page-recovery/verify-recovery.mjs',
    'reports/data/20261003-event-page-recovery/verify-browser.mjs',
    'reports/html/20261003-event-page-recovery/assets/harness.js',
    'docs/expert-alliance/21-event-delivery-contract.md',
    'docs/architecture/frontend/LOWCODE-PAGE-RUNTIME.md',
]
summary = {
    'status': 'verified_increment_full_platform_goal_still_active',
    'finalized_utc': datetime.now(timezone.utc).isoformat(),
    'head_at_finalization': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=str(root), text=True).strip(),
    'scope': 'Event recovery/shared page component and stores, low-code runtime identity injection, module navigation and orchestration provenance; not all enterprise capabilities',
    'checks': checks,
    'default_member_rust_passed': sum(int(group[0]) for group in groups),
    'default_member_rust_ignored': sum(int(group[2]) for group in groups),
    'default_member_rust_result_groups': len(groups),
    'frontend_test_files': 85, 'frontend_tests_passed': 1072,
    'mermaid_diagrams_parsed': diagrams,
    'real_probe': {'markers': markers, 'transport': 'actual TCP, production JWT/routes, temporary SQLite',
        'browser': 'independent installed Chromium context, Vite-built shared component/composable/Pinia/Axios; no API response replacement',
        'faults': ['real EventBus overflow', 'unknown cursor to actual Rust 410', 'delay actual old-identity stats response'],
        'not_covered': ['login form', 'all product routes', 'external model/OSS', 'multi-instance failover']},
    'test_boundary': 'Legacy local unit API/component stubs remain; new favorite snapshot calls real Rust. Unit count is not an enterprise readiness claim.',
    'sha256': {rel: hashlib.sha256((root / rel).read_bytes()).hexdigest() for rel in sources},
    'open_requirements': ['Webhook durable outbox/signing/dead-letter/recovery', 'plan atomicity and cross-instance CAS/live events',
        'KB extension relation/search source normalization and authority', 'real model/tool execution and OSS',
        '26-module exit criteria, all business pages, production SLO/disaster recovery'],
}
(folder / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
report = root / 'reports/markdown/20261003-event-page-recovery.md'
text = report.read_text(encoding='utf-8')
heading = '\n## 最终验证结果（2026-10-04）\n'
if heading in text:
    text = text.split(heading)[0]
text += heading + '''
- Cargo 默认成员：{rust} 项通过，{ignored} 项存量 ignored；不包含所有非默认成员。
- 前端全量：85 个文件、1072 项通过，含仓库历史局部替身；新收藏读取直接接入 Rust/JWT/SQLite。
- 实际 Node 恢复控制器与独立 Chromium：失效游标、真实广播缺口、可见错误、键盘重连、事件触发刷新、身份清游标及旧统计零瞬态回填通过。
- 根 workspace Clippy、前端生产构建与记录的 17 个治理/传输/格式检查退出码为 0；存量 Clippy 和测试组件告警仍保留。
- Mermaid 实际解析 {diagrams} 张模块关系和业务流程图。

低代码最终设计为运行时注入 identityScope(auth)。页面声明没有认证 store 依赖；未使用深路径导入例外或削弱门禁。静态导航仍保留原权威，不另建导航注册主源。最初失败和被否决的导航抽离实验保留日志，不计入最终通过项。

真实浏览器复现的 410 缓存问题通过事件 fetch cache=no-store 修复。迟到统计验收监视新租户下的每次统计回填，要求即使瞬态也不能出现旧值。浏览器会合并同 URL 在途缓存请求，探针释放旧响应后再验证新读取，不用加长等待替代归属断言。

实现台账保持完整目标开放。下一处源码复核缺口是 KB 扩展域：实体搜索仍固定返回空，文档实体关系另存全局集合，尚未绑定主 KB 的文档权限和版本；该项尚无真实修复验收，不能沿用主 KB 测试推断其可用。

浏览器截图：[恢复错误](../html/20261003-event-page-recovery/assets/recovery-error.png) · [身份切换后恢复](../html/20261003-event-page-recovery/assets/recovery-success.png)。
'''.format(rust=summary['default_member_rust_passed'], ignored=summary['default_member_rust_ignored'], diagrams=diagrams)
report.write_text(text, encoding='utf-8')
print('Finalized', len(checks), 'captured checks; full platform goal remains active')
