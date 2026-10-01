"""按 assertions-need-mutants 的纪律跑变异电池：严格串行、逐字节还原、哈希校验。
每个变异体指名一条断言；还原失败或哈希不符即整体判为无效运行。"""
import hashlib
import os
import re
import subprocess
import sys

UI = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
COLLAB = 'src/modules/expert-alliance/contract/collab.js'
PANEL = 'src/modules/expert-alliance/components/ExpertCollabPanel.vue'
CARD = 'src/modules/expert-alliance/components/ExpertCard.vue'
DTXT = 'src/modules/expert-alliance/contract/dispatcher.js'
GTXT = 'src/modules/expert-alliance/contract/graph.js'
NRM = 'src/modules/expert-alliance/model/normalize.js'
LAY = 'src/modules/expert-alliance/model/layout.js'
GRS = 'src/modules/expert-alliance/store/alliance-graph.store.js'
APIS = 'src/modules/expert-alliance/api/alliance.api.js'
SS = 'src/modules/expert-alliance/contract/sessions.js'

CT = 'src/modules/expert-alliance/contract/contract.test.js'
PT = 'src/modules/expert-alliance/components/collab-panel.test.js'
AT = 'src/modules/expert-alliance/api/alliance.api.test.js'
ST = 'src/modules/expert-alliance/store/alliance-collab.store.test.js'
SY = 'src/modules/expert-alliance/style.test.js'
DT = 'src/modules/expert-alliance/contract/dispatcher.test.js'
GT = 'src/modules/expert-alliance/contract/graph.test.js'
GST = 'src/modules/expert-alliance/store/alliance-graph.store.test.js'
SCT = 'src/modules/expert-alliance/contract/sessions.test.js'
SST = 'src/modules/expert-alliance/store/alliance-sessions.store.test.js'
EP = 'src/modules/expert-alliance/contract/endpoints.js'
ENM = 'src/modules/expert-alliance/contract/enums.js'
EXS = 'src/modules/expert-alliance/store/alliance-experts.store.js'
MTX = 'src/modules/expert-alliance/components/ExpertCapabilityMatrix.vue'
EST = 'src/modules/expert-alliance/store/alliance-experts.store.test.js'
PLZ = 'src/modules/expert-alliance/components/plaza-components.test.js'
ACS = 'src/modules/expert-alliance/store/alliance-console.store.js'
ACV = 'src/modules/expert-alliance/views/AllianceConsoleView.vue'
DAGJ = 'src/modules/expert-alliance/model/dag.js'
DTG = 'src/modules/expert-alliance/model/dag.test.js'
RG = 'src/modules/expert-alliance/contract/registry.js'
RT = 'src/modules/expert-alliance/contract/registry.test.js'
PLV = 'src/modules/expert-alliance/views/AllianceExpertsView.vue'
OTXT = 'src/modules/expert-alliance/contract/orchestration.js'
AOV = 'src/modules/expert-alliance/views/AllianceOrchestrationView.vue'
ORCHS = 'src/modules/expert-alliance/store/alliance-orch.store.js'
OT = 'src/modules/expert-alliance/contract/orchestration.test.js'
MODJ = 'src/modules/expert-alliance/contract/mode.js'
MODT = 'src/modules/expert-alliance/contract/mode.test.js'

# (编号, 说明, 文件, 锚点正则, 替换, 命名到的断言, 要跑的测试)
MUTANTS = [
    ('M1', 'constraintFields 子键改成后端不读的名字',
     COLLAB, r"\{ key: 'min_rating',", "{ key: 'min_rating_x',",
     '每个子键都被 handler c.get() 读取', [CT, PT, AT, ST]),
    ('M2', '约束无条件发出（丢掉 neutral 判定）',
     COLLAB, r"if \(Object\.keys\(picked\)\.length\) body\[wire\] = picked",
     "body[wire] = picked",
     '等于后端默认值的子键不进请求体', [CT, AT]),
    ('M3', '去掉数值夹取',
     COLLAB, r": Math\.min\(Math\.max\(n, f\.min\), f\.max\)", ": n",
     '越界先夹到边界 99→5 / 9999→240', [CT, ST]),
    ('M4', '面板不再渲染约束行',
     PANEL, r'v-for="f in store\.constraintList"', 'v-for="f in []"',
     'route 模式渲染 3 条 constraints.* 说明', [PT]),
    ('M5', '重新引入裸 hex 颜色',
     CARD, r"var\(--accent-dim\)", "#3182f6",
     '模块内无裸 hex/rgb/hsl', [SY]),
    ('M6', '重新引用未定义的 --transition',
     PANEL, r"var\(--dur-2\)", "var(--transition)",
     '每个 var(--x) 都有定义', [SY]),
    ('M7', '跨文件抢注他人块级类名',
     CARD, r"</style>", ".acw-picker{flex:1}\n</style>",
     '块级类名跨文件不共享', [SY]),
    ('M8', '把 match_threshold 的上界放宽到后端区间外',
     DTXT, r"max: 1,", "max: 2,",
     '边界取后端 400 文案里的区间', [DT]),
    ('M9', '策略项谎称自己不被后端执法',
     DTXT, r"checked: true,", "checked: false,",
     'checked 与后端 400 分支一致', [DT]),
    ('M10', 'max_retries 上界漂移 10→11',
     DTXT, r"max: 10,", "max: 11,",
     '区间数值逐条核到后端文案', [DT]),
    ('M11', '凭记忆改掉熔断默认值',
     DTXT, r"default: 5,", "default: 6,",
     '默认值取后端 default_* 函数体', [DT]),
    ('M12', '策略清单塞进后端不认的值',
     DTXT, r"\{ value: 'weighted_random'", "{ value: 'random_weighted'",
     '策略清单与后端 valid 数组等集且同序', [DT]),
    ('M13', '丢掉合并式更新的差异判定',
     DTXT, r"if \(value === current\?\.\[f\.key\]\) continue", "continue",
     '与后端同值的键不进请求体', [DT]),

    # ── 协作图谱批次：contract/graph.js + normalize + layout + store + api ──
    ('G1', '给后端收了却从不读的 constraints 挂上界面',
     GTXT, r"\{ key: 'constraints', mounted: false,", "{ key: 'constraints', mounted: true,",
     'mounted 集合等于 handler 里 body.<key> 读取集合', [GT]),
    ('G2', '凭记忆改掉 max_members 的后端缺省',
     GTXT, r"maxMembers: 5,", "maxMembers: 6,",
     '缺省值逐字取自 handler 的 unwrap_or', [GT]),
    ('G3', 'goal 与显式需求同时发出',
     GTXT, r"if \(goal && !skills\.length && !domains\.length\) body\.goal = goal",
     "if (goal) body.goal = goal",
     'goal 与显式需求互斥：同时填时不发 goal', [GT, AT]),
    ('G4', '等于后端缺省的 min_rating 也照样发',
     GTXT, r"minRating >= 0 && minRating !== OPTIMAL_TEAM_DEFAULTS\.minRating",
     "minRating >= 0",
     '等于后端缺省的值不进请求体', [GT]),
    ('G5', '协作者 limit 缺省值漂移 10→12',
     GTXT, r"COLLABORATOR_LIMIT_DEFAULT = 10", "COLLABORATOR_LIMIT_DEFAULT = 12",
     'limit 缺省取 handler 的 unwrap_or(10)', [GT]),
    ('G6', '把图版本号归一成字符串',
     NRM, r"version: num\(p\.version\)", "version: str(p.version)",
     'version 是数值而不是字符串', [GT]),
    ('G7', '归一化读取 handler 未产出的键',
     NRM, r"totalNodes: num\(p\.total_nodes\)", "totalNodes: num(p.total_node)",
     '归一化不得读后端未返回的键', [GT]),
    ('G8', '吞掉统计面里的 density',
     NRM, r"density: num\(p\.density\),", "ts: str(p.ts),",
     '后端返回的每个键都被读到（能力不被吞掉）', [GT]),
    ('G9', '无归属专家落到中环',
     LAY, r"y: round\(cy \+ Math\.sin\(angle\) \* ringRadius\[1\]\)",
     "y: round(cy + Math.sin(angle) * ringRadius[0])",
     '孤儿节点落在外环，不与簇重叠', [GT]),
    ('G10', '节点半径去掉度数封顶',
     LAY, r"9 \+ Math\.min\(cap, \(Number\(degree\) \|\| 0\) \* 1\.6\)",
     "9 + Math.max(cap, (Number(degree) || 0) * 1.6)",
     '度数与半径单调，且半径封顶', [GT]),
    ('G11', '把没有 id 的条目也画进画布',
     LAY, r"nodes\.filter\(\(n\) => n\?\.id\)\.map", "nodes.map",
     '后端塞进没有 id 的条目时不画它', [GT]),
    ('G12', '域节点也去查协作者',
     GRS, r"nodesById\.value\[id\]\?\.nodeType === GRAPH_NODE_TYPE\.expert", "true",
     'collaborators 只对专家节点请求', [GST]),
    ('G13', '换 limit 顺带重取邻居',
     GRS, r"collaborators\.value = await api\.graphCollaborators\(selectedId\.value, limit\)",
     "neighbors.value = await api.graphNeighbors(selectedId.value); collaborators.value = await api.graphCollaborators(selectedId.value, limit)",
     '改 limit 只重取协作者', [GST]),
    ('G14', '重建失败时清空已有图',
     GRS, r"error\.rebuild = e\?\.msg", "graph.value = null; error.rebuild = e?.msg",
     '重建失败不动已有图', [GST]),
    ('G15', '协作者路径参数名写错',
     APIS, r"params: \{ id: expertId \},",
     "params: { expert_id: expertId },",
     '八个图谱方法的路径与动词精确对齐路由表', [AT]),
    ('G16', 'limit 不经契约直接塞进查询串',
     APIS, r"query: collaboratorQuery\(limit\)",
     "query: { limit }",
     'limit 只有非缺省正整数才进查询串', [AT]),
    ('G17', '组队请求体绕过契约',
     APIS, r"\{ body: optimalTeamBody\(input\) \}",
     "{ body: { ...input } }",
     '组队请求体由契约生成：camelCase 不漏进请求', [AT]),

    # ── 会话面（S 系列）：请求体逐键、归一不发明字段 ────────────────────
    ('S1', '空白过滤条件照发',
     SS, r"if \(unset\(raw\) \|\| str\(raw\)\.trim\(\) === ''\) continue",
     "if (unset(raw)) continue",
     '空串过滤条件一律不发：后端 Some("") 是真的相等比较，会把结果清光', [SCT, AT]),
    ('S2', '等于后端缺省的 session_type 也占位',
     SS, r"if \(type && type !== SESSION_TYPE_DEFAULT\) body\.session_type = type",
     "if (type) body.session_type = type",
     '等于后端缺省值的 session_type 不发；空 metadata 不占位', [SCT, AT]),
    ('S3', 'tags 无条件覆盖（存量真 bug 回归）',
     SS, r"if \(Array\.isArray\(draft\.tags\) && JSON\.stringify\(tags\) !== JSON\.stringify\(list\(current\.tags\)\)\) patch\.tags = tags",
     "if (JSON.stringify(tags) !== JSON.stringify(list(current.tags))) patch.tags = tags",
     '合并式更新：与当前同值的键不发，metadata 只能改不能删', [SCT, AT]),
    ('S4', 'metadata 里数字与文本不再等价',
     SS, r"return isPrimitive\(before\) && typeof after === 'string' && String\(before\) === after",
     "return false",
     '合并式更新：与当前同值的键不发，metadata 只能改不能删', [SCT]),
    ('S5', 'metadata 表单行被当成对象',
     SS, r"function metaAsObject\(md\) \{\s*if \(Array\.isArray\(md\)\) \{",
     "function metaAsObject(md) {\n  if (false) {",
     'metadata 行按字符串上送，不偷偷 JSON.parse', [SCT, SST]),
    ('S6', '统计之外的状态被当成可见（判定反了）',
     SS, r"if \(SESSION_STATUS_COUNTED\.includes\(v\)\) return ''",
     "if (!SESSION_STATUS_COUNTED.includes(v)) return ''",
     'status 不校验但只统计三档：写出第四个值会在统计里隐形', [SCT, SST]),
    ('S7', 'rating 只校验是不是数，丢掉 0–5 区间',
     SS, r"if \(!Number\.isFinite\(n\) \|\| n < MESSAGE_RATING\.min \|\| n > MESSAGE_RATING\.max\) \{",
     "if (!Number.isFinite(n)) {",
     'rating 是 Option<u8>：文档区间 0–5，硬边界在 255', [SCT, SST]),
    ('S8', '相似检索把缺省 top_k 当自定义值发',
     SS, r"if \(Number\.isFinite\(topK\) && topK >= 1 && Math\.trunc\(topK\) !== SIMILAR_SEARCH_DEFAULTS\.topK\) \{",
     "if (Number.isFinite(topK) && topK >= 1) {",
     '两个检索的 unwrap_or 字面量即前端缺省', [SCT, AT]),
    ('S9', '全域检索发明后端没有的 min_score',
     SS, r"if \(type\) body\.session_type = type",
     "if (type) body.session_type = type; body.min_score = 0.1",
     '两个检索 body 各自只有 3 个键，query 必填', [SCT, AT]),
    ('S10', '导出把 null 归成空串，界面就有假下载值',
     NRM, r"downloadUrl: p\.download_url === undefined \|\| p\.download_url === null \? null : str\(p\.download_url\),",
     "downloadUrl: str(p.download_url),",
     '导出不发明下载链接：能给的只有文本与文件名', [AT, SST]),
    ('S11', '详情假装有 message_count，条数归零',
     NRM, r"messageCount: s\.message_count === undefined \|\| s\.message_count === null \? messages\.length : num\(s\.message_count\),",
     "messageCount: num(s.message_count),",
     '列表项是投影后的视图：13 个键、含 message_count、不含 messages', [AT, SCT]),
    ('S12', '追加成功后不并线（后端只回那一条）',
     'src/modules/expert-alliance/store/alliance-sessions.store.js',
     r"detail\.value\.messages = \[\.\.\.\(detail\.value\.messages \|\| \[\]\), message\]",
     "detail.value.messages = [...(detail.value.messages || [])]",
     '响应只有那条消息，所以并线与 messageCount 由 store 负责', [SST]),
    ('S13', '404 判定反了：还在的会话被清掉、失踪的留着',
     'src/modules/expert-alliance/store/alliance-sessions.store.js',
     r"if \(isNotFound\(e\)\) selectedId\.value = ''",
     "if (!isNotFound(e)) selectedId.value = ''",
     '404 session not found 会清掉选中态，其他错误则保留', [SST]),
    ('S14', '无改动也发 PUT',
     'src/modules/expert-alliance/store/alliance-sessions.store.js',
     r"if \(!Object\.keys\(patch\)\.length\) \{",
     "if (false) {",
     '没有任何改动时一个请求都不发', [SST]),
    ('C1', '台账里给一条定性换成非法值（原锚点随 toggle_done/dispatch 挂载而失效）',
     EP, r"verdict: 'rejected', reason: '/fusion 与 /fusion-result",
     "verdict: 'pending', reason: '/fusion 与 /fusion-result",
     '台账只认 backlog/rejected 两种定性', [CT]),
    ('C2', '把某条理由删成两个字（台账还在，但已不可核对）',
     EP, r"reason: 'findings/SWOT[^']*'",
     "reason: '写死的'",
     '每条理由都要够长', [CT]),
    ('C3', '理由写得漂亮但不指后端位置',
     EP, r"reason: 'plans 是进程内 HashMap[^']*'",
     "reason: '计划只在当前进程内存活，重启即失，界面不能写成计划库'",
     '理由必须带 .rs: 行号，可回溯', [CT]),
    ('C4', '注册端点的方法从 POST 漂成 GET',
     EP, r"expertRegister: \{ registry: 'experts\.registry\.register', method: 'POST'",
     "expertRegister: { registry: 'experts.registry.register', method: 'GET'",
     '已挂载端点的方法与路径逐字对齐注册表行', [CT]),
    ('C5', '从台账里抹掉一条未挂载面（装作它不存在）',
     EP, r"(?m)^  \{ registry: 'alliance\.tasks\.fusion_alias'[^\n]*\n",
     "",
     '每个 ready 行要么被挂载，要么在台账里定性', [CT]),
    ('C6', '端点挂到一个注册表里没有的行 ID',
     EP, r"registry: 'experts\.session\.messages'",
     "registry: 'experts.session.message'",
     '挂载集必须落在两域注册表内', [CT]),
    ('K1', '能力目录在本地按人数重排（冒充后端的 id 次序）',
     NRM, r"items: arr\(s\.capabilities\)\.map\(\(c\) => \(\{",
     "items: arr(s.capabilities).slice().sort((a, b) => Number(b?.expert_count) - Number(a?.expert_count)).map((c) => ({",
     '乱序输入必须原样输出', [CT]),
    ('K2', '把后端写死的 40/30/30 权重搬到前端重算',
     ENM, r"\{ label: '效率分', value: percent\(d\.efficiencyScore, 1\),",
     "{ label: '效率分', value: percent(d.efficiencyScore * 0.4 + 0.3, 1),",
     '三个派生值由后端算：前端只做换算', [CT]),
    ('K3', '派生指标 404 被吞成「没有错误」',
     EXS, r"error\.metrics = e\?\.msg \|\| e\?\.message \|\| '派生指标获取失败'",
     "error.metrics = ''",
     '后端 404 不折叠成零值', [EST]),
    ('K4', '去掉派生指标的竞态守卫（旧响应可覆盖新响应）',
     EXS, r"if \(token !== metricsToken\) return null\n      expertMetrics\.value = res",
     "expertMetrics.value = res",
     '先请求的慢响应不得覆盖后请求的结果', [EST]),
    ('K5', '目录面板改口称后端支持按 capability_id 精筛',
     MTX, r"后端列表接口不接收 capability_id",
     "后端列表接口支持 capability_id 精确筛选",
     '说明文字承认后端不接收 capability_id', [PLZ]),
    ('K6', '分组把后端给的 domains 次序换成本地字母序',
     MTX, r"props\.data\.domains :",
     "props.data.domains.slice().sort((a, b) => a.localeCompare(b)) :",
     '分组次序沿用后端', [PLZ]),

    # ── 动作面批次（L 系列）：任务标记完成 + 分发实跑 ─────────────────────
    ('L1', '把任何响应都当成远程分支',
     NRM, r"const remote = !local && s\.success === true",
     "const remote = true",
     '方向只由 toggled/success 判定：缺证据就是 unknown', [CT, AT]),
    ('L2', 'toggled 的两个方向对调',
     NRM, r"s\.toggled \? 'completed' : 'reopened'",
     "s.toggled ? 'reopened' : 'completed'",
     'toggled=false 对应后端把状态改回 Running', [CT, AT]),
    ('L3', '归一化读取两条分支都不产出的键',
     NRM, r"previousStatus: str\(s\.previous_status\)",
     "previousStatus: str(s.previous_state)",
     '同一端点两条分支的 data 键集不同，前端读的恰是两者并集', [CT]),
    ('L4', '标记完成后只刷任务不刷节点',
     ACS, r"api\.getNodes\(id\)\.catch\(\(\) => null\)",
     "Promise.resolve(null)",
     '本地整批置完成会改执行表并落盘，所以 store 必须连节点一起重取', [CT, DT]),
    ('L5', '丢掉"切了任务就不写详情"的竞态守卫',
     ACS, r"if \(selectedId\.value !== id\) return res",
     "if (false) return res",
     '等待期间切了任务就不把上一个的状态写进详情', [CT, DT]),
    ('L6', '替后端臆造一个 constraints 消费方',
     DTXT, r"if \(ids\.length\) body\.expert_ids = ids",
     "if (ids.length) body.expert_ids = ids; body.constraints = form.constraints || {}",
     '实跑只发 handler 读取的键：constraints 照收不读所以不发', [CT, AT]),
    ('L7', '不再拦空需求描述',
     DTXT, r"if \(!String\(form\.input \?\? ''\)\.trim\(\)\) return",
     "if (false) return",
     '空需求描述不发：空串会让后端把领域匹配对全员判满分', [CT, DT]),
    ('L8', '智能匹配关闭的指纹漂移 0.5→0.6',
     DTXT, r"scores\.every\(\(v\) => v === 0\.5\)",
     "scores.every((v) => v === 0.6)",
     '0.5 硬编码与"空串判满分"都取自后端字面量，判据跟着数字走', [CT]),
    ('L9', '把后端回退当成"按所选策略执行"',
     DTXT, r"\} else if \(strategy\.endsWith\('\(fallback\)'\)\) \{",
     "} else if (false) {",
     'specified 分支不算策略生效，fallback 单独定性', [CT, DT]),
    ('L10', '一次失败把已有实跑结论擦成空白',
     ACS, r"error\.dispatch = e\?\.msg \|\| e\?\.message \|\| '分发实跑失败'",
     "error.dispatch = e?.msg || e?.message || '分发实跑失败'; dispatchResult.value = null",
     '无可用专家是 503 而非空结果，实跑不碰任何专家负载，失败留着上次结果', [CT, DT]),
    ('L11', '远程重开按钮不再挡（等后端 409）',
     ACV, r"current\.value\?\.status === 'completed' && store\.runtime\?\.mode === 'remote'",
     "false",
     '远程已完成任务经网关重开回 409，界面先把按钮挡住而不是等报错', [CT]),
    ('L12', '实跑请求体绕过契约',
     APIS, r"\{ body: dispatchRunBody\(form\) \}",
     "{ body: { ...form } }",
     '实跑只发 handler 读取的键：constraints 照收不读所以不发', [AT, DT]),
    ('L13', '界面改口称实跑会累加负载（后端并不改）',
     ACV, r"但不会改动任何专家的 current_load",
     "并会累加专家的 current_load",
     '无可用专家是 503 而非空结果，实跑不碰任何专家负载，失败留着上次结果', [CT]),
    ('L14', '界面改口称 task_type 有 serde 缺省可省',
     ACV, r"该键无 serde 缺省，不发会被整段 JSON 拒绝",
     "该键有 serde 缺省，可以省略不发",
     '实跑只发 handler 读取的键：constraints 照收不读所以不发', [CT]),
    ('R1', '熟练度上界放宽到 999（后端是 u8，越界整条静默丢弃）',
     RG, r"c\.proficiency > EXPERT_PROFICIENCY_MAX",
     "c.proficiency > 999",
     '熟练度必须手填且在 u8 之内', [RT, EST]),
    ('R2', 'u32 上界写成 u64 的一半（as u32 截断的警告就成了假话）',
     RG, r"export const EXPERT_U32_MAX = 0xffffffff",
     "export const EXPERT_U32_MAX = 0x7fffffffffffffff",
     '时薪超过 u32 上限被拒：as u32 会截成另一个数', [RT]),
    ('R3', 'patch 丢掉差异判定（后端整值替换数组，未改的数组被清空）',
     RG, r"if \(sameWire\(f, a\[f\.key\], b\[f\.key\]\)\) continue",
     "if (false) continue",
     '只发改动过的键，未动的数组不出现（否则会被清空）', [RT, EST]),
    ('R4', '后端没回 created/expert 也当注册成功',
     EXS, r"if \(!res\.created \|\| !res\.expert\) \{",
     "if (false) {",
     '后端没回 created + expert 就不乐观收单：行不增、列表不刷', [EST]),
    ('R5', '把「只认证不授权」改口成「仅管理员可操作」',
     RG, r"网关对专家写路径没有角色判定，任何已认证身份都能注册 / 编辑 / 停用",
     "仅管理员可操作这三条写请求",
     '只说"没有角色判定"，不替后端编造授权模型', [RT, CT]),
    ('R6', '注册弹窗不再留痕（失败只剩一条会消失的 toast）',
     PLV, r"v-if=\"store\.error\.action\" class=\"ax-modal-alert\"",
     "v-if=\"false\" class=\"ax-modal-alert\"",
     '写失败要留在发起它的那个弹窗里，不能只剩一条会自己消失的 toast', [CT]),
    ('R7', '关闭错误横幅清的是 notice（成功提示被顺手抹掉）',
     PLV, r"@close=\"store\.error\.action = ''\"",
     "@close=\"store.notice = ''\"",
     '写失败要留在发起它的那个弹窗里，不能只剩一条会自己消失的 toast', [CT]),
    ('R8', '开面清旧错的条件写反（停用弹窗留着上一次的错）',
     PLV, r"if \(reg \|\| dis\) store\.error\.action = ''",
     "if (reg && dis) store.error.action = ''",
     '写失败要留在发起它的那个弹窗里，不能只剩一条会自己消失的 toast', [CT]),

    # ── 调度状态面与负载重置（D 系列）：读数、回执语义与两道破坏性动作的闸 ──
    ('D1', '状态键集里丢掉 avg_dispatch_ms（界面少一格真实读数）',
     DTXT, r"'success_rate', 'avg_dispatch_ms', 'circuit_breakers'",
     "'success_rate', 'circuit_breakers'",
     'status 的出参键集就是后端 json! 里那十个键，且每个键都被归一化读过', [DT]),
    ('D2', '归一化开始读后端不返回的 timestamp',
     NRM, r"serverTs: str\(p\.ts\)", "serverTs: str(p.timestamp)",
     'status 的出参键集就是后端 json! 里那十个键，且每个键都被归一化读过', [DT]),
    ('D3', '平均耗时被写成常量 0（后端给的数不再进界面）',
     NRM, r"avgDispatchMs: num\(p\.avg_dispatch_ms\),", "avgDispatchMs: 0,",
     'status 的出参键集就是后端 json! 里那十个键，且每个键都被归一化读过', [DT]),
    ('D4', '全量回执的被重置名单归一成空数组',
     NRM, r"resetExpertIds: arr\(p\.reset_expert_ids\)\.map\(\(v\) => String\(v \?\? ''\)\),",
     "resetExpertIds: [],",
     '两种回执归一化后互不混形：单专家有 previousLoad，全量有 resetExpertIds', [DT]),
    ('D5', '熔断空列表不再解释，界面只剩一片安静',
     DTXT, r"if \(\(list \|\| \[\]\)\.length\) return ''", "if (true) return ''",
     '熔断计数只有读侧：空列表要说成"没有数据"，不能说成健康', [DT]),
    ('D6', '无样本时的 1.0 不再被说明（默认值读起来像 100% 成功）',
     DTXT, r"if \(!Number\(status\?\.totalDispatches\)\) return", "if (false) return",
     '无终态样本时后端给的 1.0 要说成默认值，不是 100% 成功', [DT]),
    ('D7', '重置理由不再 trim（一串空格就成了合法理由）',
     DTXT, r"const text = String\(reason \?\? ''\)\.trim\(\)", "const text = String(reason ?? '')",
     '两个 handler 的请求体要求相反，由源码签名钉住', [DT]),
    ('D8', '全量重置的确认词门槛形同虚设',
     ACV, r"const resetReady = computed\(\(\) => !resetTarget\.value\?\.all \|\| resetWord\.value === DISPATCH_RESET_ALL_CONFIRM\)",
     "const resetReady = computed(() => true)",
     '全量重置要手动输入确认词才放行：后端只认证不授权，没有第二道闸', [DT]),
    ('D9', 'engine_status 的字面量被说成探活结论',
     ACV, r"），这不是探活结果", "）",
     'engine_status 是后端字面量，界面必须自陈它不是探活', [DT]),
    ('D10', '存量面板那个不存在的 circuit_breaker.states 又回到界面',
     ACV, r"store\.dispatcherStatus\.circuitBreakers\.length",
     "store.dispatcherStatus.circuit_breaker?.states?.length",
     '界面不读后端不返回的键——存量面板那三个假字段一个都不许跟过来', [DT]),
    ('D11', '单专家重置不发体（axum 会拒成非信封错误）',
     APIS, r"body: dispatchResetBody\(reason\)", "body: undefined",
     '请求形状：单专家必发对象、全量不发体，路径逐字对齐端点表', [DT, AT]),
    ('D12', '全量重置开始塞 body（后端没有 body 提取器）',
     APIS, r"await call\(httpClient, 'dispatcherResetAll', \{\}\)",
     "await call(httpClient, 'dispatcherResetAll', { body: {} })",
     '请求形状：单专家必发对象、全量不发体，路径逐字对齐端点表', [DT, AT]),
    ('D13', '重置失败时把上一次读数抹掉（看着像后端已经改了）',
     ACS, r"error\.reset = e\?\.msg \|\| e\?\.message \|\| '负载重置失败'",
     "error.reset = e?.msg || e?.message || '负载重置失败'; dispatcherStatus.value = null",
     '重置失败：错误留在 error.reset，上一次读数与回执都不被清空', [DT]),
    ('D14', '本地替后端把负载归零，不再等重取',
     ACS, r"      await loadDispatcherStatus\(\)",
     "      dispatcherStatus.value = { ...dispatcherStatus.value, expertLoads: (dispatcherStatus.value?.expertLoads || []).map((l) => ({ ...l, currentLoad: 0 })) }",
     '重置后必须重取状态，且不在本地把负载抹成 0', [DT]),
    ('D15', '全量回执打成单专家 scope（卡片分不清是哪一次动作）',
     ACS, r"scope: 'all', \.\.\.\(await api\.resetAllDispatcherLoads\(\)\)",
     "scope: 'one', ...(await api.resetAllDispatcherLoads())",
     '全量重置同样重取，回执带 scope 供卡片分清是哪一次动作', [DT]),
    ('D16', '后果清单里抹掉源码坐标（话还在，但没法核对）',
     DTXT, r"（:141-147）", "（并发控制）",
     '二次确认清单五条齐全，每条都带源码坐标', [DT]),
    ('D17', '回执改口称它证明该专家存在',
     DTXT, r"这条回执不代表该专家存在", "这条回执说明该专家存在",
     '单专家重置取不到 id 也不 404：回执不能长成存在性断言', [DT]),
    ('D18', '被重置人数不再引后端计数',
     DTXT, r"覆盖 \$\{res\.resetCount \?\? 0\} 位专家", "覆盖全部专家",
     '全量重置覆盖整张注册表（含停用者），人数只能引后端计数', [DT]),
    ('D19', '单专家重置路径漂成 reset-all/:id',
     EP, r"path: '/api/experts/dispatcher/reset/:id'", "path: '/api/experts/dispatcher/reset-all/:id'",
     '请求形状：单专家必发对象、全量不发体，路径逐字对齐端点表', [DT, CT]),
    ('P1', '页脚退回拿后端 edges 数组当依赖数（与画出来的分层不同源）',
     ACV, r"\{\{ dagView\.drawn \}\} 条依赖参与分层",
     "{{ store.detail.dag.edges.length }} 条依赖参与分层",
     '分层与依赖计数同源：页脚不用后端 edges 数组当依赖数，算法住在 model 层', [CT]),
    ('P2', '悬空依赖不再单独记账（dangling 恒 0）',
     DAGJ, r"dangling: declared - drawn", "dangling: 0",
     '悬空依赖参与声明数但不参与分层，单独记账', [DTG]),
    ('P3', '回边不再计数（环被静默吞掉）',
     DAGJ, r"backEdges \+= 1", "backEdges += 0",
     '环不抛错、不丢节点，并计入 backEdges', [DTG, CT]),
    ('P4', '后端 edges 与依赖清单不再 1:1 时不再露差值',
     DAGJ, r"edgeDelta: \(Array\.isArray\(edges\) \? edges : \[\]\)\.length - declared",
     "edgeDelta: 0",
     '后端边数与依赖清单不再 1:1 时露出差值', [DTG, CT]),
    ('P5', '取消态并进跳过一格（后端 stats 也是这么折叠的，界面不能跟着折）',
     DAGJ, r"tally\[status\] = \(tally\[status\] \?\? 0\) \+ 1",
     "tally[status === 'cancelled' ? 'skipped' : status] = (tally[status === 'cancelled' ? 'skipped' : status] ?? 0) + 1",
     '跳过与取消分开计数，不并入一个桶', [DTG]),
    ('P6', '取消态的节点样式漂成错误拼写',
     ACV, r"\.ac-dag-node\.is-cancelled", ".ac-dag-node.is-canceled",
     'NodeExecStatus 每个变体都有 wire 出口、中文标签与 DAG 节点样式', [CT]),
    ('P7', 'normDag 开始读后端 DAG 出参没有的 stats.cancelled',
     NRM, r"skipped: num\(payload\?\.stats\?\.skipped\)",
     "cancelled: num(payload?.stats?.cancelled)",
     'DAG 出参三处键集与前端读取一一对上', [CT]),
    ('P8', '界面把折叠的 stats.skipped 当成"跳过"人数显示',
     ACV, r"跳过 \{\{ dagView\.tally\.skipped \}\}",
     "跳过 {{ store.detail.dag.stats.skipped }}",
     'node_stats 把 skipped 与 cancelled 折叠成第 6 格，界面按节点级分开计数', [CT]),

    # ── 编排面批次：contract/orchestration.js + normalize + api + store + 视图 + 台账 ──
    ('Q1', '兜底表步数凭记忆写成 5（后端 _ 分支实为 6 条）',
     OTXT, r"__fallback: 6 \}\)", "__fallback: 5 })",
     '各表的步数与契约记法一致（development 与兜底各 6 步，其余三张 5 步）', [OT]),
    ('Q2', '常量清单里的 confidence 字面量与源码脱钩',
     OTXT, r"""literal: '"confidence": 0\.85',""", """literal: '"confidence": 0.9',""",
     '字面量逐条命中所声明的行号（行号漂了说明这一面又变了）', [OT]),
    ('Q3', '执行器传空专家表的坐标漂走（话术还在，但没法核对）',
     OTXT, r"at: 'experts_orchestration\.rs:488',", "at: 'experts_orchestration.rs:489',",
     '字面量逐条命中所声明的行号（行号漂了说明这一面又变了）', [OT]),
    ('Q4', '恒 0 的计数器从契约里少报一把（界面就不再质问它为什么是 0）',
     OTXT, r"ORCH_ZERO_COUNTERS = Object\.freeze\(\['plans_ready', 'plans_failed'\]\)",
     "ORCH_ZERO_COUNTERS = Object.freeze(['plans_ready'])",
     'plan.status 只有 draft/running/completed 三处赋值，因此 ready 与 failed 两个计数没有写入路径', [OT]),
    ('Q5', '历史 page_size 上限从网关口径 200 漂成 500',
     OTXT, r"ORCH_HISTORY_PAGE_SIZE_MAX = 200", "ORCH_HISTORY_PAGE_SIZE_MAX = 500",
     '请求侧夹取到 1..=200，缺省回落后端默认的 20', [OT]),
    ('Q6', '取消请求侧夹取（后端这个面不夹，越界就真的是越界）',
     OTXT, r"Math\.min\(Math\.trunc\(raw\), ORCH_HISTORY_PAGE_SIZE_MAX\)", "Math.trunc(raw)",
     '请求侧夹取到 1..=200，缺省回落后端默认的 20', [OT]),
    ('Q7', 'step_ids 空数组改成"空则省略"（Some(空集) 与 None 是两种语义）',
     OTXT, r"if \(Array\.isArray\(form\.stepIds\)\) body\.step_ids = ids", "if (ids.length) body.step_ids = ids",
     'step_ids 空数组与不发是两种语义，不能像 expert_ids 那样省略', [OT]),
    ('Q8', 'max_experts 用真值判断，0 被静默换成后端缺省 3',
     OTXT, r"if \(form\.maxExperts !== undefined && form\.maxExperts !== null && form\.maxExperts !== ''\) \{",
     "if (form.maxExperts) {",
     'max_experts=0 必须发出去（0 与"不发"在后端是两种结果）', [OT]),
    ('Q9', '归一化把 plans_ready 读成 camel 键（后端只给 snake_case）',
     NRM, r"plansReady: num\(p\.plans_ready\),", "plansReady: num(p.plansReady),",
     'stats：13 个直译 + ts→serverTs，两个分布原样透传', [OT]),
    ('Q10', '历史请求丢掉分页参数（界面翻页却还在动）',
     APIS, r"await get\('orchHistory', \{ query: orchHistoryQuery\(filters\) \}\)",
     "await get('orchHistory', {})",
     '请求侧夹取到 1..=200，缺省回落后端默认的 20', [OT, AT]),
    ('Q11', '历史路径漂成 /orch/history（注册表与 router 都不认）',
     EP, r"path: '/api/experts/orchestration/history'", "path: '/api/experts/orch/history'",
     '五条注册表行的路径与方法逐字对齐 docs/API-REGISTRY.md', [OT, CT]),
    ('Q12', '把已挂载的编排行塞回台账 backlog（账目与页面分家）',
     EP, r"\{ registry: 'experts\.orch\.plugins', verdict: 'rejected'",
     "{ registry: 'experts.orch.orchestrate', verdict: 'backlog', reason: '假称未挂载（experts_orchestration.rs:966）' },\n"
     "    { registry: 'experts.orch.plugins', verdict: 'rejected'",
     '本面 5 行已从台账移出，backlog 归零', [OT]),
    ('Q13', '编排台重新引入裸 hex 颜色',
     AOV, r"\.aov-title \{ margin: 0; font-size: 20px; color: var\(--text-primary\); \}",
     ".aov-title { margin: 0; font-size: 20px; color: #1f2d3d; }",
     '模块内无裸 hex/rgb/hsl', [SY]),
    ('Q14', '角标挂到一个未登记的字段路径（徽标会静默变成"真实计算"）',
     AOV, r"tierLabel\('orchestrate\.result\.summary'\)", "tierLabel('orchestrate.result.summaries')",
     '界面每个带角标的字段路径都在来源清单里登记过', [OT]),
    ('Q15', '常量与模拟字段图例在界面上不再展开',
     AOV, r'v-for="s in store\.simulatedLegend"', 'v-for="s in []"',
     '常量图例转发整张 ORCH_SIMULATED，不筛不减', [OT]),
    ('Q16', 'store 把图例筛掉一条（simulated 从那本账上消失）',
     ORCHS, r"ORCH_SIMULATED\.map\(\(x\) =>", "ORCH_SIMULATED.filter((x) => x.id !== 'step_result_confidence').map((x) =>",
     '常量图例转发整张 ORCH_SIMULATED，不筛不减', [OT]),

    # ── 模式拓扑批次：contract/mode.js + 控制台 DAG 页签接线 ──
    ('X1', 'dynamic 节点数按"看起来该多一个"写成 5（Rust 分支只有 4 次 n(...)）',
     MODJ, r"\[MODE_WIRE\.DYNAMIC\]: \{ nodes: 4,", "[MODE_WIRE.DYNAMIC]: { nodes: 5,",
     '每模式的节点数、Running 个数与按序节点名逐字对齐 Rust', [MODT]),
    ('X2', 'sequential 的节点名抄成 parallel 的名字（两模式在界面上长得一样）',
     MODJ, r"names: \['需求分析', '方案设计',", "names: ['需求分析', '架构设计',",
     '每模式的节点数、Running 个数与按序节点名逐字对齐 Rust', [MODT]),
    ('X3', 'voting 的三路并发记成一路',
     MODJ, r"\[MODE_WIRE\.VOTING\]: \{ nodes: 6, running: 3,", "[MODE_WIRE.VOTING]: { nodes: 6, running: 1,",
     '每模式的节点数、Running 个数与按序节点名逐字对齐 Rust', [MODT]),
    ('X4', 'Pending 计数漏减恒 Completed 的首节点',
     MODJ, r"return t \? t\.nodes - 1 - t\.running : 0", "return t ? t.nodes - t.running : 0",
     'Pending 个数由契约算出且与 Running+Completed 加起来等于节点数', [MODT]),
    ('X5', '说明里的 alliance.rs 坐标整体漂走（话术还在，指针没了）',
     MODJ, r"（alliance\.rs:439-440/:441-515）", "（alliance.rs:539-540/:541-515）",
     '模式说明里的 alliance.rs 两截坐标逐行命中契约注释与函数本体', [MODT]),
    ('X6', '把"不是执行读数"改成肯定式（界面把模板念成进度）',
     MODJ, r"——这些状态来自模板，不是执行读数", "——这些状态来自模板，就是执行读数",
     '每个模式的说明都点出"展示态/模板"与源码坐标，且两条线名都出现', [MODT]),
    ('X7', '把"展示态拓扑"写成"执行态拓扑"',
     MODJ, r"写出的展示态拓扑（", "写出的执行态拓扑（",
     '每个模式的说明都点出"展示态/模板"与源码坐标，且两条线名都出现', [MODT]),
    ('X8', '未知模式不再指名那个值（用户看不到后端给了什么）',
     MODJ, r"（模式值：\${mode \|\| '未知'}）", "（模式值见后端日志）",
     '未知模式不编造拓扑，而是指名那个值', [MODT]),
    ('X9', '末节点契约名凭印象写成"融合结果"',
     MODJ, r"last: '融合输出'", "last: '融合结果'",
     '首末节点契约位成立：首恒 Completed、末恒融合输出', [MODT]),
    ('X10', 'modeWireOf 不再认 mode_display 线名（任务出参一律读成未知模式）',
     MODJ, r"Object\.entries\(MODE_DISPLAY\)\.find\(\(\[, v\]\) => v === mode\)",
     "Object.entries(MODE_DISPLAY).find(([, v]) => v === '__no_such_line__')",
     '拓扑表按两条线都能查：出参给的是 mode_display，请求发的是 mode_serde', [MODT]),
    ('X11', '动态分支节点的 expert_id 记成 expert-routing',
     MODJ, r"gatewayExpert: 'expert-router'", "gatewayExpert: 'expert-routing'",
     '网关侧确有「动态路由」这个节点，且它是普通 expert 节点', [MODT]),
    ('X12', 'plannerAt 的结束行按记忆写成 401（真实大括号配对在 411）',
     MODJ, r"plannerAt: 'planner\.rs:332-411'", "plannerAt: 'planner.rs:332-401'",
     'plannerAt 与两根轴那句话的 types.rs 坐标都框住被引用的那段', [MODT]),
    ('X13', '两根轴那句话把 AllianceMode 的段尾写短（漏掉后两个变体）',
     MODJ, r"types\.rs:133-148", "types.rs:133-140",
     'plannerAt 与两根轴那句话的 types.rs 坐标都框住被引用的那段', [MODT]),
    ('X14', '控制台把那句模式说明从 DAG 页签摘掉（接线只剩脚本里有）',
     ACV, r'<p class="ac-dag-mode">{{ dagModeNote }}</p>', '<p class="ac-dag-mode"></p>',
     '控制台把这段说明接在 DAG 页签上，且字符串来自契约', [MODT]),
    ('X15', '说明条重新引入裸 hex 颜色',
     ACV, r"\.ac-dag-mode \{", ".ac-dag-mode {\n  color: #5a6b7d;",
     '模块内无裸 hex/rgb/hsl', [SY]),
    ('X16', '把"分支不在任何响应里"念成"已在响应里"（界面就有能力装答"走了哪条"）',
     MODJ, r"只在执行器内存里被消费、不在任何响应里", "只在执行器内存里被消费、已在响应里出现",
     'dynamic 的说明同时交代"节点"与"分支缺席"', [MODT]),
    ('X17', '说明条读的是 runtime.mode（本地预览开关）而不是任务的 mode',
     ACV, r"modeTopologyNote\(store\.detail\.task\?\.mode\)", "modeTopologyNote(store.detail.runtime?.mode)",
     '控制台把这段说明接在 DAG 页签上，且字符串来自契约', [MODT]),
]


def read(rel):
    with open(os.path.join(UI, rel), 'rb') as fh:
        return fh.read()


def sha(b):
    return hashlib.sha256(b).hexdigest()[:12]


# 变异态写盘与还原之间被杀，留下的就是一个"看起来还在但其实被改掉"的源文件
# （真实事故：一次中断让 contract/sessions.js 的 tags 差分守卫回退成无条件覆盖，
# 下一次运行把脏内容当基线，锚点自然 SKIP）。所以写盘前先立牌，还原核对后再撤牌。
PENDING = os.path.join(os.environ.get('TEMP', UI), 'collab-mutants-pending.txt')


def run_tests(targets):
    p = subprocess.run(['npx.cmd', 'vitest', 'run', *targets], cwd=UI,
                       capture_output=True, text=True, encoding='utf-8', errors='replace')
    out = (p.stdout or '') + (p.stderr or '')
    failed = re.search(r'Tests.*?(\d+) failed', out)
    passed = re.search(r'Tests.*?(\d+) passed', out)
    return {
        'failed': int(failed.group(1)) if failed else 0,
        'passed': int(passed.group(1)) if passed else 0,
        'errored': 'Errors' in out and 'Failed Suites' in out,
    }


def selected():
    """整跑一遍要 25 分钟以上，容易被工具超时挪到后台并中断在变异态；支持按编号分片前台跑。"""
    args = [a.strip().upper() for a in ','.join(sys.argv[1:]).split(',') if a.strip()]
    if not args:
        return MUTANTS
    known = {m[0].upper() for m in MUTANTS}
    unknown = sorted(set(args) - known)
    if unknown:
        print(f"没有这些变异体: {', '.join(unknown)}（现有 {len(known)} 个）")
        return None
    return [m for m in MUTANTS if m[0].upper() in set(args)]


def main():
    if os.path.exists(PENDING):
        with open(PENDING, encoding='utf-8') as fh:
            stuck = fh.read().strip()
        print(f"上一次运行在写盘后被打断，{stuck} 可能停在变异态：先核对/还原该文件再跑电池，"
              f"否则本次会把脏内容当基线、锚点 SKIP 会被误读成「没这个变异体」。")
        return 1
    mutants = selected()
    if mutants is None:
        return 2
    originals = {rel: read(rel) for rel in {m[2] for m in mutants}}
    baseline = {f: sha(b) for f, b in originals.items()}
    results = []
    for mid, desc, rel, pat, rep, claim, tests in mutants:
        src = originals[rel].decode('utf-8')
        hit = re.search(pat, src)
        if not hit:
            results.append((mid, desc, claim, 'SKIP-锚点未命中', '', ''))
            continue
        mutated = src.replace(hit.group(0), rep, 1)
        with open(PENDING, 'w', encoding='utf-8') as fh:
            fh.write(rel)
        with open(os.path.join(UI, rel), 'wb') as fh:
            fh.write(mutated.encode('utf-8'))
        try:
            r = run_tests(tests)
            verdict = 'CAPTURED' if r['failed'] or r['errored'] else 'SURVIVED'
            detail = f"{r['failed']} failed / {r['passed']} passed"
        finally:
            with open(os.path.join(UI, rel), 'wb') as fh:
                fh.write(originals[rel])
            ok = sha(read(rel)) == baseline[rel]
            if ok:
                os.remove(PENDING)
        results.append((mid, desc, claim, verdict, detail, 'restored' if ok else 'RESTORE-FAIL'))

    width = max(len(x[0]) for x in results)
    bad = 0
    for mid, desc, claim, verdict, detail, restore in results:
        flag = ''
        if verdict != 'CAPTURED' or restore != 'restored':
            flag = '  <-- 需处理'
            bad += 1
        print(f"{mid:<{width}} {verdict:<16} {restore:<13} {detail:<24} {desc} => 断言「{claim}」{flag}")
    print(f"\n捕获 {sum(1 for x in results if x[3] == 'CAPTURED')}/{len(mutants)}（全库 {len(MUTANTS)} 个），异常 {bad}")
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
