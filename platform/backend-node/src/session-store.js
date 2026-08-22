'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');
const SESSIONS_FILE = 'expert_sessions.json';
const HISTORY_FILE = 'expert_chat_history.json';
const MAX_SESSION_MESSAGES = 500;
const MAX_HISTORY_RECORDS = 5000;
const EMBEDDING_DIM = 384;

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
    console.error('[session-store] writeJSON', file, e.message);
    return false;
  }
}

function cosineSimilarity(vecA, vecB) {
  if (vecA.length !== vecB.length) return 0;
  let dotProduct = 0;
  let normA = 0;
  let normB = 0;
  for (let i = 0; i < vecA.length; i++) {
    dotProduct += vecA[i] * vecB[i];
    normA += vecA[i] * vecA[i];
    normB += vecB[i] * vecB[i];
  }
  if (normA === 0 || normB === 0) return 0;
  return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}

function simpleHashEmbedding(text) {
  const hash = crypto.createHash('sha256').update(text || '').digest();
  const baseEmbedding = new Array(EMBEDDING_DIM).fill(0);
  const stride = Math.floor(32 / (EMBEDDING_DIM / 8));
  for (let i = 0; i < 32 && i * stride < EMBEDDING_DIM; i++) {
    const bucket = Math.min(Math.floor((i / 32) * EMBEDDING_DIM), EMBEDDING_DIM - 1);
    baseEmbedding[bucket] = (hash[i] / 255) * 2 - 1;
    if (bucket + 1 < EMBEDDING_DIM) {
      baseEmbedding[bucket + 1] = ((hash[i + 1] || 0) / 255) * 2 - 1;
    }
  }
  const norm = Math.sqrt(baseEmbedding.reduce((s, v) => s + v * v, 0));
  return norm > 0 ? baseEmbedding.map(v => v / norm) : baseEmbedding;
}

class SessionStore {
  constructor() {
    this.sessions = new Map();
    this.history = [];
    this.vectorIndex = [];
    this._init();
  }

  _init() {
    const sessions = readJSON(SESSIONS_FILE, []);
    sessions.forEach(s => this.sessions.set(s.id, s));

    const history = readJSON(HISTORY_FILE, []);
    this.history = history.slice(-MAX_HISTORY_RECORDS);

    this.vectorIndex = this.history.map(record => ({
      id: record.id,
      session_id: record.session_id,
      expert_id: record.expert_id,
      embedding: simpleHashEmbedding(record.question || ''),
      timestamp: record.timestamp
    }));
  }

  _persistSessions() {
    writeJSON(SESSIONS_FILE, Array.from(this.sessions.values()));
  }

  _persistHistory() {
    writeJSON(HISTORY_FILE, this.history);
  }

  createSession(options = {}) {
    const id = `sess_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`;
    const session = {
      id,
      title: options.title || '新对话',
      mode: options.mode || 'single',
      current_expert: options.currentExpert || null,
      expert_chain: options.expertChain || [],
      messages: [],
      metadata: {
        created_by: options.createdBy || 'user',
        total_rounds: 0,
        tags: options.tags || [],
        problem_context: options.problemContext || null,
        business_constraints: options.businessConstraints || null
      },
      status: 'active',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      last_activity_at: new Date().toISOString()
    };
    this.sessions.set(id, session);
    this._persistSessions();
    return session;
  }

  getSession(sessionId) {
    return this.sessions.get(sessionId) || null;
  }

  listSessions(filters = {}) {
    let result = Array.from(this.sessions.values());

    if (filters.status) result = result.filter(s => s.status === filters.status);
    if (filters.mode) result = result.filter(s => s.mode === filters.mode);
    if (filters.expertId) result = result.filter(s => s.expert_chain.includes(filters.expertId));
    if (filters.keyword) {
      const kw = filters.keyword.toLowerCase();
      result = result.filter(s =>
        s.title.toLowerCase().includes(kw) ||
        s.messages.some(m => (m.content || '').toLowerCase().includes(kw))
      );
    }

    return result.sort((a, b) => new Date(b.updated_at) - new Date(a.updated_at));
  }

  updateSession(sessionId, updates) {
    const session = this.sessions.get(sessionId);
    if (!session) return null;
    Object.assign(session, updates, { updated_at: new Date().toISOString() });
    this._persistSessions();
    return session;
  }

  appendMessage(sessionId, message) {
    const session = this.sessions.get(sessionId);
    if (!session) return null;

    const msg = {
      role: 'user', // 企业级默认：未显式指定角色时按用户消息处理（否则轮次统计与历史入库静默失效）
      ...message,
      id: `msg_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      timestamp: new Date().toISOString()
    };
    session.messages.push(msg);

    if (session.messages.length > MAX_SESSION_MESSAGES) {
      session.messages = session.messages.slice(-MAX_SESSION_MESSAGES);
    }

    session.metadata.total_rounds = session.messages.filter(m => m.role === 'user').length;
    session.updated_at = new Date().toISOString();
    session.last_activity_at = new Date().toISOString();
    this._persistSessions();

    if (msg.role === 'user') {
      this._addToHistory(session.id, msg.content, session.current_expert || 'unknown');
    }

    return msg;
  }

  _addToHistory(sessionId, question, expertId) {
    const record = {
      id: `hist_${crypto.randomUUID ? crypto.randomUUID() : Date.now()}`,
      session_id: sessionId,
      expert_id: expertId,
      question,
      embedding: simpleHashEmbedding(question),
      timestamp: new Date().toISOString()
    };
    this.history.push(record);
    this.vectorIndex.push({
      id: record.id,
      session_id: sessionId,
      expert_id: expertId,
      embedding: record.embedding,
      timestamp: record.timestamp
    });

    if (this.history.length > MAX_HISTORY_RECORDS) {
      this.history = this.history.slice(-MAX_HISTORY_RECORDS);
      this.vectorIndex = this.vectorIndex.slice(-MAX_HISTORY_RECORDS);
    }

    this._persistHistory();
  }

  async semanticSearch(query, options = {}) {
    const queryEmbedding = simpleHashEmbedding(query);
    const threshold = options.threshold || 0.3;
    const limit = options.limit || 10;

    const results = this.vectorIndex.map(record => ({
      record,
      similarity: cosineSimilarity(queryEmbedding, record.embedding)
    }))
      .filter(r => r.similarity >= threshold)
      .sort((a, b) => b.similarity - a.similarity)
      .slice(0, limit);

    return results.map(r => ({
      id: r.record.id,
      session_id: r.record.session_id,
      expert_id: r.record.expert_id,
      question: this.history.find(h => h.id === r.record.id)?.question || '',
      similarity: Math.round(r.similarity * 10000) / 10000,
      timestamp: r.record.timestamp
    }));
  }

  async findRelevantHistory(sessionId, question, options = {}) {
    const session = this.sessions.get(sessionId);
    if (!session) return { context_messages: [], similar_history: [] };

    const recentMessages = session.messages.slice(-options.recentCount || 10);
    const similarHistory = await this.semanticSearch(question, {
      threshold: options.threshold || 0.25,
      limit: options.limit || 5
    });

    return {
      context_messages: recentMessages.filter(m => m.role !== 'system'),
      similar_history: similarHistory,
      session_summary: this._summarizeSession(session)
    };
  }

  _summarizeSession(session) {
    const userMessages = session.messages.filter(m => m.role === 'user');
    const expertMessages = session.messages.filter(m => m.role === 'assistant');

    const topics = new Set();
    userMessages.forEach(m => {
      const content = (m.content || '').toLowerCase();
      if (content.includes('架构')) topics.add('架构');
      if (content.includes('算法')) topics.add('算法');
      if (content.includes('性能')) topics.add('性能');
      if (content.includes('数据')) topics.add('数据');
      if (content.includes('安全')) topics.add('安全');
      if (content.includes('AI') || content.includes('ai')) topics.add('AI');
      if (content.includes('图谱')) topics.add('图谱');
    });

    return {
      total_messages: session.messages.length,
      user_turns: userMessages.length,
      expert_responses: expertMessages.length,
      topics: Array.from(topics),
      active_experts: [...new Set(session.expert_chain)],
      duration: session.messages.length > 0
        ? `${session.messages.length * 30}秒`
        : '0秒'
    };
  }

  getSessionStats() {
    const sessions = Array.from(this.sessions.values());
    const totalMessages = sessions.reduce((sum, s) => sum + s.messages.length, 0);
    const activeSessions = sessions.filter(s => s.status === 'active').length;
    const modeDistribution = {};
    sessions.forEach(s => {
      modeDistribution[s.mode] = (modeDistribution[s.mode] || 0) + 1;
    });

    return {
      total_sessions: sessions.length,
      active_sessions: activeSessions,
      total_messages: totalMessages,
      history_records: this.history.length,
      vector_index_size: this.vectorIndex.length,
      mode_distribution: modeDistribution,
      storage_size_estimate: Math.round(JSON.stringify(sessions).length / 1024)
    };
  }

  deleteSession(sessionId) {
    const session = this.sessions.get(sessionId);
    if (!session) return false;

    const sessionMessages = session.messages.map(m => m.id);
    this.history = this.history.filter(h => h.session_id !== sessionId);
    this.vectorIndex = this.vectorIndex.filter(v => v.session_id !== sessionId);

    this.sessions.delete(sessionId);
    this._persistSessions();
    this._persistHistory();
    return true;
  }

  archiveSession(sessionId) {
    return this.updateSession(sessionId, { status: 'archived' });
  }

  exportSession(sessionId) {
    const session = this.sessions.get(sessionId);
    if (!session) return null;
    return {
      id: session.id,
      title: session.title,
      mode: session.mode,
      messages: session.messages,
      metadata: session.metadata,
      created_at: session.created_at,
      exported_at: new Date().toISOString()
    };
  }
}

let storeInstance = null;

function getSessionStore() {
  if (!storeInstance) {
    storeInstance = new SessionStore();
  }
  return storeInstance;
}

module.exports = { SessionStore, getSessionStore };
