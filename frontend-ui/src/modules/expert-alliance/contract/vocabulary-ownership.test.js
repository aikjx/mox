// 联盟词表的"所有权"守卫：模块已导出的名字，不许在模块外再自写一份。
//
// 为什么要这一本账：词表漂不是靠"看起来重复"发现的，而是靠**同一个 wire 值在两处给出不同文案**。
// §5.37 收口的两处副本里，`sessionStatusLabel` 那份只认 active/archived，`closed` 会把英文原样印上界面；
// `bookingStatusLabel` 那份四档逐字相同，但它是第二本字典——后端加第五档时只有模块那份会更新。
//
// 判据分通道（合在一条 it 里会互相顶红）：
//   L1  模块外零自写（登记口除外）——15 处登记各有理由，不是"看着像就放过"
//   L1b 针的牙齿：定义形态打得红，导入/调用/注释/同前缀名打不红
//   L2  分母：模块导出集与扫描集都不许塌缩（"0 命中"必须来自"扫到了且没有"）
//   L3  两处收口的文案账：已知档逐字相同、新增覆盖逐条点名
//   L4  两处收口的接线账：确实从模块取，且副本没被抄回来
//   L5  与 §5.36 时间副本台账的一致性：同一事实的两本账不许各说各话
//   L6  企业面板：会话筛选/展示的档必须来自契约表（内联私表＝整档缺失）
//   L7  联盟任务页：状态档由 TASK_STATUS 全集推出，文案走出口，四本私表不许回来
//   L8  DAG 画布节点副标签：走 nodeStatusLabel 出口，不许直印 wire 原始枚举键
import { describe, it, expect } from 'vitest'
import { readFileSync, readdirSync, statSync } from 'fs'
import path from 'path'
import { bookingStatusLabel, sessionStatusLabel, sessionTypeLabel, SESSION_STATUSES, SESSION_TYPES, BOOKING_STATUS_LABELS, TASK_STATUS, taskStatusLabel, NODE_STATUS, EXPERT_AVAILABILITY, BREAKER_STATE, ROOM_STATUS, MESSAGE_ROLES, MSG_TYPES, MSG_TYPE_DEFAULT, BREAKER_STATE_LABELS , FUSION_STATUS } from '@/modules/expert-alliance/contract'

const countOf = (text, needle) => text.split(needle).length - 1

const SRC_DIR = path.resolve(__dirname, '../../..')
const MODULE_DIR = 'modules/expert-alliance'
const DEF_FILES = ['.js', '.ts', '.vue']

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist') continue
    const p = path.join(dir, name)
    if (statSync(p).isDirectory()) walk(p, out)
    else if (DEF_FILES.includes(path.extname(name))) out.push(p)
  }
  return out
}

/** 模块对外的名字：只认 contract/ 与 model/ 的具名导出（视图层拿得到的那一份词表） */
function moduleExports() {
  const names = new Set()
  for (const sub of ['contract', 'model']) {
    const dir = path.join(SRC_DIR, MODULE_DIR, sub)
    for (const name of readdirSync(dir)) {
      if (!name.endsWith('.js') || name.endsWith('.test.js')) continue
      for (const line of readFileSync(path.join(dir, name), 'utf8').split('\n')) {
        const m = /^\s*export\s+(?:async\s+)?(?:function|const|let)\s+([A-Za-z_$][\w$]*)/.exec(line)
        if (m) names.add(m[1])
        const d = /^\s*export\s+(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=\s*\{/
        if (d) names.add(d[1])
      }
    }
  }
  return names
}

const EXPORTED = moduleExports()
const externalFiles = () =>
  walk(SRC_DIR)
    .map((p) => path.relative(SRC_DIR, p).split(path.sep).join('/'))
    .filter((rel) => !rel.startsWith(MODULE_DIR) && !/\.(test|stories)\.(js|ts)$/.test(rel))

/** 命中形态：定义位点（`function NAME(` 或 `const NAME =`），导入与调用点都不算 */
const defRe = (name) =>
  new RegExp(`^\\s*(export\\s+)?(async\\s+)?(function\\s+${name}\\b|const\\s+${name}\\s*=(?!=))`)

// 一次扫描只提一个名字再查集合，比"每个名字各扫一遍全库"快两个量级（346 名 × 219 文件时单条判据要 27 s）。
// 两本实现（本判据与 scratch 普查驱动的逐名针）必须给出同一批 15 处，对不上就是扫描仪错了。
const DEF_LINE = /^\s*(?:export\s+)?(?:async\s+)?(?:function\s+([A-Za-z_$][\w$]*)|const\s+([A-Za-z_$][\w$]*)\s*=(?!=))/

function scanExternalSelfWrites() {
  const hits = []
  for (const rel of externalFiles()) {
    const lines = readFileSync(path.join(SRC_DIR, rel), 'utf8').split('\n')
    lines.forEach((line, i) => {
      const t = line.trim()
      if (t.startsWith('//') || t.startsWith('*') || t.startsWith('/*')) return
      const m = DEF_LINE.exec(line)
      if (!m) return
      const name = m[1] ?? m[2]
      if (EXPORTED.has(name)) hits.push({ rel, line: i + 1, name })
    })
  }
  return hits
}

// 登记口：15 处逐条给理由。三类——
//   T：§5.36 已按输入形状逐档量过的时间副本（不同档不并），此处只做第二本账的交叉一致（见 L5）
//   N：跨命名空间同名（别的模块自己的契约表 / 局部状态变量），名字撞了但判的是两件事
//   D：同域但档位真的不同，并入会改界面显示——没定价不许动，卡点登记在 §5.37
const REGISTERED = {
  'components/MessageBubble.vue:formatTime': 'T §5.36：复合档（当天 HH:mm、跨天 MM-DD HH:mm、坏值空串）',
  'views/admin/panels/AdminLlm.vue:formatTime': 'T §5.36：MM/DD HH:mm:ss',
  'views/admin/panels/AdminSso.vue:formatTime': 'T §5.36：纯字符串手术印服务端 UTC 挂钟，已定位未修的显示缺陷，修法要先选档',
  'views/ai/ChatView.vue:formatTime': 'T §5.36：M/D 日期档（无年份）',
  'views/expert/panels/ExpertEnterprisePanel.vue:formatTime': 'T §5.36：M/D HH:mm 短档，出口五档都没有这一形',
  'views/workflow/BrowserView.vue:formatTime': 'T §5.36：YYYY/MM/DD HH:mm:ss',
  'modules/admin-lowcode/contract/endpoints.js:ENDPOINTS': 'N 另一模块自己的端点表（跨命名空间同名；键集未逐字比对，若与别表全等另案）',
  'modules/governance/contract/endpoints.js:ENDPOINTS': 'N 同上，governance 自己的端点表',
  'modules/governance/contract/endpoints.js:requestPath': 'N governance 自己的请求路径拼装，与联盟 requestPath 不同源',
  'stores/alliance.store.js:PHASE_META': 'N 零消费者空壳（`useAllianceStore` 除 stores/ 外无 importer），退役裁决待用户',
  'stores/alliance.store.js:phaseProgress': 'N 同一空壳里的局部 ref，不是文案函数',
  'views/expert/panels/ExpertOverviewPanel.vue:phaseProgress': 'N 局部 ref：按阶段名存百分比，与 contract 的 phaseProgress(doneCount) 判的两件事不同',
  'views/workspace/ExpertWorkspaceView.vue:collabMode': 'N 局部 ref：存当前 mode 键值，不是 contract 的 collabMode(key) 文案函数',
  'components/ai/GateResult.vue:gradeLabel': 'D 同域不同档：圆环位只放得下单词（优秀/良好/合格/不合格），contract 的 GRADE_LABELS 带「· 优质交付」后缀；`retryable` 判的是 grade 值不是文案',
  'components/MessageBubble.vue:confidenceText': 'D 档位冲突：真 0 置信度这里显 0%，contract 把 n<=0 归为「—」（无值）；并入会把 0 显示成空态，需先量 wire 上 confidence 能否为 0',
}

// L9：L1 的扫描集**排除模块自身**（那是词表的主人），所以"模块内部再自写一份同名出口"对 L1 天然隐身。
// 本格把扫描集换到模块内部（contract/ 与 model/ 这两个定义源除外），命中集＝登记集双向相等，判据同样不是清零。
const INTERNAL_DIR_OWNERS = [`${MODULE_DIR}/contract`, `${MODULE_DIR}/model`]
const REGISTERED_INTERNAL = {
  'modules/expert-alliance/components/ExpertBookingPanel.vue:formatTime':
    'N 站点包装：文案交回 formatDateTimeLocale，本地只留空态「—」与坏值回显 iso 档（§5.36 同族的"档不同不并"），不是文案副本',
  'modules/expert-alliance/components/GraphNodeInspector.vue:collaboratorRows':
    'N 局部 computed：把出参里的协作者数组摊平，与 contract 的 collaboratorRows(...) 判的不是同一件事',
}

function scanModuleSelfWrites() {
  const hits = []
  const files = walk(SRC_DIR)
    .map((p) => path.relative(SRC_DIR, p).split(path.sep).join('/'))
    .filter(
      (rel) =>
        rel.startsWith(MODULE_DIR) &&
        !INTERNAL_DIR_OWNERS.some((owner) => rel.startsWith(owner)) &&
        !/\.(test|stories)\.(js|ts)$/.test(rel)
    )
  for (const rel of files) {
    const lines = readFileSync(path.join(SRC_DIR, rel), 'utf8').split('\n')
    lines.forEach((line, i) => {
      const t = line.trim()
      if (t.startsWith('//') || t.startsWith('*') || t.startsWith('/*')) return
      const m = DEF_LINE.exec(line)
      if (!m) return
      const name = m[1] ?? m[2]
      if (EXPORTED.has(name)) hits.push({ rel, line: i + 1, name })
    })
  }
  return { hits, fileCount: files.length }
}

const PANEL_REL = 'modules/expert-alliance/components/ExpertCollabPanel.vue'

describe('联盟词表所有权：模块外不许自写模块已导出的名字', () => {
  it('L2 分母：模块导出集与扫描集都没塌缩', () => {
    const files = externalFiles()
    console.log(`[词表] 模块导出 ${EXPORTED.size} 个名字 / 模块外扫描 ${files.length} 个文件 / 登记 ${Object.keys(REGISTERED).length} 处`)
    expect(EXPORTED.size).toBeGreaterThanOrEqual(300)
    expect(files.length).toBeGreaterThanOrEqual(200)
  })

  it('L1 模块外零自写（登记口除外）', () => {
    const hits = scanExternalSelfWrites()
    const keys = hits.map((h) => `${h.rel}:${h.name}`)
    expect(keys.sort()).toEqual(Object.keys(REGISTERED).sort())
    for (const h of hits) expect(REGISTERED[`${h.rel}:${h.name}`], `${h.rel}:${h.line} ${h.name} 没写理由`).toBeTruthy()
  })

  it('L1b 正对照：定义形态能被打红，导入与调用点不能被打红', () => {
    const name = 'sessionStatusLabel'
    expect(defRe(name).test(`function ${name}(status) {`)).toBe(true)
    expect(defRe(name).test(`  const ${name} = (s) => s`)).toBe(true)
    expect(defRe(name).test(`import { ${name} } from '@/modules/expert-alliance/contract'`)).toBe(false)
    expect(defRe(name).test(`  const label = ${name}(row.status)`)).toBe(false)
    expect(defRe(name).test('// function sessionStatusLabel 旧副本已删')).toBe(false)
    // 同前缀的不同名字不许撞车（这条挡住"用子串当键"的判据写法）
    expect(defRe(name).test('function sessionStatusLabelExtra(x) {')).toBe(false)
  })

  it('L3 两处收口的文案账：已知档逐字相同', () => {
    for (const [value, label] of Object.entries(BOOKING_STATUS_LABELS)) {
      expect(bookingStatusLabel(value), value).toBe(label)
    }
    expect(bookingStatusLabel('')).toBe('')
    expect(bookingStatusLabel('whatever')).toBe('whatever')
    for (const { value, label } of SESSION_STATUSES) {
      expect(sessionStatusLabel(value), value).toBe(label)
    }
  })

  it('L3b 新增覆盖（就是旧副本的缺陷）：closed 与未知状态不再原样上界面', () => {
    expect(sessionStatusLabel('closed')).toBe('已关闭')
    expect(sessionStatusLabel('unknown-x')).toBe('unknown-x（后端未统计此状态）')
    expect(sessionStatusLabel('')).toBe('未知状态')
  })

  it('L4 两处收口确实从模块取，副本没抄回来', () => {
    const sites = [
      ['views/expert/panels/ExpertEnterprisePanel.vue', 'sessionStatusLabel'],
      ['views/expert/ExpertPlazaView.vue', 'bookingStatusLabel'],
    ]
    for (const [rel, name] of sites) {
      const src = readFileSync(path.join(SRC_DIR, rel), 'utf8')
      // 针必须锚在行首：注释里留着一条 `// import { x } from ...` 也算命中，那是假绿
      expect(src, rel).toMatch(new RegExp(`^import \\{[^}]*\\b${name}\\b[^}]*\\} from '@/modules/expert-alliance/contract'`, 'm'))
      expect(src, `${rel} 又抄回一份 ${name}`).not.toMatch(defRe(name))
    }
  })

  it('L5 与 §5.36 时间副本台账同源：登记的六处 formatTime 必须也在那本账里', () => {
    const ledger = readFileSync(path.join(SRC_DIR, 'utils/time.test.js'), 'utf8')
    const block = ledger.slice(ledger.indexOf('const KNOWN_COPIES = {'), ledger.indexOf('\n}', ledger.indexOf('const KNOWN_COPIES = {')))
    const registered = [...block.matchAll(/^\s*'([^']+\.vue)':\s*\d/gm)].map((m) => m[1])
    const mine = Object.keys(REGISTERED)
      .filter((k) => k.endsWith(':formatTime') && !k.startsWith('modules/'))
      .map((k) => k.slice(0, -':formatTime'.length))
    expect(mine.length).toBeGreaterThanOrEqual(1)
    for (const rel of mine) expect(registered, `${rel} 在词表账里登记了，时间账里没有`).toContain(rel)
  })

  // §5.38：企业面板的会话筛选器曾经是一张私表——少一档（closed 筛不到）、值不对（smart/multi_expert
  // 不是 session_type 取值，选了就筛空）、展示对 multi/enterprise 落回英文。三处后果同一根因。
  const PANEL = 'views/expert/panels/ExpertEnterprisePanel.vue'
  it('L6 会话筛选与展示取自契约表，私表不许抄回来', () => {
    const src = readFileSync(path.join(SRC_DIR, PANEL), 'utf8')
    // 只钉"这个名字经由契约进来"，不钉句内同伴顺序（§5.36 的教训：钉整句会让别人改不动这一行）
    for (const name of ['SESSION_STATUSES', 'SESSION_TYPES', 'sessionTypeLabel']) {
      expect(src, `${name} 没从契约取`).toMatch(
        new RegExp(`^import \\{[^}]*\\b${name}\\b[^}]*\\} from '@\\/modules\\/expert-alliance\\/contract'`, 'm'))
    }
    // 只看这两个 select 自己的块：文件里另有 feeds 模板桩 enterpriseConsult 的模式选择，那是另一张词表
    const block = (vm) => {
      const at = src.indexOf(`<el-select v-model="${vm}"`)
      expect(at, `找不到 ${vm} 的 select`).toBeGreaterThanOrEqual(0)
      const end = src.indexOf('</el-select>', at)
      return src.slice(at, end)
    }
    for (const [vm, table] of [['sessionFilterStatus', 'SESSION_STATUSES'], ['sessionFilterMode', 'SESSION_TYPES']]) {
      const b = block(vm)
      expect(b, `${vm} 没接契约`).toContain(`v-for="o in ${table}"`)
      expect(b, `${vm} 又写回硬编码 el-option 档`).not.toMatch(/<el-option label="[^"]*" value="/)
    }
    // 全文件各只有一处接契约（第二处＝同一个筛选器被复制了一份）
    expect(countOf(src, 'v-for="o in SESSION_STATUSES"')).toBe(1)
    expect(countOf(src, 'v-for="o in SESSION_TYPES"')).toBe(1)
    expect(src, 'modeLabels 私表又回来了').not.toContain('modeLabels')
    // 客户端二次筛必须落在服务端回显的字段上：`s.mode` 是旧别名，别名一撤就恒不等
    expect(src).toContain('s.sessionType !== sessionFilterMode.value')
    expect(src, '还在筛不存在的 mode 别名').not.toContain('s.mode')
  })

  it('L6b 契约取值账：四档三档逐字钉住，标签出口各档行为点名', () => {
    expect(SESSION_TYPES.map((t) => t.value)).toEqual(['single', 'multi', 'debate', 'enterprise'])
    expect(SESSION_STATUSES.map((s) => s.value)).toEqual(['active', 'archived', 'closed'])
    expect(SESSION_TYPES.length).toBeGreaterThanOrEqual(4)
    expect(SESSION_STATUSES.length).toBeGreaterThanOrEqual(3)
    expect(sessionTypeLabel('multi')).toBe('多专家')
    expect(sessionTypeLabel('enterprise')).toBe('企业级')
    expect(sessionTypeLabel('')).toBe('未分类')
    // 未知值原样回显（与 sessionStatusLabel 的「（后端未统计此状态）」不对称，已登记为卡点，见 §5.38）
    expect(sessionTypeLabel('weird-x')).toBe('weird-x')
  })

  it('L7 联盟任务页：状态档由契约全集推出，文案走出口，四本私表不许回来', () => {
    const src = readFileSync(path.join(SRC_DIR, 'views/expert/AllianceTaskView.vue'), 'utf8')
    for (const name of ['TASK_STATUS', 'taskStatusLabel', 'nodeStatusLabel', 'fusionLabel']) {
      expect(src, `${name} 没从契约取`).toMatch(
        new RegExp(`^import \\{[^}]*\\b${name}\\b[^}]*\\} from '@/modules/expert-alliance/contract'`, 'm'))
    }
    // 筛选条要由 TASK_STATUS 全集推出：旧表自列六档，planning（规划中）的任务在界面筛不到
    expect(src, '状态筛选又不从契约全集推了').toContain(
      "...Object.values(TASK_STATUS).map((key) => ({ key, label: taskStatusLabel(key) }))")
    expect(src, '状态档文案又写回内联表').not.toMatch(/\{ key: '(pending|running|completed)', label: /)
    // 第四本状态字典换过名字（`statusLabel` 不是模块导出名），所以 L1 按名字扫不到 ⇒ 这一格按形状钉
    expect(src, 'statusLabel 私表又回来了').not.toMatch(/^\s*const statusLabel\s*=/m)
    expect(countOf(src, 'statusLabel('), '又出现裸 statusLabel 调用').toBe(0)
    expect(countOf(src, 'taskStatusLabel(task.status)')).toBe(2)
    // 档位子集可以留在视图（产品选择），但文案一律走契约出口
    expect(src, '融合策略档又写回硬编码 label').not.toMatch(/<el-option label="[^"]*" value="(weighted|voting|debate|best_of)"/)
    expect(src).toContain(':label="fusionLabel(o)"')
    expect(src).toContain('{{ nodeStatusLabel(s) }}')
    expect(countOf(src, 'const FUSION_CHOICES =')).toBe(1)
    expect(countOf(src, 'const DAG_LEGEND =')).toBe(1)
  })

  it('L7b 契约任务状态账：七档取值钉住、每档都有中文（契约少一档＝筛选器跟着少一档）', () => {
    expect(Object.values(TASK_STATUS)).toEqual(['pending', 'planning', 'running', 'paused', 'completed', 'failed', 'cancelled'])
    for (const s of Object.values(TASK_STATUS)) {
      const label = taskStatusLabel(s)
      expect(label, `${s} 没有文案`).toBeTruthy()
      expect(label, `${s} 掉到了原样回显`).not.toBe(s)
    }
    // 出口行为点名：未命中回原值、空值回空串（视图靠它把"这档叫什么"交回契约）
    expect(taskStatusLabel('weird-x')).toBe('weird-x')
    expect(taskStatusLabel('')).toBe('')
  })
  it('L8 DAG 画布节点副标签：走契约出口，不许把 wire 原始枚举键直印上画布', () => {
    const src = readFileSync(path.join(SRC_DIR, 'views/expert/AllianceTaskView.vue'), 'utf8')
    // 两条服务通道把这个键硬编成同一个常量，直印＝中文画布里嵌一个英文词且零信息量；
    // 模块自己的 normNode 干脆不携带该键 ⇒ 迁到契约口径时这一格会自动变空
    expect(src, '节点副标签又直印 wire 原始枚举键').not.toMatch(/\{\{ node\.type \}\}/)
    expect(countOf(src, '{{ nodeStatusLabel(node.status) }}')).toBe(1)
  })
  it('L9 模块内部自写同名出口：命中集＝登记集双向相等（补 L1 扫描集的模块盲区）', () => {
    const { hits, fileCount } = scanModuleSelfWrites()
    console.log(`[词表·模块内] 扫描 ${fileCount} 个文件 / 命中 ${hits.length} 处 / 登记 ${Object.keys(REGISTERED_INTERNAL).length} 处`)
    expect(fileCount, '模块内部扫描集塌缩＝判据空转').toBeGreaterThanOrEqual(30)
    const hitKeys = hits.map((h) => `${h.rel}:${h.name}`).sort()
    const registered = Object.keys(REGISTERED_INTERNAL).sort()
    expect(hitKeys, '命中集与登记集不等（新增自写＝缺陷；登记项消失＝该撤掉条目而不是留着挡路）').toEqual(registered)
    for (const h of hits) expect(REGISTERED_INTERNAL[`${h.rel}:${h.name}`], '命中没给理由').toBeTruthy()
    // 本轮真收口的那一处：撤掉局部字典、两处站点改走契约出口复合，import 必须按名在位
    const panel = readFileSync(path.join(SRC_DIR, PANEL_REL), 'utf8')
    expect(panel, '模块内的局部状态字典又回来了').not.toMatch(/^\s*const\s+availabilityLabel\s*=/m)
    expect(new RegExp("^import \\{[^}]*\\bavailabilityLabel\\b[^}]*\\} from '@/modules/expert-alliance/contract'", 'm').test(panel),
      '用了 availabilityLabel 却没按名从契约导入').toBe(true)
    expect(countOf(panel, 'availabilityLabel(expertDisplayStatus('), '状态档没走「显示归一＋文案出口」复合').toBe(2)
  })
  it('L10 熔断器状态档走契约出口：wire 的 open/closed 不许直印上界面', () => {
    const src = readFileSync(path.join(SRC_DIR, 'modules/expert-alliance/views/AllianceConsoleView.vue'), 'utf8')
    expect(src, '熔断器状态又直印 wire 原始值').not.toMatch(/\{\{\s*c\.state\s*\}\}/)
    expect(countOf(src, '{{ breakerStateLabel(c.state) }}'), '状态档出口站点数不符').toBe(1)
    expect(new RegExp("^import \\{[^}]*\\bbreakerStateLabel\\b[^}]*\\} from '@/modules/expert-alliance/contract'", 'm').test(src),
      '用了 breakerStateLabel 却没按名从契约导入').toBe(true)
    expect(src, '视图里自写 breakerStateLabel 副本').not.toMatch(/^\s*(const|function)\s+breakerStateLabel\s*=/m)
  })
  it('L10b 熔断器契约账：两档取值逐字钉住、每档都有中文、出口行为点名', async () => {
    const { BREAKER_STATE, BREAKER_STATE_LABELS, breakerStateLabel } = await import('./enums')
    const keys = Object.values(BREAKER_STATE)
    expect(keys, 'wire 权威只有 open/closed 两档（experts_dispatcher.rs:494-509）').toEqual(['open', 'closed'])
    for (const k of keys) {
      expect(BREAKER_STATE_LABELS[k], k + ' 档没有中文文案').toBeTruthy()
      expect(BREAKER_STATE_LABELS[k] === k, k + ' 档的文案等于键名＝没归一').toBe(false)
    }
    for (const k of keys) expect(breakerStateLabel(k), `${k} 档出口没走标签表`).toBe(BREAKER_STATE_LABELS[k])
    expect(breakerStateLabel('tripped'), '未知档要原样回显').toBe('tripped')
    expect(breakerStateLabel(''), '空档要回空串交回站点空态').toBe('')
  })
  it('L11 编排步骤状态档走契约出口：:75 / :126 / :133 三站点不许直印 wire 原始值', () => {
    const src = readFileSync(path.join(SRC_DIR, 'modules/expert-alliance/views/AllianceOrchestrationView.vue'), 'utf8')
    expect(src, '编排步骤状态又直印 wire 原始值').not.toMatch(/\{\{\s*s\.status\s*\}\}/)
    expect(countOf(src, '{{ orchStatusLabel(s.status) }}'), '步骤状态出口站点数不符').toBe(2)
    expect(new RegExp("^import \\{[^}]*\\borchStatusLabel\\b[^}]*\\} from '@/modules/expert-alliance/contract'", 'm').test(src),
      '用了 orchStatusLabel 却没按名从契约导入').toBe(true)
    expect(src, '视图里自写 orchStatusLabel 副本').not.toMatch(/^\s*(const|function)\s+orchStatusLabel\s*=/m)
    expect(src, 'outcome 徽章仍直印 wire 原始 status').not.toMatch(/\{\{\s*store\.outcome\.status\b/)
    expect(countOf(src, 'orchStatusLabel(store.outcome.status)'), 'outcome 状态出口站点数不符').toBe(1)
  })
  it('L11b 编排状态契约账：七档取值逐字钉住、每档都有中文、出口只钉管道', async () => {
    const { ORCH_STATUS, ORCH_STATUS_LABELS, orchStatusLabel } = await import('./orchestration')
    const keys = Object.values(ORCH_STATUS)
    expect(keys, 'wire 权威（experts_orchestration.rs :186/:203/:469/:490/:456/:508/:750）').toEqual(
      ['pending', 'draft', 'running', 'completed', 'failed', 'partial', 'unknown'])
    for (const k of keys) {
      expect(ORCH_STATUS_LABELS[k], k + ' 档没有中文文案').toBeTruthy()
      expect(ORCH_STATUS_LABELS[k] === k, k + ' 档的文案等于键名＝没归一').toBe(false)
      expect(orchStatusLabel(k), k + ' 档出口没走标签表').toBe(ORCH_STATUS_LABELS[k])
    }
    expect(orchStatusLabel('blocked'), '未知档要原样回显').toBe('blocked')
    expect(orchStatusLabel(''), '空档要回空串交回站点空态').toBe('')
  })

  // 覆盖面裁决（§5.46 实测）：把 MODE_WIRE/MODE_DISPLAY/FUSION_STRATEGY 一并纳入后普查只多 1 处命中——
  // ExpertCollabPanel.vue:132 的 store.resultKind === 'debate'。resultKind 是前端派生档（store/alliance-collab.store.js:47
  // 由 collabMode(...).resultKind 算出），与那三张 wire 表只是同字符串撞名 ⇒ (c) 档跨域同名，不纳，纳了就是假阳。
  it('L12 视图不许把契约已登记的取值重新打字：字面量比较双向台账', () => {
    const banned = new Set([
      ...Object.values(TASK_STATUS), ...Object.values(NODE_STATUS), ...Object.values(EXPERT_AVAILABILITY),
      ...Object.values(BREAKER_STATE), ...SESSION_STATUSES.map((s) => s.value), ...Object.values(ROOM_STATUS),
      // 消息族：值域权威是 sessions.js 的两张表（Rust experts_collaboration.rs:830/843/943 role，:834/847 msg_type）
      ...MESSAGE_ROLES.map((r) => r.value), ...MSG_TYPES.map((t) => t.value), MSG_TYPE_DEFAULT,
      // §5.53 F24：融合档值域由 alliance.rs:544/1235 现推（pending|partial|completed），partial 是本表独有的新增覆盖
      ...Object.values(FUSION_STATUS)
    ])
    // §5.53 F24：联盟域的非 .vue 使用位点（store/composable/api）按名在册。
    // 反事实现量（D:/tmp/f24-js-classify.mjs）：全 src 的 .js/.ts 命中 30 处，其中同串跨域（project/tenant/ai.chat/TTS 等）25 处出账；
    // 真正读联盟 wire 取值的只有下面两份 ⇒ 按名纳，纳全套就是把别的域也算成债。
    // §5.55 F26 更正：那句「.js/.ts」里的 .ts 从未有语料（src 实测 0 个 .ts），普查命中全为 .js；名单外语言的接管见下方现量格。
    const JS_IN_SCOPE = ['api/allianceTaskModel.api.js', 'composables/useAllianceTasks.js']
    const files = walk(SRC_DIR)
    // 前缀过滤器先按目录收（模块 views/components ＋ views/expert/**），在册 .js 靠名单旁路进来：
      .map((p) => path.relative(SRC_DIR, p).split(path.sep).join('/'))
      .filter((rel) => rel.startsWith(MODULE_DIR + '/views/') || rel.startsWith(MODULE_DIR + '/components/')
        // §5.51 F21：模块外同族视图一直在集外＝覆盖面洞。反事实扩集现量：纳 views/expert/**（7 文件）命中 27 处，
        // 其中 18 处判为同域债并已并成契约常量（文案一字未动），余 9 处（4 个唯一键）逐条读原文登记在 EXEMPT。
        || rel.startsWith('views/expert/') || JS_IN_SCOPE.includes(rel))
      .filter((rel) => rel.endsWith('.vue') || JS_IN_SCOPE.includes(rel))
    // 断言顺序＝仪器自检在前（名单自身→成员构成→名单外语言），再是被检对象的扫描集尺寸：
    // vitest 一遇首条失败即止，所以每格都要有一枚"它是第一个说谎的"变异体，否则永远被前面的格顶掉。
    expect(JS_IN_SCOPE.length, 'L12 按名名单塌缩实测=' + JS_IN_SCOPE.length
      + '（撤掉一条＝覆盖面静默缩回，要撤先改判据口径再留痕）').toBe(2)
    // §5.55 F26：非 .vue 成员数与按名名单长度**恰等**（旧格写死 `.js` 后缀，换成别的语言就会自相矛盾）。
    // 两个失效方向各管一头：名单写了却没进集＝红；名单项其实是个 .vue（等于旁路被白拿）＝红。
    expect(files.filter((r) => !r.endsWith('.vue')).length,
      'L12 非 .vue 成员实测=' + files.filter((r) => !r.endsWith('.vue')).length
      + ' 必须恰等于按名名单长度（名单没进集／名单项并非非 .vue 位点都算覆盖面不符）').toBe(JS_IN_SCOPE.length)
    // §5.56 F27：覆盖面还要按**目录**算——模块内的目录既不在前缀池、又没带理由出账，就必须红（新目录不许静默隐身）。
    // 反事实扩集现量（D:/tmp/f27-e-shape-census.mjs，未落库＝没牙；先在在册集上复算到与判据同读数 A=9 B=0 C=0 D=0 E=2 才算同口径）：
    // 集外引用联盟契约的 34 个文件命中 16 处，逐条读原文 0 处属"读联盟 wire 取值"的同域债——
    // stores/ai.store.js 与 composables/workspace/useTaskOrchestration.js 是本地状态机（后者用 Math.random 定成败、
    // 自己写 task.status），views/workspace/panels/CollaborationPanel.vue 直印的是本地编排消息的 role/status，
    // stores/alliance.store.js 的 runState 只在本地赋值（:96 idle|running|done|error）
    // ⇒ 值域扫描集**不扩**（与 §5.46 F15 的 MODE_WIRE 同型裁决），但"模块里新增一个目录就没人管"改由下面这本账接管。
    const POOLED_DIRS = ['views', 'components']
    const DIR_ACCOUNT = {
      contract: '值域定义处（十张表就在这里），纳进来等于自己查自己',
      model: 'wire→视图的归一层，键名账在 normalize.js 与 L13，不重新打字取值',
      store: 'pinia 容器，取值一律从契约引；宽口径逐文件复扫（不要求引用契约）实测 2 处命中全在 alliance-experts.store.test.js 的夹具默认值与派生布尔（测试自己造数据，非界面出口）',
      api: '请求封装，携带的是路径与键，不是状态档名',
      _verification: 'markdown 证据，不是源码'
    }
    const moduleDirs = readdirSync(path.join(SRC_DIR, MODULE_DIR))
      .filter((n) => statSync(path.join(SRC_DIR, MODULE_DIR, n)).isDirectory())
    // 顺序＝塌缩→理由→空壳→未登记：每格各有一枚"它是第一个说谎的"变异体（撤条目／空理由／改名目录／新目录）。
    expect(Object.keys(DIR_ACCOUNT).length, 'L12 出账目录表塌缩实测=' + Object.keys(DIR_ACCOUNT).length
      + '（少一条＝覆盖面静默扩大；要撤条目必须先改池子口径并留痕）').toBe(5)
    const blankWhy = Object.keys(DIR_ACCOUNT).filter((k) => !DIR_ACCOUNT[k] || DIR_ACCOUNT[k].length < 8)
    expect(blankWhy, 'L12 出账理由为空或过短（没理由的出账不存在，同 EXEMPT 的 why 规矩）实测='
      + JSON.stringify(blankWhy)).toEqual([])
    const staleDirs = Object.keys(DIR_ACCOUNT).filter((k) => !moduleDirs.includes(k))
    expect(staleDirs, 'L12 出账表里有已不存在的目录（该删条目，别留空壳理由）实测='
      + JSON.stringify(staleDirs)).toEqual([])
    const unaccountedDirs = moduleDirs.filter((d) => !POOLED_DIRS.includes(d)
      && !Object.prototype.hasOwnProperty.call(DIR_ACCOUNT, d))
    console.log('[L12 目录] 模块内目录 ' + moduleDirs.length + ' 个（在册 ' + POOLED_DIRS.length
      + '／出账 ' + Object.keys(DIR_ACCOUNT).length + '／未登记 ' + unaccountedDirs.length
      + '）实测=' + JSON.stringify(moduleDirs))
    expect(unaccountedDirs, 'L12 模块内出现未登记的目录：进 POOLED_DIRS 纳管，或写进 DIR_ACCOUNT 带理由出账（实测='
      + JSON.stringify(unaccountedDirs) + '）').toEqual([])
    // §5.57 F28：调色板三函数的实参形态账。针取"调用点直接传成员访问"这一形态——
    // 归一化专家（normalize.js normExpert）根本没有 `type` 键（:248 把它折进 expertType），
    // 所以 `expertEmoji(exp.type)` 这类写法在真数据上恒落 EXPERT_EMOJI_FALLBACK，而不是"某个专家的 emoji"。
    const PALLETTE_FNS = ['expertColor', 'expertEmoji', 'expertGradient']
    const PALLETTE_ALLOWED = /^(?:expertVisualKey|rowVisualKey|visualKey)\(|^[A-Za-z_$][\w$]*$/
    const PALLETTE_LEDGER = {
      'views/workspace/panels/TaskOrchestrationPanel.vue': {
        n: 3,
        why: '路由建议键 task.suggestedExpertType 与调色板同词表（useTaskOrchestration.js:127 就拿 expertVisualKey(e) 与它相等比较），读的是出参字段不是重新打字'
      },
      'views/workspace/panels/SmartRouteDialog.vue': {
        n: 2,
        why: 'model 层已把 visualKey 算进出参（读 item.visualKey 这个键名本身），不再套一层调用'
      }
    }
    const paletteSrc = walk(SRC_DIR)
      .map((p) => path.relative(SRC_DIR, p).split(path.sep).join('/'))
      .filter((rel) => !rel.endsWith('.test.js') && rel !== 'constants/expert.constants.js')
    const paletteHits = []
    for (const rel of paletteSrc) {
      const lines = readFileSync(path.join(SRC_DIR, rel), 'utf8').split('\n')
      lines.forEach((line, i) => {
        for (const fn of PALLETTE_FNS) {
          const re = new RegExp('\\b' + fn + '\\s*(?:\\?\\.)?\\(([^)]*)\\)', 'g')
          let m
          while ((m = re.exec(line)) !== null) {
            const arg = m[1].trim()
            if (arg && !PALLETTE_ALLOWED.test(arg)) paletteHits.push({ rel, line: i + 1, arg })
          }
        }
      })
    }
    const paletteByFile = {}
    for (const h of paletteHits) paletteByFile[h.rel] = (paletteByFile[h.rel] || 0) + 1
    console.log('[L14 调色板] 调用点扫描集 ' + paletteSrc.length + ' 个文件／命中 ' + paletteHits.length
      + ' 处／非白名单形态文件 ' + Object.keys(paletteByFile).length + ' 个 实测='
      + JSON.stringify(paletteHits.map((h) => h.rel + ':' + h.line + ' ' + h.arg)))
    expect(paletteSrc.length, 'L14 调色板扫描集塌缩（判集为空时台账恒真）实测=' + paletteSrc.length).toBeGreaterThanOrEqual(100)
    expect(Object.keys(paletteByFile).sort(), 'L14 出现未登记的调色板调用点（新增形态要写进 PALLETTE_LEDGER 或改走 expertVisualKey）实测='
      + JSON.stringify(paletteByFile)).toEqual(Object.keys(PALLETTE_LEDGER).sort())
    for (const rel of Object.keys(PALLETTE_LEDGER)) {
      expect(paletteByFile[rel], 'L14 ' + rel + ' 的命中数与台账登记数不符（实测=' + paletteByFile[rel]
        + ' 登记=' + PALLETTE_LEDGER[rel].n + '）').toBe(PALLETTE_LEDGER[rel].n)
      expect(PALLETTE_LEDGER[rel].why.length, 'L14 ' + rel + ' 的台账缺理由').toBeGreaterThanOrEqual(8)
    }
    // 这里曾有第五格（逐条命中 toBeTruthy 查台账），本轮删掉：它的成立条件＝"某命中的文件不在台账里"，
    // 与上一格的 toEqual 是同一条判据，而 toEqual 排在前面 ⇒ 没有任何变异体能让它成为第一个说谎的。
    // 没有自己变异体的格是装饰，留它只会让账面比实际更严。
    // §5.58 F29：调色板"转发口"在册账。L14 的针只认 expertColor/expertEmoji/expertGradient 三个名字，
    // 而库里有 5 处写成 `function typeColor(type) { return expertColor(type) }` 的单表达式转发——
    // 实参形态被藏在转发口后面，L14 恒绿而债照样在。本格不钉文案，钉的是"转发口集合＋转发目标＋其非白名单调用点数"。
    const WRAPPER_LEDGER = {
      getEmojiByType: {
        file: 'views/expert/ExpertPlazaView.vue', to: 'expertEmoji', n: 2,
        why: '广场预设行的 type 键本身就是领域名（与调色板同词表），:720 传的 b.expertType 也属同一词表，两处都不必再过一层 expertVisualKey'
      },
      getGradientByType: {
        file: 'views/expert/ExpertPlazaView.vue', to: 'expertGradient', n: 1,
        why: '与 getEmojiByType 成对出现，读同一张预设表的同一个键'
      },
      getTypeColor: {
        file: 'views/expert/panels/ExpertEnterprisePanel.vue', to: 'expertColor', n: 3,
        why: '三处实参分别是图谱节点 node.type（服务端常量戳 "expert"，§5.40）、同一节点赋值的 selectedExpert.type、以及 dispatch 出参 s.expert.type，三者都不是归一化专家的键，收口要等服务侧契约裁决'
      },
      typeColor: {
        file: 'components/expert/RegisterExpertDialog.vue', to: 'expertColor', n: 2,
        why: 'formData.type 是该表单自己的领域名下拉值（键域＝EXPERT_TYPES），构造性正确'
      },
      typeEmoji: {
        file: 'components/expert/RegisterExpertDialog.vue', to: 'expertEmoji', n: 2,
        why: '与 typeColor 成对，出现在预览与提交摘要两处，同一个 formData.type'
      }
    }
    const WRAPPER_ARG_BARE = /^[A-Za-z_$][\w$]*$/
    const WRAPPER_DEF_RES = [
      new RegExp('function\\s+([A-Za-z_$][\\w$]*)\\s*\\([^)]*\\)\\s*\\{[^{}]*?\\breturn\\s+(expertColor|expertEmoji|expertGradient)\\b\\s*\\(([^)]*)\\)\\s*;?\\s*\\}', 'g'),
      new RegExp('(?:const|let)\\s+([A-Za-z_$][\\w$]*)\\s*=\\s*\\(?[^)=]*\\)?\\s*=>\\s*(expertColor|expertEmoji|expertGradient)\\b\\s*\\(([^)]*)\\)', 'g')
    ]
    const wrapperDefs = {}
    for (const rel of paletteSrc) {
      const whole = readFileSync(path.join(SRC_DIR, rel), 'utf8')
      for (const re2 of WRAPPER_DEF_RES) {
        re2.lastIndex = 0
        let m2
        while ((m2 = re2.exec(whole)) !== null) {
          if (!WRAPPER_ARG_BARE.test(m2[3].trim())) continue
          wrapperDefs[m2[1]] = { rel, to: m2[2], line: whole.slice(0, m2.index).split('\n').length }
        }
      }
    }
    const wrapperBad = {}
    for (const name of Object.keys(wrapperDefs)) {
      const def = wrapperDefs[name]
      const rx = new RegExp('\\b' + name + '\\s*\\(([^)]*)\\)', 'g')
      let n2 = 0
      for (const rel of paletteSrc) {
        const lines = readFileSync(path.join(SRC_DIR, rel), 'utf8').split('\n')
        lines.forEach((line, i) => {
          if (rel === def.rel && i + 1 === def.line) return
          let m3
          rx.lastIndex = 0
          while ((m3 = rx.exec(line)) !== null) {
            const arg = m3[1].trim()
            if (arg && !/^(?:expertVisualKey|rowVisualKey|visualKey)\(/.test(arg) && !WRAPPER_ARG_BARE.test(arg)) n2 += 1
          }
        })
      }
      wrapperBad[name] = n2
    }
    console.log('[L14b 转发口] 扫描集 ' + paletteSrc.length + ' 个文件／转发口 ' + Object.keys(wrapperDefs).length
      + ' 个／非白名单调用点合计 ' + Object.values(wrapperBad).reduce((a, b) => a + b, 0)
      + ' 处 实测=' + JSON.stringify(wrapperBad))
    expect(paletteSrc.length, 'L14b 扫描集塌缩（判集为空时台账恒真）实测=' + paletteSrc.length).toBeGreaterThanOrEqual(270)
    expect(Object.keys(wrapperDefs).sort(), 'L14b 出现未登记或已消失的调色板转发口（新增要写进 WRAPPER_LEDGER，收口要删条目）实测='
      + JSON.stringify(Object.keys(wrapperDefs).sort())).toEqual(Object.keys(WRAPPER_LEDGER).sort())
    for (const name of Object.keys(WRAPPER_LEDGER)) {
      const e = WRAPPER_LEDGER[name]
      expect(wrapperDefs[name], 'L14b 台账在册的转发口 ' + name + ' 在磁盘上找不到（收口了就删条目）').toBeTruthy()
      expect(wrapperDefs[name].rel, 'L14b ' + name + ' 所在文件与台账不符 实测=' + wrapperDefs[name].rel).toBe(e.file)
      expect(wrapperDefs[name].to, 'L14b ' + name + ' 转发目标与台账不符 实测=' + wrapperDefs[name].to).toBe(e.to)
      expect(wrapperBad[name], 'L14b ' + name + ' 的非白名单调用点数与台账不符 实测=' + wrapperBad[name]).toBe(e.n)
      expect(e.why.length, 'L14b ' + name + ' 的台账缺理由').toBeGreaterThanOrEqual(8)
    }
    expect(files.length, 'L12 扫描集塌缩（判集为空时台账恒真）实测=' + files.length).toBeGreaterThanOrEqual(30)
    // §5.55 F26：`.ts` 此前挂在"下一族"里属未现量的散文——walk 的 DEF_FILES 本就收 `.ts`，
    // 而 src 实测 0 个（find 两条口径复核）。真正的债不是"没纳 .ts"，是"覆盖面靠人记得"，
    // 所以这里起：目录前缀池里出现名单外 `.ts` 就必须红（按名在册才能继续，散文不用再更新）。
    const prefixPool = walk(SRC_DIR)
      .map((p) => path.relative(SRC_DIR, p).split(path.sep).join('/'))
      .filter((rel) => rel.startsWith(MODULE_DIR + '/views/') || rel.startsWith(MODULE_DIR + '/components/')
        || rel.startsWith('views/expert/'))
    const offRosterTs = prefixPool.filter((rel) => rel.endsWith('.ts') && !JS_IN_SCOPE.includes(rel))
    console.log('[L12 语言] 前缀池 ' + prefixPool.length + ' 个文件（.vue='
      + prefixPool.filter((r) => r.endsWith('.vue')).length + '／.ts='
      + prefixPool.filter((r) => r.endsWith('.ts')).length + '）／扫描集 ' + files.length
      + '（非 .vue=' + files.filter((r) => !r.endsWith('.vue')).length + '）')
    expect(offRosterTs, 'L12 出现名单外 .ts，必须按名进 JS_IN_SCOPE 才算纳管（实测='
      + JSON.stringify(offRosterTs) + '）').toEqual([])
    // §5.59 F30：§5.56 的目录账只覆盖**模块内** 7 个目录；模块外那些"面向联盟但不在值域扫描集里"的目录
    // 此前只靠人记得。现量口径＝池外（既不在三个前缀里，也不是按名在册的两份 .js，也不在模块内）
    // 且文件里有一条 import 的源指向联盟；测试文件不算界面出口。普查驱动 D:/tmp/f30-dir-census.py 未落库＝没牙。
    const outImportRe = /(?:^|[\s;])(?:import|export)\b[^\n]*?from\s*['"]([^'"]+)['"]/g
    const outPoolPrefix = [MODULE_DIR + '/views/', MODULE_DIR + '/components/', 'views/expert/']
    const outHits = new Map()
    for (const abs of walk(SRC_DIR)) {
      const rel = path.relative(SRC_DIR, abs).split(path.sep).join('/')
      if (rel.endsWith('.test.js') || rel.startsWith(MODULE_DIR + '/')) continue
      if (outPoolPrefix.some((p) => rel.startsWith(p)) || JS_IN_SCOPE.includes(rel)) continue
      const txt = readFileSync(abs, 'utf8').replace(/\r\n/g, '\n')
      outImportRe.lastIndex = 0
      if (!([...txt.matchAll(outImportRe)].some((m) => /expert-alliance|alliance/i.test(m[1])))) continue
      const dir = rel.includes('/') ? rel.slice(0, rel.lastIndexOf('/')) : '(src-root)'
      outHits.set(dir, (outHits.get(dir) || 0) + 1)
    }
    const outDirs = [...outHits.keys()].sort()
    // 出账表：目录名＋一句理由。这里**不钉文件数**——往这些目录加文件是日常开发，钉数会把针磨钝；
    // 要钉的是"池外还有哪些面向联盟的目录"这件事本身可见。
    const OUT_DIR_ACCOUNT = {
      'api': { why: '两份在册 .js 的同目录兄弟；此层只做请求体与出参透传，不是界面出口' },
      'composables': { why: 'index.js 只做 re-export；真正读 wire 的 useAllianceTasks.js 已按名在册' },
      'composables/workspace': { why: '直接 import 模块 api/contract，但状态是本地编排演示态（§5.56 逐处读：0 处同域债）' },
      'router': { why: '路由表只挂 path/name 与懒加载组件，不比较 wire 取值' },
      'stores': { why: 'pinia 本地状态机与聊天域词汇，与联盟 wire 取值同串不同域（§5.56）' },
      'views/workspace': { why: '调色板调用走 expertVisualKey 键（§5.57 R2），五型比较实测 0 处同域命中' },
      'views/workspace/panels': { why: '六个面板 import 契约；§5.56 逐处读 CollaborationPanel 的 failed 属聊天消息域' }
    }
    console.log('[L12 模块外目录] 面向联盟的池外目录 ' + outDirs.length + ' 个／非测试文件 '
      + [...outHits.values()].reduce((a, b) => a + b, 0) + ' 份 实测=' + JSON.stringify(outDirs))
    expect(outDirs.length, 'L12 模块外面向联盟的目录数与出账表不符（新增目录没出账，或仪器静默判集为空）实测='
      + JSON.stringify(outDirs)).toBe(7)
    expect(Object.keys(OUT_DIR_ACCOUNT).sort(), 'L12 模块外目录台账与实测集合不符（多＝条目该删，少＝有目录没出账）实测='
      + JSON.stringify(outDirs)).toEqual(outDirs)
    for (const [k, v] of Object.entries(OUT_DIR_ACCOUNT)) {
      expect(v.why.length, k + ' 的出账缺理由（无理由的出账＝把覆盖面洞读成合法）').toBeGreaterThanOrEqual(8)
    }
    // §5.52 形状加宽：只咬 `op '字面量'` 的针会放过"字面量在左／switch-case／includes·indexOf"三种重写形态。
    // 反事实现量（28 个 .vue 文件）：A 9 处（＝下面 EXEMPT 的全部），B/C/D 各 0 处 ⇒ 加宽不新增债也不新增豁免，
    // 只是让"以后有人用别的写法重新打字"落进同一本账。三型各带一枚"必须打红"的电池（见 §5.52）。
    const NEEDLES = [
      ['A 比较符在左', /(?:===|!==|==|!=)\s*['"]([a-z_]+)['"]/g],
      ['B 字面量在左', /['"]([a-z_]+)['"]\s*(?:===|!==|==|!=)(?!\s*['"])/g],
      ['C switch-case', /\bcase\s+['"]([a-z_]+)['"]/g],
      ['D includes/indexOf', /\.(?:includes|indexOf)\(\s*['"]([a-z_]+)['"]/g],
      // §5.54 F25：第五型＝字面量当 Set/数组的元素（动作表、图例、选项表都是"值域的第二本源"）。
      // 反事实普查（D:/tmp/f25-e-shape-census.mjs，未落库＝没牙）：扫描集内 22 处／3 文件，集外跨域 7 处／5 文件。
      ['E 元素表成员', /[\[,]\s*(?:\.\.\.)?['"]([a-z_][a-z0-9_]*)['"]\s*(?:,|\])/g]
    ]
    const hits = []
    const shapeCount = Object.fromEntries(NEEDLES.map(([name]) => [name[0], 0]))
    expect(NEEDLES.length, 'L12 针形集塌缩（覆盖面按形状算，不是按文件数算）实测=' + NEEDLES.length).toBe(5)
    for (const rel of files) {
      for (const line of readFileSync(path.join(SRC_DIR, rel), 'utf8').split('\n')) {
        for (const [name, re] of NEEDLES) {
          for (const m of line.matchAll(re)) {
            if (!banned.has(m[1])) continue
            hits.push(rel + '|' + m[1])
            shapeCount[name[0]]++
          }
        }
      }
    }
    console.log('[L12 形状] 扫描 ' + files.length + ' 个文件（.vue＋在册 .js）／五型命中 A=' + shapeCount.A
      + ' B=' + shapeCount.B + ' C=' + shapeCount.C + ' D=' + shapeCount.D + ' E=' + shapeCount.E)
    const LEDGER = [] // 任务族 5 处（3×completed／paused／running）已在 §5.45 F14c 收口成 TASK_STATUS/NODE_STATUS 常量
    // 跨域同串／本地状态豁免表：n＝该键出现次数（多＝新债，少＝那行已改、条目该删）；why 必填——没理由的豁免不存在
    const EXEMPT = {
      'views/expert/AllianceTaskView.vue|user': { n: 1, why: '本地 AI 助手消息（aiMessages 只在 :412-429 写 user/assistant），与 wire 消息族同串不同域' },
      'views/expert/ExpertConfigView.vue|system': { n: 6, why: '专家来源 type（system＝系统内置 vs 自定义）与 testScene，非 MESSAGE_ROLES 的 system' },
      'views/expert/ExpertConfigView.vue|failed': { n: 1, why: 'llmConnectionStatus 是前端连接自检结果，非任务/节点状态机' },
      'views/expert/ExpertPlazaView.vue|online': { n: 1, why: '快切标签 id（activeQuickTab）与 EXPERT_AVAILABILITY 同串，标签域非 wire 域' },
      // §5.54 F25 第五型新增两处：都是"同串不同域"，不是联盟 wire 取值
      'modules/expert-alliance/components/ExpertRegistryForm.vue|text': { n: 1, why: '表单字段 kind（text/textarea/select/number）是 UI 描述符，与 MSG_TYPES 的 text 同串不同域（:104 过滤的是 f.kind）' },
      'views/expert/ExpertConfigView.vue|code': { n: 1, why: '内置专家配置夹具的 trigger.taskTypes（代码/调试类型名），非任务·节点·会话任一 wire 值域' }
    }
    const expected = [
      ...LEDGER,
      ...Object.entries(EXEMPT).flatMap(([k, v]) => Array.from({ length: v.n }, () => k))
    ].sort()
    hits.sort()
    expect(hits, '视图字面量比较台账与扫描集不符（多＝新债，少＝台账该删）实测=' + JSON.stringify(hits))
      .toEqual(expected)
    for (const [k, v] of Object.entries(EXEMPT)) {
      expect(v.why, k + ' 的豁免必须带走掉理由（无理由的豁免＝把缺陷读成合法）').toBeTruthy()
    }
    expect(Object.keys(EXEMPT).length, '豁免表塌缩实测=' + Object.keys(EXEMPT).length).toBeGreaterThanOrEqual(4)
    expect(countOf(hits.join('\n'), 'components/SessionListPanel.vue|'), '会话族应已清零').toBe(0)
    expect(countOf(hits.join('\n'), 'views/AllianceExpertsView.vue|'), '房间族应已清零').toBe(0)
    // §5.51：模块外八处站点已改比常量，任何一个取值被重新打字都必须红
    for (const k of ['views/expert/AllianceTaskView.vue|completed', 'views/expert/AllianceTaskView.vue|paused',
      'views/expert/panels/ExpertEnterprisePanel.vue|active', 'views/expert/panels/ExpertEnterprisePanel.vue|open',
      'views/expert/panels/ExpertEnterprisePanel.vue|user', 'views/expert/panels/ExpertOrchestratorPanel.vue|completed',
      'views/expert/ExpertPlazaView.vue|pending', 'views/expert/ExpertPlazaView.vue|cancelled']) {
      expect(countOf(hits.join('\n'), k), k + ' 已收口成契约常量，必须保持 0 命中').toBe(0)
    }
    // §5.53：两处 .js 站点已改比常量（TASK_STATUS.COMPLETED／FUSION_STATUS.PENDING），被重新打字就必须红
    // §5.54 F25：三张动作表＋图例＋canCancel＋编排筛选表改成引用常量后，这几个键必须保持 0 命中
    for (const k of ['api/allianceTaskModel.api.js|pending', 'composables/useAllianceTasks.js|completed',
      'composables/useAllianceTasks.js|pending', 'composables/useAllianceTasks.js|running',
      'views/expert/AllianceTaskView.vue|pending', 'views/expert/AllianceTaskView.vue|completed',
      'modules/expert-alliance/views/AllianceConsoleView.vue|pending',
      'modules/expert-alliance/views/AllianceOrchestrationView.vue|completed']) {
      expect(countOf(hits.join('\n'), k), k + ' 已收口成契约常量，必须保持 0 命中').toBe(0)
    }
    expect(banned.has('partial'), '禁串集合须含融合档 partial（本表独有的值）').toBe(true)
    expect(banned.has('available') && banned.has('archived'), '禁串集合须含会话族与房间族取值').toBe(true)
  })

  // L13（§5.47 F16）熔断器档与熔断器行键的**服务侧真值账**：期望从 Rust 出参现推，不写死档位。
  // 现场证据：`experts_dispatcher.rs:498-502` 的 `state_str` 只会是 "open"/"closed"（`half_open` 只是
  // scheduler-core 的配置项 `half_open_probes`，从不进这个端点）⇒ 契约两档是完备的，不是缺档。
  // 若后端加第三档，这本账会先红并说清"补 BREAKER_STATE 与中文文案"，而不是让界面原样回显英文。
  it('L13 熔断器档位与行键实时解析 Rust 出参：值域⇄契约、snake⇄camel 双向相等', () => {
    const RS = path.resolve(SRC_DIR, '../../platform/gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs')
    const text = readFileSync(RS, 'utf8').replace(/\r\n/g, '\n')
    const assign = text.match(/let\s+state_str\s*=\s*if[\s\S]{0,200}?;/)
    expect(assign, '在 experts_dispatcher.rs 里没找到 state_str 赋值（服务侧换了写法 ⇒ 这本账要跟着改，不许静默恒真）').not.toBeNull()
    const wireStates = [...new Set([...assign[0].matchAll(/"([a-z_]+)"/g)].map((m) => m[1]))].sort()
    expect(wireStates.length, 'wire 熔断器档塌缩（少于两档时相等判断失去意义）实测=' + JSON.stringify(wireStates))
      .toBeGreaterThanOrEqual(2)
    const contractStates = [...new Set(Object.values(BREAKER_STATE))].sort()
    expect(contractStates, '契约熔断器档与 Rust 出参不符（Rust 加档 ⇒ 界面原样回显英文 ⇒ 要补 BREAKER_STATE 与文案）实测 wire='
      + JSON.stringify(wireStates)).toEqual(wireStates)
    for (const s of wireStates) {
      const label = BREAKER_STATE_LABELS[s]
      expect(label, s + ' 档没有中文文案').toBeTruthy()
      expect(label === s, s + ' 档文案等于键名＝没归一').toBe(false)
    }
    const row = text.match(/json!\(\{\s*"expert_id"[\s\S]{0,200}?\}\)/)
    expect(row, '没找到熔断器行的 json! 出参块').not.toBeNull()
    const wireKeys = [...new Set([...row[0].matchAll(/"([a-z_]+)":/g)].map((m) => m[1]))]
      .map((k) => k.replace(/_([a-z])/g, (_, c) => c.toUpperCase())).sort()
    const norm = readFileSync(path.join(SRC_DIR, MODULE_DIR, 'model/normalize.js'), 'utf8').replace(/\r\n/g, '\n')
    const mapped = norm.match(/circuitBreakers:[\s\S]{0,400}?\}\)\)/)
    expect(mapped, 'normalize.js 里找不到 circuitBreakers 行映射（归一化换了写法 ⇒ 这本账要跟着改）').not.toBeNull()
    const camelKeys = [...new Set([...mapped[0].matchAll(/^\s*(\w+):\s/gm)].map((m) => m[1]))]
      .filter((k) => k !== 'circuitBreakers').sort()
    expect(camelKeys, '熔断器行的归一化键与 wire 键不是一一对应（漏一个界面就显示空/undefined）实测 wire='
      + JSON.stringify(wireKeys)).toEqual(wireKeys)
  })
})
