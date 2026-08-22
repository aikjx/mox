'use strict';

/**
 * 路由域：自动任务
 * /tasks/auto 分析对话 → 创建任务 → 自动执行
 */
module.exports = function registerAutoTasksRoutes(ctx) {
  const { gateway, uid, readJSON, writeJSON, ok, fail, readBody, appendLog, reg } = ctx;

  // ===== 自动任务：分析对话 → 创建任务 → 自动执行 =====
  reg('post', '/tasks/auto', async (req, res) => {
    const body = await readBody(req)
    const message = body.message || body.text || ''
    const sessionId = body.session_id || null
    const contextMessages = body.messages || []

    if (!message) return fail(res, 400, '缺少消息内容')

    try {
      const analysis = await gateway.chat({
        messages: [
          { role: 'system', content: '你是一个任务分析专家。分析用户的消息，判断是否需要创建任务。返回JSON格式：{"is_task":true/false,"task_type":"类型","title":"任务标题","description":"详细描述","steps":["步骤1","步骤2"],"priority":"high|medium|low","should_execute":true/false,"execution_plan":"执行计划说明"}。只返回JSON。' },
          { role: 'user', content: `请分析这条消息是否为一个任务请求："${message}"` }
        ]
      })

      let parsed = {}
      try {
        const text = (analysis.content || '').replace(/```json|```/g, '').trim()
        const match = text.match(/\{[\s\S]*\}/)
        if (match) parsed = JSON.parse(match[0])
      } catch {}

      const isTask = parsed.is_task !== false
      const shouldExecute = parsed.should_execute !== false

      const result = {
        is_task: isTask,
        analysis: analysis.content,
        task: null,
        execution: null
      }

      if (isTask) {
        const tasks = readJSON('tasks.json', [])
        const newTask = {
          id: uid('task'),
          title: parsed.title || message.slice(0, 50),
          description: parsed.description || message,
          status: shouldExecute ? 'in_progress' : 'todo',
          priority: parsed.priority || 'medium',
          category: parsed.task_type || 'auto',
          tags: ['AI自动', parsed.task_type || 'task'],
          source: 'auto_chat',
          source_id: sessionId,
          messages: contextMessages,
          ai_reply: analysis.content,
          steps: parsed.steps || [],
          execution_plan: parsed.execution_plan || '',
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          due_date: null,
          assignee: null,
          metadata: { auto_created: true, auto_executed: shouldExecute }
        }
        tasks.unshift(newTask)
        writeJSON('tasks.json', tasks)
        appendLog({ type: 'task', msg: 'auto-create', task_id: newTask.id, title: newTask.title, auto_exec: shouldExecute })

        result.task = newTask

        if (shouldExecute) {
          const execResult = await gateway.chat({
            messages: [
              { role: 'system', content: '你是一个任务执行引擎。根据给定的任务信息，生成执行结果。格式：{"status":"completed","result":"执行结果描述","outputs":{},"next_steps":[]}。只返回JSON。' },
              { role: 'user', content: `执行任务：标题=${newTask.title}，描述=${newTask.description}，步骤=${(newTask.steps || []).join('、')}，执行计划=${newTask.execution_plan || '按步骤执行'}` }
            ]
          })

          let execParsed = {}
          try {
            const text = (execResult.content || '').replace(/```json|```/g, '').trim()
            const match = text.match(/\{[\s\S]*\}/)
            if (match) execParsed = JSON.parse(match[0])
          } catch {}

          const finalStatus = execParsed.status || 'completed'
          const tasks2 = readJSON('tasks.json', [])
          const idx = tasks2.findIndex(t => t.id === newTask.id)
          if (idx >= 0) {
            tasks2[idx].status = finalStatus
            tasks2[idx].result = execParsed.result || execResult.content
            tasks2[idx].outputs = execParsed.outputs || {}
            tasks2[idx].next_steps = execParsed.next_steps || []
            tasks2[idx].completed_at = new Date().toISOString()
            tasks2[idx].updated_at = new Date().toISOString()
            writeJSON('tasks.json', tasks2)
          }

          result.execution = {
            status: finalStatus,
            result: execParsed.result || execResult.content,
            outputs: execParsed.outputs || {},
            next_steps: execParsed.next_steps || [],
            raw: execResult.content
          }
        }
      }

      ok(res, result)
    } catch (e) {
      ok(res, {
        is_task: false,
        analysis: '',
        task: null,
        execution: null,
        error: e.message
      })
    }
  })

};
