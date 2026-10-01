// 会话 store 单元测试：四区错误隔离、"打开即补详情"、写操作后并线而非重取、
// 合并式更新只发差分量、以及后端不回的东西必须由 store 自己记账。
// wire 名与出参面由 contract/sessions.test.js 守住，这里只验状态机。
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    listSessions: vi.fn(), sessionStats: vi.fn(), getSession: vi.fn(), createSession: vi.fn(),
    appendSessionMessage: vi.fn(), updateSession: vi.fn(), deleteSession: vi.fn(), archiveSession: vi.fn(),
    exportSession: vi.fn(), sessionSimilarSearch: vi.fn(), semanticSearch: vi.fn()
  }
}))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceSessionsStore } = await import('./alliance-sessions.store.js')

const fail = (msg) => Object.assign(new Error(msg), { name: 'ApiError', msg })

const sessionFixture = (over = {}) => ({
  id: 'sess-1',
  title: '架构评审',
  expertIds: ['e1'],
  userId: '',
  sessionType: 'single',
  status: 'active',
  topic: '如何拆库',
  messageCount: 1,
  tags: ['rust'],
  metadata: [{ key: 'priority', value: 'high' }, { key: 'score', value: 3 }],
  messages: [{ id: 'msg-1', role: 'user', content: '原文', createdAt: '2026-09-01T10:00:00Z' }],
  createdAt: '2026-09-01T10:00:00Z',
  lastActiveAt: '2026-09-01T10:00:00Z',
  archivedAt: '',
  archived: false,
  ...over
})

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  api.listSessions.mockResolvedValue({ items: [sessionFixture()], total: 1, page: 1, pageSize: 20 })
  api.sessionStats.mockResolvedValue({ totalSessions: 1, activeSessions: 1, archivedSessions: 0, closedSessions: 0 })
  api.getSession.mockResolvedValue(sessionFixture())
})

describe('列表与统计各自记账', () => {
  it('过滤条件整份交给 api，页码由后端回传值呈现', async () => {
    const store = useAllianceSessionsStore()
    store.setFilter('status', 'active')
    await store.loadList()
    expect(api.listSessions).toHaveBeenCalledWith(expect.objectContaining({ status: 'active', page: 1, pageSize: 20 }))
    expect(store.list.items[0].id).toBe('sess-1')
    expect(store.totalPages).toBe(1)
    expect(store.error.list).toBe('')
  })

  it('改过滤条件回到第 1 页，翻页本身不算改过滤', async () => {
    const store = useAllianceSessionsStore()
    store.setFilter('page', 4)
    expect(store.filters.page).toBe(4)
    store.setFilter('search', '甲')
    expect(store.filters.page).toBe(1)
    expect(store.filters.search).toBe('甲')
  })

  it('列表失败只清列表，统计与已打开的详情原地不动', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.sessionStats.mockRejectedValue(fail('统计不可用'))
    api.listSessions.mockRejectedValue(fail('列表不可用'))
    await store.loadList()
    await store.loadStats()
    expect(store.error.list).toBe('列表不可用')
    expect(store.error.stats).toBe('统计不可用')
    expect(store.list.items).toEqual([])
    expect(store.detail.id).toBe('sess-1')
    expect(store.loading.list).toBe(false)
    expect(store.loading.stats).toBe(false)
  })

  it('refreshAll 并发取两份，互不阻塞', async () => {
    const store = useAllianceSessionsStore()
    await store.refreshAll()
    expect(api.listSessions).toHaveBeenCalledTimes(1)
    expect(api.sessionStats).toHaveBeenCalledTimes(1)
  })
})

describe('打开会话', () => {
  it('取列表不会自动打开会话，选中才补一次详情请求', async () => {
    const store = useAllianceSessionsStore()
    await store.loadList()
    expect(api.getSession).not.toHaveBeenCalled()
    await store.selectSession('sess-1')
    expect(api.getSession).toHaveBeenCalledWith('sess-1')
    expect(store.messages.length).toBe(1)
    expect(store.editDraft.title).toBe('架构评审')
    expect(store.editDraft.tags).toEqual(['rust'])
  })

  it('非字符串的 metadata 值以 JSON 文本进表单行，读回来仍是原样字符串', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    expect(store.metaRows).toEqual([{ key: 'priority', value: 'high' }, { key: 'score', value: '3' }])
    expect(store.editStatusProblem).toBe('')
  })

  it('404 session not found 会清掉选中态，其他错误则保留', async () => {
    const store = useAllianceSessionsStore()
    api.getSession.mockRejectedValue(fail('session not found: ghost'))
    await store.selectSession('ghost')
    expect(store.selectedId).toBe('')
    expect(store.detail).toBe(null)
    expect(store.error.detail).toBe('session not found: ghost')

    api.getSession.mockRejectedValue(fail('网关重启中'))
    await store.selectSession('sess-1')
    expect(store.selectedId).toBe('sess-1')
    expect(store.error.detail).toBe('网关重启中')
  })

  it('切换会话时上一条的相似检索与导出结果必须作废', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.sessionSimilarSearch.mockResolvedValue({ results: [], totalFound: 0 })
    api.exportSession.mockResolvedValue({ sessionId: 'sess-1', downloadUrl: null, messageCount: 1, content: {} })
    store.similarDraft.query = '甲'
    await store.runSimilarSearch()
    await store.exportSession()
    expect(store.similar).not.toBe(null)
    await store.selectSession('sess-2')
    expect(store.similar).toBe(null)
    expect(store.exportResult).toBe(null)
    expect(store.similarDraft.query).toBe('')
  })

  it('空 id 直接返回，不发请求', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('')
    expect(api.getSession).not.toHaveBeenCalled()
  })
})

describe('新建会话', () => {
  it('类型越界时不发请求，错误落在 create 区', async () => {
    const store = useAllianceSessionsStore()
    store.createDraft.sessionType = 'nope'
    expect(store.createProblem).toContain('single')
    expect(await store.createSession()).toBe(null)
    expect(api.createSession).not.toHaveBeenCalled()
    expect(store.error.create).toBe(store.createProblem)
  })

  it('草稿整份交给 api，成功后清表单、回第 1 页并打开新会话', async () => {
    const store = useAllianceSessionsStore()
    api.createSession.mockResolvedValue(sessionFixture({ id: 'sess-new' }))
    Object.assign(store.createDraft, { title: '新会话', userId: 'u1', topic: '主题', expertIds: ['e9'], tags: ['t'] })
    store.setFilter('page', 3)
    const created = await store.createSession()
    expect(api.createSession).toHaveBeenCalledWith({
      title: '新会话', sessionType: 'single', userId: 'u1', topic: '主题', expertIds: ['e9'], tags: ['t']
    })
    expect(created.id).toBe('sess-new')
    expect(store.createDraft.title).toBe('')
    expect(store.createDraft.expertIds).toEqual([])
    expect(store.filters.page).toBe(1)
    expect(api.getSession).toHaveBeenCalledWith('sess-new')
    // 初始状态是后端写死的，回执必须把这件事说出来
    expect(store.notice).toContain('active')
    expect(store.loading.create).toBe(false)
  })

  it('创建失败保留表单内容，不静默吞掉后端错误', async () => {
    const store = useAllianceSessionsStore()
    api.createSession.mockRejectedValue(fail('网关 500'))
    store.createDraft.title = '别丢'
    expect(await store.createSession()).toBe(null)
    expect(store.error.create).toBe('网关 500')
    expect(store.createDraft.title).toBe('别丢')
    expect(api.getSession).not.toHaveBeenCalled()
  })
})

describe('追加消息', () => {
  it('没有选中会话时一条都不发', async () => {
    const store = useAllianceSessionsStore()
    store.composer.content = '甲'
    store.composer.role = 'user'
    expect(await store.appendMessage()).toBe(null)
    expect(api.appendSessionMessage).not.toHaveBeenCalled()
  })

  it('content 为空时拦截：后端该字段无缺省，发了必 422', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    store.composer.content = '   '
    expect(store.composerProblem).toContain('content')
    expect(await store.appendMessage()).toBe(null)
    expect(api.appendSessionMessage).not.toHaveBeenCalled()
    expect(store.error.append).toBe(store.composerProblem)
  })

  it('响应只有那条消息，所以并线与 messageCount 由 store 负责', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.appendSessionMessage.mockResolvedValue({
      id: 'msg-2', role: 'expert', content: '答复', createdAt: '2026-09-01T11:00:00Z'
    })
    store.composer.content = '答复'
    store.composer.role = 'expert'
    store.composer.rating = 4
    const msg = await store.appendMessage()
    expect(api.appendSessionMessage).toHaveBeenCalledWith('sess-1', expect.objectContaining({ role: 'expert' }))
    expect(msg.id).toBe('msg-2')
    expect(store.messages.map((m) => m.id)).toEqual(['msg-1', 'msg-2'])
    expect(store.detail.messageCount).toBe(2)
    expect(store.detail.lastActiveAt).toBe('2026-09-01T11:00:00Z')
    // 输入框只清正文与评分：角色/发送者是本轮对话的上下文，不该跟着消失
    expect(store.composer.content).toBe('')
    expect(store.composer.rating).toBe(null)
    expect(store.composer.role).toBe('expert')
    // 列表里的 message_count 落后一条，必须重取
    expect(api.listSessions).toHaveBeenCalled()
  })

  it('追加失败不动已渲染的线程', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.appendSessionMessage.mockRejectedValue(fail('session not found: sess-1'))
    store.composer.content = '甲'
    expect(await store.appendMessage()).toBe(null)
    expect(store.messages.length).toBe(1)
    expect(store.error.append).toBe('session not found: sess-1')
  })
})

describe('合并式更新', () => {
  it('没有任何改动时一个请求都不发', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    expect(await store.saveEdit()).toBe(null)
    expect(api.updateSession).not.toHaveBeenCalled()
    expect(store.notice).toContain('没有任何改动')
  })

  it('只把改过的键交给 api，未改的键不出现在请求体里', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    store.editDraft.title = '改过的标题'
    api.updateSession.mockResolvedValue(sessionFixture({ title: '改过的标题' }))
    await store.saveEdit()
    const [id, patch] = api.updateSession.mock.calls[0]
    expect(id).toBe('sess-1')
    expect(Object.keys(patch)).toEqual(['title'])
    expect(patch.title).toBe('改过的标题')
    expect(store.notice).toContain('title')
    expect(store.editDraft.title).toBe('改过的标题')
  })

  it('写出统计之外的状态会被拦住，并说明后果', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    store.editDraft.status = 'paused'
    expect(store.editStatusProblem).toContain('隐形')
    expect(await store.saveEdit()).toBe(null)
    expect(api.updateSession).not.toHaveBeenCalled()
    expect(store.error.update).toBe(store.editStatusProblem)
  })

  it('删掉的 metadata 键只是本地移除：后端没有删除语义', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    store.removeMetaRow(0)
    expect(store.metaRows.map((r) => r.key)).toEqual(['score'])
    expect(store.notice).toContain('priority')
    expect(store.notice).toContain('没有删除 metadata 键')

    store.notice = ''
    store.addMetaRow()
    store.setMetaRow(1, 'key', 'fresh')
    store.setMetaRow(1, 'value', '1')
    api.updateSession.mockResolvedValue(sessionFixture())
    await store.saveEdit()
    const patch = api.updateSession.mock.calls.at(-1)[1]
    expect(patch.metadata).toEqual({ fresh: '1' })
  })

  it('更新失败保留详情与表单，错误落在 update 区', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    store.editDraft.topic = '新主题'
    api.updateSession.mockRejectedValue(fail('session not found: sess-1'))
    expect(await store.saveEdit()).toBe(null)
    expect(store.error.update).toBe('session not found: sess-1')
    expect(store.detail.topic).toBe('如何拆库')
    expect(store.editDraft.topic).toBe('新主题')
  })
})

describe('归档、删除与导出', () => {
  it('归档响应只有三个字段，详情按它局部覆盖而不是整份重取', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.archiveSession.mockResolvedValue({ sessionId: 'sess-1', status: 'archived', archivedAt: '2026-09-02T00:00:00Z', messageCount: 7 })
    await store.archiveSession()
    expect(store.detail.status).toBe('archived')
    expect(store.detail.archivedAt).toBe('2026-09-02T00:00:00Z')
    expect(store.detail.messageCount).toBe(7)
    expect(store.editDraft.status).toBe('archived')
    expect(store.archived).toBe(true)
    expect(api.getSession).toHaveBeenCalledTimes(1)
    expect(store.notice).toContain('没有反归档端点')
    expect(api.sessionStats).toHaveBeenCalledTimes(1)
  })

  it('删除成功后清掉选中态与两份派生结果', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.deleteSession.mockResolvedValue({ deleted: true, sessionId: 'sess-1' })
    await store.deleteSession()
    expect(api.deleteSession).toHaveBeenCalledWith('sess-1')
    expect(store.detail).toBe(null)
    expect(store.selectedId).toBe('')
    expect(store.exportResult).toBe(null)
    expect(store.notice).toContain('已删除')
    expect(api.listSessions).toHaveBeenCalledTimes(1)
  })

  it('没有详情时归档/删除/导出都不发请求', async () => {
    const store = useAllianceSessionsStore()
    expect(await store.archiveSession()).toBe(null)
    expect(await store.deleteSession()).toBe(null)
    expect(await store.exportSession()).toBe(null)
    expect(api.archiveSession).not.toHaveBeenCalled()
    expect(api.deleteSession).not.toHaveBeenCalled()
    expect(api.exportSession).not.toHaveBeenCalled()
  })

  it('导出不发明下载链接：能给的只有文本与文件名', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.exportSession.mockResolvedValue({
      sessionId: 'sess-1', format: 'json', exportedAt: '2026-09-03T08:00:00Z',
      content: sessionFixture(), downloadUrl: null, messageCount: 1
    })
    const result = await store.exportSession()
    expect(result.downloadUrl).toBe(null)
    expect(store.exportDownloadName()).toBe('expert-session-sess-1-2026-09-03.json')
    expect(store.exportPayloadText()).toContain('"title": "架构评审"')
    expect(store.notice).toContain('后端不提供下载地址')
  })

  it('导出失败清空上一次结果，避免把旧内容当新的存盘', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    api.exportSession.mockResolvedValue({ sessionId: 'sess-1', content: {}, messageCount: 1, downloadUrl: null })
    await store.exportSession()
    api.exportSession.mockRejectedValue(fail('导出不可用'))
    expect(await store.exportSession()).toBe(null)
    expect(store.exportResult).toBe(null)
    expect(store.exportDownloadName()).toBe('')
    expect(store.exportPayloadText()).toBe('')
  })
})

describe('两种检索', () => {
  it('会话内检索空词不发请求，结果与截断事实一起呈现', async () => {
    const store = useAllianceSessionsStore()
    await store.selectSession('sess-1')
    expect(await store.runSimilarSearch()).toBe(null)
    expect(api.sessionSimilarSearch).not.toHaveBeenCalled()
    expect(store.error.similar).toContain('query')

    store.similarDraft.query = '架构'
    api.sessionSimilarSearch.mockResolvedValue({
      query: '架构', sessionId: 'sess-1', totalFound: 9,
      results: [{ message: { id: 'msg-1', role: 'user' }, similarityScore: 0.4, rank: 1 }]
    })
    await store.runSimilarSearch()
    expect(api.sessionSimilarSearch).toHaveBeenCalledWith('sess-1', expect.objectContaining({ query: '架构' }))
    expect(store.similar.results.length).toBe(1)
    expect(store.similar.totalFound).toBe(9)
  })

  it('全域检索自己记住过滤条件：后端不回显它们', async () => {
    const store = useAllianceSessionsStore()
    Object.assign(store.semanticDraft, { query: '微服务', sessionType: 'multi', expertId: 'e1' })
    api.semanticSearch.mockResolvedValue({ query: '微服务', results: [], totalSessionsScanned: 0, totalMessagesScanned: 0 })
    await store.runSemanticSearch()
    expect(api.semanticSearch).toHaveBeenCalledWith({ query: '微服务', topK: null, sessionType: 'multi', expertId: 'e1' })
    expect(store.semanticFilter).toEqual({ sessionType: 'multi', expertId: 'e1' })
  })

  it('全域检索的类型越界在前端拦住', async () => {
    const store = useAllianceSessionsStore()
    store.semanticDraft.query = 'x'
    store.semanticDraft.sessionType = 'nope'
    expect(await store.runSemanticSearch()).toBe(null)
    expect(api.semanticSearch).not.toHaveBeenCalled()
    expect(store.error.semantic).toContain('single')
  })

  it('点命中行只换选中会话：后端没有按会话 id 过滤列表的参数', async () => {
    const store = useAllianceSessionsStore()
    expect(await store.openSemanticResult({ sessionId: 'sess-9' })).toBe('sess-9')
    expect(api.getSession).toHaveBeenCalledWith('sess-9')
    expect(store.filters.search).toBe('')
    expect(await store.openSemanticResult({})).toBe('')
    expect(await store.openSemanticResult(null)).toBe('')
  })
})
