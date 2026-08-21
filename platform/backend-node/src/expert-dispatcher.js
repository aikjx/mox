'use strict';

const fs = require('fs');
const path = require('path');

const DATA_DIR = path.join(__dirname, '..', 'data');
const DISPATCHER_CONFIG = 'dispatcher_config.json';

function readJSON(file, fallback) {
  try {
    const fp = path.join(DATA_DIR, file);
    if (!fs.existsSync(fp)) return fallback;
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : fallback;
  } catch (e) {
    return fallback;
  }
}

function writeJSON(file, data) {
  try {
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[dispatcher] writeJSON', file, e.message);
    return false;
  }
}

const STRATEGY_TYPES = {
  ROUND_ROBIN: 'round_robin',
  LEAST_LOADED: 'least_loaded',
  PERFORMANCE_BASED: 'performance_based',
  CONTENT_AWARE: 'content_aware',
  AFFINITY: 'affinity',
  CUSTOM: 'custom'
};

class CircuitBreaker {
  constructor(options = {}) {
    this.failureThreshold = options.failureThreshold || 5;
    this.resetTimeout = options.resetTimeout || 60000;
    this.halfOpenMaxRequests = options.halfOpenMaxRequests || 3;
    this.states = new Map();
  }

  getState(key) {
    const state = this.states.get(key);
    if (!state) return { status: 'closed', failures: 0, lastFailure: null };
    if (state.status === 'open' && Date.now() > state.lastFailure + this.resetTimeout) {
      state.status = 'half_open';
      state.halfOpenRequests = 0;
    }
    return state;
  }

  canExecute(key) {
    const state = this.getState(key);
    if (state.status === 'closed') return true;
    if (state.status === 'half_open') {
      if (state.halfOpenRequests >= this.halfOpenMaxRequests) return false;
      state.halfOpenRequests = (state.halfOpenRequests || 0) + 1;
      return true;
    }
    return false;
  }

  recordSuccess(key) {
    const state = this.states.get(key) || { status: 'closed', failures: 0 };
    state.status = 'closed';
    state.failures = 0;
    state.lastFailure = null;
    state.halfOpenRequests = 0;
    this.states.set(key, state);
  }

  recordFailure(key) {
    const state = this.states.get(key) || { status: 'closed', failures: 0 };
    state.failures = (state.failures || 0) + 1;
    state.lastFailure = Date.now();

    if (state.failures >= this.failureThreshold) {
      state.status = 'open';
    }
    this.states.set(key, state);
  }

  reset(key) {
    this.states.delete(key);
  }

  getAllStates() {
    return Array.from(this.states.entries()).map(([key, state]) => ({
      key,
      status: state.status,
      failures: state.failures || 0,
      last_failure: state.lastFailure
    }));
  }
}

class RateLimiter {
  constructor(options = {}) {
    this.windowMs = options.windowMs || 60000;
    this.maxRequests = options.maxRequests || 100;
    this.limits = new Map();
  }

  check(key) {
    const now = Date.now();
    const entry = this.limits.get(key) || { timestamps: [], blockedUntil: 0 };

    if (now < entry.blockedUntil) {
      return { allowed: false, retryAfter: entry.blockedUntil - now, blocked: true };
    }

    entry.timestamps = entry.timestamps.filter(t => now - t < this.windowMs);

    if (entry.timestamps.length >= this.maxRequests) {
      entry.blockedUntil = now + 5000;
      this.limits.set(key, entry);
      return { allowed: false, retryAfter: 5000, blocked: true };
    }

    entry.timestamps.push(now);
    this.limits.set(key, entry);
    return { allowed: true, remaining: this.maxRequests - entry.timestamps.length };
  }

  getStatus(key) {
    const entry = this.limits.get(key);
    if (!entry) return { current: 0, max: this.maxRequests, resetMs: 0 };
    const now = Date.now();
    entry.timestamps = entry.timestamps.filter(t => now - t < this.windowMs);
    return {
      current: entry.timestamps.length,
      max: this.maxRequests,
      resetMs: this.windowMs - (now - (entry.timestamps[0] || now))
    };
  }

  reset(key) {
    this.limits.delete(key);
  }
}

class ExpertDispatcher {
  constructor(alliance) {
    this.alliance = alliance;
    this.circuitBreaker = new CircuitBreaker();
    this.rateLimiter = new RateLimiter();
    this.config = this._loadConfig();
    this.roundRobinIndex = 0;
    this.dispatchCount = 0;
    this.dispatchHistory = [];
  }

  _loadConfig() {
    const saved = readJSON(DISPATCHER_CONFIG, null);
    if (saved) return saved;
    const defaultConfig = {
      default_strategy: STRATEGY_TYPES.CONTENT_AWARE,
      strategies: {
        [STRATEGY_TYPES.ROUND_ROBIN]: { enabled: true, description: '轮询策略' },
        [STRATEGY_TYPES.LEAST_LOADED]: { enabled: true, description: '最少负载策略' },
        [STRATEGY_TYPES.PERFORMANCE_BASED]: { enabled: true, description: '性能优先策略' },
        [STRATEGY_TYPES.CONTENT_AWARE]: { enabled: true, description: '内容感知策略' },
        [STRATEGY_TYPES.AFFINITY]: { enabled: true, description: '亲和度策略' }
      },
      circuit_breaker: {
        enabled: true,
        failure_threshold: 5,
        reset_timeout_ms: 60000
      },
      rate_limiter: {
        enabled: true,
        max_per_minute: 100,
        burst_size: 20
      },
      affinity_map: {},
      performance_weights: {}
    };
    writeJSON(DISPATCHER_CONFIG, defaultConfig);
    return defaultConfig;
  }

  _saveConfig() {
    writeJSON(DISPATCHER_CONFIG, this.config);
  }

  setStrategy(strategy) {
    if (!Object.values(STRATEGY_TYPES).includes(strategy)) return false;
    this.config.default_strategy = strategy;
    this._saveConfig();
    return true;
  }

  getConfig() {
    return {
      strategy: this.config.default_strategy,
      strategies: this.config.strategies,
      circuit_breaker: this.config.circuit_breaker,
      rate_limiter: this.config.rate_limiter,
      affinity_map_count: Object.keys(this.config.affinity_map).length
    };
  }

  async dispatch(question, options = {}) {
    const strategy = options.strategy || this.config.default_strategy;
    const expertIds = options.expertIds || await this._selectExperts(question, strategy);

    if (this.config.rate_limiter.enabled) {
      const limiterKey = `expert_dispatch:${options.requester || 'anonymous'}`;
      const rateCheck = this.rateLimiter.check(limiterKey);
      if (!rateCheck.allowed) {
        return {
          success: false,
          error: `请求被限流，请 ${Math.ceil(rateCheck.retryAfter / 1000)} 秒后重试`,
          retry_after: rateCheck.retryAfter,
          strategy
        };
      }
    }

    const availableExperts = expertIds.filter(id => {
      const cbKey = `expert:${id}`;
      return this.circuitBreaker.canExecute(cbKey);
    });

    if (availableExperts.length === 0) {
      return {
        success: false,
        error: '所有专家服务当前不可用（熔断器已触发）',
        recovery_hint: '请稍后重试或联系管理员检查专家服务状态',
        strategy
      };
    }

    const targetExpertId = this._selectOne(availableExperts, strategy, question, options);

    this.dispatchCount++;
    const dispatchRecord = {
      id: `disp_${Date.now()}`,
      question_preview: question.slice(0, 100),
      expert_id: targetExpertId,
      strategy,
      timestamp: new Date().toISOString()
    };
    this.dispatchHistory.push(dispatchRecord);
    if (this.dispatchHistory.length > 1000) {
      this.dispatchHistory = this.dispatchHistory.slice(-500);
    }

    return {
      success: true,
      expert_id: targetExpertId,
      expert_count: availableExperts.length,
      strategy,
      dispatch_id: dispatchRecord.id
    };
  }

  async _selectExperts(question, strategy) {
    const routing = await this.alliance.routeExperts(question, { maxExperts: 5 });
    return routing.selected.map(s => s.expert.id);
  }

  _selectOne(candidates, strategy, question, options = {}) {
    switch (strategy) {
      case STRATEGY_TYPES.ROUND_ROBIN:
        return this._roundRobin(candidates);
      case STRATEGY_TYPES.LEAST_LOADED:
        return this._leastLoaded(candidates);
      case STRATEGY_TYPES.PERFORMANCE_BASED:
        return this._performanceBased(candidates);
      case STRATEGY_TYPES.CONTENT_AWARE:
        return this._contentAware(candidates, question);
      case STRATEGY_TYPES.AFFINITY:
        return this._affinity(candidates, options.affinityKey);
      default:
        return this._roundRobin(candidates);
    }
  }

  _roundRobin(candidates) {
    const idx = this.roundRobinIndex % candidates.length;
    this.roundRobinIndex++;
    return candidates[idx];
  }

  _leastLoaded(candidates) {
    let best = candidates[0];
    let bestLoad = Infinity;
    for (const id of candidates) {
      const expert = this.alliance.getExpert(id);
      if (expert && expert.metrics) {
        const load = expert.metrics.avg_duration * (1 - expert.metrics.success_rate);
        if (load < bestLoad) {
          bestLoad = load;
          best = id;
        }
      }
    }
    return best;
  }

  _performanceBased(candidates) {
    const performanceScores = candidates.map(id => {
      const expert = this.alliance.getExpert(id);
      if (!expert || !expert.metrics) return { id, score: 0.5 };
      const rate = expert.metrics.success_rate || 0.5;
      const conf = expert.metrics.avg_confidence || 0.5;
      const speed = 1 / (1 + (expert.metrics.avg_duration || 1000) / 1000);
      return { id, score: rate * 0.4 + conf * 0.3 + speed * 0.3 };
    });

    performanceScores.sort((a, b) => b.score - a.score);
    return performanceScores[0].id;
  }

  _contentAware(candidates, question) {
    const questionLower = (question || '').toLowerCase();
    let best = candidates[0];
    let bestScore = 0;

    for (const id of candidates) {
      const expert = this.alliance.getExpert(id);
      if (!expert) continue;
      let score = 0;
      for (const cap of expert.capabilities) {
        if (questionLower.includes(cap.toLowerCase())) score += 3;
      }
      if (expert.type && questionLower.includes(expert.type.toLowerCase())) score += 2;
      if (score > bestScore) {
        bestScore = score;
        best = id;
      }
    }
    return best;
  }

  _affinity(candidates, affinityKey) {
    if (!affinityKey) return this._roundRobin(candidates);
    const affinityMap = this.config.affinity_map;
    const preferred = affinityMap[affinityKey];
    if (preferred && candidates.includes(preferred)) {
      return preferred;
    }
    const expert = candidates[Math.floor(Math.random() * candidates.length)];
    if (!affinityMap[affinityKey]) {
      affinityMap[affinityKey] = expert;
      this._saveConfig();
    }
    return expert;
  }

  async dispatchAndConsult(question, options = {}) {
    const dispatch = await this.dispatch(question, options);
    if (!dispatch.success) {
      return {
        success: false,
        dispatch,
        error: dispatch.error
      };
    }

    const cbKey = `expert:${dispatch.expert_id}`;
    const startTime = Date.now();

    try {
      const result = await this.alliance.consult(
        dispatch.expert_id,
        [{ role: 'user', content: question }],
        options
      );

      this.circuitBreaker.recordSuccess(cbKey);

      return {
        success: true,
        dispatch: {
          ...dispatch,
          duration_ms: Date.now() - startTime
        },
        result
      };
    } catch (error) {
      this.circuitBreaker.recordFailure(cbKey);
      return {
        success: false,
        dispatch,
        error: error.message,
        duration_ms: Date.now() - startTime
      };
    }
  }

  async dispatchMultiExpert(question, options = {}) {
    const maxExperts = options.maxExperts || 3;
    const routing = await this.alliance.routeExperts(question, { maxExperts });

    const dispatchResults = [];
    const expertResults = [];

    for (const candidate of routing.selected) {
      const expertId = candidate.expert.id;
      const cbKey = `expert:${expertId}`;

      if (!this.circuitBreaker.canExecute(cbKey)) {
        dispatchResults.push({ expert_id: expertId, status: 'circuit_open' });
        continue;
      }

      try {
        const result = await this.alliance.consult(
          expertId,
          [{ role: 'user', content: question }],
          options
        );
        this.circuitBreaker.recordSuccess(cbKey);
        expertResults.push(result);
        dispatchResults.push({ expert_id: expertId, status: 'success' });
      } catch (error) {
        this.circuitBreaker.recordFailure(cbKey);
        dispatchResults.push({ expert_id: expertId, status: 'failed', error: error.message });
      }
    }

    return {
      success: expertResults.length > 0,
      total_dispatched: dispatchResults.length,
      successful: expertResults.length,
      failed: dispatchResults.filter(d => d.status === 'failed').length,
      results: expertResults,
      dispatch_log: dispatchResults
    };
  }

  getStatus() {
    return {
      dispatcher: {
        total_dispatches: this.dispatchCount,
        recent_dispatches: this.dispatchHistory.slice(-10),
        current_strategy: this.config.default_strategy
      },
      circuit_breaker: {
        states: this.circuitBreaker.getAllStates(),
        total_tracked: this.circuitBreaker.states.size
      },
      rate_limiter: {
        tracked_keys: this.rateLimiter.limits.size
      },
      config: this.getConfig()
    };
  }

  resetExpert(expertId) {
    this.circuitBreaker.reset(`expert:${expertId}`);
    return true;
  }

  resetAll() {
    this.circuitBreaker.states.clear();
    this.rateLimiter.limits.clear();
    this.dispatchHistory = [];
    return true;
  }
}

let dispatcherInstance = null;

function getDispatcher(alliance) {
  if (!dispatcherInstance && alliance) {
    dispatcherInstance = new ExpertDispatcher(alliance);
  }
  return dispatcherInstance;
}

function resetDispatcher() {
  dispatcherInstance = null;
}

module.exports = { ExpertDispatcher, getDispatcher, resetDispatcher, STRATEGY_TYPES };
