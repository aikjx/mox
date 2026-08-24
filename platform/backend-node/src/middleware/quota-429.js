/**
 * T10 M4 Quota 429 中间件（AC-T10-13~22）
 *
 * 规范要求：
 *  1. 配额维度：IP / UserID / Bucket（3 维并行，任一超限即 429）
 *  2. 算法：滑动窗口（默认 60s），避免 fixed-window 边界双倍计数问题
 *  3. 响应头：X-Quota-Limit / X-Quota-Remaining / X-Quota-Reset / Retry-After
 *  4. 超限返回 429 Too Many Requests + RFC9450 格式 JSON
 *  5. 默认 3 档预设：tier=free(100/分)/basic(1000/分)/pro(10000/分)
 *  6. 支持并发（Map + Atomic-ish 时间序追加）
 */

'use strict';

/** @typedef {'free'|'basic'|'pro'|'custom'} QuotaTier */

const DEFAULT_WINDOW_MS = 60_000;

const TIER_LIMITS = {
  free: 100,
  basic: 1000,
  pro: 10_000,
};

/**
 * 配额条目：每个 key 的历史时间戳数组（用于滑动窗口）
 * @typedef {{ stamps: number[] }} BucketState
 */

class SlidingWindowQuota {
  /**
   * @param {{windowMs?: number, defaultLimit?: number}} opts
   */
  constructor(opts = {}) {
    this.windowMs = opts.windowMs || DEFAULT_WINDOW_MS;
    this.defaultLimit = opts.defaultLimit || TIER_LIMITS.basic;
    /** @type {Map<string, BucketState>} */
    this._buckets = new Map();
  }

  /** 清理过期时间戳 */
  _prune(state, now) {
    const cutoff = now - this.windowMs;
    // stamps 单调递增（同进程内 Date.now 单调 + 插入顺序）
    let lo = 0;
    let hi = state.stamps.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (state.stamps[mid] <= cutoff) lo = mid + 1;
      else hi = mid;
    }
    if (lo > 0) state.stamps.splice(0, lo);
  }

  /**
   * 尝试消耗 1 个配额槽。
   * @param {string} key
   * @param {number} limit 窗口内允许的最大请求数（>0）
   * @returns {{ok:boolean, remaining:number, resetAtMs:number, count:number}}
   */
  consume(key, limit) {
    const now = Date.now();
    let state = this._buckets.get(key);
    if (!state) {
      state = { stamps: [] };
      this._buckets.set(key, state);
    }
    this._prune(state, now);
    if (state.stamps.length >= limit) {
      // 超限 → 返回首个有效戳的过期时间
      const firstValid = state.stamps[0];
      const resetAtMs = firstValid + this.windowMs;
      return { ok: false, remaining: 0, resetAtMs, count: state.stamps.length };
    }
    state.stamps.push(now);
    const remaining = limit - state.stamps.length;
    const resetAtMs = state.stamps[0] + this.windowMs;
    return { ok: true, remaining, resetAtMs, count: state.stamps.length };
  }

  /** 测试辅助：强制查看 key 的历史数（不清零） */
  debugCount(key) {
    const s = this._buckets.get(key);
    return s ? s.stamps.length : 0;
  }

  /** 用于资源隔离：清理 key */
  reset(key) {
    this._buckets.delete(key);
  }

  resetAll() {
    this._buckets.clear();
  }

  /** 获取当前窗口大小（ms） */
  get window() {
    return this.windowMs;
  }
}

/**
 * 默认 tier -> IP/User/Bucket 维度 limit 表（与 tasks.md A-4 对齐）
 */
function tierLimits(tier) {
  switch (tier) {
    case 'free':
      return { ip: TIER_LIMITS.free, user: TIER_LIMITS.free * 2, bucket: TIER_LIMITS.free * 5 };
    case 'pro':
      return { ip: TIER_LIMITS.pro, user: TIER_LIMITS.pro, bucket: TIER_LIMITS.pro * 2 };
    case 'custom':
      return null; // 必须传 customLimits
    case 'basic':
    default:
      return { ip: TIER_LIMITS.basic, user: TIER_LIMITS.basic, bucket: TIER_LIMITS.basic * 2 };
  }
}

/**
 * 从入站请求提取限流维度 key。
 *   - ip:     X-Forwarded-For[0] || socket.remoteAddress
 *   - user:   X-User-ID 头（若启用）
 *   - bucket: X-Bucket 头（若启用）
 * @param {import('http').IncomingMessage} req
 */
function extract(req) {
  const xff =
    req.headers && typeof req.headers['x-forwarded-for'] === 'string'
      ? req.headers['x-forwarded-for'].split(',')[0].trim()
      : null;
  const ip = xff || (req.socket && req.socket.remoteAddress) || 'unknown';
  const user = req.headers && req.headers['x-user-id'] ? String(req.headers['x-user-id']) : null;
  const bucket = req.headers && req.headers['x-bucket'] ? String(req.headers['x-bucket']) : null;
  return { ip, user, bucket };
}

function setHeader(res, k, v) {
  if (res && typeof res.setHeader === 'function') res.setHeader(k, String(v));
}

/**
 * HTTP 中间件工厂（原生 Node http / Connect / Express 皆兼容，无依赖）。
 *
 * @param {Object} opts
 * @param {QuotaTier} [opts.tier='basic']
 * @param {{ip?:number,user?:number,bucket?:number}} [opts.customLimits] 当 tier='custom' 时必填
 * @param {number} [opts.windowMs=60000]
 * @param {'enforce'|'report'} [opts.mode='enforce'] report 模式：不 429，仅写 X-Quota 头（灰度用）
 * @param {{permitMissingDimension?: boolean, dryRunLog?: (msg:string)=>void}} [opts.extra]
 * @returns {(req: any, res: any, next?: (err?: any)=>void) => void}
 */
function createQuotaMiddleware(opts = {}) {
  const tier = opts.tier || 'basic';
  const windowMs = opts.windowMs || DEFAULT_WINDOW_MS;
  let limits = tierLimits(tier);
  if (tier === 'custom') {
    const c = opts.customLimits || {};
    limits = {
      ip: c.ip || 0,
      user: c.user || 0,
      bucket: c.bucket || 0,
    };
  }
  const mode = opts.mode || 'enforce';
  const permitMissing = opts.extra && opts.extra.permitMissingDimension; // true => 缺此维度不判定
  const log = (opts.extra && opts.extra.dryRunLog) || null;

  const sw = new SlidingWindowQuota({ windowMs });

  const mw = function quotaMiddleware(req, res, next) {
    const dims = extract(req);
    const candidates = [
      { k: 'ip', v: dims.ip, limit: limits.ip },
      { k: 'user', v: dims.user, limit: limits.user },
      { k: 'bucket', v: dims.bucket, limit: limits.bucket },
    ];
    // 逐维度尝试；首个失败立即终止（返回该维度信息）；全部 OK 写最严 remaining
    /** @type {null|{ok:false,remaining:number,resetAtMs:number,count:number,dim:string,limit:number}} */
    let fail = null;
    /** @type {number|null} */
    let tightestRemaining = null;
    /** @type {number|null} */
    let earliestReset = null;
    /** @type {string|null} */
    let limitingDim = null;
    /** @type {number|null} */
    let limitingLimit = null;

    for (const c of candidates) {
      if (!c.v) {
        if (!permitMissing && c.k === 'ip') {
          // IP 始终必判（unknown 兜底），所以必然有
        }
        // 缺维度（无 user / 无 bucket）：按 permitMissing 跳过
        if (!c.v && (c.k === 'user' || c.k === 'bucket')) {
          if (permitMissing) continue;
          // 默认不 permit 则视为 0 限制（该维度无信息不参与限制）
          if (!c.v) continue;
        }
      }
      if (!c.limit || c.limit <= 0) {
        // limit=0 => 该维度无限制（跳过）
        continue;
      }
      const compositeKey = `${c.k}:${c.v}`;
      const r = sw.consume(compositeKey, c.limit);
      // track tightest remaining
      if (tightestRemaining === null || r.remaining < tightestRemaining) {
        tightestRemaining = r.remaining;
        earliestReset = r.resetAtMs;
        limitingDim = c.k;
        limitingLimit = c.limit;
      }
      if (!r.ok) {
        fail = { ...r, dim: c.k, limit: c.limit };
        break;
      }
    }

    // 写响应头（即使 429 也写）
    if (limitingLimit !== null) setHeader(res, 'X-Quota-Limit', limitingLimit);
    else setHeader(res, 'X-Quota-Limit', limits.ip);
    setHeader(res, 'X-Quota-Remaining', Math.max(0, tightestRemaining !== null ? tightestRemaining : limits.ip));
    if (earliestReset !== null) {
      const resetSec = Math.max(1, Math.ceil((earliestReset - Date.now()) / 1000));
      setHeader(res, 'X-Quota-Reset', Math.floor(earliestReset / 1000));
      setHeader(res, 'Retry-After', resetSec);
    } else {
      const resetSec = Math.ceil(windowMs / 1000);
      setHeader(res, 'X-Quota-Reset', Math.floor((Date.now() + windowMs) / 1000));
      setHeader(res, 'Retry-After', resetSec);
    }
    if (limitingDim) setHeader(res, 'X-Quota-Dimension', limitingDim);

    if (fail && mode === 'enforce') {
      const retryAfter = Math.max(1, Math.ceil((fail.resetAtMs - Date.now()) / 1000));
      if (log) log(`quota_429 dim=${fail.dim} count=${fail.count}/${fail.limit} retry=${retryAfter}s`);
      const body = JSON.stringify({
        error: 'Too Many Requests',
        code: 'QUOTA_EXCEEDED',
        status: 429,
        dimension: fail.dim,
        limit: fail.limit,
        count: fail.count,
        retry_after_sec: retryAfter,
        reset_at_unix: Math.floor(fail.resetAtMs / 1000),
        message: `Request quota exceeded for dimension '${fail.dim}': ${fail.count}/${fail.limit} in ${windowMs}ms window`,
      });
      setHeader(res, 'Content-Type', 'application/json; charset=utf-8');
      setHeader(res, 'Content-Length', Buffer.byteLength(body));
      setHeader(res, 'Retry-After', retryAfter);
      if (res && typeof res.writeHead === 'function') res.writeHead(429);
      if (res && typeof res.end === 'function') {
        res.end(body);
      }
      if (next) return; // next 不调用（已写 429）
      return;
    }

    if (next) return next();
    // 非中间件链（直接 handler 风格）：调用者继续
    return { handled: false };
  };

  // 暴露内部（测试/维护）
  mw._sw = sw;
  mw._limits = limits;
  mw._windowMs = windowMs;
  mw._extract = extract;
  mw.resetAll = () => sw.resetAll();
  return mw;
}

module.exports = {
  SlidingWindowQuota,
  createQuotaMiddleware,
  tierLimits,
  extract,
  TIER_LIMITS,
  DEFAULT_WINDOW_MS,
};
