# 静态 class 引用尺子与两处回落样式欠账（2026-09-28）

权威叙述：`docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md` §5.29。同日承 `upload-row-collapse-20260929.md`、`time-f2-collapse-20260929.md`。

## 1. 尺子错过一次，现测才是数

- 第一版匹配器把 `:class="…"` 尾巴里的 `class="` 当静态属性 ⇒ 报 **63 文件 / 644 个**"未定义类名"，其中 `active:`、`===`、`{`、`+`、`'done'` 全是表达式碎片。误报读数只在会话 stdout，**没有归档成债表**（避免下一轮把假账当存量引用）。
- 修正（属性名前不得是 `:`／`-`／字母；值含 `{` 不算静态）后现测 **71 文件 / 230 个**静态类名在其 SFC `<style>` 与 6 个全局 css（629 个类名）里都无定义 ⇒ 归档 `reports/data/static-class-missing-20260929.json`（7,207 B）。
- **这 230 个不是债务总额**：尺子只有"同文件 scoped + 全局 css"两个来源，第三方组件内部类、父级 unscoped 命中、写在拼接值里的类名都会混进来。逐个判要不要补样式需要渲染层证据，而 `:3020`／`:3080` 本轮 DOWN ⇒ 登记为待逐文件裁决的余账（task #23）。

## 2. 本轮只收自己欠的两条

| 文件 | 类名 | 补的规则 | 依据 |
|------|------|---------|------|
| `views/admin/panels/AdminRole.vue` | `muted` | `.muted { color: var(--text-3); }` | 与 `AdminMenu.vue:472` 同值同写法 |
| `views/project/ProjectsView.vue` | `cat-icon-fallback` | `.cat-icon-fallback { font-size: 12px; line-height: 1; }` | 该 span 已 `:style` 继承分类色，规则只管字号与行高 |

两处都是 §5.25/§5.26 我加的图标文本回落分支——画得出来但没样式。

新钉 `src/views/_fallback-classes.test.js`（6 例：4 组「用到＋同文件有规则」配对／去掉注释后仍判得出定义存在／两处回落分支仍挂着 `nav-icons` 解析）。两枚变异体各删掉刚补的那条规则 ⇒ 各打红 2 例（配对例＋去注释例），还原 sha `c5090a0604ff`／`253267048968`，见 `reports/data/fallback-class-mutation-witness.txt`（924 B）。

## 3. 测试自身的失败先记账

① 我写了 `t.rindex('<style')`（Python 习惯）而非 `t.lastIndexOf`，首跑 7 例全 TypeError——仪器坏不是判据红。② 断言 `toContain('from nav-icons')` 而真实 import 是 `from '@/modules/_kernel/nav-icons'`，子句本就不存在。③ 有一对 `(ProjectsView, muted)` 在删前没验过"模板是否真用它"，直接从表里撤下，不留着赌它绿。

## 4. 门禁

`check-sfc-dead-refs.mjs` `verdict=PASS`；`check-frontend-module.py rc=0`；`npx vite build rc=0`（`✓ built in 31.43s`，`reports/data/fallback-build.txt`）；`_fallback-classes.test.js` 单跑 6 例绿；`src/views src/utils src/modules` 整跑通过但该次 `tail -3` 只截到 `Duration` 行，判决行的用例数没被读到 ⇒ 数字不复述，引用请复算。`check-doc-links.py` rc=1 属存量，WARN 120 未增。

## 5. 未提交

本轮新增 2 处样式规则（在 2 个已改文件里）+ 1 个新测试 + §5.29 + 3 份 `reports/data` 工件；加上同日前三件累计：11 个源文件、4 个新测试、1 个新 `_kernel` 模块、文档、15+3 份工件。**不做 `git add -A`／`stash`，等用户点名。**
