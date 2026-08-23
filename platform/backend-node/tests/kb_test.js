const http = require('http');

function api(method, path, data) {
  return new Promise((resolve, reject) => {
    const postData = data ? JSON.stringify(data) : null;
    const opts = {
      hostname: 'localhost',
      port: 3010,
      path: path,
      method: method,
      headers: { 'Content-Type': 'application/json' }
    };
    if (postData) opts.headers['Content-Length'] = Buffer.byteLength(postData);
    const req = http.request(opts, (res) => {
      let body = '';
      res.on('data', (c) => body += c);
      res.on('end', () => {
        try { resolve(JSON.parse(body)); } catch (e) { resolve(body); }
      });
    });
    req.on('error', reject);
    if (postData) req.write(postData);
    req.end();
  });
}

async function test() {
  let passed = 0, failed = 0;
  const check = (name, cond) => {
    if (cond) { passed++; console.log(`  ✅ ${name}`); }
    else { failed++; console.log(`  ❌ ${name}`); }
  };

  console.log('=== 云盘知识库 API 全维测试 ===\n');

  // 1. Create Document
  console.log('1. 创建文档');
  const doc = await api('POST', '/kb/documents', {
    title: '算子统一系统架构文档',
    content: '本文档描述了算子统一系统(OUS)的核心架构。\n\n## 核心特性\n- PageRank 影响力排序算法\n- 标签传播社区发现\n- 激活传播能量扩散\n- 度中心性与中介中心性\n\n## 技术栈\n- Rust 核心引擎\n- Node.js API网关\n- Vue3 前端界面\n- 知识图谱集成',
    type: 'markdown',
    category: 'tech.architecture',
    tags: ['架构', '算法', 'Rust', '图谱', 'OUS'],
    description: '系统核心架构说明文档，涵盖图算法与技术栈'
  });
  check('创建文档成功', doc && doc.success && doc.data && doc.data.id);
  const docId = doc.data.id;
  console.log(`  文档ID: ${docId}`);

  // 2. List Documents
  console.log('\n2. 文档列表');
  const list = await api('GET', '/kb/documents');
  check('获取列表成功', list.success && Array.isArray(list.data.documents));
  check('列表不为空', list.data.pagination.total > 0);

  // 3. Get Document
  console.log('\n3. 获取单个文档');
  const getDoc = await api('GET', `/kb/documents/${docId}`);
  check('获取文档成功', getDoc.success && getDoc.data.title === '算子统一系统架构文档');
  check('内容完整', getDoc.data.content.length > 100);
  check('分类正确', getDoc.data.category === 'tech.architecture');
  check('标签正确', getDoc.data.tags.length >= 4);

  // 4. AI Analyze
  console.log('\n4. AI 智能分析');
  const analyzed = await api('POST', `/kb/documents/${docId}/analyze`);
  check('AI分析成功', analyzed.success);
  const analysis = analyzed.data.analysis;
  check('包含摘要', analysis && analysis.summary && analysis.summary.length > 0);
  check('包含关键词', analysis && analysis.keywords && analysis.keywords.length > 0);
  check('包含实体', analysis && analysis.entities && analysis.entities.length > 0);
  check('建议分类', analysis && analysis.suggestedCategory);
  check('阅读时间', analysis && analysis.readingTime >= 0);
  check('字数统计', analysis && analysis.wordCount > 0);

  // 5. Update Document (creates version)
  console.log('\n5. 更新文档（自动创建版本）');
  const updated = await api('PUT', `/kb/documents/${docId}`, {
    title: '算子统一系统架构文档 V2',
    content: '本文档描述了算子统一系统(OUS)的核心架构。\n\n## 核心特性\n- PageRank 影响力排序算法\n- 标签传播社区发现\n- 激活传播能量扩散\n- 度中心性与中介中心性\n- 知识图谱自动构建\n- AI智能分类与分析\n\n## 技术栈\n- Rust 核心引擎\n- Node.js API网关\n- Vue3 前端界面\n- 知识图谱集成\n- 向量检索引擎',
    tags: ['架构', '算法', 'Rust', '图谱', 'OUS', 'AI', '向量']
  });
  check('更新成功', updated.success);
  check('版本号增加', updated.data.version === 2);

  // 6. Get Versions
  console.log('\n6. 版本历史');
  const versions = await api('GET', `/kb/documents/${docId}/versions`);
  check('获取版本成功', versions.success && Array.isArray(versions.data));
  check('版本数量>=2', versions.data.length >= 2);
  const allVersions = versions.data;

  // 7. Create Manual Version
  console.log('\n7. 创建手动版本快照');
  const ver = await api('POST', `/kb/documents/${docId}/versions`, {
    changeNote: '更新了核心特性列表，新增AI智能分类和向量检索'
  });
  check('创建版本成功', ver.success);

  // 8. Compare Versions
  console.log('\n8. 版本差异比较');
  if (allVersions.length >= 2) {
    const compare = await api('POST', `/kb/documents/${docId}/versions/compare`, {
      fromVer: allVersions[0].version,
      toVer: allVersions[allVersions.length - 1].version
    });
    check('版本比较成功', compare.success);
    check('包含差异数据', compare.data && compare.data.diff);
    check('包含相似度', compare.data && compare.data.diff && typeof compare.data.diff.similarity === 'number');
  }

  // 9. Get AI Analysis (persisted)
  console.log('\n9. 获取AI分析结果');
  const getAnalyze = await api('GET', `/kb/documents/${docId}`);
  check('分析已持久化', getAnalyze.data.aiAnalysis !== null);
  check('分析包含关键词', getAnalyze.data.aiAnalysis.keywords.length > 0);

  // 10. Get Entities
  console.log('\n10. 实体提取');
  const entities = await api('GET', `/kb/documents/${docId}/entities`);
  check('实体提取成功', entities.success && entities.data && Array.isArray(entities.data.entities));
  check('实体非空', entities.data.entities.length > 0);

  // 11. Categories
  console.log('\n11. 分类体系');
  const cats = await api('GET', '/kb/categories');
  check('分类获取成功', cats.success && Array.isArray(cats.data));
  check('默认分类存在', cats.data.length >= 5);

  // 12. Tags
  console.log('\n12. 标签统计');
  const tags = await api('GET', '/kb/tags');
  check('标签获取成功', tags.success && Array.isArray(tags.data));

  // 13. KB Stats
  console.log('\n13. 知识库统计');
  const stats = await api('GET', '/kb/stats');
  check('统计获取成功', stats.success);
  check('文档数>0', stats.data.total > 0);

  // 14. Search
  console.log('\n14. AI增强搜索');
  const search = await api('POST', '/kb/search', { query: '算法架构' });
  check('搜索成功', search.success);
  check('返回搜索结果', search.data && Array.isArray(search.data.results));

  // 15. Document History
  console.log('\n15. 变更历史');
  const docHistory = await api('GET', `/kb/documents/${docId}/history`);
  check('文档历史成功', docHistory.success && Array.isArray(docHistory.data));
  check('历史记录>0', docHistory.data.length > 0);
  const historyActions = docHistory.data.map(h => h.action);
  check('包含创建记录', historyActions.includes('create'));

  // 16. Global History
  console.log('\n16. 全局变更历史');
  const globalHistory = await api('GET', '/kb/history');
  check('全局历史成功', globalHistory.success);

  // 17. Batch Analyze
  console.log('\n17. 批量AI分析');
  const batch = await api('POST', '/kb/batch-analyze', {
    docIds: [docId]
  });
  check('批量分析成功', batch.success);

  // 18. Graph Link
  console.log('\n18. 知识图谱关联');
  const linkResult = await api('POST', `/kb/documents/${docId}/graph-link`, {
    entityIds: ['Rust', 'PageRank']
  });
  check('图谱关联成功', linkResult.success);

  // 19. Verify graphLinks updated
  console.log('\n19. 验证图谱关联已持久化');
  const verify = await api('GET', `/kb/documents/${docId}`);
  check('图谱链接已保存', verify.data.graphLinks && verify.data.graphLinks.length > 0);

  // 20. Revert Version
  console.log('\n20. 版本回退');
  if (allVersions.length >= 2) {
    const revert = await api('POST', `/kb/documents/${docId}/versions/revert`, {
      version: allVersions[0].version
    });
    check('回退成功', revert.success);
  }

  // 21. Verify History After Revert
  console.log('\n21. 回退后历史记录');
  const hist2 = await api('GET', `/kb/documents/${docId}/history`);
  const hasRevert = hist2.data.some(h => h.action === 'revert');
  check('回退记录已写入', hasRevert);

  // 22. Delete (Soft)
  console.log('\n22. 软删除文档');
  const deleted = await api('DELETE', `/kb/documents/${docId}`);
  check('删除成功', deleted.success);

  // 23. Verify Status
  console.log('\n23. 验证删除状态');
  const verifyDel = await api('GET', `/kb/documents/${docId}`);
  check('状态为deleted', verifyDel.data.status === 'deleted');

  // 24. Create second document for comprehensive tests
  console.log('\n24. 创建第二份文档');
  const doc2 = await api('POST', '/kb/documents', {
    title: '需求编译系统设计',
    content: '需求编译器(Caomei)将自然语言需求编译为流程蓝图。\n\n## 功能\n- 自然语言解析\n- 意图识别\n- 任务编排\n- 验证闸门\n\n## 应用场景\n- 自动化业务处理\n- AI驱动工作流生成',
    type: 'markdown',
    category: 'business.requirement',
    tags: ['需求', '编译', 'Caomei', '自动化'],
    description: 'Caomei需求编译器设计文档'
  });
  check('第二文档创建成功', doc2.success && doc2.data.id);
  const doc2Id = doc2.data.id;

  // 25. Search by category
  console.log('\n25. 按分类筛选');
  const byCat = await api('GET', '/kb/documents?category=business.requirement');
  check('分类筛选成功', byCat.success && byCat.data.documents.length > 0);

  // 26. Search by tag
  console.log('\n26. 按标签筛选');
  const byTag = await api('GET', '/kb/documents?tag=Rust');
  check('标签筛选成功', byTag.success);

  // 27. Batch analyze second doc
  console.log('\n27. 批量分析第二文档');
  const batch2 = await api('POST', '/kb/batch-analyze', {
    docIds: [doc2Id]
  });
  check('批量分析成功2', batch2.success);

  // 28. Verify second doc has analysis
  console.log('\n28. 验证第二文档分析结果');
  const verify2 = await api('GET', `/kb/documents/${doc2Id}`);
  check('第二文档已分析', verify2.data.aiAnalysis !== null);

  // 29. Final stats
  console.log('\n29. 最终统计');
  const finalStats = await api('GET', '/kb/stats');
  check('最终统计获取成功', finalStats.success);
  console.log(`  文档总数: ${finalStats.data.total}, 已分析: ${finalStats.data.analyzed}, 版本数: ${finalStats.data.versions}`);

  // 30. Cleanup - delete second doc
  console.log('\n30. 清理测试数据');
  await api('DELETE', `/kb/documents/${doc2Id}`);

  console.log(`\n=== 测试结果: ${passed}/${passed + failed} 通过 ===`);
  console.log(passed + failed === passed ? '🎉 全部通过！' : `⚠️ 有 ${failed} 个失败`);
}

test().catch(e => console.error('Test error:', e));
