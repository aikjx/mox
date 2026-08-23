'use strict';

const path = require('path');
const fs = require('fs');

const DATA_DIR = process.env.DATA_DIR
  ? path.resolve(process.env.DATA_DIR)
  : path.join(__dirname, '..', 'data');
if (!fs.existsSync(DATA_DIR)) fs.mkdirSync(DATA_DIR, { recursive: true });

const _parseBool = (v, fallback) => {
  if (v === undefined || v === null || v === '') return fallback;
  const s = String(v).toLowerCase();
  if (s === '1' || s === 'true' || s === 'yes' || s === 'on') return true;
  if (s === '0' || s === 'false' || s === 'no' || s === 'off') return false;
  return fallback;
};

const _tier = (() => {
  const raw = (process.env.INFOTIER || process.env.TIER || 'oss').toLowerCase().trim();
  if (raw === 'enterprise' || raw === 'ent' || raw === 'pro') return 'enterprise';
  return 'oss';
})();

const config = {
  app: {
    name: '璇玑信息知识图谱关联关系系统',
    shortName: '璇玑系统',
    version: '4.0.0',
    port: parseInt(process.env.PORT || '3010', 10),
    mode: process.env.NODE_ENV || 'development'
  },
  // 开源 / 企业 版分级：企业版审计追加 hash_chain，开源版仅 JSON 条目
  tier: _tier,
  storage: {
    provider: process.env.DB_PROVIDER || 'sqlite',
    // dual-write 过渡期：写 primary + secondary（目前 secondary 固定为 sqlite），
    // readPref 取值：'auto'（优先 provider；空读回源 sqlite） / 'primary'（只读 primary） / 'secondary'（只读 sqlite）
    dualWrite: _parseBool(process.env.DB_DUAL_WRITE, false),
    readPref: (process.env.DB_READ_PREF || 'auto').toLowerCase(),
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

// 存储提供商别名：对外允许 postgres / postgresql 等价
const _PROVIDER_ALIASES = { postgres: 'postgresql' };
function _canonicalProvider(name) {
  if (!name) return name;
  return _PROVIDER_ALIASES[name] || name;
}

function getStorageConfig() {
  return config.storage.providers[_canonicalProvider(config.storage.provider)];
}

function switchProvider(providerName) {
  const canonical = _canonicalProvider(providerName);
  if (!config.storage.providers[canonical]) {
    throw new Error(`未知的数据库提供商: ${providerName}，可选: ${Object.keys(config.storage.providers).join(', ')}（别名 postgres=>postgresql 已内建）`);
  }
  const old = config.storage.provider;
  const normalizedOld = _canonicalProvider(old);
  config.storage.provider = canonical;
  console.log(`[config] 数据库提供商切换: ${normalizedOld} → ${canonical}`);
  return { old, new: canonical };
}

function listProviders() {
  return Object.entries(config.storage.providers).map(([name, cfg]) => ({
    name,
    driver: cfg.driver,
    current: name === _canonicalProvider(config.storage.provider)
  }));
}

module.exports = { config, getStorageConfig, switchProvider, listProviders, DATA_DIR };
