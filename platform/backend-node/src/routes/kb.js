'use strict';

/**
 * 路由域：知识库
 * /kb/* 文档、版本、实体抽取、语义搜索、图谱关联
 */
module.exports = function registerKbRoutes(ctx) {
  const { url, uid, p, readJSON, writeJSON, ok, fail, readBody, log, appendLog, reg } = ctx;

  function ensureKBCategories() {
    const cats = readJSON('kb_categories.json', null);
    if (cats && Array.isArray(cats) && cats.length > 0) return cats;
    const defaults = [
      { id: 'general', name: '通用', parent: null, count: 0 },
      { id: 'tech', name: '技术文档', parent: null, count: 0 },
      { id: 'tech.code', name: '代码', parent: 'tech', count: 0 },
      { id: 'tech.architecture', name: '架构', parent: 'tech', count: 0 },
      { id: 'business', name: '业务文档', parent: null, count: 0 },
      { id: 'business.requirement', name: '需求', parent: 'business', count: 0 },
      { id: 'business.process', name: '流程', parent: 'business', count: 0 },
      { id: 'design', name: '设计文档', parent: null, count: 0 },
      { id: 'design.ui', name: 'UI设计', parent: 'design', count: 0 },
      { id: 'design.spec', name: '规范', parent: 'design', count: 0 },
      { id: 'research', name: '研究文档', parent: null, count: 0 },
      { id: 'meeting', name: '会议纪要', parent: null, count: 0 },
      { id: 'policy', name: '政策制度', parent: null, count: 0 }
    ];
    writeJSON('kb_categories.json', defaults);
    return defaults;
  }

  function analyzeDocument(doc) {
    const content = doc.content || '';
    const title = doc.title || '';
    const text = (title + ' ' + content).toLowerCase();
    const wordCount = content.trim() ? content.trim().split(/\s+/).length : 0;
    const readingTime = Math.ceil(wordCount / 200);
    const entities = [];
    const entityPatterns = [
      { type: 'technical', regex: /\b(algorithm|api|sdk|framework|library|module|function|class|method|database|server|client|interface|protocol|system)\b/gi },
      { type: 'person', regex: /\b(?:dr|mr|mrs|ms|prof|professor|director|manager|engineer|designer|analyst)\s+[a-z][a-z\s]+?(?:\.|,|\s{2,}|$)/gi },
      { type: 'date', regex: /\b\d{4}[-/]\d{1,2}[-/]\d{1,2}\b/g },
      { type: 'system', regex: /\b([A-Z][a-z]+(?:[A-Z][a-z]+)+|[A-Z]{2,}(?:[a-z]+|[A-Z]+))\b/g }
    ];
    entityPatterns.forEach(ep => {
      const matches = text.match(ep.regex) || [];
      matches.forEach(m => {
        if (m.trim()) entities.push({ type: ep.type, value: m.trim(), confidence: 0.7 + Math.random() * 0.3 });
      });
    });
    const uniqueEntities = [];
    const seen = {};
    entities.forEach(e => { if (!seen[e.value]) { seen[e.value] = true; uniqueEntities.push(e); } });
    const summary = content.length > 300 ? content.slice(0, 300) + '...' : content;
    const keywordScores = {};
    uniqueEntities.forEach(e => { keywordScores[e.value] = e.confidence; });
    const catKeywords = {
      'tech': ['algorithm', 'api', 'code', 'function', 'class', 'system', 'module', 'library', 'framework'],
      'business': ['requirement', 'process', 'business', 'workflow', 'stakeholder', 'delivery'],
      'design': ['design', 'ui', 'spec', 'pattern', 'interface', 'ux', 'prototype'],
      'research': ['research', 'analysis', 'study', 'experiment', 'finding', 'hypothesis'],
      'meeting': ['meeting', 'discussion', 'agenda', 'minutes', 'action', 'decision'],
      'policy': ['policy', 'regulation', 'compliance', 'standard', 'rule', 'governance']
    };
    let suggestedCategory = doc.category || 'general';
    let bestScore = 0;
    Object.keys(catKeywords).forEach(cat => {
      const score = catKeywords[cat].reduce((s, kw) => s + (text.indexOf(kw) !== -1 ? 1 : 0), 0);
      if (score > bestScore) { bestScore = score; suggestedCategory = cat; }
    });
    const suggestedTags = uniqueEntities.slice(0, 5).map(e => e.value.toLowerCase()).filter((t, i, arr) => arr.indexOf(t) === 0 && t.length > 2);
    return {
      keywords: Object.keys(keywordScores).slice(0, 10),
      entities: uniqueEntities,
      summary: summary,
      suggestedCategory: suggestedCategory,
      suggestedTags: suggestedTags,
      wordCount: wordCount,
      readingTime: readingTime,
      confidence: Math.min(0.95, 0.5 + uniqueEntities.length * 0.05),
      analyzedAt: new Date().toISOString()
    };
  }

  function extractEntitiesFromContent(content) {
    const text = (content || '').toLowerCase();
    const entities = [];
    const patterns = [
      { type: 'technical_term', regex: /\b(algorithm|api|sdk|framework|library|module|function|class|method|database|server|client|interface|protocol|system|architecture)\b/gi },
      { type: 'date', regex: /\b\d{4}[-/]\d{1,2}[-/]\d{1,2}\b/g },
      { type: 'system_name', regex: /\b([A-Z][a-z]+[A-Z][a-z]+|[A-Z]{2,}[a-z]+|[A-Z][a-z]+[A-Z][a-z]+)\b/g },
      { type: 'organization', regex: /\b([A-Z][a-z]+(?:\s[A-Z][a-z]+)*(?:Inc|Corp|LLC|Ltd|Co))\b/g }
    ];
    patterns.forEach(p => {
      const matches = text.match(p.regex) || [];
      matches.forEach(m => {
        const v = m.trim();
        if (v && v.length > 1) entities.push({ type: p.type, value: v, confidence: 0.7 + Math.random() * 0.3 });
      });
    });
    const seen = {};
    return entities.filter(e => { if (seen[e.value]) return false; seen[e.value] = true; return true; });
  }

  function diffVersions(ver1, ver2) {
    const lines1 = (ver1.content || '').split('\n');
    const lines2 = (ver2.content || '').split('\n');
    const lcs = [];
    for (let i = 0; i <= lines1.length; i++) {
      lcs[i] = [];
      for (let j = 0; j <= lines2.length; j++) lcs[i][j] = 0;
    }
    for (let i = 1; i <= lines1.length; i++) {
      for (let j = 1; j <= lines2.length; j++) {
        if (lines1[i - 1] === lines2[j - 1]) lcs[i][j] = lcs[i - 1][j - 1] + 1;
        else lcs[i][j] = Math.max(lcs[i - 1][j], lcs[i][j - 1]);
      }
    }
    const added = [];
    const removed = [];
    let i = lines1.length, j = lines2.length;
    while (i > 0 && j > 0) {
      if (lines1[i - 1] === lines2[j - 1]) { i--; j--; }
      else if (lcs[i - 1][j] >= lcs[i][j - 1]) { removed.unshift(lines1[i - 1]); i--; }
      else { added.unshift(lines2[j - 1]); j--; }
    }
    while (i > 0) { removed.unshift(lines1[i - 1]); i--; }
    while (j > 0) { added.unshift(lines2[j - 1]); j--; }
    const total = Math.max(lines1.length, lines2.length);
    const similarity = total > 0 ? Math.round((lcs[lines1.length][lines2.length] / total) * 1000) / 10 : 0;
    return { added: added, removed: removed, changed: [], similarity: similarity, fromVersion: ver1.version, toVersion: ver2.version };
  }

  function addHistory(docId, action, detail, user) {
    const history = readJSON('kb_history.json', []);
    history.unshift({
      id: uid('kb_hist'),
      documentId: docId,
      action: action,
      detail: detail,
      user: user || 'user',
      ts: new Date().toISOString()
    });
    if (history.length > 1000) history.length = 1000;
    writeJSON('kb_history.json', history);
  }

  // === 1. Document CRUD ===

  reg('get', '/kb/documents', (req, res) => {
    const q = url.parse(req.url, true).query;
    let docs = readJSON('kb_documents.json', []);
    if (q.q) {
      const s = String(q.q).toLowerCase();
      docs = docs.filter(d =>
        (d.title || '').toLowerCase().indexOf(s) !== -1 ||
        (d.content || '').toLowerCase().indexOf(s) !== -1 ||
        (d.description || '').toLowerCase().indexOf(s) !== -1 ||
        (d.tags || []).some(t => t.toLowerCase().indexOf(s) !== -1)
      );
    }
    if (q.category) docs = docs.filter(d => d.category === q.category);
    if (q.tag) docs = docs.filter(d => (d.tags || []).indexOf(q.tag) !== -1);
    if (q.type) docs = docs.filter(d => d.type === q.type);
    if (q.status) docs = docs.filter(d => d.status === q.status);
    const page = parseInt(q.page, 10) || 1;
    const pageSize = parseInt(q.pageSize, 10) || 20;
    const total = docs.length;
    const start = (page - 1) * pageSize;
    const paged = docs.slice(start, start + pageSize);
    ok(res, { documents: paged, pagination: { page: page, pageSize: pageSize, total: total, totalPages: Math.ceil(total / pageSize) } });
  });

  reg('post', '/kb/documents', async (req, res) => {
    const body = await readBody(req);
    const docs = readJSON('kb_documents.json', []);
    const now = new Date().toISOString();
    const doc = Object.assign({
      id: uid('kb_doc'),
      title: '未命名文档',
      content: '',
      type: 'markdown',
      category: 'general',
      tags: [],
      description: '',
      status: 'active',
      version: 1,
      currentVersionId: null,
      aiAnalysis: null,
      entities: [],
      graphLinks: [],
      metadata: {},
      created_by: 'user',
      created_at: now,
      updated_at: now
    }, body);
    const versions = readJSON('kb_versions.json', []);
    const initVersionId = uid('kb_ver');
    const initVersion = {
      id: initVersionId,
      documentId: doc.id,
      version: 1,
      content: doc.content,
      title: doc.title,
      changeNote: '初始版本',
      isAI: false,
      created_by: doc.created_by || 'user',
      created_at: now,
      diff: null
    };
    versions.unshift(initVersion);
    writeJSON('kb_versions.json', versions);
    doc.currentVersionId = initVersionId;
    docs.unshift(doc);
    writeJSON('kb_documents.json', docs);
    addHistory(doc.id, 'create', '创建文档: ' + doc.title);
    appendLog({ type: 'kb', msg: 'create document', id: doc.id });
    ok(res, doc);
  });

  reg('get', '/kb/documents/:id', (req, res, params) => {
    const docs = readJSON('kb_documents.json', []);
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    ok(res, doc);
  });

  reg('put', '/kb/documents/:id', async (req, res, params) => {
    const body = await readBody(req);
    const docs = readJSON('kb_documents.json', []);
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    const doc = docs[idx];
    const versions = readJSON('kb_versions.json', []);
    const versionId = uid('kb_ver');
    const version = {
      id: versionId,
      documentId: doc.id,
      version: doc.version + 1,
      content: doc.content,
      title: doc.title,
      changeNote: '更新前的版本快照',
      isAI: false,
      created_by: doc.created_by || 'user',
      created_at: new Date().toISOString(),
      diff: null
    };
    versions.unshift(version);
    writeJSON('kb_versions.json', versions);
    docs[idx] = Object.assign({}, doc, body, {
      id: params.id,
      version: doc.version + 1,
      currentVersionId: versionId,
      updated_at: new Date().toISOString()
    });
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'update', '更新文档: ' + (body.title || doc.title));
    appendLog({ type: 'kb', msg: 'update document', id: params.id, version: docs[idx].version });
    ok(res, docs[idx]);
  });

  reg('delete', '/kb/documents/:id', (req, res, params) => {
    const docs = readJSON('kb_documents.json', []);
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    docs[idx].status = 'deleted';
    docs[idx].updated_at = new Date().toISOString();
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'delete', '删除文档: ' + docs[idx].title);
    appendLog({ type: 'kb', msg: 'delete document (soft)', id: params.id });
    ok(res, { success: true, id: params.id, status: 'deleted' });
  });

  // === 2. Version Management ===

  reg('get', '/kb/documents/:id/versions', (req, res, params) => {
    const versions = readJSON('kb_versions.json', []);
    const docVersions = versions.filter(v => v.documentId === params.id).sort((a, b) => b.version - a.version);
    ok(res, docVersions);
  });

  reg('get', '/kb/documents/:id/versions/:ver', (req, res, params) => {
    const versions = readJSON('kb_versions.json', []);
    const ver = versions.find(v => v.documentId === params.id && String(v.version) === String(params.ver));
    if (!ver) return fail(res, 404, '版本不存在');
    ok(res, ver);
  });

  reg('post', '/kb/documents/:id/versions', async (req, res, params) => {
    const body = await readBody(req);
    const docs = readJSON('kb_documents.json', []);
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const versions = readJSON('kb_versions.json', []);
    const maxVer = versions.filter(v => v.documentId === params.id).reduce((m, v) => Math.max(m, v.version), 0);
    const newVersion = {
      id: uid('kb_ver'),
      documentId: params.id,
      version: maxVer + 1,
      content: body.content || doc.content,
      title: body.title || doc.title,
      changeNote: body.changeNote || '手动创建版本',
      isAI: body.isAI || false,
      created_by: body.created_by || 'user',
      created_at: new Date().toISOString(),
      diff: null
    };
    versions.unshift(newVersion);
    writeJSON('kb_versions.json', versions);
    const docIdx = docs.findIndex(d => d.id === params.id);
    docs[docIdx].version = newVersion.version;
    docs[docIdx].currentVersionId = newVersion.id;
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'version', '创建版本 v' + newVersion.version);
    appendLog({ type: 'kb', msg: 'create version', docId: params.id, version: newVersion.version });
    ok(res, newVersion);
  });

  reg('post', '/kb/documents/:id/versions/compare', async (req, res, params) => {
    const body = await readBody(req);
    if (!body.fromVer || !body.toVer) return fail(res, 400, 'fromVer 和 toVer 为必填');
    const versions = readJSON('kb_versions.json', []);
    const ver1 = versions.find(v => v.documentId === params.id && String(v.version) === String(body.fromVer));
    const ver2 = versions.find(v => v.documentId === params.id && String(v.version) === String(body.toVer));
    if (!ver1 || !ver2) return fail(res, 404, '版本不存在');
    const diff = diffVersions(ver1, ver2);
    ok(res, {
      from: { version: ver1.version, title: ver1.title, content: ver1.content },
      to: { version: ver2.version, title: ver2.title, content: ver2.content },
      diff: diff
    });
  });

  reg('post', '/kb/documents/:id/versions/revert', async (req, res, params) => {
    const body = await readBody(req);
    if (!body.version) return fail(res, 400, 'version 为必填');
    const docs = readJSON('kb_documents.json', []);
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const versions = readJSON('kb_versions.json', []);
    const targetVer = versions.find(v => v.documentId === params.id && String(v.version) === String(body.version));
    if (!targetVer) return fail(res, 404, '版本不存在');
    const idx = docs.findIndex(d => d.id === params.id);
    docs[idx].content = targetVer.content;
    docs[idx].title = targetVer.title;
    docs[idx].version = doc.version + 1;
    docs[idx].currentVersionId = targetVer.id;
    docs[idx].updated_at = new Date().toISOString();
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'revert', '回退到版本 v' + targetVer.version);
    appendLog({ type: 'kb', msg: 'revert version', docId: params.id, toVersion: targetVer.version });
    ok(res, docs[idx]);
  });

  // === 3. AI Analysis & Classification ===

  reg('post', '/kb/documents/:id/analyze', async (req, res, params) => {
    const docs = readJSON('kb_documents.json', []);
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    const analysis = analyzeDocument(docs[idx]);
    docs[idx].aiAnalysis = analysis;
    docs[idx].entities = analysis.entities || [];
    if (analysis.suggestedCategory && analysis.suggestedCategory !== docs[idx].category) {
      docs[idx].category = analysis.suggestedCategory;
    }
    if (analysis.suggestedTags && analysis.suggestedTags.length > 0) {
      const existingTags = docs[idx].tags || [];
      analysis.suggestedTags.forEach(t => { if (existingTags.indexOf(t) === -1) existingTags.push(t); });
      docs[idx].tags = existingTags;
    }
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'analyze', 'AI 分析文档完成');
    appendLog({ type: 'kb', msg: 'analyze document', id: params.id });
    ok(res, { document: docs[idx], analysis: analysis });
  });

  reg('post', '/kb/batch-analyze', async (req, res) => {
    const body = await readBody(req);
    const docIds = body.docIds || [];
    if (docIds.length === 0) return fail(res, 400, 'docIds 列表为必填');
    const docs = readJSON('kb_documents.json', []);
    const results = [];
    docIds.forEach(id => {
      const idx = docs.findIndex(d => d.id === id);
      if (idx === -1) { results.push({ id: id, success: false, error: '文档不存在' }); return; }
      const analysis = analyzeDocument(docs[idx]);
      docs[idx].aiAnalysis = analysis;
      docs[idx].entities = analysis.entities || [];
      if (analysis.suggestedCategory) docs[idx].category = analysis.suggestedCategory;
      results.push({ id: id, success: true, analysis: analysis });
    });
    writeJSON('kb_documents.json', docs);
    addHistory('batch', 'analyze', '批量分析 ' + docIds.length + ' 个文档');
    appendLog({ type: 'kb', msg: 'batch analyze', count: docIds.length });
    ok(res, { total: docIds.length, results: results });
  });

  reg('get', '/kb/categories', (req, res) => {
    const cats = ensureKBCategories();
    ok(res, cats);
  });

  reg('get', '/kb/tags', (req, res) => {
    const docs = readJSON('kb_documents.json', []);
    const tagCounts = {};
    docs.filter(d => d.status !== 'deleted').forEach(d => {
      (d.tags || []).forEach(t => { tagCounts[t] = (tagCounts[t] || 0) + 1; });
    });
    const tags = Object.keys(tagCounts).map(t => ({ name: t, count: tagCounts[t] })).sort((a, b) => b.count - a.count);
    ok(res, tags);
  });

  reg('post', '/kb/search', async (req, res) => {
    const body = await readBody(req);
    const query = (body.query || '').toLowerCase();
    const filters = body.filters || {};
    if (!query) return fail(res, 400, 'query 为必填');
    let docs = readJSON('kb_documents.json', []);
    docs = docs.filter(d => d.status !== 'deleted');
    if (filters.category) docs = docs.filter(d => d.category === filters.category);
    if (filters.type) docs = docs.filter(d => d.type === filters.type);
    if (filters.tags && filters.tags.length) {
      docs = docs.filter(d => filters.tags.some(t => (d.tags || []).indexOf(t) !== -1));
    }
    const scored = docs.map(d => {
      const titleMatch = (d.title || '').toLowerCase();
      const contentMatch = (d.content || '').toLowerCase();
      const descMatch = (d.description || '').toLowerCase();
      let score = 0;
      if (titleMatch.indexOf(query) !== -1) score += 10;
      if (contentMatch.indexOf(query) !== -1) score += 5;
      if (descMatch.indexOf(query) !== -1) score += 3;
      if (d.tags && d.tags.some(t => t.toLowerCase().indexOf(query) !== -1)) score += 8;
      if (d.aiAnalysis && d.aiAnalysis.keywords) {
        d.aiAnalysis.keywords.forEach(k => { if (k.toLowerCase().indexOf(query) !== -1) score += 4; });
      }
      return { doc: d, score: score };
    });
    const results = scored.filter(s => s.score > 0).sort((a, b) => b.score - a.score);
    ok(res, { query: query, results: results.map(r => ({ document: r.doc, score: r.score })), total: results.length });
  });

  // === 4. Knowledge Graph Integration ===

  reg('get', '/kb/documents/:id/entities', (req, res, params) => {
    const docs = readJSON('kb_documents.json', []);
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const entities = extractEntitiesFromContent(doc.content || '');
    ok(res, { documentId: params.id, entities: entities, count: entities.length });
  });

  reg('post', '/kb/documents/:id/graph-link', async (req, res, params) => {
    const body = await readBody(req);
    const entityIds = body.entityIds || [];
    if (entityIds.length === 0) return fail(res, 400, 'entityIds 为必填');
    const docs = readJSON('kb_documents.json', []);
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    const existingLinks = docs[idx].graphLinks || [];
    entityIds.forEach(eid => { if (existingLinks.indexOf(eid) === -1) existingLinks.push(eid); });
    docs[idx].graphLinks = existingLinks;
    writeJSON('kb_documents.json', docs);
    addHistory(params.id, 'update', '关联图谱节点: ' + entityIds.join(', '));
    appendLog({ type: 'kb', msg: 'graph link', docId: params.id, entityIds: entityIds });
    ok(res, { success: true, documentId: params.id, graphLinks: docs[idx].graphLinks });
  });

  reg('get', '/kb/stats', (req, res) => {
    const docs = readJSON('kb_documents.json', []);
    const versions = readJSON('kb_versions.json', []);
    const activeDocs = docs.filter(d => d.status === 'active');
    const archivedDocs = docs.filter(d => d.status === 'archived');
    const deletedDocs = docs.filter(d => d.status === 'deleted');
    const catCounts = {};
    activeDocs.forEach(d => { catCounts[d.category] = (catCounts[d.category] || 0) + 1; });
    const totalWords = activeDocs.reduce((s, d) => s + (d.content || '').trim().split(/\s+/).length, 0);
    const linkedDocs = activeDocs.filter(d => (d.graphLinks || []).length > 0);
    const analyzedDocs = activeDocs.filter(d => d.aiAnalysis);
    ok(res, {
      total: docs.length,
      active: activeDocs.length,
      archived: archivedDocs.length,
      deleted: deletedDocs.length,
      categories: catCounts,
      versions: versions.length,
      analyzed: analyzedDocs.length,
      graphLinked: linkedDocs.length,
      totalWords: totalWords,
      lastUpdated: new Date().toISOString()
    });
  });

  // === 5. Change History ===

  reg('get', '/kb/documents/:id/history', (req, res, params) => {
    const history = readJSON('kb_history.json', []);
    const docHistory = history.filter(h => h.documentId === params.id).sort((a, b) => new Date(b.ts) - new Date(a.ts));
    ok(res, docHistory);
  });

  reg('get', '/kb/history', (req, res) => {
    const q = url.parse(req.url, true).query;
    let history = readJSON('kb_history.json', []);
    if (q.action) history = history.filter(h => h.action === q.action);
    if (q.documentId) history = history.filter(h => h.documentId === q.documentId);
    const page = parseInt(q.page, 10) || 1;
    const pageSize = parseInt(q.pageSize, 10) || 50;
    const total = history.length;
    const start = (page - 1) * pageSize;
    ok(res, { history: history.slice(start, start + pageSize), pagination: { page: page, pageSize: pageSize, total: total } });
  });

  log('Knowledge base endpoints registered: document CRUD, versions, AI analysis, graph integration, history');

};
