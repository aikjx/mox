# AdminMenu 图标登记口收口见证（2026-09-28）

单元：`frontend-ui/src/views/admin/panels/AdminMenu.vue`（菜单配置面板）。权威叙述在
`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.25；本报告只存**印出这些数字的那份原始产物**，
不复述结论。

## 1. 本轮改的文件（三口径状态，`git status --porcelain -uall` 原样）

```
AM docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md
AM frontend-ui/src/views/admin/panels/AdminMenu.smoke.test.js
MM frontend-ui/src/views/admin/panels/AdminMenu.vue
AM frontend-ui/src/views/admin/panels/_smoke.js
```

四个名字都是**未提交**的工作树改动；`MM`/`AM` 表示索引里另有一份更早的拷贝（本会话不碰索引，见 §5.21）。
入库点名清单在 #19，须由用户逐名点。

## 2. 变异电池判决原始产物

`reports/data/adminmenu-icon-mutation-witness-20260928.txt`（742 B，本轮 stdout 原样归档）：

- 基线 `rc=0 2 passed (2)`，三枚变异体各自 `rc=1 1 failed | 1 passed (2)`，每枚还原后复跑 `rc=0`。
- 驱动 `D:\tmp\mox-icon-mut.py` 未落库 ⇒ 对 CI 没牙；本节数字只有重跑驱动才复现。

## 3. 门禁产物

| 产物 | 大小 | 判决行 |
|---|---:|---|
| `reports/data/adminmenu-vite-build-20260928.txt` | 19,423 B | `✓ built in 34.39s` |
| `reports/data/adminmenu-panel-suite-20260928.txt` | 278,032 B | `Test Files 17 passed (17)` / `Tests 23 passed (23)` |
| `reports/data/adminmenu-sfc-deadrefs-20260928.txt` | 155 B | `deadRefFiles=1 verdict=FAIL`，唯一一条 `src/views/project/ProjectsView.vue :: projectMemberCount`（存量，本轮 0 新增） |

另两条命令本轮只落 stdout、未单独归档：`npx vitest run src/modules/_kernel/nav-icons.test.js` → `9 passed (9)`；
`python scripts/gate/check-frontend-module.py` → `ERROR=0`。

## 4. 没拿到的证据（不许当已通过）

`:3020`（Vite dev）、`:3080`（网关）、`:3001`（编排器）本轮 `netstat -ano | grep LISTEN` 三条全零命中 ⇒
真机页面上"图标列画得出 `<svg>`／未登记名打印原始文本"从未被眼看。三枚变异体证明的是**测试有牙**，
不是**页面渲染正确**。补这条证据需要 dev server ＋ 网关同时在线（菜单树来自 `/api` 真实回包），
留作下轮开工第一件事。

## 5. 余账：登记口未接解析器的 19 处裸 `:is`

复算命令（`frontend-ui/` 下）：

```bash
grep -rnE ':is="[a-z][A-Za-z.]*[Ii]con"' src/ --include=*.vue | grep -v "navIcon("
```

19 处 / 15 文件，逐条坐标见 §5.25.3。本文件从 2 处降到 0 处；`ProjectPicker.vue:193` 属孤儿组件、
`views/project/panels/KnowledgeBasePanel.vue:828` 落在说明文字里——两条都要在收口时先归因再动手，
不做机械替换（#14）。

## 6. 同一轮的下一单元：19 处逐位归因 ⇒ 只有 2 处是真缺陷（已收口）

权威叙述在 `docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.26。本节只存产物。

- 真缺陷 2 处（外部字符串直达 `:is`）：`views/admin/panels/AdminRole.vue:184`（菜单树 ← `getMenuTree()`，
  与本报告 §1 那张菜单表同源）、`views/project/ProjectsView.vue:383`（分类 ← `getProjectTypes()`）。两处均已改
  `navIcon()` ＋ 名单外文本回落。
- 明确不动的 12 处：常量字符串且名字全在登记口内（`nav.config.js` 现场数得 `NAV_MODULES` 12／`QUICK_CREATE_COMMANDS` 6，
  名单外 PascalCase 各 0 个），换解析器零用户收益。
- 明确禁止换的 3 处：喂的是组件对象（`AgentFlowPanel.vue:31`、`ExpertEnterprisePanel.vue:25` 的 `markRaw(...)`、
  `project/panels/KnowledgeBasePanel.vue:19`）——`navIcon()` 对非字符串返回 `null`，机械替换会制造新空白。
- 仪器假阳 2 处：注释行 `KnowledgeBasePanel.vue:828`、孤儿 `ProjectPicker.vue:193`。
- 新落库门禁 `src/views/_icon-registry-consumers.test.js`（按编译产物钉"外部 icon 名不许直达 `:is`"）。
  它的第四枚变异体（只撤 `:is` 一侧、留 `v-if` 的 `navIcon`）在**第一版正则下是绿的**，即那道断言原本是死针；
  按实测编译形态改写正则后才红。这一段是本轮最耐久的产出。

原始产物（本轮归档）：

| 文件 | 大小 | 内容 |
|---|---:|---|
| `reports/data/icon-consumers-mutation-witness-20260928.txt` | 1,010 B | 基线＋四枚变异体＋每枚还原复跑（M1–M4 全 `rc=1`，还原后 `rc=0`） |
| `reports/data/icon-consumers-views-suite-20260928.txt` | 371,700 B | `src/views` ＋ `_kernel` 全套：`23 files / 108 tests / rc=0` |
| `reports/data/icon-consumers-vite-build-20260928.txt` | 19,423 B | `✓ built in 33.59s` |
| `reports/data/adminmenu-compiled-render-20260928.txt` | 36,685 B | `AdminMenu.render` 编译产物原样（两条正则的形状证据） |

未拿到的证据同 §4：A 类两处的真机渲染（角色权限树节点、项目分类选择器）仍未被眼看，三端口 `:3020`／`:3080`／`:3001`
本轮全部无 LISTEN 记录。驱动的"哪道断言打红"通道也没读数（抓失败行的格式假设错了），
所以 M1–M3 只报"红"，M4 的红由构造保证挂在第二道断言上。

