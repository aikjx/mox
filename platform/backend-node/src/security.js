'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');

// ---- O2 · TokenBucket 限流（对比 Dify/LangGraph/Flowise/AutoGen 的原生多租户治理能力）----
//   * capacity：突发允许的最大令牌数（burst）
//   * tokensPerSec：每秒补充令牌数（长期 QPS 上限）
//   * resetMs：超限后建议客户端等待时间（由令牌数反推）
class TokenBucket {
  constructor(capacity, tokensPerSec) {
    this.capacity = Math.max(1, capacity);
    this.tokensPerSec = Math.max(0.0001, tokensPerSec);
    this.tokens = this.capacity;
    this.lastRefillMs = Date.now();
  }
  _refill() {
    const now = Date.now();
    const dt = Math.max(0, (now - this.lastRefillMs) / 1000);
    if (dt > 0) {
      this.tokens = Math.min(this.capacity, this.tokens + dt * this.tokensPerSec);
      this.lastRefillMs = now;
    }
  }
  // 尝试取 n 个令牌，返回 { allowed, remaining, resetMs, tokensPerSec, capacity }
  tryAcquire(n = 1) {
    this._refill();
    if (this.tokens >= n) {
      this.tokens -= n;
      return {
        allowed: true,
        remaining: Math.floor(this.tokens),
        resetMs: 0,
        tokensPerSec: this.tokensPerSec,
        capacity: this.capacity,
      };
    }
    // 计算需要多少秒才能补充 n 个令牌
    const deficit = n - this.tokens;
    const secondsNeeded = deficit / this.tokensPerSec;
    return {
      allowed: false,
      remaining: Math.floor(this.tokens),
      resetMs: Math.ceil(secondsNeeded * 1000),
      tokensPerSec: this.tokensPerSec,
      capacity: this.capacity,
    };
  }
  state() { this._refill(); return { tokens: this.tokens, capacity: this.capacity, tps: this.tokensPerSec }; }
}

// O2 · 租户级别配额（按 Tier + 单 Key 双维）—— 对照 Dify 的 workspace rate limit
const DEFAULT_TENANT_QUOTAS = {
  VIP:       { qps: 200, burst: 400 },   // 企业白金
  NORMAL:    { qps: 20,  burst: 60 },    // 标准用户
  TRIAL:     { qps: 5,   burst: 10 },
  ANONYMOUS: { qps: 2,   burst: 4 },     // 匿名兜底
  _default:  { qps: 10,  burst: 20 },
};

class SecurityManager {
  constructor(config = {}) {
    this.apiKeys = new Map();
    this.rateLimiters = new Map();      // legacy 滑动窗口
    // O2 TokenBucket 实例：per-key → TokenBucket
    this._tokenBuckets = new Map();
    // O2 租户级（group）：tenantId → TokenBucket
    this._tenantBuckets = new Map();
    this.auditLog = [];
    this.inputSanitizers = new Map();
    this.config = Object.assign({
      rateLimitWindow: 60000,
      rateLimitMaxRequests: 1000,
      apiKeyExpiry: 24 * 60 * 60 * 1000,
      auditLogMaxEntries: 10000,
      // O2 开关：当 SEC_ENABLE_TOKEN_BUCKET=1 或配置 forceTokenBucket=true 时启用 TokenBucket；
      // 否则回落到既有滑动窗口（向后兼容）。
      forceTokenBucket: false,
      tenantQuotas: Object.assign({}, DEFAULT_TENANT_QUOTAS, config.tenantQuotas || {}),
      // O2 单 key 默认 qps / burst（可被 apiKey 覆盖）
      defaultKeyQps: 20,
      defaultKeyBurst: 50,
      // O2 GC：bucket 闲置超过此值即删除（防止内存无限增长）
      bucketIdleCleanupMs: 10 * 60 * 1000,
      bucketIdleCleanupEveryMs: 60 * 1000,
    }, config || {});
    this._lastCleanup = Date.now();
    this._init();
  }

  _init() {
    this._loadApiKeys();
    this._loadAuditLog();
    this._setupSanitizers();
  }

  _loadApiKeys() {
    try {
      const fp = path.join(DATA_DIR, 'api_keys.json');
      if (fs.existsSync(fp)) {
        const data = JSON.parse(fs.readFileSync(fp, 'utf8'));
        data.forEach(k => {
          this.apiKeys.set(k.key, {
            ...k,
            createdAt: new Date(k.createdAt),
            lastUsed: k.lastUsed ? new Date(k.lastUsed) : null
          });
        });
      }
    } catch (e) {
      console.warn('[security] Failed to load API keys:', e.message);
    }
  }

  _saveApiKeys() {
    try {
      const fp = path.join(DATA_DIR, 'api_keys.json');
      const data = Array.from(this.apiKeys.values()).map(k => ({
        ...k,
        createdAt: k.createdAt.toISOString(),
        lastUsed: k.lastUsed ? k.lastUsed.toISOString() : null
      }));
      fs.writeFileSync(fp, JSON.stringify(data, null, 2), 'utf8');
    } catch (e) {
      console.error('[security] Failed to save API keys:', e.message);
    }
  }

  _loadAuditLog() {
    try {
      const fp = path.join(DATA_DIR, 'audit_log.json');
      if (fs.existsSync(fp)) {
        this.auditLog = JSON.parse(fs.readFileSync(fp, 'utf8'));
      }
    } catch (e) {
      console.warn('[security] Failed to load audit log:', e.message);
    }
  }

  _saveAuditLog() {
    try {
      const fp = path.join(DATA_DIR, 'audit_log.json');
      const trimmed = this.auditLog.slice(-this.config.auditLogMaxEntries);
      fs.writeFileSync(fp, JSON.stringify(trimmed, null, 2), 'utf8');
    } catch (e) {
      console.error('[security] Failed to save audit log:', e.message);
    }
  }

  _setupSanitizers() {
    this.inputSanitizers.set('email', (v) => {
      if (!v || typeof v !== 'string') return null;
      const re = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/;
      return re.test(v) ? v.trim() : null;
    });

    this.inputSanitizers.set('string', (v, maxLen = 5000) => {
      if (typeof v !== 'string') return null;
      const sanitized = v.replace(/<script[^>]*>.*?<\/script>/gi, '')
        .replace(/<[^>]+>/g, '')
        .replace(/javascript:/gi, '')
        .replace(/on\w+=/gi, 'safe_evt=');
      return sanitized.slice(0, maxLen);
    });

    this.inputSanitizers.set('id', (v) => {
      if (!v || typeof v !== 'string') return null;
      return v.replace(/[^a-zA-Z0-9_-]/g, '').slice(0, 64);
    });

    this.inputSanitizers.set('url', (v) => {
      if (!v || typeof v !== 'string') return null;
      try {
        const u = new URL(v);
        return u.toString();
      } catch {
        return null;
      }
    });

    this.inputSanitizers.set('json', (v) => {
      if (typeof v === 'string') {
        try {
          return JSON.parse(v);
        } catch {
          return null;
        }
      }
      if (typeof v === 'object' && v !== null) {
        return v;
      }
      return null;
    });
  }

  createApiKey(name, permissions = ['read']) {
    const key = crypto.randomBytes(32).toString('hex');
    const hash = crypto.createHash('sha256').update(key).digest('hex');
    const now = new Date();
    
    const record = {
      id: crypto.randomUUID(),
      name,
      key: hash,
      permissions,
      createdAt: now.toISOString(),
      lastUsed: null,
      active: true
    };
    
    this.apiKeys.set(hash, record);
    this._saveApiKeys();
    this._logAudit('api_key_created', { name, permissions });
    
    return { key, id: record.id };
  }

  validateApiKey(key) {
    if (!key) return { valid: false, reason: 'no key provided' };
    
    const hash = crypto.createHash('sha256').update(key).digest('hex');
    const record = this.apiKeys.get(hash);
    
    if (!record) {
      this._logAudit('auth_failed', { reason: 'invalid key' });
      return { valid: false, reason: 'invalid API key' };
    }
    
    if (!record.active) {
      return { valid: false, reason: 'key deactivated' };
    }
    
    record.lastUsed = new Date();
    this._saveApiKeys();
    
    return {
      valid: true,
      permissions: record.permissions,
      name: record.name,
      id: record.id
    };
  }

  revokeApiKey(keyId) {
    let revoked = false;
    for (const [hash, record] of this.apiKeys) {
      if (record.id === keyId) {
        record.active = false;
        revoked = true;
        break;
      }
    }
    if (revoked) {
      this._saveApiKeys();
      this._logAudit('api_key_revoked', { keyId });
    }
    return revoked;
  }

  getApiKeys() {
    return Array.from(this.apiKeys.values()).map(k => ({
      id: k.id,
      name: k.name,
      permissions: k.permissions,
      createdAt: k.createdAt,
      lastUsed: k.lastUsed,
      active: k.active
    }));
  }

  _enableTokenBucket() {
    return !!this.config.forceTokenBucket || process.env.SEC_ENABLE_TOKEN_BUCKET === '1';
  }

  _quotaForTier(tier) {
    if (!tier) return this.config.tenantQuotas._default;
    const t = String(tier).toUpperCase();
    return this.config.tenantQuotas[t] || this.config.tenantQuotas._default || DEFAULT_TENANT_QUOTAS._default;
  }

  _ensureBucket(map, key, qps, burst) {
    let b = map.get(key);
    if (!b) {
      b = new TokenBucket(burst, qps);
      b._lastUsedMs = Date.now();
      map.set(key, b);
    }
    return b;
  }

  _maybeCleanupIdleBuckets() {
    const now = Date.now();
    if (now - this._lastCleanup < this.config.bucketIdleCleanupEveryMs) return;
    this._lastCleanup = now;
    const ttl = this.config.bucketIdleCleanupMs;
    for (const [k, b] of this._tokenBuckets) if (now - (b._lastUsedMs||0) > ttl) this._tokenBuckets.delete(k);
    for (const [k, b] of this._tenantBuckets) if (now - (b._lastUsedMs||0) > ttl) this._tenantBuckets.delete(k);
  }

  /**
   * O2 · checkRateLimit 多签名兼容：
   *   (key) → legacy
   *   (key, { tier, tenantId, cost }) → O2 token bucket 双维（per-key + per-tenantId）
   * 返回：
   *   { allowed, remaining, resetMs, mode: 'sliding_window'|'token_bucket',
   *     bucketKeyState?: {...}, bucketTenantState?: {...} }
   */
  checkRateLimit(key, opts) {
    this._maybeCleanupIdleBuckets();

    const tier = opts && opts.tier;
    const tenantId = opts && opts.tenantId;
    const cost = Math.max(1, parseInt((opts && opts.cost) || 1, 10));

    if (!this._enableTokenBucket()) {
      // ====== 原 sliding window（向后兼容）======
      const now = Date.now();
      let limiter = this.rateLimiters.get(key);
      if (!limiter || now > limiter.resetTime) {
        limiter = { count: 0, resetTime: now + this.config.rateLimitWindow, blocked: false };
        this.rateLimiters.set(key, limiter);
      }
      limiter.count++;
      if (limiter.count > this.config.rateLimitMaxRequests) {
        if (!limiter.blocked) this._logAudit('rate_limit_exceeded', { key, count: limiter.count, mode: 'sliding_window' });
        limiter.blocked = true;
        return { allowed: false, resetMs: limiter.resetTime - now, mode: 'sliding_window' };
      }
      if (limiter.blocked && limiter.count <= this.config.rateLimitMaxRequests / 2) limiter.blocked = false;
      return { allowed: true, remaining: this.config.rateLimitMaxRequests - limiter.count, mode: 'sliding_window' };
    }

    // ====== O2 TokenBucket（per-key + per-tenant 两级）======
    //   1. per-key：按 key 自身 qps / burst（若 apiKey 中设置了，则覆盖默认值）
    const keyMeta = this.apiKeys.get(key);
    const keyQps = (keyMeta && typeof keyMeta.qps === 'number') ? keyMeta.qps : this.config.defaultKeyQps;
    const keyBurst = (keyMeta && typeof keyMeta.burst === 'number') ? keyMeta.burst : this.config.defaultKeyBurst;
    const keyBucket = this._ensureBucket(this._tokenBuckets, key, keyQps, keyBurst);
    keyBucket._lastUsedMs = Date.now();
    const keyRes = keyBucket.tryAcquire(cost);

    if (!keyRes.allowed) {
      this._logAudit('rate_limit_exceeded', { key, cause: 'key_bucket', tier, tenantId, cost });
      return Object.assign({}, keyRes, {
        allowed: false,
        mode: 'token_bucket',
        bucketKeyState: keyBucket.state(),
      });
    }

    //   2. per-tenantId：按 tier 配额（tenantId 为空时用 tier 作为聚合 key）
    let tenantBucket = null;
    if (tenantId || tier) {
      const tkey = tenantId || `tier:${String(tier||'UNKNOWN').toUpperCase()}`;
      const q = this._quotaForTier(tier);
      tenantBucket = this._ensureBucket(this._tenantBuckets, tkey, q.qps, q.burst);
      tenantBucket._lastUsedMs = Date.now();
      const tRes = tenantBucket.tryAcquire(cost);
      if (!tRes.allowed) {
        this._logAudit('rate_limit_exceeded', { key, cause: 'tenant_bucket', tier, tenantId, cost });
        // 回滚 keyBucket 本次 acquire（避免两级不对称扣减）
        keyBucket.tokens = Math.min(keyBucket.capacity, keyBucket.tokens + cost);
        return Object.assign({}, tRes, {
          allowed: false,
          mode: 'token_bucket',
          bucketKeyState: keyBucket.state(),
          bucketTenantState: tenantBucket.state(),
        });
      }
    }

    return {
      allowed: true,
      remaining: keyRes.remaining,
      resetMs: 0,
      mode: 'token_bucket',
      bucketKeyState: keyBucket.state(),
      bucketTenantState: tenantBucket ? tenantBucket.state() : null,
    };
  }

  sanitizeInput(value, type = 'string') {
    const sanitizer = this.inputSanitizers.get(type);
    if (!sanitizer) return value;
    return sanitizer(value);
  }

  validateSchema(data, schema) {
    const errors = [];
    
    for (const [field, rules] of Object.entries(schema)) {
      const value = data[field];
      
      if (rules.required && (value === undefined || value === null || value === '')) {
        errors.push(`${field} is required`);
        continue;
      }
      
      if (value === undefined || value === null) continue;
      
      if (rules.type === 'string' && typeof value !== 'string') {
        errors.push(`${field} must be a string`);
      } else if (rules.type === 'number' && typeof value !== 'number') {
        errors.push(`${field} must be a number`);
      } else if (rules.type === 'boolean' && typeof value !== 'boolean') {
        errors.push(`${field} must be a boolean`);
      } else if (rules.type === 'array' && !Array.isArray(value)) {
        errors.push(`${field} must be an array`);
      } else if (rules.type === 'object' && (typeof value !== 'object' || Array.isArray(value))) {
        errors.push(`${field} must be an object`);
      }
      
      if (value && rules.maxLength && typeof value === 'string' && value.length > rules.maxLength) {
        errors.push(`${field} exceeds max length of ${rules.maxLength}`);
      }
      
      if (value && rules.pattern && !rules.pattern.test(value)) {
        errors.push(`${field} has invalid format`);
      }
    }
    
    return { valid: errors.length === 0, errors };
  }

  _logAudit(action, details, actor = 'system') {
    const entry = {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      action,
      actor,
      details: this._redactSensitive(details)
    };
    
    this.auditLog.push(entry);
    if (this.auditLog.length > this.config.auditLogMaxEntries * 1.2) {
      this.auditLog = this.auditLog.slice(-this.config.auditLogMaxEntries);
    }
    
    if (this.auditLog.length % 100 === 0) {
      this._saveAuditLog();
    }
  }

  _redactSensitive(obj) {
    if (!obj || typeof obj !== 'object') return obj;
    
    const sensitiveKeys = ['api_key', 'password', 'token', 'secret', 'key'];
    const redacted = {};
    
    for (const [k, v] of Object.entries(obj)) {
      if (sensitiveKeys.some(sk => k.toLowerCase().includes(sk))) {
        redacted[k] = typeof v === 'string' && v.length > 8 ? 
          v.slice(0, 4) + '****' + v.slice(-4) : '****';
      } else if (typeof v === 'object' && v !== null) {
        redacted[k] = this._redactSensitive(v);
      } else {
        redacted[k] = v;
      }
    }
    
    return redacted;
  }

  getAuditLog(filters = {}) {
    let logs = [...this.auditLog];
    
    if (filters.action) {
      logs = logs.filter(l => l.action === filters.action);
    }
    if (filters.actor) {
      logs = logs.filter(l => l.actor === filters.actor);
    }
    if (filters.since) {
      logs = logs.filter(l => new Date(l.timestamp) >= new Date(filters.since));
    }
    
    return logs.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp)).slice(0, filters.limit || 100);
  }

  getSecurityStatus() {
    const now = Date.now();
    const activeKeys = Array.from(this.apiKeys.values()).filter(k => k.active).length;
    const rateLimiters = Array.from(this.rateLimiters.values());
    const blockedKeys = rateLimiters.filter(l => l.blocked).length;
    
    return {
      active_api_keys: activeKeys,
      rate_limiters_active: rateLimiters.length,
      rate_limiters_blocked: blockedKeys,
      audit_log_entries: this.auditLog.length,
      security_health: 'good',
      recommendations: this._generateRecommendations(activeKeys, blockedKeys)
    };
  }

  _generateRecommendations(activeKeys, blockedKeys) {
    const recommendations = [];
    
    if (activeKeys > 10) {
      recommendations.push('Consider rotating inactive API keys');
    }
    if (blockedKeys > 0) {
      recommendations.push('Investigate rate-limited clients for potential abuse');
    }
    if (this.auditLog.length > 5000) {
      recommendations.push('Archive old audit logs to cold storage');
    }
    
    return recommendations;
  }
}

let instance = null;

function getSecurityManager() {
  if (!instance) {
    instance = new SecurityManager();
  }
  return instance;
}

module.exports = { SecurityManager, getSecurityManager, TokenBucket, DEFAULT_TENANT_QUOTAS };