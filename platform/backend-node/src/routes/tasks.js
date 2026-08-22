'use strict';

/**
 * 路由域：任务管理
 * /tasks/* 任务 CRUD、对话↔任务双向转换、任务执行
 */
module.exports = function registerTasksRoutes(ctx) {
  const { gateway, uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg } = ctx;

  // ===== 任务管理（对话/任务双向转换） =====
  reg('get', '/tasks', (req, res) => {
    const tasks = readJSON('tasks.json', [])
    ok(res, tasks)
  })

  reg('get', '/tasks/:id', (req, res, params) => {
    const tasks = readJSON('tasks.json', [])
    const task = tasks.find(t => t.id === params.id)
    if (!task) return fail(res, 404, '任务不存在')
    ok(res, task)
  })

  reg('post', '/tasks', async (req, res) => {
    const body = await readBody(req)
    const tasks = readJSON('tasks.json', [])
    const task = {
      id: uid('task'),
      title: body.title || '未命名任务',
      description: body.description || '',
      status: body.status || 'todo',
      priority: body.priority || 'medium',
      category: body.category || 'general',
      tags: body.tags || [],
      source: body.source || 'manual',
      source_id: body.source_id || null,
      messages: body.messages || [],
      ai_reply: body.ai_reply || '',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      due_date: body.due_date || null,
      assignee: body.assignee || null,
      metadata: body.metadata || {}
    }
    tasks.unshift(task)
    writeJSON('tasks.json', tasks)
    appendLog({ type: 'task', msg: 'create', task_id: task.id, title: task.title })
    ok(res, task)
  })

  reg('put', '/tasks/:id', async (req, res, params) => {
    const body = await readBody(req)
    const tasks = readJSON('tasks.json', [])
    const idx = tasks.findIndex(t => t.id === params.id)
    if (idx < 0) return fail(res, 404, '任务不存在')
    tasks[idx] = { ...tasks[idx], ...body, id: params.id, updated_at: new Date().toISOString() }
    writeJSON('tasks.json', tasks)
    ok(res, tasks[idx])
  })

  reg('delete', '/tasks/:id', (req, res, params) => {
    const tasks = readJSON('tasks.json', [])
    const idx = tasks.findIndex(t => t.id === params.id)
    if (idx < 0) return fail(res, 404, '任务不存在')
    tasks.splice(idx, 1)
    writeJSON('tasks.json', tasks)
    ok(res, { deleted: true, id: params.id })
  })

  reg('post', '/tasks/from-chat', async (req, res) => {
    const body = await readBody(req)
    try {
      const chatMessages = body.messages || []
      const chatHistory = chatMessages.map(m => `${m.role}: ${m.content}`).join('\n')
      const result = await gateway.chat({
        messages: [
          { role: 'system', content: '你是一个任务分解专家。请将以下对话内容分析后，提取出核心任务点，以JSON格式返回，格式为：{"title":"任务标题","description":"任务描述","steps":["步骤1","步骤2"],"priority":"high|medium|low","category":"分类"}。只返回JSON，不要其他文字。' },
          { role: 'user', content: chatHistory || body.text || '' }
        ],
        expertType: 'requirement'
      })
      let parsed = {}
      try {
        const text = (result.content || '').replace(/```json|```/g, '').trim()
        const match = text.match(/\{[\s\S]*\}/)
        if (match) parsed = JSON.parse(match[0])
      } catch {}
      const tasks = readJSON('tasks.json', [])
      const newTask = {
        id: uid('task'),
        title: parsed.title || body.title || '对话转任务',
        description: parsed.description || body.text || '从对话转换而来',
        status: 'todo',
        priority: parsed.priority || 'medium',
        category: parsed.category || 'chat_convert',
        tags: ['对话转换', ...(parsed.steps ? ['AI分析'] : [])],
        source: 'chat',
        source_id: body.session_id || null,
        messages: chatMessages,
        ai_reply: result.content || '',
        steps: parsed.steps || [],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        due_date: null,
        assignee: null,
        metadata: { converted_from_chat: true, expert_analysis: result.metadata }
      }
      tasks.unshift(newTask)
      writeJSON('tasks.json', tasks)
      appendLog({ type: 'task', msg: 'from-chat', task_id: newTask.id })
      ok(res, { task: newTask, analysis: result.content, parsed })
    } catch (e) {
      const tasks = readJSON('tasks.json', [])
      const fallbackTask = {
        id: uid('task'),
        title: body.title || '对话转任务',
        description: body.text || '从对话转换而来',
        status: 'todo',
        priority: 'medium',
        category: 'chat_convert',
        tags: ['对话转换'],
        source: 'chat',
        source_id: body.session_id || null,
        messages: body.messages || [],
        ai_reply: '',
        steps: [],
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        metadata: { converted_from_chat: true, ai_failed: true }
      }
      tasks.unshift(fallbackTask)
      writeJSON('tasks.json', tasks)
      ok(res, { task: fallbackTask, analysis: '', parsed: {}, note: 'AI分析失败，已创建基础任务' })
    }
  })

  reg('post', '/tasks/:id/to-chat', async (req, res, params) => {
    const tasks = readJSON('tasks.json', [])
    const task = tasks.find(t => t.id === params.id)
    if (!task) return fail(res, 404, '任务不存在')
    try {
      const messages = [
        { role: 'system', content: '你是一个智能助手。请根据以下任务信息，生成一段自然语言对话回复，帮助用户理解和执行该任务。' },
        { role: 'user', content: `任务标题：${task.title}\n任务描述：${task.description}\n任务状态：${task.status}\n优先级：${task.priority}\n步骤：${(task.steps || []).join('、')}\n\n请生成一段友好的对话回复。` }
      ]
      const result = await gateway.chat({ messages })
      ok(res, {
        session_id: uid('s'),
        task_id: task.id,
        reply: result.content,
        messages: [
          { role: 'user', content: `关于任务「${task.title}」，请帮我分析如何执行。` },
          { role: 'assistant', content: result.content }
        ],
        metadata: result.metadata
      })
    } catch (e) {
      ok(res, {
        session_id: uid('s'),
        task_id: task.id,
        reply: `任务「${task.title}」：${task.description}。请按步骤执行。`,
        messages: [
          { role: 'user', content: `关于任务「${task.title}」，请帮我分析如何执行。` },
          { role: 'assistant', content: `任务「${task.title}」：${task.description}。请按步骤执行。` }
        ],
        metadata: {}
      })
    }
  })

  reg('post', '/tasks/:id/execute', async (req, res, params) => {
    const tasks = readJSON('tasks.json', [])
    const idx = tasks.findIndex(t => t.id === params.id)
    if (idx < 0) return fail(res, 404, '任务不存在')
    const body = await readBody(req)
    tasks[idx].status = body.status || 'in_progress'
    tasks[idx].updated_at = new Date().toISOString()
    if (body.result) tasks[idx].result = body.result
    writeJSON('tasks.json', tasks)
    appendLog({ type: 'task', msg: 'execute', task_id: params.id, status: tasks[idx].status })
    ok(res, tasks[idx])
  })

// ===== 知识库 (KB) 端点 =====

};
