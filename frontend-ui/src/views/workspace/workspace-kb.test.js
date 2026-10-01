// 知识库读路径的守卫：四种 payload 形状 × 一份 wire 真相词表 × 一本"不许再读后端没有的键"的账。
//
// 为什么这一本账值得单独开：本轮之前工作台右侧的知识库三个列表**恒空**——
// `http.js` 的拦截器已经把信封剥掉一层（93-97 行 `if ('data' in body) return body.data`），
// 视图却又去找 `res.data`，于是 `Array.isArray(res.data)` 恒假 ⇒ documents/docVersions/检索结果全落到 `[]`。
// 分类与标签侥幸能用，只因为那两个端点返回裸数组。
// 另一半病灶是**词表**：前端拿 article/tutorial/api… 与 published/draft/archived 去比，
// 而后端 `mox-kb-svc` 只有 cat-* 四档分类（document.rs:28-33）与 draft/analyzed/linked 三种状态（model.rs:14-16），
// 两边零重叠 ⇒ 按"类型"筛选恒空、状态标签永远走兜底显形。
//
// 所以这里三层各钉各的：形状（unwrap/unwrapList）、词表（对着 Rust 源解析）、接线（棘轮台账）。
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { readFileSync, existsSync } from 'fs'
import path from 'path'
import { unwrap, unwrapList } from '@/modules/_kernel/envelope'
import {
  KB_CATEGORIES,
  KB_STATUSES,
  getCategoryLabel,
  getCategoryTagType,
  getCategoryIcon,
  getStatusLabel,
  getStatusType,
  isAiAnalyzed,
  mapDoc
} from '@/utils'
import KnowledgeBasePanel from './panels/KnowledgeBasePanel.vue'

const MINUTE = 60 * 1000
const isoAt = (offsetMs) => new Date(Date.now() - offsetMs).toISOString()
const REPO = path.resolve(__dirname, '..', '..', '..', '..')
const CRATE = path.join(REPO, 'platform', 'domains', 'kg', 'svc', 'mox-kb-svc', 'src')

/** 后端真相：/kb/documents 的 items 恰好五个键（document.rs:123-141 的 json!） */
function wireSummary(overrides = {}) {
  return {
    id: 'kb-1',
    title: '接口设计说明',
    category: 'cat-tech',
    status: 'analyzed',
    updated_at: isoAt(5 * MINUTE),
    ...overrides
  }
}

// ---------- 一、四种 payload 形状 ----------

describe('KB 端点形状：信封只剥一层，列表键各自点名', () => {
  const rows = [wireSummary()]

  it('/kb/documents → {items,total}：裸 body 与整封两种输入都给出行', () => {
    expect(unwrapList(unwrap({ items: rows, total: 1 }), 'items')).toEqual(rows)
    expect(unwrapList(unwrap({ code: 0, msg: 'ok', data: { items: rows, total: 1 } }), 'items')).toEqual(rows)
  })

  it('/kb/categories 与 /kb/tags 是裸数组，不套壳也读得出', () => {
    const cats = [{ id: 'cat-tech', name: '技术文档', count: 3 }]
    expect(unwrapList(unwrap(cats))).toEqual(cats)
    const tags = [{ name: '灰度', count: 2 }]
    expect(unwrapList(unwrap({ code: 0, msg: 'ok', data: tags }))).toEqual(tags)
  })

  it('/kb/documents/:id/versions → {doc_id,versions}：列表键是 versions', () => {
    const v = [{ version: 'v2', title: '甲', content: '', note: '当前版本', created_at: isoAt(MINUTE) }]
    expect(unwrapList(unwrap({ doc_id: 'kb-1', versions: v }), 'versions')).toEqual(v)
  })

  it('/kb/search → {results,graph_hits,total}：列表键是 results', () => {
    const hit = { id: 'kb-2', title: '灰度方案', category: 'cat-research', snippet: '…', score: 0.87, tags: [] }
    expect(unwrapList(unwrap({ results: [hit], graph_hits: [], total: 1 }), 'results')).toEqual([hit])
  })

  it('code!==0 一律抛 ApiError，不许把错误体当空列表咽下去', () => {
    expect(() => unwrap({ code: 500, msg: 'boom', data: null })).toThrowError(/boom/)
  })

  it('反对照：旧的"猜 res.data"写法在这三种 payload 上恒为空（病灶是真的，不是我在描述幻觉）', () => {
    const legacy = (res) => (res && Array.isArray(res.data) ? res.data : Array.isArray(res) ? res : [])
    expect(legacy({ items: rows, total: 1 })).toEqual([])
    expect(legacy({ doc_id: 'kb-1', versions: rows })).toEqual([])
    expect(legacy({ results: rows, graph_hits: [], total: 1 })).toEqual([])
  })
})

// ---------- 二、词表与 Rust 源同源 ----------

function rustCategories() {
  const text = readFileSync(path.join(CRATE, 'document.rs'), 'utf-8')
  const block = /pub const CATEGORIES: &\[\(&str, &str\)\] = &\[(.*?)\];/s.exec(text)
  expect(block, 'Rust 源里的 CATEGORIES 形状变了，台账得跟着改').toBeTruthy()
  return [...block[1].matchAll(/\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)/g)].map((m) => ({ value: m[1], label: m[2] }))
}

function rustStatuses() {
  const text = readFileSync(path.join(CRATE, 'model.rs'), 'utf-8')
  return [...text.matchAll(/pub const STATUS_[A-Z_]+: &str = "([^"]+)"/g)].map((m) => m[1])
}

describe('分类/状态词表：逐对钉在后端源上，两边不再各写一套', () => {
  it('Rust 侧确实解析得出四档分类与三种状态（分母本身要断言）', () => {
    expect(rustCategories().length).toBe(4)
    expect(rustStatuses().sort()).toEqual(['analyzed', 'draft', 'linked'])
  })

  it('KB_CATEGORIES 的 id 与文案与 Rust CATEGORIES 逐项同序', () => {
    expect(KB_CATEGORIES.map((c) => c.value)).toEqual(rustCategories().map((c) => c.value))
    expect(KB_CATEGORIES.map((c) => c.label)).toEqual(rustCategories().map((c) => c.label))
    for (const c of KB_CATEGORIES) {
      expect(c.icon, c.value + ' 缺图标').toBeTruthy()
      expect(['info', 'success', 'warning', 'danger', 'primary']).toContain(c.tagType)
    }
  })

  it('KB_STATUSES 覆盖 Rust 的三种状态，且各自的配色不塌成一档', () => {
    expect(KB_STATUSES.map((s) => s.value).sort()).toEqual(rustStatuses().sort())
    expect(new Set(KB_STATUSES.map((s) => s.tagType)).size).toBe(KB_STATUSES.length)
    for (const s of KB_STATUSES) expect(getStatusLabel(s.value)).toBe(s.label)
  })

  it('词表外的值原样显形，不许显示空白把新增档吞掉', () => {
    expect(getCategoryLabel('cat-future')).toBe('cat-future')
    expect(getStatusLabel('future_state')).toBe('future_state')
    expect(getCategoryTagType('cat-future')).toBe('info')
    expect(getCategoryIcon('cat-future')).toBe('📄')
    expect(getStatusType('future_state')).toBe('info')
  })

  it('analyzed 与 linked 都算"AI 分析过"，draft 不算', () => {
    expect(isAiAnalyzed('analyzed')).toBe(true)
    expect(isAiAnalyzed('linked')).toBe(true)
    expect(isAiAnalyzed('draft')).toBe(false)
    expect(isAiAnalyzed(undefined)).toBe(false)
  })
})

// ---------- 三、mapDoc 只从真实键推导 ----------

describe('mapDoc：版本数与"分析过"从 wire 上真有的键算出来', () => {
  const fullDoc = {
    id: 'kb-9',
    title: '甲',
    content: 'x',
    category: 'cat-business',
    tags: ['a'],
    status: 'linked',
    summary: '摘要',
    entities: [],
    relations: [],
    current_version: 'v3',
    versions: [{ version: 'v1' }, { version: 'v2' }],
    created_at: isoAt(MINUTE),
    updated_at: isoAt(MINUTE)
  }

  it('历史快照 2 条 + 当前版 1 条 = 3 个版本（d.version 这个键后端没有）', () => {
    expect(mapDoc(fullDoc).version_count).toBe(3)
  })

  it('status 说了算：linked 算分析过（d.aiAnalysis 这个键后端没有）', () => {
    expect(mapDoc(fullDoc).ai_analyzed).toBe(true)
    expect(mapDoc({ ...fullDoc, status: 'draft' }).ai_analyzed).toBe(false)
    expect(mapDoc({ ...fullDoc, versions: [] }).version_count).toBe(1)
  })

  it('列表行（DocSummary 五键）映射后原键一个不少', () => {
    // 夹具只铸一次：wireSummary() 里的 updated_at 读 Date.now()，两次调用跨毫秒就会漂移，
    // 这条断言会变成随机红（实测被 M80 那一轮撞出一次）
    const base = wireSummary()
    const row = mapDoc(base)
    for (const k of ['id', 'title', 'category', 'status', 'updated_at']) {
      expect(row[k], k).toBe(base[k])
    }
    expect(row.version_count).toBe(1)
  })
})

// ---------- 四、面板渲染只认后端真有的键 ----------

describe('工作台知识库面板：列表行与版本行的渲染', () => {
  const mountPanel = (props = {}) =>
    mount(KnowledgeBasePanel, {
      props: { collapsed: false, activeKbTab: 'docs', documents: [wireSummary()], ...props }
    })

  it('图标与分类文案来自词表，不是 doc.type', () => {
    const w = mountPanel()
    const row = w.findAll('.ws-doc-item')[0]
    expect(row.find('.ws-doc-icon').text()).toBe('💻')
    expect(row.find('.ws-doc-name').text()).toBe('接口设计说明')
    expect(row.find('.ws-doc-meta').text()).toContain('技术文档')
  })

  it('词表外的分类在行里也原样显形，不是空白一格', () => {
    // 与纯函数那一格各自成证：只改 getCategoryLabel 的兜底会两格都红，只在视图里换成硬编码映射则只有这一格红
    const w = mountPanel({ documents: [wireSummary({ category: 'cat-future' })] })
    expect(w.find('.ws-doc-meta').text()).toContain('cat-future')
    expect(w.find('.ws-doc-meta').text()).not.toMatch(/^\s*·/)
  })

  it('时间走相对档：5 分钟前要真的说得出口（旧副本对 RFC3339 恒 NaN）', () => {
    expect(mountPanel().find('.ws-doc-meta').text()).toContain('5 分钟前')
  })

  it('状态标签按 wire 词表出文案，draft 不给标签占位', () => {
    expect(mountPanel().find('.ws-doc-item').text()).toContain('已分析')
    expect(mountPanel({ documents: [wireSummary({ status: 'draft' })] }).find('.ws-doc-item').text()).not.toContain('草稿')
  })

  it('按分类筛选认 category：读不存在的 category_id 会把两档都筛成空', () => {
    const docs = [wireSummary(), wireSummary({ id: 'kb-2', title: '乙', category: 'cat-business' })]
    expect(mountPanel({ documents: docs, activeCategory: 'cat-tech' }).findAll('.ws-doc-item')).toHaveLength(1)
    expect(mountPanel({ documents: docs, activeCategory: 'cat-business' }).findAll('.ws-doc-item')[0].text()).toContain('乙')
    expect(mountPanel({ documents: docs }).findAll('.ws-doc-item')).toHaveLength(2)
  })

  it('检索行没有时间就显相关度，不给"未知 · -"这种编出来的占位', () => {
    const hit = { id: 'kb-3', title: '灰度方案', category: 'cat-research', snippet: '…', score: 0.87, tags: [] }
    const meta = mountPanel({ documents: [hit] }).find('.ws-doc-meta').text()
    expect(meta).toContain('研究文档')
    expect(meta).toContain('相关度 0.87')
    expect(meta).not.toContain('未知')
  })

  it('版本行只认 KbVersion 的五键：note 显出来，v 前缀不重复', () => {
    const w = mountPanel({
      activeKbTab: 'versions',
      docVersions: [
        { version: 'v2', title: '甲', content: '', note: '当前版本', created_at: isoAt(MINUTE) },
        { version: 'v1', title: '甲（旧）', content: '', note: '初稿', created_at: isoAt(3 * 24 * 60 * MINUTE) }
      ]
    })
    const rows = w.findAll('.ws-version-item')
    expect(rows).toHaveLength(2)
    expect(rows[0].find('.ws-version-label').text()).toBe('v2')
    expect(rows[0].find('.ws-version-badge').text()).toBe('当前版本')
    expect(rows[1].find('.ws-version-author').text()).toContain('初稿')
    expect(rows[1].find('.ws-version-author').text()).not.toContain('系统 · 更新')
  })

  it('徽标通道已撤：多余的后端没有的键不会渲染出任何东西', () => {
    const w = mountPanel({ documents: [wireSummary({ graph_linked: true, size: 4096, name: '幽灵' })] })
    expect(w.find('.ws-doc-item').html()).not.toContain('ws-doc-badge')
    expect(w.find('.ws-doc-item').text()).toContain('接口设计说明')
  })
})

// ---------- 五、棘轮台账：幻影键与私有词表副本不许回来 ----------

const FILES = [
  'views/workspace/ExpertWorkspaceView.vue',
  'views/workspace/panels/KnowledgeBasePanel.vue',
  'views/project/panels/KnowledgeBasePanel.vue',
  'composables/useKnowledgeBase.js',
  'utils/knowledgeBase.utils.js',
  'utils/index.js'
]

const BANNED = [
  { re: /\.category_id\b/, why: '分类键是 category，后端没有 category_id', sample: 'list.filter(d => d.category_id === props.activeCategory)' },
  { re: /\.graph_linked\b/, why: 'DocSummary 只有五个键，是否关联图谱要看 status', sample: ':class="{ linked: doc.graph_linked }"' },
  { re: /\bdoc(tument)?\.size\b|formatFileSize/, why: 'wire 上没有字节数，别印"未知"占位', sample: '{{ formatFileSize(doc.size) }}' },
  { re: /\bver\.(author|action)\b/, why: 'KbVersion 是 version/title/content/note/created_at 五键', sample: "{{ ver.author || '系统' }}" },
  { re: /\bgetTypeLabel\b|\bgetTagType\s*\(/, why: '"文档类型"这套词表后端不存在，分类才是 cat-*', sample: '{{ getTypeLabel(doc.type) }}' },
  { re: /\bfilterType\b|\bdocTypes\b/, why: '按不存在的 type 筛选的那条死通道已经删掉', sample: "const filterType = ref('')" },
  { re: /\b(\w*[dD]oc|document|d)\??\.type\b/, why: '后端不发 type，doc 形对象上读 .type 恒为假（selectedDoc?.type 也算）', sample: 'if (filterType.value) result = result.filter(d => d.type === v)' },
  { re: /\bdoc(t)?\.description\b/, why: '摘要是 summary，文档对象没有 description', sample: "description: doc.description || ''," },
  { re: /Array\.isArray\(\s*\w+\s*&&?\s*\w*\.?data\s*\)|Array\.isArray\(\s*\w+\.data\s*\)/, why: '信封已被 http.js 剥过一层，再找 .data 恒假', sample: 'if (res && Array.isArray(res.data)) documents.value = res.data' },
  { re: /data\?\.documents/, why: '这个兜底键后端从不给', sample: "const list = Array.isArray(data) ? data : (data?.items || data?.documents || [])" },
  { re: /!!d\.aiAnalysis|\bd\.version \|\|/, why: 'mapDoc 的两个幻影来源，改成 status 与 versions 推导', sample: 'return { ...d, version_count: d.version || 1, ai_analyzed: !!d.aiAnalysis }' },
  { re: /\bt\.tag\b/, why: '/kb/tags 给的是 name，没有 tag 别名', sample: "name: t.name || t.tag" },
  // ↓ 写路径那一轮补的五枚（请求体错名 / 快照行错键 / 置信度幻影 / v 前缀双写）
  { re: /\bh\.(action|user|detail)\b/, why: '/kb/documents/:id/history 的行是版本快照，只有 version/note/title/created_at', sample: '{{ getActionLabel(h.action) }}' },
  { re: /\.confidence\b/, why: 'KbEntity 只有 id/name/type/frequency/snippet，没有置信度', sample: ':percentage="ent.confidence"' },
  { re: /\b(linked_entities|doc_ids|analyzed_ids|target_version|version_from|version_to|entity_ids)\b/, why: '这批请求/响应键后端一概不收，挂图只有文档级', sample: 'await api.kbBatchAnalyze({ doc_ids: selectedDocs.value })' },
  { re: /\?\.(aiAnalysis|from|to)\b/, why: '响应里没有 aiAnalysis / from / to 三个键', sample: 'compareFrom.value = data?.from || v1' },
  { re: /v\{\{\s*[\w$]+\??\.version\s*\}\}/, why: '版本串自带 v 前缀（next_version 产出 v{n+1}），模板不许再补一次', sample: '<div class="version-badge">v{{ ver.version }}</div>' },
  { re: /kbSearch\(\{\s*q\b/, why: 'SearchRequest 的必填键是 query，发 q 会被 axum 的 Json 提取器拒成 422（且响应不是 {code,msg} 信封）', sample: "api.kbSearch({ q: linkSearchQuery.value, type: 'entity' })" },
  { re: /Array\.isArray\(\s*\w+\s*\)\s*\?/, why: '猜形状三元链：信封已被 http.js 剥过一层，列表键要由 unwrapList 点名', sample: "docVersions.value = Array.isArray(data) ? data : (data?.versions || [])" }
]

const WIRING = {
  'views/workspace/ExpertWorkspaceView.vue': [
    "unwrapList(unwrap(res), 'items')",
    "unwrapList(unwrap(await kbGetVersions(docId)), 'versions')",
    "unwrapList(unwrap(await kbSearch({ query: kbSearchQuery.value, limit: 50 })), 'results')",
    "const list = unwrapList(unwrap(data), 'items')",
    "from '@/modules/_kernel/envelope'"
  ],
  'views/workspace/panels/KnowledgeBasePanel.vue': [
    'getCategoryIcon(doc.category)',
    'd.category === props.activeCategory',
    "import { getCategoryIcon, getCategoryLabel, getStatusType, getStatusLabel, isAiAnalyzed } from '@/utils'"
  ],
  'views/project/panels/KnowledgeBasePanel.vue': [
    "unwrapList(unwrap(data), 'items')",
    'getCategoryTagType(doc.category)',
    'v-for="s in KB_STATUSES"',
    // 写路径：建版走 {note}，响应用 result.document，挂图不收请求体，分类树绑 cat-* id
    'mapDoc(result?.document)',
    'await api.kbSaveDocumentEdit(data)',
    'current_version: doc.current_version',
    '{ ids: selectedDocs.value }',
    '{ v1: fromVer.version, v2: toVer.version }',
    '{ version: version.version }',
    "unwrapList(unwrap(data), 'versions')",
    "unwrapList(unwrap(data), 'history')",
    'api.kbGraphLink(docId)',
    'api.kbGraphUnlink(docId)',
    'result?.analyzed',
    'data?.graph_nodes',
    'node-key="id"',
    "value: 'id'"
  ],
  'composables/useKnowledgeBase.js': [
    "unwrapList(unwrap(data), 'items')",
    "docVersions.value = unwrapList(unwrap(data), 'versions')",
    'docHistory.value = unwrapList(unwrap(data))',
    'searchResults.value = unwrapList(unwrap(data))',
    'categories: KB_CATEGORIES'
  ],
  'utils/knowledgeBase.utils.js': ['export const KB_CATEGORIES', 'export const KB_STATUSES', 'export function isAiAnalyzed'],
  'utils/index.js': ['KB_CATEGORIES', 'getCategoryIcon', 'isAiAnalyzed']
}

function codeOnly(text) {
  return text
    .split('\n')
    .filter((line) => !/^\s*(\/\/|\*|\/\*|<!--)/.test(line))
    .join('\n')
}

function read(rel) {
  const p = path.join(REPO, 'frontend-ui', 'src', ...rel.split('/'))
  expect(existsSync(p), `${rel} 不在了，这一格会空转`).toBe(true)
  return codeOnly(readFileSync(p, 'utf-8'))
}

describe('KB 读路径棘轮（新增幻影键即红，清零不许靠删接线）', () => {
  it('扫描集非空：六个文件都在，且都真读到内容', () => {
    let chars = 0
    for (const rel of FILES) chars += read(rel).length
    expect(chars).toBeGreaterThan(20000)
  })

  it('后端没有的键与私有词表副本，一处都不许再被读', () => {
    const hits = []
    for (const rel of FILES) {
      const text = read(rel)
      text.split('\n').forEach((line, i) => {
        for (const { re, why } of BANNED) {
          if (re.test(line)) hits.push(`${rel}:${i + 1} ${why} → ${line.trim()}`)
        }
      })
    }
    expect(hits, `KB 读路径又在猜字段：${hits.join(' ;; ')}`).toEqual([])
  })

  it('正对照：每一枚禁令都能点名它要拦的那一行（台账不是空转）', () => {
    for (const { re, why, sample } of BANNED) {
      expect(re.test(sample), `禁令失效：${why}`).toBe(true)
    }
  })

  it('反对照：本轮的合法写法不得被误伤', () => {
    const legal = [
      "documents.value = unwrapList(unwrap(res), 'items')",
      "list = list.filter(d => d.category === props.activeCategory)",
      '{{ getCategoryLabel(doc.category) }} · {{ docMetaText(doc) }}',
      "<div class=\"ws-version-author\">{{ ver.title }} · {{ ver.note || '（无备注）' }}</div>",
      "export const KB_STATUSES = [",
      "  { value: 'cat-tech', label: '技术文档', tagType: 'info', icon: '💻' },",
      // 同族不同主：这三处的 .type 后端真给（通知配色 / KbEntity.type / 写侧入参），禁令不许把它们一起吃下
      ":type=\"notif.type || 'info'\"",
      '{{ ent.type }}',
      'type: data.type,'
    ]
    // 同族不同主：写路径归一后仍在用的合法写法，禁令表一枚都不许碰
    const legalWrite = [
      '{{ ver.version }}',
      ':key="ver.version"',
      '<span>v{{ doc.version_count || 1 }}</span>',
      "await api.kbCreateVersion(data.id, { note })",
      '{{ ent.frequency ?? 0 }}',
      '<div class="history-detail" v-if="h.note">{{ h.note }}</div>',
      "const status = result?.status || 'analyzed'",
      "api.kbSearch({ query: kbSearchQuery.value, limit: 50 })",
      // 禁令 #19（猜形状三元）的反对照：裸数组端点的 if 形式、真键的可选链兜底都不算猜形状
      "      if (Array.isArray(data)) {",
      "graphNodes.value = data?.graph_nodes || []",
      "entities.value = data?.entities || []",
      "docHistory.value = unwrapList(unwrap(data))"
    ]
    for (const line of [...legal, ...legalWrite]) {
      for (const { re, why } of BANNED) {
        expect(re.test(line), `误伤合法行（${why}）：${line}`).toBe(false)
      }
    }
  })

  it('归一后的接线确实在（防止靠删代码通过上一格）', () => {
    for (const [rel, needles] of Object.entries(WIRING)) {
      const text = read(rel)
      for (const needle of needles) {
        expect(text, `${rel} 里找不到 ${needle}`).toContain(needle)
      }
    }
  })

  it('猜形状的旧写法没有以别的名义回来：信封解包只在内核一处', () => {
    for (const rel of FILES) {
      const text = read(rel)
      if (rel.endsWith('envelope.js')) continue
      expect(text, `${rel} 自己写了解包逻辑`).not.toMatch(/'data' in \w+/)
    }
  })
})

// ---------- 六、写路径：请求体与响应体的键名从 Rust 源里现读 ----------

const PANEL = 'views/project/panels/KnowledgeBasePanel.vue'

/** 读 handlers.rs 里某张请求体的字段名（属性行以 # 开头，天然不会被 `^\s*(\w+):` 收进） */
function rustReqFields(name) {
  const text = readFileSync(path.join(CRATE, 'handlers.rs'), 'utf-8')
  const block = new RegExp(`struct ${name} \\{([\\s\\S]*?)\\n\\}`).exec(text)
  expect(block, `Rust 源里的 ${name} 形状变了，台账得跟着改`).toBeTruthy()
  return [...block[1].matchAll(/^\s*(\w+):\s/gm)].map((m) => m[1])
}

/** /kb/stats 真正发出的计数键：document.rs 的 stats() 四个 + handlers.rs 的 kb_stats 补两个图计数 */
function rustStatsKeys() {
  const doc = readFileSync(path.join(CRATE, 'document.rs'), 'utf-8')
  const block = /pub async fn stats\([\s\S]*?json!\(\{([\s\S]*?)\}\)/.exec(doc)
  expect(block, 'Rust 侧的 stats() 形状变了，台账得跟着改').toBeTruthy()
  const keys = [...block[1].matchAll(/"(\w+)":/g)].map((m) => m[1])
  const h = readFileSync(path.join(CRATE, 'handlers.rs'), 'utf-8')
  const fn = /async fn kb_stats\(([\s\S]*?)\n\}/.exec(h)
  expect(fn, 'handlers.rs 的 kb_stats 不在了，台账得跟着改').toBeTruthy()
  return keys.concat([...fn[1].matchAll(/\.insert\("(\w+)"\.into\(\)/g)].map((m) => m[1]))
}

describe('KB 写路径：面板发出的请求体只含 Rust 认的键', () => {
  const panel = read(PANEL)

  it('Rust 侧五张请求体的字段清单（分母本身要断言）', () => {
    expect(rustReqFields('CreateDocReq')).toEqual(['title', 'content', 'category', 'tags'])
    expect(rustReqFields('BatchAnalyzeReq')).toEqual(['ids'])
    expect(rustReqFields('VersionNoteReq')).toEqual(['note'])
    expect(rustReqFields('CompareReq')).toEqual(['v1', 'v2'])
    expect(rustReqFields('RevertReq')).toEqual(['version'])
  })

  it('创建 payload 为四键；编辑使用单次条件更新接口', () => {
    const blocks = [...panel.matchAll(/const payload = \{([\s\S]*?)\n\s*\}/g)]
    // 创建仍为四键；编辑 payload 已移到 API 层，由功能测试校验六键。
    expect(blocks).toHaveLength(1)
    expect(panel).toContain("await api.kbSaveDocumentEdit(data)")
    expect(panel).not.toContain("await api.kbCreateVersion(data.id, { note })")
    expect(panel).toContain("const full = await api.kbGetDocument(doc.id)")
    for (const m of blocks) {
      const keys = [...m[1].matchAll(/^\s*(\w+):/gm)].map((x) => x[1]).sort()
      expect(keys, m[0]).toEqual(['category', 'content', 'tags', 'title'])
    }
  })

  it('响应都从 document/analyzed/graph_nodes 这些真键取，字符串里也不补 v 前缀', () => {
    // 建/存两处各要一枚：只 toContain 的话，改坏其中一处会由另一处替它过关
    expect([...panel.matchAll(/mapDoc\(result\?\.document\)/g)]).toHaveLength(2)
    expect(panel).not.toMatch(/mapDoc\(result\)/)
    expect(panel).toContain('result?.analyzed')
    expect(panel).toContain('data?.graph_nodes')
    expect(panel).not.toMatch(/v\$\{/)
  })

  it('分类选择器绑 cat-* id：绑中文名会把中文落库、分类计数恒 0', () => {
    expect(panel).toContain("value: 'id'")
    expect(panel).toContain('filterCategory.value = node?.id')
    expect(panel).not.toMatch(/value:\s*'name'/)
  })

  it('统计卡只挂 /kb/stats 真发的计数键（total/versions/analyzed 都是编出来的）', () => {
    const emitted = rustStatsKeys().sort()
    expect(emitted).toEqual(['categories', 'documents', 'graph_edges', 'graph_nodes', 'storage_bytes', 'tags'])
    const used = [...panel.matchAll(/stats\.value\.(\w+)/g)].map((m) => m[1])
    // 用了 0 个键时这格会恒真，所以先把"卡片确实读了 stats"钉住
    expect(used.length).toBeGreaterThan(0)
    for (const k of used) expect(emitted, `统计卡读了后端不发的 ${k}`).toContain(k)
  })
})
