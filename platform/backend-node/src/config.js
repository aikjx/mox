'use strict';

const path = require('path');
const fs = require('fs');

const DATA_DIR = path.join(__dirname, '..', 'data');
if (!fs.existsSync(DATA_DIR)) fs.mkdirSync(DATA_DIR, { recursive: true });

const config = {
  app: {
    name: '璇玑信息知识图谱关联关系系统',
    shortName: '璇玑系统',
    version: '4.0.0',
    port: parseInt(process.env.PORT || '3010', 10),
    mode: process.env.NODE_ENV || 'development'
  },
  storage: {
    provider: process.env.DB_PROVIDER || 'sqlite',
    providers: {
      sqlite: {
        driver: 'better-sqlite3',
        path: path.join(DATA_DIR, 'ous.db'),
        options: { journal_mode: 'WAL', synchronous: 'NORMAL' }
      },
      memory: {
        driver: 'memory',
        options: {}
      },
      mysql: {
        driver: 'mysql2',
        host: process.env.DB_HOST || 'localhost',
        port: parseInt(process.env.DB_PORT || '3306', 10),
        database: process.env.DB_NAME || 'ous',
        user: process.env.DB_USER || 'root',
        password: process.env.DB_PASSWORD || '',
        options: {}
      },
      postgresql: {
        driver: 'pg',
        host: process.env.DB_HOST || 'localhost',
        port: parseInt(process.env.DB_PORT || '5432', 10),
        database: process.env.DB_NAME || 'ous',
        user: process.env.DB_USER || 'postgres',
        password: process.env.DB_PASSWORD || '',
        options: {}
      }
    }
  },
  features: {
    autoMigrate: true,
    autoSync: true,
    aiInsights: true,
    graphAnalytics: true,
    expertSystem: true,
    workflowEngine: true
  },
  storageDir: DATA_DIR
};

function getStorageConfig() {
  return config.storage.providers[config.storage.provider];
}

function switchProvider(providerName) {
  if (!config.storage.providers[providerName]) {
    throw new Error(`未知的数据库提供商: ${providerName}，可选: ${Object.keys(config.storage.providers).join(', ')}`);
  }
  const old = config.storage.provider;
  config.storage.provider = providerName;
  console.log(`[config] 数据库提供商切换: ${old} → ${providerName}`);
  return { old, new: providerName };
}

function listProviders() {
  return Object.entries(config.storage.providers).map(([name, cfg]) => ({
    name,
    driver: cfg.driver,
    current: name === config.storage.provider
  }));
}

module.exports = { config, getStorageConfig, switchProvider, listProviders, DATA_DIR };
