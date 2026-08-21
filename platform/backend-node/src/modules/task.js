'use strict';

const { registerModule, BaseModule } = require('./index');
const { getStorage } = require('../storage');

const routes = [
  {
    method: 'get', path: '/tasks',
    handler: (req, res) => {
      BaseModule.ok(res, getStorage().getList('tasks'));
    }
  },
  {
    method: 'get', path: '/tasks/:id',
    handler: (req, res, params) => {
      const tasks = getStorage().getList('tasks');
      const t = tasks.find(x => x.id === params.id);
      if (!t) return BaseModule.fail(res, 404, 'task not found');
      BaseModule.ok(res, t);
    }
  },
  {
    method: 'post', path: '/tasks',
    handler: async (req, res) => {
      const s = getStorage();
      const body = await BaseModule.readBody(req);
      const tasks = s.getList('tasks');
      const task = {
        id: 'task_' + Math.random().toString(36).slice(2, 12),
        title: body.title || '未命名任务',
        description: body.description || '',
        status: body.status || 'todo',
        priority: body.priority || 'medium',
        category: body.category || 'general',
        tags: body.tags || [],
        source: body.source || 'manual',
        messages: body.messages || [],
        ai_reply: body.ai_reply || '',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        ...body
      };
      tasks.unshift(task);
      s.saveList('tasks', tasks);
      BaseModule.ok(res, task);
    }
  },
  {
    method: 'patch', path: '/tasks/:id',
    handler: async (req, res, params) => {
      const s = getStorage();
      const body = await BaseModule.readBody(req);
      const tasks = s.getList('tasks');
      const idx = tasks.findIndex(t => t.id === params.id);
      if (idx === -1) return BaseModule.fail(res, 404, 'task not found');
      tasks[idx] = { ...tasks[idx], ...body, id: params.id, updated_at: new Date().toISOString() };
      s.saveList('tasks', tasks);
      BaseModule.ok(res, tasks[idx]);
    }
  },
  {
    method: 'delete', path: '/tasks/:id',
    handler: (req, res, params) => {
      const s = getStorage();
      const tasks = s.getList('tasks');
      const idx = tasks.findIndex(t => t.id === params.id);
      if (idx === -1) return BaseModule.fail(res, 404, 'task not found');
      tasks.splice(idx, 1);
      s.saveList('tasks', tasks);
      BaseModule.ok(res, { success: true });
    }
  }
];

registerModule('task', routes, { description: '任务管理模块', version: '2.0' });
