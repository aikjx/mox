'use strict';

/**
 * 路由域：知识库
 * /kb/* 文档、版本、实体抽取、语义搜索、图谱关联
 * 算法与存储已下沉至 kb/ 域包（domain 纯算法 + infrastructure 存储）。
 */
module.exports = function registerKbRoutes(ctx) {
  const { url, uid, ok, fail, readBody, log, appendLog, reg } = ctx;
  const kb = require('../kb');
  const { analyzeDocument, extractEntitiesFromContent, diffVersions } = kb;
  const {
    ensureCategories, addHistory,
    readDocuments, writeDocuments,
    readVersions, writeVersions
  } = kb.store;

  // === 1. Document CRUD ===

  reg('get', '/kb/documents', (req, res) => {
    const q = url.parse(req.url, true).query;
    let docs = readDocuments();
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
    const docs = readDocuments();
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
    const versions = readVersions();
    const initVersionId = uid('kb_ver');
    versions.unshift({
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
    });
    writeVersions(versions);
    doc.currentVersionId = initVersionId;
    docs.unshift(doc);
    writeDocuments(docs);
    addHistory(doc.id, 'create', '创建文档: ' + doc.title);
    appendLog({ type: 'kb', msg: 'create document', id: doc.id });
    ok(res, doc);
  });

  reg('get', '/kb/documents/:id', (req, res, params) => {
    const doc = readDocuments().find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    ok(res, doc);
  });

  reg('put', '/kb/documents/:id', async (req, res, params) => {
    const body = await readBody(req);
    const docs = readDocuments();
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    const doc = docs[idx];
    const versions = readVersions();
    const versionId = uid('kb_ver');
    versions.unshift({
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
    });
    writeVersions(versions);
    docs[idx] = Object.assign({}, doc, body, {
      id: params.id,
      version: doc.version + 1,
      currentVersionId: versionId,
      updated_at: new Date().toISOString()
    });
    writeDocuments(docs);
    addHistory(params.id, 'update', '更新文档: ' + (body.title || doc.title));
    appendLog({ type: 'kb', msg: 'update document', id: params.id, version: docs[idx].version });
    ok(res, docs[idx]);
  });

  reg('delete', '/kb/documents/:id', (req, res, params) => {
    const docs = readDocuments();
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    docs[idx].status = 'deleted';
    docs[idx].updated_at = new Date().toISOString();
    writeDocuments(docs);
    addHistory(params.id, 'delete', '删除文档: ' + docs[idx].title);
    appendLog({ type: 'kb', msg: 'delete document (soft)', id: params.id });
    ok(res, { success: true, id: params.id, status: 'deleted' });
  });

  // === 2. Version Management ===

  reg('get', '/kb/documents/:id/versions', (req, res, params) => {
    const docVersions = readVersions().filter(v => v.documentId === params.id).sort((a, b) => b.version - a.version);
    ok(res, docVersions);
  });

  reg('get', '/kb/documents/:id/versions/:ver', (req, res, params) => {
    const ver = readVersions().find(v => v.documentId === params.id && String(v.version) === String(params.ver));
    if (!ver) return fail(res, 404, '版本不存在');
    ok(res, ver);
  });

  reg('post', '/kb/documents/:id/versions', async (req, res, params) => {
    const body = await readBody(req);
    const docs = readDocuments();
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const versions = readVersions();
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
    writeVersions(versions);
    const docIdx = docs.findIndex(d => d.id === params.id);
    docs[docIdx].version = newVersion.version;
    docs[docIdx].currentVersionId = newVersion.id;
    writeDocuments(docs);
    addHistory(params.id, 'version', '创建版本 v' + newVersion.version);
    appendLog({ type: 'kb', msg: 'create version', docId: params.id, version: newVersion.version });
    ok(res, newVersion);
  });

  reg('post', '/kb/documents/:id/versions/compare', async (req, res, params) => {
    const body = await readBody(req);
    if (!body.fromVer || !body.toVer) return fail(res, 400, 'fromVer 和 toVer 为必填');
    const versions = readVersions();
    const ver1 = versions.find(v => v.documentId === params.id && String(v.version) === String(body.fromVer));
    const ver2 = versions.find(v => v.documentId === params.id && String(v.version) === String(body.toVer));
    if (!ver1 || !ver2) return fail(res, 404, '版本不存在');
    ok(res, {
      from: { version: ver1.version, title: ver1.title, content: ver1.content },
      to: { version: ver2.version, title: ver2.title, content: ver2.content },
      diff: diffVersions(ver1, ver2)
    });
  });

  reg('post', '/kb/documents/:id/versions/revert', async (req, res, params) => {
    const body = await readBody(req);
    if (!body.version) return fail(res, 400, 'version 为必填');
    const docs = readDocuments();
    const doc = docs.find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const targetVer = readVersions().find(v => v.documentId === params.id && String(v.version) === String(body.version));
    if (!targetVer) return fail(res, 404, '版本不存在');
    const idx = docs.findIndex(d => d.id === params.id);
    docs[idx].content = targetVer.content;
    docs[idx].title = targetVer.title;
    docs[idx].version = doc.version + 1;
    docs[idx].currentVersionId = targetVer.id;
    docs[idx].updated_at = new Date().toISOString();
    writeDocuments(docs);
    addHistory(params.id, 'revert', '回退到版本 v' + targetVer.version);
    appendLog({ type: 'kb', msg: 'revert version', docId: params.id, toVersion: targetVer.version });
    ok(res, docs[idx]);
  });

  // === 3. AI Analysis & Classification ===

  reg('post', '/kb/documents/:id/analyze', async (req, res, params) => {
    const docs = readDocuments();
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
    writeDocuments(docs);
    addHistory(params.id, 'analyze', 'AI 分析文档完成');
    appendLog({ type: 'kb', msg: 'analyze document', id: params.id });
    ok(res, { document: docs[idx], analysis: analysis });
  });

  reg('post', '/kb/batch-analyze', async (req, res) => {
    const body = await readBody(req);
    const docIds = body.docIds || [];
    if (docIds.length === 0) return fail(res, 400, 'docIds 列表为必填');
    const docs = readDocuments();
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
    writeDocuments(docs);
    addHistory('batch', 'analyze', '批量分析 ' + docIds.length + ' 个文档');
    appendLog({ type: 'kb', msg: 'batch analyze', count: docIds.length });
    ok(res, { total: docIds.length, results: results });
  });

  reg('get', '/kb/categories', (req, res) => {
    ok(res, ensureCategories());
  });

  reg('get', '/kb/tags', (req, res) => {
    const tagCounts = {};
    readDocuments().filter(d => d.status !== 'deleted').forEach(d => {
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
    let docs = readDocuments().filter(d => d.status !== 'deleted');
    if (filters.category) docs = docs.filter(d => d.category === filters.category);
    if (filters.type) docs = docs.filter(d => d.type === filters.type);
    if (filters.tags && filters.tags.length) {
      docs = docs.filter(d => filters.tags.some(t => (d.tags || []).indexOf(t) !== -1));
    }
    const scored = docs.map(d => {
      let score = 0;
      if ((d.title || '').toLowerCase().indexOf(query) !== -1) score += 10;
      if ((d.content || '').toLowerCase().indexOf(query) !== -1) score += 5;
      if ((d.description || '').toLowerCase().indexOf(query) !== -1) score += 3;
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
    const doc = readDocuments().find(d => d.id === params.id);
    if (!doc) return fail(res, 404, '文档不存在');
    const entities = extractEntitiesFromContent(doc.content || '');
    ok(res, { documentId: params.id, entities: entities, count: entities.length });
  });

  reg('post', '/kb/documents/:id/graph-link', async (req, res, params) => {
    const body = await readBody(req);
    const entityIds = body.entityIds || [];
    if (entityIds.length === 0) return fail(res, 400, 'entityIds 为必填');
    const docs = readDocuments();
    const idx = docs.findIndex(d => d.id === params.id);
    if (idx === -1) return fail(res, 404, '文档不存在');
    const existingLinks = docs[idx].graphLinks || [];
    entityIds.forEach(eid => { if (existingLinks.indexOf(eid) === -1) existingLinks.push(eid); });
    docs[idx].graphLinks = existingLinks;
    writeDocuments(docs);
    addHistory(params.id, 'update', '关联图谱节点: ' + entityIds.join(', '));
    appendLog({ type: 'kb', msg: 'graph link', docId: params.id, entityIds: entityIds });
    ok(res, { success: true, documentId: params.id, graphLinks: docs[idx].graphLinks });
  });

  reg('get', '/kb/stats', (req, res) => {
    const docs = readDocuments();
    const versions = readVersions();
    const activeDocs = docs.filter(d => d.status === 'active');
    const archivedDocs = docs.filter(d => d.status === 'archived');
    const deletedDocs = docs.filter(d => d.status === 'deleted');
    const catCounts = {};
    activeDocs.forEach(d => { catCounts[d.category] = (catCounts[d.category] || 0) + 1; });
    const totalWords = activeDocs.reduce((s, d) => s + (d.content || '').trim().split(/\s+/).length, 0);
    ok(res, {
      total: docs.length,
      active: activeDocs.length,
      archived: archivedDocs.length,
      deleted: deletedDocs.length,
      categories: catCounts,
      versions: versions.length,
      analyzed: activeDocs.filter(d => d.aiAnalysis).length,
      graphLinked: activeDocs.filter(d => (d.graphLinks || []).length > 0).length,
      totalWords: totalWords,
      lastUpdated: new Date().toISOString()
    });
  });

  // === 5. Change History ===

  reg('get', '/kb/documents/:id/history', (req, res, params) => {
    const docHistory = kb.store.readHistory().filter(h => h.documentId === params.id).sort((a, b) => new Date(b.ts) - new Date(a.ts));
    ok(res, docHistory);
  });

  reg('get', '/kb/history', (req, res) => {
    const q = url.parse(req.url, true).query;
    let history = kb.store.readHistory();
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
