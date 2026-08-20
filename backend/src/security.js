'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');

class SecurityManager {
  constructor() {
    this.apiKeys = new Map();
    this.rateLimiters = new Map();
    this.auditLog = [];
    this.inputSanitizers = new Map();
    this.config = {
      rateLimitWindow: 60000,
      rateLimitMaxRequests: 1000,
      apiKeyExpiry: 24 * 60 * 60 * 1000,
      auditLogMaxEntries: 10000
    };
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

  checkRateLimit(key) {
    const now = Date.now();
    let limiter = this.rateLimiters.get(key);
    
    if (!limiter || now > limiter.resetTime) {
      limiter = {
        count: 0,
        resetTime: now + this.config.rateLimitWindow,
        blocked: false
      };
      this.rateLimiters.set(key, limiter);
    }
    
    limiter.count++;
    
    if (limiter.count > this.config.rateLimitMaxRequests) {
      if (!limiter.blocked) {
        this._logAudit('rate_limit_exceeded', { key, count: limiter.count });
      }
      limiter.blocked = true;
      return { allowed: false, resetMs: limiter.resetTime - now };
    }
    
    if (limiter.blocked && limiter.count <= this.config.rateLimitMaxRequests / 2) {
      limiter.blocked = false;
    }
    
    return { allowed: true, remaining: this.config.rateLimitMaxRequests - limiter.count };
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

module.exports = { SecurityManager, getSecurityManager };