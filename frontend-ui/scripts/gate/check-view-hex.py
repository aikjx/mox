#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""存量裸 hex 棘轮 + 分类色板 + 正文灰度四级 + 状态色角色契约 + 表面阶梯四档（任务 #17/#24/#16）。

六类判定：

1. **裸 hex 棘轮**：扫 `src/**` 的 .vue/.js（减去 `SCAN_EXCLUDE`，见末尾「扫描范围」），按**文件**记数。任何一个文件的裸 hex 数
   比基线多就 FAIL；变少了要更新基线（否则同名问题复发无人拦）。
   为什么按文件而不是按总数：总数会被"这里删了 10 处、那里新加 10 处"互相抵掉。
2. **分类色板**：`--cat-1`…`--cat-8` 必须在 global 与每套皮肤里各自定义、各自取不同值，
   且值必须是颜色字面量。少一个 ⇒ 换肤后该档仍是默认皮的老值（这病已复发四次）。
   两个档撞色 ⇒ 图表里两条系列画成一个颜色，色板失去区分作用。
3. **正文灰度四级**：`--text-primary/secondary/tertiary/quaternary` 在 global 与每套皮肤里
   各自定义、各自取不同值。外壳曾把 tertiary 与 quaternary 共用一个值（AA 余量不够往下压，
   见 global.css 注释），于是按四级写层级差的页面在这一皮下静默退成三级；
   修法是抬 secondary 而不是压 quaternary。比值本身不在这里判 —— `src/modules/expert-alliance/style.test.js`
   的灰阶用例已经在算 AA，两处都算就是同题双源。
4. **状态色命名空间不许串门**：契约写在 global.css 的注释里 —— `--x` 专职文字与描边、
   `--x-fill` 专职色块底、`--on-x` 是那块底上的字。于是 `color:` 读 `--x-fill`、
   `background:` 读 `--on-x` 都是把两职重新并回一档（这病已经复发病四次，见 #13/#16/#23）。
   为什么现在钉得住：默认皮下 #047857 压卡面只有 2.67:1、#b45309 2.91:1，而同一语义作为
   `--success` 是 5.77:1 —— 串门不是风格问题，是跌破 AA。
   这条判据是被一次真实误伤逼出来的：把裸 hex 归一化到令牌时按「值相等」替换，
   会把 `color: #047857` 换成 `color: var(--success-fill)`（字面对、角色反，HEAD 里原本 0 处），
   当时只有人眼发现 ⇒ 让机器也看得见。
5. **同一契约的另两种串门形态**（判据 4 只读单个声明的值，看不需要配对的这两种）：
   - `text-as-fill`：`background:` 单值读 `--x` 而 `--x-fill` 存在 ⇒ 拿文字档当色块底。
     存量走**棘轮**（`ROLE_TIER_BASELINE`，现 36 处 / 16 文件）：这一批多是圆点、进度条、
     滚动条一类装饰性色块，换成 `--x-fill` 会让它们在四皮下同时变深，属设计取舍不是机械修，
     所以先钉住不许增长，逐处收要用户点头。
   - `fill-unpaired`：`background:` 读 `--x-fill`，**同一条规则里**有 `color:` 却不取 `--on-x`
     ⇒ 字色档串门。零容忍（实测 0，故不需基线）。
   两条都必须按「同一条规则内」配对才成立：跨规则的 `color` 是继承，不是配套。所以这里自己
   解析 CSS 规则，只看最内层 —— `@media` 里的子规则各自成一条，否则外层一条 `.l{color:var(--on-danger)}`
   会替内层 `.k{background:var(--danger-fill);color:#fff}` 圆场（selftest 里有这一对）。
   行号一律在**整份文件**上数换行，不在 `<style>` 切片上数（切片坐标系对不上真行号）。
6. **表面阶梯四档不许塌**：`--bg-*` 按层级分成四组（page / panel / mid / card，见 `SURFACE_GROUPS`），
   每套皮肤（含 default 皮 = global.css）里：组内别名跟完 `var()` 链后必须解析到同一个值，
   组与组之间必须各不相同。别名层（判据 B 那批 `--bg-surface`/`--bg-panel`/`--bg-raised`）写错或
   漏覆写 ⇒ `surface-alias`；两层同值 ⇒ `surface-collapse`，抬起的卡片与它下面的面板看不出边界，
   层级在视觉上消失（sky 皮下 `--bg-panel` 与 `--bg-card` 都是 #ffffff，已登记在
   `SURFACE_COLLAPSE_OK` 等设计裁决，见 #16/#23；这条豁免由 selftest 反证它今天确实还对应那笔账）。
   档名是**契约**不是从值推出来的：反过来推会把"某一皮下两层塌成一样"读成"它们本来就同一层"，
   而那正是这条判据要抓的东西。断链（`var(--不存在)` 且无兜底）报 `surface-missing`、成环报
   `surface-cycle`，都不许冒充"值不同"。亮度差与 AA 不在这里算 —— 判据 3 的同一条理由：算了就是同题双源。

扫描范围：判据 1 是 **`src/**` 全集减去 `SCAN_EXCLUDE`（现只有 `src/modules/`）**。
早先是三根白名单（views → +constants → +components），那样棘轮自己就是可洗账的：
把字面量搬进 `stores/`、或新建一个不在名单里的目录，就凭空"清掉"整份账 ——
实测欠账 81 处 / 13 个文件（`src/App.vue` 14、`src/stores/alliance.store.js` 14、
`src/stores/ai.store.js` 12…），而"搬家即清账"这件事在 constants 那一轮已经发生过一次
（去重把字面量从视图搬进常量表，只数视图会凭空少 300 处）。倒成"全集减排除"之后，
新目录默认入账，要逃出账必须显式改 `SCAN_EXCLUDE`，那是有人在名单上写名字，不是顺手搬家。
components 是专家类型去重时补进来的：注册对话框 RegisterExpertDialog 住在 components，
它那份色表分叉躲过了"只扫 views"的副本守卫（当时实测 25 个文件 696 处）。
`.test.js/.spec.js` 不参与：测试里的 hex 是"断言源表现在长什么样"，不是会渲染到像素上的东西。

「`src/modules/` 不进判据 1」：那一层的裸 hex 由 `src/modules/expert-alliance/style.test.js`
（变异体验证过的守卫）看着，两处都数会把同一件事报两遍。
判据 4/5 反过来，扫 `src/**/*.vue` 与 `src/**/*.css` 全集，**含** modules 那一层 ——
它原先只被裸 hex 与 AA 管着，角色串门没人记；纯 .css 没有 `<style>` 段，整份当 CSS 看。
`.vue` 里没有 `<style>` 时返回空片段，绝不退化成"扫整份文件"：`<script>` 里的字符串长得像
CSS，那么退化会假红。

    python scripts/gate/check-view-hex.py              # 打印台账
    python scripts/gate/check-view-hex.py --check      # 棘轮门禁（CI 用）
    python scripts/gate/check-view-hex.py --baseline   # 生成可粘贴的裸 hex 基线字典
    python scripts/gate/check-view-hex.py --role-baseline  # 生成 ROLE_TIER_BASELINE（判据 5 存量）
    python scripts/gate/check-view-hex.py --selftest   # 判据能否被证伪
"""
import glob
import io
import os
import re
import sys

UI = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(UI, 'src')
# 裸 hex 棘轮的扫描集是「`src/**` 全集 − 显式排除」，不是根目录白名单。
# 白名单会让棘轮自己变成可洗账的：把字面量搬进 stores/ 或新建一个目录就逃出账上，
# 而"搬家即清账"这件事在 constants 那一轮已经发生过一次（见开头说明）。
SCAN_EXCLUDE = ('src/modules/',)
STYLES = os.path.join(SRC, 'styles')
SKINS = ('dark', 'sky', 'cyberpunk')
CAT_COUNT = 8

if hasattr(sys.stdout, 'reconfigure'):
    # Windows 控制台默认 GBK，中文报告会变乱码；被管道给脚本时更会以 UnicodeDecodeError 直接失败
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')
CAT_NAMES = ['--cat-%d' % i for i in range(1, CAT_COUNT + 1)]
# 正文灰度的四级阶梯。四套皮肤各自要有这四档、各自取不同值：
# 外壳曾在 tertiary/quaternary 上共用一个值（理由记在 global.css 的注释里），
# 于是「默认皮」只有三级而其它皮有四级 —— 页面按四级写的层级差在这一皮下静默失效。
# 撞色与缺档在这里钉住；AA 比值不在本闸门重复计算，那由 style.test.js 的灰阶用例负责。
TEXT_TIERS = ['--text-primary', '--text-secondary', '--text-tertiary', '--text-quaternary']

# 表面阶梯的四个层级（判据 6）。名字是**契约**，不是从值里推出来的 —— 反过来推会把
# "某一皮下两层塌成一样"读成"它们本来就是同一层"，那正是这条判据要抓的东西。
# 每组内的别名必须解析到同一个值（别名层写错/漏覆写 ⇒ 立刻红），组与组必须各不相同
# （塌陷 ⇒ 抬起的卡片和它下面的面板看不出边界，这一层在 sky 皮下真的塌成一过）。
# `--bg-input` 不在表内：三套皮下它分别贴着 page / mid / card，没有稳定的层级，钉它=凭空造规则。
SURFACE_GROUPS = {
    'page': ('--bg-primary', '--bg-page'),
    'panel': ('--bg-secondary', '--bg-surface', '--bg-panel'),
    'mid': ('--bg-tertiary', '--bg-surface-2'),
    'card': ('--bg-card', '--bg-raised'),
}
# 已登记、待裁决的塌陷豁免（只许减少不许增加；某条对应的塌陷修好后这条必须一起删）。
SURFACE_COLLAPSE_OK = {('sky', 'panel', 'card')}

# 状态色命名空间串门：见开头判定 4。只认 var() 引用，裸 hex 判不了角色（值相同可以兼任两职，
# 那正是串门的成因，所以裸 hex 由棘轮单独收口）。
FILL_AS_TEXT_RE = re.compile(r'(?<![-\w])color\s*:\s*var\(\s*(--[\w-]+-fill)\b')
ON_AS_FILL_RE = re.compile(r'(?<![-\w])background(?:-color)?\s*:\s*var\(\s*(--on-[\w-]+)\b')
STYLE_BLOCK_RE = re.compile(r'<style[^>]*>.*?</style>', re.S)
STYLE_BODY_RE = re.compile(r'<style[^>]*>(.*?)</style>', re.S)
TOKEN_DEF_RE = re.compile(r'(--[A-Za-z0-9-]+)\s*:\s*([^;{}]+)')
# 上面两条只认「值以 var(--x-fill) 开头」这类形，判不出另外两种串门，因为它们要按
# **同一条规则内**配对才成立（跨规则的 color 是继承，不是配套）：
#   text-as-fill ：background 单值读 --x，而 --x-fill 存在 ⇒ 拿文字档当色块底（存量走棘轮）
#   fill-unpaired：background 读 --x-fill，同规则里有 color 却不取 --on-x ⇒ 字色档串门（零容忍）
FILL_IN_VALUE_RE = re.compile(r'var\(\s*--([\w-]+)-fill\b')
ON_IN_VALUE_RE = re.compile(r'var\(\s*--on-([\w-]+)\b')
SINGLE_VAR_RE = re.compile(r'^var\(\s*(--[\w-]+)\b[^)]*\)$')
BG_DECL_RE = re.compile(r'(?<![-\w])(background(?:-color)?)\s*:\s*([^;{]+)')
COLOR_DECL_RE = re.compile(r'(?<![-\w])color\s*:\s*([^;{]+)')

HEX_RE = re.compile(r'#[0-9a-fA-F]{3,8}\b')
COMMENT_RE = re.compile(r'/\*.*?\*/', re.S)
CAT_DEF_RE = re.compile(r'(--cat-\d+)\s*:\s*([^;]+);')

# 按文件钉住的裸 hex 基线（--baseline 生成，改完记得同步）
BASELINE = {
    'src/components/MessageBubble.vue': 154,
    'src/constants/expert.constants.js': 51,
    'src/views/expert/ExpertConfigView.vue': 49,
    'src/views/expert/panels/ExpertEnterprisePanel.vue': 2,
    'src/views/admin/panels/AdminLlm.vue': 23,
    'src/views/expert/ExpertPlazaView.vue': 42,
    'src/components/AgentFlowPanel.vue': 13,
    'src/components/ai/PhasePipeline.vue': 37,
    'src/views/graph/GraphView.vue': 18,
    'src/views/ai/InfiniteOptimizerView.vue': 8,
    'src/constants/nav.config.js': 24,
    'src/components/FlowDetailDialog.vue': 11,
    'src/views/project/panels/KnowledgeBasePanel.vue': 11,
    'src/views/project/Dashboard.vue': 27,
    'src/components/AgentTaskRunner.vue': 12,
    'src/components/layout/TheTopbar.vue': 5,
    'src/views/expert/panels/ExpertOverviewPanel.vue': 6,
    'src/components/AssistantSelector.vue': 12,
    'src/components/DagViewer.vue': 21,
    'src/components/ai/GateResult.vue': 21,
    'src/constants/operator.constants.js': 20,
    'src/components/PhasePipeline.vue': 18,
    'src/components/ai/ExpertCard.vue': 17,
    'src/views/expert/ExpertCenterView.vue': 17,
    'src/components/OnboardingGuide.vue': 14,
    'src/views/admin/panels/AdminOverview.vue': 14,
    'src/views/operators/OperatorsView.vue': 13,
    'src/views/workspace/panels/TaskOrchestrationPanel.vue': 14,
    'src/stores/alliance.store.js': 14,
    'src/components/ProjectChip.vue': 13,
    'src/components/ProjectPicker.vue': 13,
    'src/components/ai/AIChatPanel.vue': 1,
    'src/views/ai/CaomeiView.vue': 6,
    'src/views/market/MarketView.vue': 13,
    'src/views/workflow/panels/WorkflowFlowsPanel.vue': 7,
    'src/views/admin/panels/AdminMonitor.vue': 6,
    'src/views/graph/FlowGraph.vue': 11,
    'src/views/workspace/panels/ExpertPanel.vue': 12,
    'src/stores/ai.store.js': 12,
    'src/views/workspace/ExpertWorkspaceView.vue': 11,
    'src/components/common/StatusTag.vue': 10,
    'src/views/auth/Login.vue': 9,
    'src/views/public/BusinessHall.vue': 10,
    'src/views/project/ResourcesView.vue': 10,
    'src/views/workflow/panels/AutomationPanel.vue': 10,
    'src/views/ai/BotCenterView.vue': 6,
    'src/components/NotificationCenter.vue': 8,
    'src/views/workflow/BrowserView.vue': 8,
    'src/views/workflow/panels/McpPanel.vue': 7,
    'src/views/workspace/panels/GraphCanvasPanel.vue': 8,
    'src/views/workspace/panels/WhiteboardPanel.vue': 8,
    'src/composables/useKnowledgeBase.js': 8,
    'src/composables/workspace/useAlliance.js': 8,
    'src/stores/app.store.js': 8,
    'src/views/market/MarketDetailView.vue': 7,
    'src/views/admin/panels/AdminRole.vue': 6,
    'src/views/admin/panels/AdminTenant.vue': 6,
    'src/views/auth/Register.vue': 6,
    'src/views/workflow/panels/PluginsPanel.vue': 6,
    'src/components/common/ConfirmDialog.vue': 5,
    'src/views/admin/panels/AdminLogs.vue': 5,
    'src/views/workspace/panels/CollaborationPanel.vue': 5,
    'src/api/system.api.js': 5,
    'src/views/admin/panels/AdminAudit.vue': 4,
    'src/views/admin/panels/AdminDocs.vue': 4,
    'src/composables/workspace/useTaskOrchestration.js': 4,
    'src/components/common/LoadingState.vue': 3,
    'src/views/workflow/WorkflowView.vue': 1,
    'src/components/ThemeSwitcher.vue': 2,
    'src/components/common/EmptyState.vue': 2,
    'src/views/admin/panels/AdminHitl.vue': 2,
    'src/views/ai/ChatView.vue': 2,
    'src/views/expert/AllianceTaskView.vue': 2,
    'src/views/graph/MoxFusionView.vue': 2,
    'src/composables/workspace/useGraphCanvas.js': 2,
    'src/composables/workspace/useWorkspaceData.js': 2,
    'src/utils/projectMember.utils.js': 2,
    'src/views/admin/AdminView.vue': 1,
    'src/views/project/TaskView.vue': 1,
    'src/main.js': 1,
    'src/composables/workspace/useWhiteboard.js': 1,
}

# 「拿文字档当色块底」的存量（判据 5 的 text-as-fill，棘轮收口）。
# 这一批多是圆点/进度条/滚动条一类装饰性色块：换成 --x-fill 会让它们在四皮下同时变深，
# 属于设计取舍而不是机械修，所以先钉住不许增长，等用户点头再逐处收。
# 回填：python scripts/gate/check-view-hex.py --role-baseline
ROLE_TIER_BASELINE = {
    'src/views/project/ProjectsView.vue': 6,
    'src/components/layout/IconSidebar.vue': 4,
    'src/views/admin/panels/AdminMonitor.vue': 4,
    'src/views/expert/AllianceTaskView.vue': 4,
    'src/views/project/panels/KnowledgeBasePanel.vue': 3,
    'src/modules/expert-alliance/components/ExpertCard.vue': 2,
    'src/views/expert/ExpertPlazaView.vue': 2,
    'src/components/AgentFlowPanel.vue': 1,
    'src/components/AgentTaskRunner.vue': 1,
    'src/components/SessionSidebar.vue': 1,
    'src/modules/expert-alliance/components/GraphCanvas.vue': 1,
    'src/styles/themes/index.css': 1,
    'src/views/admin/panels/AdminLogs.vue': 1,
    'src/views/graph/GraphView.vue': 1,
    'src/views/project/TaskView.vue': 1,
}


def strip_comments(text):
    """用等量换行替换注释：行号必须仍指向原文件，否则报告里的 file:line 是错的。"""
    return COMMENT_RE.sub(lambda m: '\n' * m.group(0).count('\n'), text)


def read(path):
    with io.open(path, encoding='utf-8', errors='replace') as fh:
        return fh.read()


def hex_sites(text):
    """(行号, 字面量) 序列，注释已剥掉。"""
    body = strip_comments(text)
    return [(body[:m.start()].count('\n') + 1, m.group(0)) for m in HEX_RE.finditer(body)]


def scan_ok(rel):
    """rel（相对 UI、`/` 分隔）是否进判据 1 的账。纯函数 ⇒ 覆盖面能拿合成路径证伪，
    不必依赖真语料此刻长什么样（真语料只有 0 处时，"排除生效"与"排除是装饰"不可区分）。"""
    if rel.startswith(SCAN_EXCLUDE):
        return False
    if rel.endswith('.vue'):
        return True
    # .test.js/.spec.js 不参与：见开头说明
    return rel.endswith('.js') and not rel.endswith(('.test.js', '.spec.js'))


def scan_sources(root=SRC):
    """rel-path → [(行号, 字面量)]。扫 `src/**` 的 .vue/.js 全集，只减去 SCAN_EXCLUDE。"""
    out = {}
    for ext in ('vue', 'js'):
        for path in glob.glob(os.path.join(root, '**', '*.' + ext), recursive=True):
            rel = os.path.relpath(path, UI).replace(os.sep, '/')
            if not scan_ok(rel):
                continue
            hits = hex_sites(read(path))
            if hits:
                out[rel] = hits
    return out


def ratchet(hits, baseline):
    """(grown, brand_new, gone) —— 棘轮判定的纯函数，便于自证伪。

    grown: 这个文件比基线多了裸 hex；
    brand_new: 基线里没见过这个文件（整份都是新增）—— 基线为空时不判，否则首建即全红；
    gone: 比基线少，只提示回填，不 FAIL。
    """
    grown = sorted([(r, len(hits[r]), baseline[r]) for r in hits
                    if r in baseline and len(hits[r]) > baseline[r]])
    brand_new = sorted([(r, len(hits[r])) for r in hits if r not in baseline and baseline])
    gone = sorted([(r, baseline[r], len(hits.get(r, []))) for r in baseline
                   if len(hits.get(r, [])) < baseline[r]])
    return grown, brand_new, gone


def palette():
    """皮肤键 → {令牌名: 值}。global 用 'default' 键。"""
    files = {'default': os.path.join(STYLES, 'global.css')}
    for k in SKINS:
        files[k] = os.path.join(STYLES, 'themes', 'theme-%s.css' % k)
    out = {}
    for k, path in files.items():
        got = {}
        if os.path.exists(path):
            for name, val in CAT_DEF_RE.findall(strip_comments(read(path))):
                got.setdefault(name, val.strip())
        out[k] = got
    return out


def palette_problems(pal):
    """[(kind, skin, detail)]，kind ∈ missing / notcolor / dup。"""
    problems = []
    for skin, defs in sorted(pal.items()):
        missing = [n for n in CAT_NAMES if n not in defs]
        for n in missing:
            problems.append(('missing', skin, n))
        colors = {}
        for n in CAT_NAMES:
            v = defs.get(n)
            if v is None:
                continue
            if not HEX_RE.fullmatch(v):
                problems.append(('notcolor', skin, '%s=%s' % (n, v)))
                continue
            colors.setdefault(v.lower(), []).append(n)
        for val, names in sorted(colors.items()):
            if len(names) > 1:
                problems.append(('dup', skin, '%s ← %s' % (val, '+'.join(names))))
    return problems


TIER_DEF_RE = re.compile(r'(--text-(?:primary|secondary|tertiary|quaternary))\s*:\s*([^;]+);')


def tier_table():
    """皮肤键 → {正文灰度档名: 值}。与 palette() 读同一批文件。"""
    files = {'default': os.path.join(STYLES, 'global.css')}
    for k in SKINS:
        files[k] = os.path.join(STYLES, 'themes', 'theme-%s.css' % k)
    out = {}
    for k, path in files.items():
        got = {}
        if os.path.exists(path):
            for name, val in TIER_DEF_RE.findall(strip_comments(read(path))):
                got.setdefault(name, val.strip())
        out[k] = got
    return out


def tier_problems(tiers):
    """[(kind, skin, detail)]，kind ∈ tier-missing / tier-notcolor / tier-dup。
    比值不在这里判（style.test.js 的灰阶用例已经管着 AA），这里只判「四级存不存在、各不相同」。"""
    problems = []
    for skin, defs in sorted(tiers.items()):
        for n in TEXT_TIERS:
            if n not in defs:
                problems.append(('tier-missing', skin, n))
        vals = {}
        for n in TEXT_TIERS:
            v = defs.get(n)
            if v is None:
                continue
            if not HEX_RE.fullmatch(v):
                problems.append(('tier-notcolor', skin, '%s=%s' % (n, v)))
                continue
            vals.setdefault(v.lower(), []).append(n)
        for val, names in sorted(vals.items()):
            if len(names) > 1:
                problems.append(('tier-dup', skin, '%s ← %s' % (val, '+'.join(names))))
    return problems


VAR_ALIAS_RE = re.compile(r'^var\(\s*(--[\w-]+)\b([^)]*)\)')


def _norm_color(raw):
    return raw.strip().lower().replace(' ', '')


def resolve_token(name, table):
    """跟 var() 链走到字面值。别名层（判据 B 那一批）就是靠链写的，不跟链会读到 `var(...)` 本身。

    断链且没有兜底 ⇒ None；断链但有兜底 ⇒ 返回兜底值（浏览器就这么算）。成环 ⇒ '∅环'。
    """
    chain = [name]
    cur = name
    while True:
        raw = table.get(cur)
        if raw is None:
            return None
        m = VAR_ALIAS_RE.match(raw)
        if not m:
            return _norm_color(raw)
        base, rest = m.group(1), m.group(2)
        if base not in table:
            return _norm_color(rest.split(',', 1)[1]) if ',' in rest else None
        if base in chain:
            return '∅环:' + '→'.join(chain[chain.index(base):])
        cur = base
        chain.append(cur)


def skin_token_table(skin):
    """皮肤键 → 令牌表（global 打底，皮肤文件覆写；注释已剥）。"""
    files = [os.path.join(STYLES, 'global.css')]
    if skin != 'default':
        files.append(os.path.join(STYLES, 'themes', 'theme-%s.css' % skin))
    table = {}
    for path in files:
        if not os.path.exists(path):
            continue
        for name, val in TOKEN_DEF_RE.findall(strip_comments(read(path))):
            table[name] = val
    return table


def surface_table(tables=None):
    """皮肤键 → {阶梯档名: 解析到底的字面值（None=断链）}。"""
    tables = tables or {k: skin_token_table(k) for k in ('default',) + SKINS}
    return {skin: {n: resolve_token(n, tab)
                   for names in SURFACE_GROUPS.values() for n in names}
            for skin, tab in tables.items()}


def _surface_scan(tables=None):
    """→ (缺档/环/别名三类问题, 全量塌陷档对 [(skin, (a, b), 值)])。

    塌陷先全量产出、豁免在外层筛 ⇒ "每条豁免今天是否还对应真账"能直接对表核，
    不必回头解析给人看的字符串。
    只判"层级存不存在、组内别名同不同、组与组撞不撞" —— 亮度差与 AA 不在这里算
    （判据 3 的同一条理由：算了就是同题双源）。
    """
    problems, collapses = [], []
    for skin, resolved in sorted(surface_table(tables).items()):
        by_tier = {}
        for tier, names in sorted(SURFACE_GROUPS.items()):
            for n in names:
                if resolved[n] is None:
                    problems.append(('surface-missing', skin, n))
                elif resolved[n].startswith('∅'):
                    problems.append(('surface-cycle', skin, '%s=%s' % (n, resolved[n])))
            vals = {resolved[n] for n in names
                    if resolved[n] and not resolved[n].startswith('∅')}
            if len(vals) > 1:
                problems.append(('surface-alias', skin,
                                 '%s ← %s' % (tier, ' | '.join('%s=%s' % (n, resolved[n])
                                                               for n in names))))
            elif len(vals) == 1:
                by_tier[tier] = vals.pop()
        tiers = sorted(by_tier.items())
        for i in range(len(tiers)):
            for j in range(i + 1, len(tiers)):
                (a, va), (b, vb) = tiers[i], tiers[j]
                if va == vb:
                    collapses.append((skin, (a, b), va))
    return problems, collapses


def _collapse_ok():
    """豁免集合归一：常量按"层级顺序"记（panel 在 card 之上），比较按档名集合，
    免得 card < panel 的字典序把它翻成另一种键 ⇒ 假红。"""
    return {(s, frozenset(p)) for s, *p in SURFACE_COLLAPSE_OK}


def surface_problems(tables=None):
    problems, collapses = _surface_scan(tables)
    ok = _collapse_ok()
    for skin, pair, val in collapses:
        if (skin, frozenset(pair)) not in ok:
            problems.append(('surface-collapse', skin, '%s ← %s' % ('+'.join(pair), val)))
    return problems


def surface_exempted_collapses(tables=None):
    """今天真的塌着、且已被豁免的档对 ⇒ 每条 SURFACE_COLLAPSE_OK 都必须出现在这里。
    还了债却留着豁免，它就替将来的复发挡刀（与 --role-baseline 的死条目同一种病）。

    收 tables 参数是为了让这条对表能被合成表证伪：只吃真语料的话，"返回常量"这种
    假绿变异体永远撞不上它。
    """
    _, collapses = _surface_scan(tables)
    ok = _collapse_ok()
    return sorted({(s, tuple(sorted(p))) for s, p, _ in collapses if (s, frozenset(p)) in ok})


def token_defs():
    """外壳令牌表（global.css + themes/*.css）里的自定义属性定义，注释已剥掉。"""
    for path in [os.path.join(STYLES, 'global.css')] + \
            sorted(glob.glob(os.path.join(STYLES, 'themes', '*.css'))):
        for name, val in TOKEN_DEF_RE.findall(strip_comments(read(path))):
            yield name, val.strip()


def families():
    """家族 = --x-fill 与 --on-x 同时存在的那个 x，从令牌表推导而不钉死名字。

    取两个后缀的交集即可自然挡掉 EP 自带的 --el-fill-color-*（没有配对的 --on-el-*），
    不需要维护黑名单。新增一个状态色若忘了配 --on-x，它就不成家族、判据跟着漏 ——
    那是 --x-fill 缺配套，由 theme-tokens 那道闸门的断链检查兜住。
    """
    fills, ons = set(), set()
    for name, _v in token_defs():
        if name.endswith('-fill'):
            fills.add(name[2:-len('-fill')])
        elif name.startswith('--on-'):
            ons.add(name[len('--on-'):])
    return {f for f in fills if f in ons}


def css_spans(text, whole_file=False):
    """算 CSS 的片段 [(起始偏移, 结束偏移)]：.vue 只看 <style> 段，纯 .css 看整份。

    <script> 里的字符串长得像 CSS，把它当 CSS 判会假红，所以 .vue 没有 <style> 就返回空，
    绝不退化成「扫整份文件」。
    """
    if whole_file:
        return [(0, len(text))]
    return [(m.start(1), m.end(1)) for m in STYLE_BODY_RE.finditer(text)]


def iter_rules(text, start, end):
    """产出 (选择器, 声明体, 声明体所在行号)，只给最内层规则；偏移按整份文件算。

    `{` 入栈、`}` 出栈，所以 @media 里的每条子规则各自成一条。这是配对判据的地基：
    把整个 @media 块当一条规则，就会拿「A 子规则的底」去配「B 子规则的字」，
    既可能假绿（真串门被别人的配套遮掉）也可能假红（本规则没写字色却替它找）。
    行号必须在整份文件上数换行：在切片上数会连 `<style>` 前面那几行一起漏掉，
    报告里的 file:line 就指向别的行。
    """
    out = []
    stack, seg = [], start
    for i in range(start, end):
        ch = text[i]
        if ch == '{':
            stack.append((text[seg:i].strip().replace('\n', ' '), i))
            seg = i + 1
        elif ch == '}':
            if stack:
                sel, at = stack.pop()
                decls = text[seg:i]
                if '{' not in decls and decls.strip():
                    out.append((sel[:70] or '（无名规则）', decls, text[:at].count('\n') + 1))
            seg = i + 1
    return out


def role_hits(text, fams, whole_file=False):
    """给定源文本，返回 (硬串门, 拿文字档做底) 两组 [(kind, 行号, 细节)]。

    行号在**剥掉注释**的文本上算，但 strip_comments 用等长空白替换，所以行号仍指向原文件。
    模板里的 `style="color: var(--x-fill)"` 内联样式不在扫描范围（SFC 内联样式极少，
    且这里漏掉的是少数；宁可少判也不要把 <script> 里的字符串当 CSS）。
    """
    body = strip_comments(text)
    hard, tier = [], []
    for start, end in css_spans(body, whole_file):
        blk = body[start:end]
        for rx, kind in ((FILL_AS_TEXT_RE, 'fill-as-text'), (ON_AS_FILL_RE, 'on-as-fill')):
            for m in rx.finditer(blk):
                hard.append((kind, body[:start + m.start()].count('\n') + 1, m.group(1)))
        for sel, decls, rline in iter_rules(body, start, end):
            colors = [c.group(1).strip() for c in COLOR_DECL_RE.finditer(decls)]
            on_fams = set()
            for c in colors:
                on_fams |= set(ON_IN_VALUE_RE.findall(c))
            for m in BG_DECL_RE.finditer(decls):
                val = m.group(2).strip()
                line = rline + decls[:m.start()].count('\n')
                want = {f for f in FILL_IN_VALUE_RE.findall(val) if f in fams}
                if want:
                    # 渐变底里混了几个家族的色块，字色取其中任一 --on-x 即算配套
                    if colors and not (on_fams & want):
                        hard.append(('fill-unpaired', line,
                                     '底=--%s 字=%s 规则=%s' % ('/'.join(sorted(want)),
                                                                ' | '.join(colors)[:44], sel)))
                elif not val.startswith(('linear-gradient', 'radial', 'repeating')):
                    t = SINGLE_VAR_RE.match(val)
                    if t and t.group(1)[2:] in fams and val.count('var(') == 1:
                        tier.append(('text-as-fill', line, '%s 规则=%s' % (val, sel)))
    return sorted(hard, key=lambda h: h[1]), sorted(tier, key=lambda h: h[1])


def role_scan(fams):
    """一次遍历出两组账：硬串门（零容忍）与「文字档做底」（存量走棘轮）。

    覆盖 src 下全部 .vue + .css，含 modules 那一层（它原先只被裸 hex 与 AA 管着）。
    """
    hard, tier = [], {}
    paths = sorted(glob.glob(os.path.join(SRC, '**', '*.vue'), recursive=True) +
                   glob.glob(os.path.join(SRC, '**', '*.css'), recursive=True))
    for path in paths:
        rel = os.path.relpath(path, UI).replace(os.sep, '/')
        h, t = role_hits(read(path), fams, rel.endswith('.css'))
        for kind, line, detail in h:
            hard.append((kind, '%s:%d' % (rel, line), detail))
        if t:
            tier[rel] = t
    return hard, tier


def role_problems(fams=None):
    """硬串门清单：--x 管文字、--x-fill 管色块底、--on-x 管底上的字，串门会跌破 AA。"""
    return role_scan(fams or families())[0]


def selftest():
    """正例必须被抓到，反例必须不被抓到——只有正例的自检等于空转。"""
    cases = []

    def check(label, got, want):
        cases.append((label, got, want))

    css_like = '''
    <style>
    /* 注释里的 #deadbeef 不算 */
    .a { color: #fff000; }
    .b { background: var(--cat-1); }
    #app { display: block; }
    </style>
    <script>const c = ['#6366f1', 'var(--cat-2)']</script>
    '''
    sites = hex_sites(css_like)
    lits = [h for _, h in sites]
    check('注释里的 hex 不计', 'deadbeef' in ''.join(lits), False)
    check('CSS 里的 hex 计入', '#fff000' in lits, True)
    check('var(--cat-1) 不计入', 'cat' in ''.join(lits), False)
    check('JS 数据里的 hex 计入', '#6366f1' in lits, True)
    check('行号指向原文件而非剥注释后', [n for n, _ in sites if _ == '#fff000'][0], 4)

    frag = '<template><a href="#cafe">锚点</a><a href="#sec-1">锚点</a></template>'
    check('锚点 #cafe 会被算进裸 hex（已知过计：宁过计不漏计，#sec-1 因含 s 不匹配）',
          [h for _, h in hex_sites(frag)], ['#cafe'])

    # 覆盖面本身是判据 1 的账；它错了，下面的判定都建立在缺账的语料上。
    # 这里只喂**合成路径**：真语料在 modules 那层此刻 0 处裸 hex，"排除生效"和"排除是装饰"
    # 在真语料上不可区分 ⇒ 必须让纯函数自己交代（见下方 scan_sources 的账目核对）。
    path_cases = [('src/views/a.vue', True),
                  ('src/stores/a.store.js', True),        # 旧三根白名单会漏 ⇒ 打"白名单复辟"
                  ('src/new-deck/chart.js', True),        # 新目录默认入账，不用改代码
                  ('src/modules/expert-alliance/views/G.vue', False),
                  ('src/modules/index.js', False),
                  ('src/modules_x/keepme.vue', True),     # 前缀相近的兄弟目录不许被误伤
                  ('src/views/a.test.js', False),
                  ('src/views/a.spec.js', False),
                  ('src/views/a.md', False)]
    check('扫描集判定 scan_ok 是纯函数且逐条如预期（新目录入账 / test-spec 不入 / modules 排除 / '
          'modules_x 不误伤）',
          [(r, g) for r, w in path_cases if (g := scan_ok(r)) != w], [])

    pal = {
        'default': {n: '#%06x' % (i * 7) for i, n in enumerate(CAT_NAMES)},
        'dark': {n: '#000000' for n in CAT_NAMES[:-1]},
        'sky': dict((n, '#%06x' % (i * 11)) for i, n in enumerate(CAT_NAMES)),
        'cyberpunk': dict((n, '#%06x' % (i * 13)) for i, n in enumerate(CAT_NAMES)),
    }
    pal['sky'][CAT_NAMES[2]] = pal['sky'][CAT_NAMES[0]]
    probs = palette_problems(pal)
    check('缺档被抓到（dark 少最后一档）', ('missing', 'dark', CAT_NAMES[-1]) in probs, True)
    check('同皮两档撞色被抓到', any(p[0] == 'dup' and p[1] == 'sky' for p in probs), True)
    check('合法皮肤不误报', any(p[1] == 'cyberpunk' for p in probs), False)
    pal_ok = {k: (v if k != 'dark' else dict((n, '#%06x' % (i * 3)) for i, n in enumerate(CAT_NAMES)))
              for k, v in pal.items()}
    pal_ok['sky'][CAT_NAMES[2]] = '#%06x' % (99 * 11)
    check('补齐后必须全绿（否则上面两条是常量自比）', palette_problems(pal_ok), [])
    pal_bad = dict(pal_ok)
    pal_bad['default'] = dict(pal_ok['default'])
    pal_bad['default'][CAT_NAMES[0]] = 'transparent'
    check('值不是颜色字面量也要被抓到',
          any(p[0] == 'notcolor' for p in palette_problems(pal_bad)), True)

    # 棘轮本体：判据得能被"这里少 4 处、那里多 4 处"骗不过去
    at_base = {'src/views/a.vue': [(1, '#fff000')] * 3, 'src/constants/b.js': [(1, '#000000')]}
    check('与基线持平 ⇒ 三项皆空',
          ratchet(at_base, {'src/views/a.vue': 3, 'src/constants/b.js': 1}), ([], [], []))
    check('低于基线只记 SHRANK（提示回填）',
          ratchet({'src/views/a.vue': [(1, '#fff000')]}, {'src/views/a.vue': 3})[2],
          [('src/views/a.vue', 3, 1)])
    check('多一处就 FAIL，且按文件记（不许被别处的减少抵掉）',
          ratchet({'src/views/a.vue': [(1, '#fff000')] * 4, 'src/constants/b.js': []},
                  {'src/views/a.vue': 3, 'src/constants/b.js': 5})[0],
          [('src/views/a.vue', 4, 3)])
    check('基线没见过的文件算 NEW',
          ratchet({'src/constants/new.js': [(1, '#111111')]}, {'src/views/a.vue': 1})[1],
          [('src/constants/new.js', 1)])
    check('基线为空 ⇒ 不判 NEW（首建基线那次不该全红）',
          ratchet(at_base, {})[1], [])

    scanned = scan_sources()
    check('constants 目录确实在扫描范围内（去重把字面量搬进单源，不数就是洗账）',
          any(r.startswith('src/constants/') for r in scanned), True)
    # components 用"文件数 + 处数"双下限，而不是只看有没有一条命中：
    # 只看命中的话，glob 静默退化成一两个文件也能过（这是实测过的坑，见开头说明）。
    comp_files = [r for r in scanned if r.startswith('src/components/')]
    comp_sites = sum(len(scanned[r]) for r in comp_files)
    check('components 在账上且不是零头（实测 25 个文件 696 处，退化成扫不到就是分叉无处拦）',
          (len(comp_files) >= 20, comp_sites >= 500), (True, True))
    check('测试文件不进账（它们断言字面量，不渲染字面量）',
          [r for r in scanned if r.endswith('.test.js') or r.endswith('.spec.js')], [])
    # 旧三根之外的覆盖面：白名单复辟（把 scan_sources 改回三根遍历）会让这一条红。
    outside = {r: len(scanned[r]) for r in scanned
               if not r.startswith(('src/views/', 'src/constants/', 'src/components/'))}
    check('旧三根之外的文件在账上（白名单时代实测欠 81 处/13 文件，含 App.vue 与 stores/*.store.js）',
          (len(outside) >= 13, sum(outside.values()) >= 81), (True, True))
    check('src/modules 一个都不进扫描集（排除由 scan_ok 真做到，不靠"基线里没有这条"蒙混）',
          [r for r in scanned if r.startswith('src/modules/')], [])
    # 排除项的代价：绕过 scan_ok 直扫 modules。当前为 0 ⇒ 这一层确实没账可藏（它由 style.test.js
    # 看着裸 hex）；哪天有人往 modules 写字面量并指望它不被数，这条就红。
    mod = sum(len(hex_sites(read(p)))
              for ext in ('vue', 'js')
              for p in glob.glob(os.path.join(SRC, 'modules', '**', '*.' + ext), recursive=True)
              if not p.endswith(('.test.js', '.spec.js')))
    check('被排除那一层此刻 0 处裸 hex ⇒ 排除不藏账（红了＝字面量搬进了看不见的目录）', mod, 0)
    check('台账与扫描集同集合：账上没有已消失的文件，也没有没记上的活文件',
          (sorted(set(scanned) - set(BASELINE)), sorted(set(BASELINE) - set(scanned))),
          ([], []))

    # ---- 判据 6：表面阶梯（page/panel/mid/card）----
    # 全部拿合成表打样：真语料此刻要么正好干净（判据是否生效看不出来），要么正好有那
    # 一笔 sky 塌陷（只证得了那一种形态）。合成表能让四类问题各自独立地红一次。
    def ladder(skin='x', **over):
        t = {'--bg-primary': '#111111', '--bg-page': '#111111',
             '--bg-secondary': '#222222', '--bg-surface': '#222222', '--bg-panel': '#222222',
             '--bg-tertiary': '#333333', '--bg-surface-2': '#333333',
             '--bg-card': '#444444', '--bg-raised': '#444444'}
        for k, v in over.items():
            t.pop(k) if v is None else t.__setitem__(k, v)
        return {skin: t}

    def surf(tabs, kind):
        return [p[2] for p in surface_problems(tabs) if p[0] == kind]

    check('四档齐备、组内同值、组间不同 ⇒ 无账（判据不假红）', surface_problems(ladder()), [])
    check('少一个别名 ⇒ surface-missing', surf(ladder(**{'--bg-page': None}), 'surface-missing'),
          ['--bg-page'])
    check('组内别名不同值 ⇒ surface-alias，且该档退出撞色比较（值无定义 ⇒ 不许猜）',
          surface_problems(ladder(**{'--bg-page': '#999999'})),
          [('surface-alias', 'x', 'page ← --bg-primary=#111111 | --bg-page=#999999')])
    check('两档塌成同一个值 ⇒ surface-collapse（抬起的卡片与面板无缝 ⇒ 层级消失）',
          surf(ladder(**{'--bg-tertiary': '#222222', '--bg-surface-2': '#222222'}),
               'surface-collapse'), ['mid+panel ← #222222'])
    # 同一张塌陷表，换皮名跑两遍：'x' 没登记 ⇒ 必须红；'sky' 登记过（写成 panel,card 的
    # 层级顺序，而 card < panel 的字典序会把档对翻过来）⇒ 必须绿。只按写入顺序配键的
    # 实现会在 sky 那行红，这条用例就是它的反例。
    collapse_card_panel = {'--bg-card': '#222222', '--bg-raised': '#222222',
                           '--bg-secondary': '#222222', '--bg-surface': '#222222',
                           '--bg-panel': '#222222'}
    check('未登记的塌陷照红，且档对按字典序记名',
          surf(ladder(**collapse_card_panel), 'surface-collapse'), ['card+panel ← #222222'])
    check('已登记的塌陷不再报（豁免键与档位书写顺序无关）',
          surface_problems(ladder('sky', **collapse_card_panel)), [])
    check('豁免只放行登记的那一档对：同皮其它档塌陷照红（按皮名整片放行＝换个地方塌就行）',
          surface_problems(ladder('sky', **{'--bg-page': '#333333', '--bg-primary': '#333333'})),
          [('surface-collapse', 'sky', 'mid+page ← #333333')])
    check('var 链跟到底：别名写成 var( --bg-primary ) 也算同值（空格不算串门）',
          surface_problems(ladder(**{'--bg-page': 'var( --bg-primary )'})), [])
    check('断链且无兜底 ⇒ surface-missing（断链不许被读成"值不同"）',
          surf(ladder(**{'--bg-page': 'var(--bg-nope)'}), 'surface-missing'), ['--bg-page'])
    check('断链有兜底 ⇒ 按兜底值判（浏览器同款回退语义）',
          surface_problems(ladder(**{'--bg-page': 'var(--bg-nope, #111111)'})), [])
    cyc = {'--bg-page': 'var(--bg-primary)', '--bg-primary': 'var(--bg-page)'}
    check('成环 ⇒ surface-cycle 两个成员各报一次，且不冒充缺档（红字要指对病灶）',
          (sorted(d.split('=')[0] for d in surf(ladder(**cyc), 'surface-cycle')),
           surf(ladder(**cyc), 'surface-missing')),
          (['--bg-page', '--bg-primary'], []))
    # 豁免只能是"已登记的真账"：既不许凭空豁免（左红），也不许还了债留着（右红 ⇒ 替复发挡刀）
    check('豁免对表能被合成表证伪：登记的那档不塌 ⇒ 空（返回常量的实现会在这里红）',
          surface_exempted_collapses(ladder('sky', **{'--bg-card': '#555555',
                                                      '--bg-raised': '#555555'})), [])
    check('豁免对表正例：登记的那档真塌着 ⇒ 记名（档对按档名归一）',
          surface_exempted_collapses(ladder('sky', **collapse_card_panel)),
          [('sky', ('card', 'panel'))])
    check('每条塌陷豁免今天都还塌着（SURFACE_COLLAPSE_OK 与真账一一对应）',
          surface_exempted_collapses(),
          sorted({(s, tuple(sorted(p))) for s, *p in SURFACE_COLLAPSE_OK}))
    check('真语料此刻除已登记豁免外阶梯无账（surface=0 才是 PASS 的根据）',
          surface_problems(), [])

    # 正文灰度四级阶梯：撞档与缺档都要被抓到，补齐后必须全绿（否则上面两条是常量自比）
    tiers = {skin: dict((n, '#%06x' % (i * step + seed)) for i, n in enumerate(TEXT_TIERS))
             for skin, (step, seed) in [('default', (7, 1)), ('dark', (9, 2)),
                                         ('sky', (11, 3)), ('cyberpunk', (13, 4))]}
    tiers['cyberpunk'][TEXT_TIERS[3]] = tiers['cyberpunk'][TEXT_TIERS[2]]
    del tiers['sky'][TEXT_TIERS[1]]
    tprobs = tier_problems(tiers)
    check('两档撞色被抓到（cyberpunk 的 quaternary 抄了 tertiary）',
          any(p[0] == 'tier-dup' and p[1] == 'cyberpunk' for p in tprobs), True)
    check('缺一档被抓到（sky 少了 secondary）',
          ('tier-missing', 'sky', TEXT_TIERS[1]) in tprobs, True)
    tiers['sky'][TEXT_TIERS[1]] = '#0aa0b4'
    tiers['cyberpunk'][TEXT_TIERS[3]] = '#0b0c0d'
    check('补齐后必须全绿', tier_problems(tiers), [])
    tiers['default'][TEXT_TIERS[0]] = 'inherit'
    check('档位值不是颜色字面量也要被抓到',
          any(p[0] == 'tier-notcolor' for p in tier_problems(tiers)), True)
    check('真语料现在必须全绿（default 皮曾把 tertiary/quaternary 共用一个值 ⇒ 四级只剩三级）',
          tier_problems(tier_table()), [])

    # 命名空间串门：正例（该红的红）与反例（不该红的别红）都要有，缺一反例就是空转
    fams = {'success', 'danger'}   # 家族真由令牌表推导，自检里手工给两个，不依赖真表内容
    sfc = '''<template><div style="color: var(--success-fill)">内联不算</div></template>
<script>const css = 'color: var(--danger-fill)'; /* 字符串不是 CSS */</script>
<style>
/* 注释里的 color: var(--warning-fill) 不算 */
.a { color: var(--success-fill); }
.b { color: var(--success); background: var(--success-fill); }
.c { color: var(--on-success); background: var(--success-fill); }
.d { background: var(--on-danger); }
.e { color: #047857; }
.f { background: var(--success); }
.g { background: var(--surface); color: var(--text-primary); }
.h { background: linear-gradient(135deg, var(--success-fill), var(--danger-fill)); color: var(--on-danger); }
.i { background: linear-gradient(135deg, var(--success-fill), var(--danger-fill)); color: #ffffff; }
.j { background: var(--success-fill); }
.m { background: var(--danger-fill); color: var(--on-success); }
.n { background: var(--nomesh-fill); color: #ffffff; }
@media (max-width: 900px) {
  .k { background: var(--danger-fill); color: #ffffff; }
  .l { color: var(--on-danger); }
}
</style>
'''
    hard, tier_hits = role_hits(sfc, fams)
    both = hard + tier_hits
    check('color: 读 --x-fill 被抓到，且行号指向真行（第 5 行）',
          [h for h in hard if h[0] == 'fill-as-text'], [('fill-as-text', 5, '--success-fill')])
    check('background: 读 --on-x 被抓到',
          [h for h in hard if h[0] == 'on-as-fill'], [('on-as-fill', 8, '--on-danger')])
    check('填充档上的字取文字档/裸白/别家的 --on-x 都算串门（角色契约：底上的字只走自家的 --on-x）',
          [h[1] for h in hard if h[0] == 'fill-unpaired'], [6, 13, 15, 18])
    check('不成家族的名字（--nomesh-fill）不参与本判据，EP 那类影子名才不会假红（第 16 行）',
          [h for h in both if h[1] == 16], [])
    check('合法配套不误报（--on-success 配 --success-fill；渐变底取任一在底家族的 --on-x）',
          [h for h in both if h[1] in (7, 12)], [])
    check('background 单值读文字档被抓为 text-as-fill（第 10 行 var(--success)）',
          [(h[0], h[1]) for h in tier_hits], [('text-as-fill', 10)])
    check('色块规则没有 color 时不判（字色是继承，交给 AA 那面管：第 14 行）',
          [h for h in both if h[1] == 14], [])
    check('非家族档位不误报（--surface / --text-primary：第 11 行）',
          [h for h in both if h[1] == 11], [])
    check('裸 hex 不参与本判据（值同角色不明，交给棘轮收口）', [h for h in both if h[1] == 9], [])
    check('注释里、<script> 字符串里、template 内联样式里都不算（只有 <style> 段判）',
          [h for h in both if h[1] in (1, 2, 4)], [])
    check('@media 里的子规则各自配对，不许拿邻规则的字色替它圆场（抓到 .k 不放过 .l）',
          [h[1] for h in hard if h[0] == 'fill-unpaired' and h[1] in (18, 19)], [18])
    css_doc = ('.x { background: var(--success); }\n'
               '.y { background: var(--danger-fill); color: var(--brand); }\n')
    check('纯 .css 没有 <style> 也照判（整份当 CSS）：文字档做底抓到第 1 行',
          [h[1] for h in role_hits(css_doc, fams, True)[1]], [1])
    check('纯 .css 的串门也抓到第 2 行',
          [h[1] for h in role_hits(css_doc, fams, True)[0]], [2])
    check('.vue 没有 <style> 段时不退化成扫整份（同一文本按 .vue 判 ⇒ 两组都空）',
          role_hits(css_doc, fams), ([], []))

    fam = families()
    defs = {n for n, _v in token_defs()}
    check('家族从令牌表推出：brand/accent 在内，且没有一个来自 EP 的 el-* 影子名',
          {'brand', 'accent'} <= fam and not any(f.startswith('el-') for f in fam), True)
    check('成家族必须三档齐备（--x / --x-fill / --on-x；缺一档判据会静默漏）',
          sorted(f for f in fam
                 if not {'--%s' % f, '--%s-fill' % f, '--on-%s' % f} <= defs), [])
    rhard, rtier = role_scan(fam)
    check('真语料此刻 0 处硬串门（零容忍 ⇒ 不需要基线豁免）', rhard, [])
    check('真语料的「文字档做底」不许越过基线（棘轮：减少只提示回填，增长即红）',
          ratchet(rtier, ROLE_TIER_BASELINE)[:2], ([], []))
    # 上面那格的 [:2] 把 ratchet 的第三条腿（gone／回填提示）切掉了，所以"清零了但账没删"
    # 这一形态在上面是静默的。这一格专门见证那条腿：台账与扫描集必须是同一个集合
    # （⇒ 把 role_hits 对 text-as-fill 打盲的变异体，在此格外还只红合成表针）。
    check('「文字档做底」台账与扫描集同集合（[:2] 切掉的 gone 腿由此格见证）',
          (sorted(set(rtier) - set(ROLE_TIER_BASELINE)),
           sorted(set(ROLE_TIER_BASELINE) - set(rtier))), ([], []))

    ok = True
    for label, got, want in cases:
        good = got == want
        ok = ok and good
        print('  %s %s（得 %r 期望 %r）' % ('ok  ' if good else 'FAIL', label, got, want))
    print('SELFTEST PASS=%d FAIL=%d' % (sum(1 for _, g, w in cases if g == w),
                                        sum(1 for _, g, w in cases if g != w)))
    return 0 if ok else 1


def main():
    if '--selftest' in sys.argv:
        return selftest()

    if '--role-baseline' in sys.argv:
        _hard, rtri = role_scan(families())
        print('ROLE_TIER_BASELINE = {')
        for rel in sorted(rtri, key=lambda r: -len(rtri[r])):
            print("    '%s': %d," % (rel, len(rtri[rel])))
        print('}')
        print('# 合计 %d 处 / %d 个文件' %
              (sum(len(v) for v in rtri.values()), len(rtri)))
        return 0

    hits = scan_sources()
    probs = palette_problems(palette())
    tprobs = tier_problems(tier_table())
    sprobs = surface_problems()
    rprobs, rtri = role_scan(families())
    rtier_total = sum(len(v) for v in rtri.values())
    rgrown, rnew, rgone = ratchet(rtri, ROLE_TIER_BASELINE)

    if '--baseline' in sys.argv:
        print('BASELINE = {')
        for rel in sorted(hits, key=lambda r: -len(hits[r])):
            print("    '%s': %d," % (rel, len(hits[rel])))
        print('}')
        print('# 合计 %d 处 / %d 个文件；色板问题 %d；灰阶问题 %d；表面阶梯问题 %d；串门 %d；文字档做底 %d' %
              (sum(len(v) for v in hits.values()), len(hits), len(probs), len(tprobs),
               len(sprobs), len(rprobs), rtier_total))
        return 0

    total = sum(len(v) for v in hits.values())
    grown, brand_new, gone = ratchet(hits, BASELINE)

    if '--check' in sys.argv:
        print('裸 hex：现存 %d 处 / %d 个文件，基线 %d 处 / %d 个文件' %
              (total, len(hits), sum(BASELINE.values()), len(BASELINE)))
        print('家族（--x/--x-fill/--on-x 齐备）：%s' % ' '.join(sorted(families())))
        print('文字档做底：现存 %d 处 / %d 个文件，基线 %d 处 / %d 个文件' %
              (rtier_total, len(rtri), sum(ROLE_TIER_BASELINE.values()), len(ROLE_TIER_BASELINE)))
        for r, now, base in grown:
            print('  GROWN  %s  %d → %d  （新写的裸 hex，换成 var(--cat-n) 或语义档位）' % (r, base, now))
        for r, now in brand_new:
            print('  NEW    %s  %d 处  （基线里没有这个文件 ⇒ 整份都是新增）' % (r, now))
        for r, base, now in gone:
            print('  SHRANK %s  %d → %d  （记得跑 --baseline 回填，否则减少量白丢）' % (r, base, now))
        for r, now, base in rgrown:
            print('  TIERGROWN %s  %d → %d  （新写的 background: var(--x)，x 有 --x-fill 就换它）' % (r, base, now))
        for r, now in rnew:
            print('  TIERNEW   %s  %d 处  （基线里没有这个文件 ⇒ 整份都是新增）' % (r, now))
        for r, base, now in rgone:
            print('  TIEREMPTY %s  %d → %d  （记得跑 --role-baseline 回填）' % (r, base, now))
        for kind, skin, detail in probs:
            print('  PALETTE %s %s %s' % (kind, skin, detail))
        for kind, skin, detail in tprobs:
            print('  TIER %s %s %s' % (kind, skin, detail))
        for kind, skin, detail in sprobs:
            print('  SURFACE %s %s %s  （四档表面层级：page/panel/mid/card，组内别名须同值、组间须不同值）'
                  % (kind, skin, detail))
        for kind, where, tok in rprobs:
            print('  ROLE %s %s %s  （--x 管文字、--x-fill 管色块底、--on-x 管底上的字，串门会跌破 AA）'
                  % (kind, where, tok))
        verdict = 1 if (grown or brand_new or probs or tprobs or sprobs or rprobs
                        or rgrown or rnew) else 0
        print('HEXCHECK grown=%d new=%d palette=%d tier=%d surface=%d role=%d textasfill=%d/%d grown=%d new=%d verdict=%s' %
              (len(grown), len(brand_new), len(probs), len(tprobs), len(sprobs), len(rprobs),
               rtier_total, len(rgrown) + len(rnew), len(rgrown), len(rnew),
               'PASS' if not verdict else 'FAIL'))
        return verdict

    print('裸 hex 存量 %d 处，分布在 %d 个文件（前 12）：' % (total, len(hits)))
    for rel in sorted(hits, key=lambda r: -len(hits[r]))[:12]:
        sample = hits[rel][0]
        print('  %4d  %-56s 首处 :%d %s' % (len(hits[rel]), rel, sample[0], sample[1]))
    print('色板：')
    for skin, defs in sorted(palette().items()):
        print('  %-10s 定义 %d/%d 档  %s' % (skin, len(defs), CAT_COUNT,
                                             ' '.join(defs[n] for n in CAT_NAMES if n in defs)))
    print('正文灰度四级：')
    for skin, defs in sorted(tier_table().items()):
        print('  %-10s 定义 %d/%d 档  %s' % (skin, len(defs), len(TEXT_TIERS),
                                             ' '.join(defs[n] for n in TEXT_TIERS if n in defs)))
    for kind, skin, detail in probs:
        print('  !! %s %s %s' % (kind, skin, detail))
    for kind, skin, detail in tprobs:
        print('  !! %s %s %s' % (kind, skin, detail))
    print('表面阶梯（跟 var() 链解析后的最终值，括号内是组内别名各自的值）：')
    for skin, resolved in sorted(surface_table().items()):
        print('  %-10s %s' % (skin, '  '.join(
            '%s[%s]' % (t, ' | '.join('%s=%s' % (n, resolved[n]) for n in names))
            for t, names in sorted(SURFACE_GROUPS.items()))))
    for kind, skin, detail in sprobs:
        print('  !! %s %s %s' % (kind, skin, detail))
    # 豁免不等于没账：欠着的塌陷要在台账上露面，否则"待裁决"会变成"没人记得"
    ex = surface_exempted_collapses()
    if ex:
        ladder = surface_table()
        print('  已登记待裁决的塌陷豁免 %d 条：%s' % (
            len(ex), ' ; '.join('%s %s ← %s' % (s, '+'.join(p), ladder[s][SURFACE_GROUPS[p[0]][0]])
                                for s, p in ex)))
    print('状态色命名空间串门：%d 处' % len(rprobs))
    for kind, where, tok in rprobs:
        print('  !! %s %s %s' % (kind, where, tok))
    return 0


if __name__ == '__main__':
    sys.exit(main())
