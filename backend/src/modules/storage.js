'use strict';

const { registerModule, BaseModule } = require('./index');
const { getStorage, switchDatabase, listProviders, currentProvider } = require('../storage');
const { config } = require('../config');

const routes = [
  {
    method: 'get', path: '/storage/providers',
    handler: (req, res) => {
      BaseModule.ok(res, listProviders());
    }
  },
  {
    method: 'get', path: '/storage/status',
    handler: (req, res) => {
      const s = getStorage();
      BaseModule.ok(res, {
        current: currentProvider(),
        name: s.name,
        entities: {
          graph_nodes: s.countByType('graph_nodes'),
          graph_edges: s.countByType('graph_edges'),
          tasks: s.countByType('tasks'),
          operators: s.countByType('operators'),
          plugins: s.countByType('plugins'),
          dialogue_sessions: s.countByType('dialogue_sessions'),
          workflows: s.countByType('workflows'),
          automation: s.countByType('automation'),
          total: s.listAllEntities().length
        },
        features: config.features,
        storageDir: config.storageDir
      });
    }
  },
  {
    method: 'post', path: '/storage/switch',
    handler: async (req, res) => {
      const body = await BaseModule.readBody(req);
      const provider = body.provider;
      if (!provider) return BaseModule.fail(res, 400, 'provider 为必填项');
      try {
        const newStorage = switchDatabase(provider);
        BaseModule.ok(res, { success: true, provider: newStorage.name, message: `已切换到 ${provider} 数据库` });
      } catch (e) {
        BaseModule.fail(res, 500, e.message);
      }
    }
  },
  {
    method: 'post', path: '/storage/migrate',
    handler: async (req, res) => {
      const s = getStorage();
      const { DATA_DIR } = require('../config');
      try {
        const count = s.migrateFromJSON(DATA_DIR);
        BaseModule.ok(res, { migrated: count });
      } catch (e) {
        BaseModule.fail(res, 500, e.message);
      }
    }
  }
];

registerModule('storage', routes, { description: '存储管理模块（支持数据库热切换）', version: '1.0' });
