'use strict';

/**
 * 会话与专家链仓储（infrastructure 层）
 * ------------------------------------------------------------------
 * 职责：会话（session）与专家链（sessionChain）的内存态管理。
 * 会话上限 MAX_SESSION_HISTORY 条，超限淘汰最旧会话。
 */
const crypto = require('crypto');

const MAX_SESSION_HISTORY = 1000;

class SessionChainStore {
  constructor() {
    this.sessions = new Map();
    this.sessionChains = new Map();
  }

  // ===== 专家链 =====

  createChain(name, expertIds, options = {}) {
    const chain = {
      id: `chain_${crypto.randomUUID ? crypto.randomUUID() : 'chain_' + Date.now()}`,
      name,
      experts: expertIds,
      mode: options.mode || 'sequential',
      created_at: new Date().toISOString(),
      interactions: [],
      status: 'created'
    };
    this.sessionChains.set(chain.id, chain);
    return chain;
  }

  getChain(id) {
    return this.sessionChains.get(id);
  }

  listChains() {
    return Array.from(this.sessionChains.values());
  }

  chainCount() {
    return this.sessionChains.size;
  }

  // ===== 会话 =====

  createSession(options = {}) {
    const sessionId = `sess_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const session = {
      id: sessionId,
      title: options.title || '新会话',
      mode: options.mode || 'single',
      current_expert: options.currentExpert || null,
      messages: [],
      metadata: {
        created_by: options.createdBy || 'user',
        total_rounds: 0,
        expert_chain: []
      },
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    this.sessions.set(sessionId, session);

    if (this.sessions.size > MAX_SESSION_HISTORY) {
      const oldest = this.sessions.keys().next().value;
      this.sessions.delete(oldest);
    }

    return session;
  }

  getSession(sessionId) {
    return this.sessions.get(sessionId);
  }

  listSessions() {
    return Array.from(this.sessions.values())
      .sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));
  }

  appendMessage(sessionId, message) {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    session.messages.push({
      ...message,
      timestamp: message.timestamp || new Date().toISOString()
    });
    session.metadata.total_rounds = session.messages.filter(m => m.role === 'user').length;
    session.updated_at = new Date().toISOString();

    return session;
  }
}

module.exports = { SessionChainStore, MAX_SESSION_HISTORY };
