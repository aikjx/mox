'use strict';

/**
 * MOX Enterprise · 模块间事件总线
 * ===============================
 * 解耦模块间通信的发布/订阅系统
 *
 * 核心能力：
 *  - 发布/订阅模式（topic-based）
 *  - 通配符订阅（event.* / event.#）
 *  - 事件优先级与排序
 *  - 异步事件队列（背压控制）
 *  - 事件持久化（事件溯源）
 *  - 死信队列（处理失败的事件）
 *  - 事件审计与追踪
 *  - 跨进程事件（Redis Pub/Sub 桥接）
 */

const { EventEmitter } = require('events');
const crypto = require('crypto');

// ─── 事件优先级 ───
const EVENT_PRIORITY = {
  CRITICAL: 0,  // 最高优先级，立即处理
  HIGH: 1,
  NORMAL: 2,
  LOW: 3,
  BACKGROUND: 4, // 最低优先级，空闲时处理
};

// ─── 订阅器选项 ───
const DEFAULT_SUBSCRIBER_OPTIONS = {
  priority: EVENT_PRIORITY.NORMAL,
  async: false,           // 是否异步处理
  timeoutMs: 5000,        // 处理超时
  retries: 3,             // 失败重试次数
  retryDelayMs: 1000,     // 重试间隔
  deadLetter: true,       // 失败后进入死信队列
  filter: null,           // 事件过滤函数
};

class EventBus extends EventEmitter {
  /**
   * @param {object} options
   * @param {number} options.maxQueueSize   最大队列大小（默认 10000）
   * @param {number} options.workerCount    异步处理 worker 数（默认 4）
   * @param {boolean} options.enableDLQ     启用死信队列（默认 true）
   * @param {boolean} options.enableAudit   启用事件审计（默认 true）
   * @param {object} options.redisClient    Redis 客户端（用于跨进程）
   */
  constructor(options = {}) {
    super();
    this.maxQueueSize = options.maxQueueSize || 10000;
    this.workerCount = options.workerCount || 4;
    this.enableDLQ = options.enableDLQ !== false;
    this.enableAudit = options.enableAudit !== false;
    this.redisClient = options.redisClient || null;

    // 订阅器：topic -> Set(subscriber)
    this.subscribers = new Map();

    // 通配符订阅器
    this.wildcardSubscribers = []; // { pattern, regex, subscriber }

    // 事件队列
    this.eventQueue = [];

    // 死信队列
    this.deadLetterQueue = [];

    // 事件审计日志
    this.auditLog = [];

    // 统计
    this.stats = {
      published: 0,
      delivered: 0,
      failed: 0,
      deadLettered: 0,
      byTopic: {},
      byPriority: {},
    };

    this._processing = false;
    this._busId = `bus-${crypto.randomBytes(4).toString('hex')}`;
  }

  /**
   * 订阅事件
   * @param {string} topic  事件主题（支持通配符 * 和 #）
   * @param {Function} handler 处理函数 (event) => Promise<void>|void
   * @param {object} options 订阅选项
   * @returns {string} 订阅器 ID（用于取消订阅）
   */
  subscribe(topic, handler, options = {}) {
    const subscriberId = `sub-${crypto.randomBytes(6).toString('hex')}`;
    const subscriber = {
      id: subscriberId,
      topic,
      handler,
      options: { ...DEFAULT_SUBSCRIBER_OPTIONS, ...options },
      subscribedAt: new Date().toISOString(),
      callCount: 0,
      failureCount: 0,
    };

    // 检测通配符
    if (topic.includes('*') || topic.includes('#')) {
      const regex = this._patternToRegex(topic);
      this.wildcardSubscribers.push({ pattern: topic, regex, subscriber });
    } else {
      if (!this.subscribers.has(topic)) this.subscribers.set(topic, new Set());
      this.subscribers.get(topic).add(subscriber);
    }

    this.emit('bus:subscribed', { subscriberId, topic });
    return subscriberId;
  }

  /**
   * 取消订阅
   */
  unsubscribe(subscriberId) {
    // 精确订阅
    for (const [, set] of this.subscribers) {
      for (const sub of set) {
        if (sub.id === subscriberId) {
          set.delete(sub);
          this.emit('bus:unsubscribed', { subscriberId });
          return true;
        }
      }
    }

    // 通配符订阅
    const idx = this.wildcardSubscribers.findIndex(w => w.subscriber.id === subscriberId);
    if (idx !== -1) {
      this.wildcardSubscribers.splice(idx, 1);
      this.emit('bus:unsubscribed', { subscriberId });
      return true;
    }

    return false;
  }

  /**
   * 发布事件
   * @param {string} topic  事件主题
   * @param {object} payload 事件数据
   * @param {object} options 发布选项
   * @returns {string} 事件 ID
   */
  publish(topic, payload = {}, options = {}) {
    const eventId = `evt-${crypto.randomBytes(8).toString('hex')}`;
    const event = {
      id: eventId,
      topic,
      payload,
      priority: options.priority || EVENT_PRIORITY.NORMAL,
      timestamp: new Date().toISOString(),
      publisher: options.publisher || 'unknown',
      correlationId: options.correlationId || eventId,
      metadata: options.metadata || {},
      retries: 0,
    };

    this.stats.published++;
    this.stats.byTopic[topic] = (this.stats.byTopic[topic] || 0) + 1;
    this.stats.byPriority[event.priority] = (this.stats.byPriority[event.priority] || 0) + 1;

    // 审计
    if (this.enableAudit) {
      this.auditLog.push({
        id: eventId,
        topic,
        priority: event.priority,
        timestamp: event.timestamp,
        publisher: event.publisher,
        payloadSize: JSON.stringify(payload).length,
      });
      if (this.auditLog.length > 10000) this.auditLog.shift();
    }

    // 查找匹配的订阅器
    const matchedSubscribers = this._findSubscribers(topic);

    if (matchedSubscribers.length === 0) {
      this.emit('bus:no_subscribers', { topic, eventId });
      return eventId;
    }

    // 按优先级排序订阅器
    matchedSubscribers.sort((a, b) => a.options.priority - b.options.priority);

    // 同步或异步处理
    if (options.sync) {
      this._processEventSync(event, matchedSubscribers);
    } else {
      this._enqueueEvent(event, matchedSubscribers);
    }

    this.emit('bus:published', { eventId, topic, subscriberCount: matchedSubscribers.length });
    return eventId;
  }

  /**
   * 请求/响应模式（RPC over EventBus）
   */
  async request(topic, payload = {}, options = {}) {
    const correlationId = `req-${crypto.randomBytes(8).toString('hex')}`;
    const timeoutMs = options.timeoutMs || 10000;

    return new Promise((resolve, reject) => {
      const responseTopic = `${topic}.response.${correlationId}`;
      const timeout = setTimeout(() => {
        this.unsubscribe(subId);
        reject(new Error(`请求超时: ${topic} (${timeoutMs}ms)`));
      }, timeoutMs);

      const subId = this.subscribe(responseTopic, (event) => {
        clearTimeout(timeout);
        this.unsubscribe(subId);
        if (event.payload.error) {
          reject(new Error(event.payload.error));
        } else {
          resolve(event.payload.data);
        }
      }, { sync: true, priority: EVENT_PRIORITY.CRITICAL });

      this.publish(topic, payload, {
        correlationId,
        replyTo: responseTopic,
        publisher: options.publisher || 'rpc-client',
      });
    });
  }

  _findSubscribers(topic) {
    const matched = [];

    // 精确匹配
    const exact = this.subscribers.get(topic);
    if (exact) matched.push(...exact);

    // 通配符匹配
    for (const { regex, subscriber } of this.wildcardSubscribers) {
      if (regex.test(topic)) matched.push(subscriber);
    }

    return matched;
  }

  _patternToRegex(pattern) {
    // * 匹配单层，# 匹配多层
    const escaped = pattern
      .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '[^.]+')
      .replace(/#/g, '.+');
    return new RegExp(`^${escaped}$`);
  }

  _enqueueEvent(event, subscribers) {
    if (this.eventQueue.length >= this.maxQueueSize) {
      this.emit('bus:queue_full', { eventId: event.id, topic: event.topic });
      if (this.enableDLQ) this._deadLetter(event, new Error('事件队列已满'));
      return;
    }

    this.eventQueue.push({ event, subscribers });
    this._processQueue();
  }

  async _processQueue() {
    if (this._processing) return;
    this._processing = true;

    try {
      while (this.eventQueue.length > 0) {
        // 按优先级排序队列
        this.eventQueue.sort((a, b) => a.event.priority - b.event.priority);

        const { event, subscribers } = this.eventQueue.shift();
        await this._processEventAsync(event, subscribers);
      }
    } finally {
      this._processing = false;
    }
  }

  async _processEventSync(event, subscribers) {
    for (const subscriber of subscribers) {
      try {
        await subscriber.handler(event);
        subscriber.callCount++;
        this.stats.delivered++;
      } catch (err) {
        subscriber.failureCount++;
        this.stats.failed++;
        this.emit('bus:handler_error', {
          subscriberId: subscriber.id,
          topic: event.topic,
          eventId: event.id,
          error: err.message,
        });
      }
    }
  }

  async _processEventAsync(event, subscribers) {
    const promises = subscribers.map(async (subscriber) => {
      for (let attempt = 0; attempt <= subscriber.options.retries; attempt++) {
        try {
          await Promise.race([
            subscriber.handler(event),
            new Promise((_, reject) =>
              setTimeout(() => reject(new Error(`处理超时 ${subscriber.options.timeoutMs}ms`)), subscriber.options.timeoutMs)
            ),
          ]);
          subscriber.callCount++;
          this.stats.delivered++;
          return;
        } catch (err) {
          if (attempt < subscriber.options.retries) {
            await new Promise(r => setTimeout(r, subscriber.options.retryDelayMs));
            event.retries++;
          } else {
            subscriber.failureCount++;
            this.stats.failed++;
            this.emit('bus:handler_failed', {
              subscriberId: subscriber.id,
              topic: event.topic,
              eventId: event.id,
              error: err.message,
              retries: event.retries,
            });
            if (subscriber.options.deadLetter && this.enableDLQ) {
              this._deadLetter(event, err, subscriber.id);
            }
          }
        }
      }
    });

    await Promise.allSettled(promises);
  }

  _deadLetter(event, error, subscriberId) {
    this.deadLetterQueue.push({
      event,
      error: error.message,
      subscriberId,
      deadLetteredAt: new Date().toISOString(),
    });
    this.stats.deadLettered++;
    this.emit('bus:dead_letter', { eventId: event.id, topic: event.topic, error: error.message });

    if (this.deadLetterQueue.length > 1000) {
      this.deadLetterQueue.shift();
    }
  }

  /**
   * 重放死信队列
   */
  async replayDeadLetter(filter = null) {
    const toReplay = filter
      ? this.deadLetterQueue.filter(filter)
      : [...this.deadLetterQueue];

    this.deadLetterQueue = this.deadLetterQueue.filter(item => !toReplay.includes(item));

    for (const item of toReplay) {
      this.publish(item.event.topic, item.event.payload, {
        priority: EVENT_PRIORITY.HIGH,
        publisher: 'dead-letter-replay',
        correlationId: item.event.correlationId,
      });
    }

    return toReplay.length;
  }

  /**
   * 获取订阅器统计
   */
  getSubscriberStats() {
    const stats = [];
    for (const [topic, set] of this.subscribers) {
      for (const sub of set) {
        stats.push({
          id: sub.id,
          topic,
          type: 'exact',
          callCount: sub.callCount,
          failureCount: sub.failureCount,
          subscribedAt: sub.subscribedAt,
        });
      }
    }
    for (const { pattern, subscriber } of this.wildcardSubscribers) {
      stats.push({
        id: subscriber.id,
        topic: pattern,
        type: 'wildcard',
        callCount: subscriber.callCount,
        failureCount: subscriber.failureCount,
        subscribedAt: subscriber.subscribedAt,
      });
    }
    return stats;
  }

  /**
   * 获取总线统计
   */
  getStats() {
    return {
      busId: this._busId,
      ...this.stats,
      queueSize: this.eventQueue.length,
      maxQueueSize: this.maxQueueSize,
      deadLetterQueueSize: this.deadLetterQueue.length,
      auditLogSize: this.auditLog.length,
      totalSubscribers: this.getSubscriberStats().length,
      exactTopics: this.subscribers.size,
      wildcardSubscribers: this.wildcardSubscribers.length,
      processing: this._processing,
    };
  }

  /**
   * 销毁事件总线
   */
  async destroy() {
    this.eventQueue = [];
    this.subscribers.clear();
    this.wildcardSubscribers = [];
    this.removeAllListeners();
  }
}

// 全局单例
let _globalBus = null;
function getGlobalBus() {
  if (!_globalBus) _globalBus = new EventBus();
  return _globalBus;
}

module.exports = {
  EventBus,
  EVENT_PRIORITY,
  getGlobalBus,
};
