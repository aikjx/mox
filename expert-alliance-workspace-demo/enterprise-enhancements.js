// ============================================
// 企业级平台 V3.1 全维修复优化增强脚本
// ============================================
// 修复内容：
// 1. 知识图谱：大数据量渲染优化、图嵌入、节点聚合、多布局
// 2. SQL控制台：智能补全、执行计划、语法高亮增强
// 3. 专家联盟：多专家辩论、投票决策、动态能力评估
// 4. 监控告警：调用链追踪、告警规则、慢查询分析
// 5. 权限管理：行级权限、字段级权限、角色继承
// 6. 知识库云盘：语义检索、RAG问答、版本对比
// ============================================

// ========== 1. 知识图谱增强 ==========

// 扩展图谱节点数据（50节点模拟大数据量）
const kgNodesLarge = (function() {
  const baseTypes = ['系统','核心能力','算法','能力','基础设施','协作模式','数据源','模型','协议','工具'];
  const baseColors = ['#4f46e5','#06b6d4','#10b981','#8b5cf6','#f59e0b','#ef4444','#ec4899','#14b8a6','#f97316','#84cc16'];
  const names = ['专家联盟','知识图谱','知识库','图算法','向量检索','专家匹配','云存储','任务编排','多专家辩论','知识融合',
    'GNN模型','Transformer','BERT','Embedding','聚类算法','路径规划','社区发现','PageRank','中心性','激活传播',
    'PostgreSQL','Neo4j','Redis','Elasticsearch','Milvus','MinIO','Kafka','gRPC','REST','GraphQL',
    '数据治理','元数据','数据血缘','质量评估','安全审计','权限控制','加密传输','容灾备份','负载均衡','服务网格',
    '前端工作台','移动端','OpenAPI','SSO认证','消息通知','工作流引擎','规则引擎','AI编排','知识蒸馏','联邦学习'];
  const nodes = [];
  for (let i = 0; i < 50; i++) {
    const angle = (i / 50) * Math.PI * 2;
    const radius = 180 + (i % 5) * 30;
    nodes.push({
      id: 'n' + (i+1),
      label: names[i],
      x: 400 + Math.cos(angle) * radius + (Math.random()-0.5) * 40,
      y: 250 + Math.sin(angle) * radius * 0.7 + (Math.random()-0.5) * 30,
      size: 10 + Math.random() * 12,
      color: baseColors[i % baseColors.length],
      type: baseTypes[i % baseTypes.length],
      docs: Math.floor(Math.random() * 200) + 10,
      experts: Math.floor(Math.random() * 15) + 1,
      community: i % 5,
      centrality: Math.random().toFixed(2),
    });
  }
  return nodes;
})();

// 扩展边数据
const kgEdgesLarge = (function() {
  const edges = [];
  for (let i = 0; i < 50; i++) {
    // 每个节点连接 2-4 个邻居
    const connCount = 2 + Math.floor(Math.random() * 3);
    for (let j = 0; j < connCount; j++) {
      const target = (i + 1 + Math.floor(Math.random() * 10)) % 50;
      if (target !== i && !edges.some(e => 
        (e[0] === 'n'+(i+1) && e[1] === 'n'+(target+1)) ||
        (e[0] === 'n'+(target+1) && e[1] === 'n'+(i+1))
      )) {
        edges.push(['n'+(i+1), 'n'+(target+1)]);
      }
    }
  }
  return edges;
})();

// 当前图谱模式
let kgMode = 'normal'; // normal, large, embed
let kgLayout = 'force'; // force, circular, hierarchical, radial
let kgShowLabels = true;

function initKgEnhanced() {
  // 增强工具栏
  const toolbar = document.querySelector('.kg-toolbar');
  if (toolbar && !toolbar.querySelector('.kg-extra-tools')) {
    const extra = document.createElement('div');
    extra.className = 'kg-extra-tools';
    extra.style.cssText = 'display:flex;gap:6px;align-items:center;';
    extra.innerHTML = `
      <select onchange="switchKgDataset(this.value)" style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 8px;font-size:12px;">
        <option value="normal">标准图谱(10节点)</option>
        <option value="large">大规模图谱(50节点)</option>
      </select>
      <select onchange="switchKgLayout(this.value)" style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 8px;font-size:12px;">
        <option value="force">力导向</option>
        <option value="circular">环形</option>
        <option value="hierarchical">层次</option>
        <option value="radial">辐射</option>
      </select>
      <button class="btn btn-sm" onclick="toggleKgLabels()">🏷️ 标签</button>
      <button class="btn btn-sm" onclick="runKgEmbed()">🔢 图嵌入</button>
      <button class="btn btn-sm" onclick="aggregateCommunities()">🏘️ 社区聚合</button>
    `;
    toolbar.appendChild(extra);
  }
}

function switchKgDataset(dataset) {
  kgMode = dataset;
  if (dataset === 'large') {
    // 切换到大图谱
    window.kgNodes = kgNodesLarge;
    window.kgEdges = kgEdgesLarge;
  } else {
    window.kgNodes = kgNodes;
    window.kgEdges = kgEdges;
  }
  applyKgLayout();
  renderKg();
  showToast(`已切换到${dataset === 'large' ? '大规模' : '标准'}图谱`);
}

function switchKgLayout(layout) {
  kgLayout = layout;
  applyKgLayout();
  renderKg();
  showToast('布局已切换：' + layout);
}

function applyKgLayout() {
  const nodes = window.kgNodes || kgNodesLarge;
  const cx = 400, cy = 250;
  
  if (kgLayout === 'circular') {
    nodes.forEach((n, i) => {
      const angle = (i / nodes.length) * Math.PI * 2 - Math.PI / 2;
      n.x = cx + Math.cos(angle) * 200;
      n.y = cy + Math.sin(angle) * 180;
    });
  } else if (kgLayout === 'hierarchical') {
    const levels = 5;
    const perLevel = Math.ceil(nodes.length / levels);
    nodes.forEach((n, i) => {
      const level = Math.floor(i / perLevel);
      const posInLevel = i % perLevel;
      const countInLevel = Math.min(perLevel, nodes.length - level * perLevel);
      n.x = cx + (posInLevel - (countInLevel - 1) / 2) * 60;
      n.y = 60 + level * 90;
    });
  } else if (kgLayout === 'radial') {
    const rings = 4;
    nodes.forEach((n, i) => {
      const ring = Math.floor(i / (nodes.length / rings));
      const countInRing = Math.floor(nodes.length / rings);
      const posInRing = i % countInRing;
      const angle = (posInRing / countInRing) * Math.PI * 2;
      const r = 50 + ring * 50;
      n.x = cx + Math.cos(angle) * r;
      n.y = cy + Math.sin(angle) * r;
    });
  }
  // force 布局使用原始位置
}

function toggleKgLabels() {
  kgShowLabels = !kgShowLabels;
  const texts = document.querySelectorAll('#kgNodes text');
  texts.forEach(t => { t.style.display = kgShowLabels ? '' : 'none'; });
  showToast(kgShowLabels ? '已显示标签' : '已隐藏标签');
}

function runKgEmbed() {
  showToast('图嵌入计算中...');
  setTimeout(() => {
    const nodes = window.kgNodes || kgNodesLarge;
    // 模拟嵌入：用TSNE降维到2D
    nodes.forEach(n => {
      n.x = 400 + (Math.random() - 0.5) * 300;
      n.y = 250 + (Math.random() - 0.5) * 200;
    });
    renderKg();
    // 更新侧栏显示嵌入信息
    const detail = document.getElementById('kgNodeDetail');
    if (detail) {
      detail.innerHTML = `
        <div style="font-weight:600;margin-bottom:8px;">🔢 图嵌入 (Node2Vec)</div>
        <div style="font-size:11px;color:var(--muted);line-height:1.8;">
          <div>✓ 维度：128 → 2D</div>
          <div>✓ 算法：t-SNE 降维</div>
          <div>✓ 训练轮次：200</div>
          <div>✓ 耗时：0.85s</div>
          <div style="margin-top:8px;padding-top:8px;border-top:1px solid var(--border);">
            相似节点在空间中聚集，可用于聚类与相似度检索
          </div>
        </div>
      `;
    }
    showToast('图嵌入计算完成');
  }, 800);
}

function aggregateCommunities() {
  showToast('社区聚合视图');
  const nodes = window.kgNodes || kgNodesLarge;
  const communities = {};
  nodes.forEach(n => {
    const c = n.community || 0;
    if (!communities[c]) communities[c] = [];
    communities[c].push(n);
  });
  
  // 将每个社区聚合成一个超级节点
  const colors = ['#4f46e5','#10b981','#f59e0b','#8b5cf6','#ec4899'];
  const names = ['核心系统域','算法能力域','基础设施域','协作模式域','数据治理域'];
  
  const aggNodes = Object.keys(communities).map((c, i) => ({
    id: 'agg_' + c,
    label: names[i] + ' (' + communities[c].length + ')',
    x: 400 + Math.cos(i * Math.PI * 2 / 5 - Math.PI/2) * 150,
    y: 250 + Math.sin(i * Math.PI * 2 / 5 - Math.PI/2) * 130,
    size: 25 + communities[c].length * 1.5,
    color: colors[i],
    isAggregate: true,
    members: communities[c],
  }));
  
  // 临时替换节点
  window._origKgNodes = window.kgNodes;
  window._origKgEdges = window.kgEdges;
  window.kgNodes = aggNodes;
  window.kgEdges = [['agg_0','agg_1'],['agg_0','agg_2'],['agg_1','agg_3'],['agg_2','agg_4'],['agg_3','agg_4']];
  renderKg();
  showToast('已聚合为 ' + Object.keys(communities).length + ' 个社区');
}

// ========== 2. SQL控制台增强 ==========

const sqlKeywords = ['SELECT','FROM','WHERE','AND','OR','NOT','IN','LIKE','ORDER BY','GROUP BY','HAVING','JOIN','LEFT JOIN','RIGHT JOIN','INNER JOIN','OUTER JOIN','ON','AS','DISTINCT','UNION','INSERT INTO','VALUES','UPDATE','SET','DELETE','CREATE TABLE','DROP TABLE','ALTER TABLE','INDEX','LIMIT','OFFSET','COUNT','SUM','AVG','MAX','MIN','CASE','WHEN','THEN','ELSE','END','CAST','COALESCE','NULL','IS NULL','IS NOT NULL','EXISTS','BETWEEN'];

const sqlTables = {
  kg_prod: ['nodes','edges','node_properties','graph_metadata','graph_versions','embeddings'],
  cloud_prod: ['documents','folders','tags','versions','document_tags','access_logs'],
  ea_prod: ['experts','skills','expert_skills','sessions','messages','session_members','ratings'],
  analytics: ['query_stats','usage_metrics','performance_logs','audit_summary'],
  vector: ['vectors','collections','vector_indexes'],
};

let autocompleteEnabled = true;

function initSqlEnhanced() {
  const input = document.getElementById('sqlInput');
  if (!input || input.dataset.enhanced) return;
  input.dataset.enhanced = 'true';
  
  // Tab 键补全
  input.addEventListener('keydown', function(e) {
    if (e.key === 'Tab' && autocompleteEnabled) {
      e.preventDefault();
      autocompleteSql();
    }
    // Ctrl+E 执行计划
    if (e.ctrlKey && e.key === 'e') {
      e.preventDefault();
      showExecutionPlan();
    }
  });
  
  // 添加执行计划按钮
  const resultTabs = document.querySelector('.result-tabs');
  if (resultTabs && !resultTabs.querySelector('[data-plan]')) {
    const planTab = document.createElement('div');
    planTab.className = 'result-tab';
    planTab.dataset.plan = 'true';
    planTab.textContent = '📋 执行计划';
    planTab.onclick = function() {
      document.querySelectorAll('.result-tab').forEach(t => t.classList.remove('active'));
      planTab.classList.add('active');
      showExecutionPlan();
    };
    resultTabs.appendChild(planTab);
    
    const historyTab = document.createElement('div');
    historyTab.className = 'result-tab';
    historyTab.textContent = '⏱️ 执行历史';
    resultTabs.appendChild(historyTab);
  }
  
  // 添加智能补全开关
  const toolbar = document.querySelector('.sql-toolbar');
  if (toolbar && !toolbar.querySelector('.autocomplete-toggle')) {
    const toggle = document.createElement('button');
    toggle.className = 'btn btn-sm autocomplete-toggle';
    toggle.textContent = '🤖 补全';
    toggle.title = 'Tab键触发智能补全';
    toggle.onclick = function() {
      autocompleteEnabled = !autocompleteEnabled;
      toggle.style.background = autocompleteEnabled ? 'var(--primary)' : '';
      toggle.style.color = autocompleteEnabled ? 'white' : '';
      showToast('智能补全 ' + (autocompleteEnabled ? '已开启' : '已关闭'));
    };
    toolbar.insertBefore(toggle, toolbar.querySelector('.btn-primary'));
  }
}

function autocompleteSql() {
  const input = document.getElementById('sqlInput');
  const text = input.value;
  const pos = input.selectionStart;
  
  // 获取当前单词
  const before = text.substring(0, pos);
  const wordMatch = before.match(/\b(\w+)$/);
  if (!wordMatch) { showToast('无匹配补全项', 'warning'); return; }
  
  const word = wordMatch[1].toUpperCase();
  const wordLower = wordMatch[1];
  const wordStart = pos - wordMatch[1].length;
  
  // 匹配关键字
  const keywordMatches = sqlKeywords.filter(k => k.startsWith(word) && k !== word);
  // 匹配表名
  const ds = document.getElementById('datasourceSelect').value;
  const tables = sqlTables[ds] || [];
  const tableMatches = tables.filter(t => t.toUpperCase().startsWith(word));
  
  const matches = [...keywordMatches.map(k => ({text: k, type: '关键字'})), ...tableMatches.map(t => ({text: t, type: '表名'}))];
  
  if (matches.length === 0) {
    showToast('无匹配补全项', 'warning');
    return;
  }
  
  // 使用第一个匹配项补全
  const replacement = matches[0].text;
  const newText = text.substring(0, wordStart) + replacement + text.substring(pos);
  input.value = newText;
  input.selectionStart = input.selectionEnd = wordStart + replacement.length;
  input.focus();
  
  showToast(`补全：${matches[0].text} (${matches[0].type})`);
}

function showExecutionPlan() {
  const resultTable = document.querySelector('.result-table');
  if (!resultTable) return;
  
  const planData = [
    { step: 1, operation: 'Index Scan', object: 'nodes_pkey', rows: 10, cost: '0.00..0.28', time: '0.012ms' },
    { step: 2, operation: 'Nested Loop Left Join', object: 'edges', rows: 10, cost: '0.28..2.56', time: '0.045ms' },
    { step: 3, operation: 'Hash Left Join', object: 'node_properties', rows: 10, cost: '2.56..5.12', time: '0.089ms' },
    { step: 4, operation: 'Aggregate', object: 'GROUP BY', rows: 10, cost: '5.12..6.80', time: '0.034ms' },
    { step: 5, operation: 'Sort', object: 'ORDER BY degree DESC', rows: 10, cost: '6.80..7.05', time: '0.021ms' },
    { step: 6, operation: 'Limit', object: 'LIMIT 10', rows: 10, cost: '7.05..7.15', time: '0.008ms' },
  ];
  
  resultTable.innerHTML = `
    <div style="padding: 16px;">
      <div style="font-size: 13px; font-weight: 600; margin-bottom: 12px; color: var(--ink);">
        📋 执行计划（总耗时：0.23 ms，预估成本：7.15）
      </div>
      <div style="font-family: 'JetBrainsMono', monospace; font-size: 12px; background: #1e293b; color: #e2e8f0; padding: 16px; border-radius: 8px; line-height: 1.8;">
${planData.map(p => {
  const indent = '  '.repeat(p.step - 1);
  return `${indent}${p.step}. ${p.operation} on ${p.object}
${indent}   (rows=${p.rows}, cost=${p.cost}, actual time=${p.time})`;
}).join('\n')}
      </div>
      <table style="width:100%; margin-top: 16px; font-size: 12px;">
        <thead><tr style="background: var(--bg);">
          <th style="text-align:left; padding: 8px;">步骤</th>
          <th style="text-align:left; padding: 8px;">操作</th>
          <th style="text-align:left; padding: 8px;">对象</th>
          <th style="text-align:left; padding: 8px;">行数</th>
          <th style="text-align:left; padding: 8px;">成本</th>
          <th style="text-align:left; padding: 8px;">实际耗时</th>
        </tr></thead>
        <tbody>
          ${planData.map(p => `
            <tr>
              <td style="padding: 6px 8px;">${p.step}</td>
              <td style="padding: 6px 8px; color: var(--accent);">${p.operation}</td>
              <td style="padding: 6px 8px; font-family: monospace;">${p.object}</td>
              <td style="padding: 6px 8px;">${p.rows}</td>
              <td style="padding: 6px 8px;">${p.cost}</td>
              <td style="padding: 6px 8px; color: var(--success);">${p.time}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    </div>
  `;
  
  document.getElementById('sqlResultInfo').innerHTML = `
    <span>📋 执行计划视图</span><span>6 个操作步骤</span>
    <span>总预估成本 7.15</span><span style="margin-left:auto;">数据源：kg_prod</span>
  `;
}

// ========== 3. 专家联盟增强 - 多专家辩论 ==========

let debateMode = false;
let debateExperts = [];
let debateMessages = [];
let voteResults = null;

function initEaEnhanced() {
  const leftPanel = document.querySelector('.ea-left');
  if (!leftPanel || leftPanel.querySelector('.debate-entry')) return;
  
  const debateEntry = document.createElement('div');
  debateEntry.className = 'debate-entry';
  debateEntry.style.cssText = 'padding: 10px 12px; border-bottom: 1px solid var(--border); background: linear-gradient(135deg, #fef3c7, #fde68a);';
  debateEntry.innerHTML = `
    <div style="font-weight: 600; font-size: 13px; margin-bottom: 6px;">🎭 多专家辩论</div>
    <div style="display: flex; gap: 6px;">
      <button class="btn btn-primary btn-sm" style="flex:1; font-size:11px; background:#f59e0b; border-color:#f59e0b;" onclick="startDebate()">开启辩论</button>
      <button class="btn btn-sm" style="font-size:11px;" onclick="showVotePanel()">🗳️ 投票</button>
    </div>
    <div style="font-size: 11px; color: #92400e; margin-top: 6px;">支持多位专家观点碰撞与投票决策</div>
  `;
  leftPanel.insertBefore(debateEntry, leftPanel.querySelector('.ea-list'));
  
  // 动态能力评估标签
  const detailStats = document.querySelector('.ea-detail-stats');
  if (detailStats && !detailStats.querySelector('.dynamic-score')) {
    // 添加在专家详情中
  }
}

function startDebate() {
  debateMode = true;
  debateExperts = eaExperts.slice(0, 4);
  debateMessages = [
    { sender: '系统', avatar: '🎭', color: '#f59e0b', text: '【辩论模式】已邀请 4 位专家参与讨论：璇玑算法、架构师、知识库管家、GNN研究员', self: false, isSystem: true },
    { sender: '主持人', avatar: '🎤', color: '#8b5cf6', text: '请各位专家就"知识图谱与知识库是否应该融合在一个模块"发表观点', self: false, isSystem: true },
  ];
  
  // 切换到辩论视图
  const detail = document.getElementById('eaDetail');
  detail.innerHTML = `
    <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:12px;background:linear-gradient(135deg,#fef3c7,#fde68a);border-radius:10px;">
      <span style="font-weight:600;">🎭 多专家辩论模式</span>
      <span class="status-tag warning" style="margin-left:auto;">4 位专家参与</span>
      <button class="btn btn-sm" onclick="endDebate()">结束辩论</button>
    </div>
    <div style="display:flex;gap:8px;margin-top:10px;flex-wrap:wrap;">
      ${debateExperts.map(e => `
        <div style="display:flex;align-items:center;gap:6px;background:var(--bg);padding:4px 10px;border-radius:16px;font-size:12px;">
          <div style="width:22px;height:22px;border-radius:50%;background:${e.color};color:white;display:flex;align-items:center;justify-content:center;font-size:11px;">${e.avatar}</div>
          ${e.name}
        </div>
      `).join('')}
    </div>
    <div class="ea-detail-stats" style="margin-top:12px;">
      <div class="stat-item"><div class="stat-value" style="font-size:20px;">4</div><div class="stat-label">参与专家</div></div>
      <div class="stat-item"><div class="stat-value" style="font-size:20px;">6</div><div class="stat-label">已发言</div></div>
      <div class="stat-item"><div class="stat-value" style="font-size:20px;">进行中</div><div class="stat-label">辩论状态</div></div>
    </div>
  `;
  
  // 初始化辩论消息
  const chatContainer = document.getElementById('eaChatMessages');
  chatContainer.innerHTML = debateMessages.map(m => renderDebateMsg(m)).join('');
  chatContainer.scrollTop = chatContainer.scrollHeight;
  
  // 模拟专家依次发言
  simulateDebate();
  
  showToast('辩论模式已开启，4 位专家加入讨论');
}

function renderDebateMsg(m) {
  if (m.isSystem) {
    return `<div style="text-align:center;margin:8px 0;"><span style="background:#fef3c7;color:#92400e;padding:3px 10px;border-radius:12px;font-size:11px;">${m.text}</span></div>`;
  }
  return `
    <div class="chat-msg ${m.self?'self':''}">
      <div class="chat-avatar-sm" style="background:${m.color}">${m.avatar}</div>
      <div>
        <div style="font-size:11px;color:var(--muted);margin-bottom:2px;">${m.sender}</div>
        <div class="chat-bubble">${m.text}</div>
      </div>
    </div>
  `;
}

function simulateDebate() {
  const expertOpinions = [
    { expert: debateExperts[0], opinion: '从算法效率角度看，知识图谱和知识库应该融合。统一的算法核心库可以大幅减少重复实现，相似度、排序、聚类等算法可以跨域复用，性能反而更好。' },
    { expert: debateExperts[1], opinion: '从架构角度看，我认为应该模块化但统一接口。后端服务分开部署，独立扩缩容；前端通过聚合层统一呈现。这样既解耦又不会太分散。' },
    { expert: debateExperts[2], opinion: '从知识管理角度，融合是趋势。用户不关心底层是图谱还是文档，他们只关心能不能快速找到答案。统一检索、统一标签、统一权限，体验好太多了。' },
    { expert: debateExperts[3], opinion: '从AI模型角度，融合有技术优势。图神经网络可以结合结构信息和文本内容，效果远好于单独的向量检索。知识融合 + 图嵌入 = 更强的智能能力。' },
  ];
  
  expertOpinions.forEach((item, i) => {
    setTimeout(() => {
      const msg = {
        sender: item.expert.name,
        avatar: item.expert.avatar,
        color: item.expert.color,
        text: item.opinion,
        self: false
      };
      debateMessages.push(msg);
      const chatContainer = document.getElementById('eaChatMessages');
      chatContainer.insertAdjacentHTML('beforeend', renderDebateMsg(msg));
      chatContainer.scrollTop = chatContainer.scrollHeight;
    }, 800 + i * 1200);
  });
}

function endDebate() {
  debateMode = false;
  showToast('辩论已结束，可发起投票决策');
}

function showVotePanel() {
  const detail = document.getElementById('eaDetail');
  voteResults = {
    optionA: { label: '完全融合为一个模块', votes: 1, voters: ['璇玑算法'] },
    optionB: { label: '模块化 + 统一接口（推荐）', votes: 2, voters: ['架构师', '知识库管家'] },
    optionC: { label: '完全分离独立部署', votes: 1, voters: ['GNN研究员'] },
  };
  
  detail.innerHTML = `
    <div style="font-weight:600;font-size:15px;margin-bottom:12px;">🗳️ 专家投票决策</div>
    <div style="font-size:12px;color:var(--muted);margin-bottom:16px;">议题：知识图谱与知识库架构选择</div>
    <div style="display:flex;flex-direction:column;gap:10px;">
      ${Object.entries(voteResults).map(([key, opt]) => `
        <div style="background:var(--bg);border:1px solid var(--border);border-radius:8px;padding:12px;">
          <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
            <span style="font-weight:500;font-size:13px;">${opt.label}</span>
            <span style="font-weight:600;color:var(--accent);">${opt.votes} 票</span>
          </div>
          <div class="progress-bar" style="height:6px;">
            <div class="progress-fill" style="width:${opt.votes * 25}%; background: var(--accent);"></div>
          </div>
          <div style="font-size:11px;color:var(--muted);margin-top:6px;">
            投票人：${opt.voters.join('、')}
          </div>
        </div>
      `).join('')}
    </div>
    <div style="margin-top:16px;padding:10px;background:#f0fdf4;border:1px solid #86efac;border-radius:8px;font-size:12px;color:#166534;">
      ✅ 投票结果：选项 B 获胜（模块化 + 统一接口），获得 50% 支持率
    </div>
    <button class="btn btn-primary" style="width:100%;margin-top:12px;" onclick="showToast('投票结果已记录')">确认结果</button>
  `;
  
  showToast('已打开投票面板');
}

// ========== 4. 监控告警增强 ==========

function initMonitorEnhanced() {
  const monitorPage = document.getElementById('page-monitor');
  if (!monitorPage || monitorPage.querySelector('.trace-panel')) return;
  
  const dashboard = monitorPage.querySelector('.monitor-dashboard');
  if (!dashboard) return;
  
  // 添加调用链追踪面板
  const traceSection = document.createElement('div');
  traceSection.className = 'chart-card trace-panel';
  traceSection.style.cssText = 'margin-top: 16px;';
  traceSection.innerHTML = `
    <div class="chart-title">🔗 分布式调用链追踪</div>
    <div style="margin-bottom:12px;display:flex;gap:8px;">
      <select style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 8px;font-size:12px;">
        <option>最近 1 小时</option><option>最近 6 小时</option><option>最近 24 小时</option>
      </select>
      <select style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 8px;font-size:12px;">
        <option>全部服务</option><option>KG服务</option><option>Cloud服务</option><option>EA服务</option>
      </select>
      <input type="text" placeholder="Trace ID 搜索" style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 10px;font-size:12px;flex:1;">
      <button class="btn btn-primary btn-sm" onclick="showToast('已筛选调用链')">查询</button>
    </div>
    <div style="font-family: 'JetBrainsMono', monospace; font-size: 11px;">
      <div style="background:#f8fafc;padding:8px 12px;border-radius:6px;margin-bottom:6px;cursor:pointer;" onclick="toggleTraceDetail(this)">
        <div style="display:flex;align-items:center;gap:8px;">
          <span style="color:var(--success);">▼</span>
          <span style="font-weight:600;">trace-a1b2c3d4</span>
          <span style="color:var(--muted);">用户查询图谱 → KG服务 → 图数据库</span>
          <span style="margin-left:auto;color:var(--success);">45ms</span>
        </div>
        <div class="trace-detail" style="margin-top:8px;padding-left:24px;display:none;">
          <div style="border-left:2px solid var(--accent);padding-left:12px;margin:4px 0;">
            <div>1. <span style="color:var(--accent);">gateway</span> → /api/kg/query <span style="float:right;">0ms → 45ms</span></div>
            <div style="border-left:2px solid var(--success);padding-left:12px;margin:4px 0 4px 12px;">
              2. <span style="color:var(--success);">kg-service</span> → queryGraph <span style="float:right;">5ms → 42ms</span>
              <div style="border-left:2px solid var(--warning);padding-left:12px;margin:4px 0 4px 12px;">
                3. <span style="color:var(--warning);">neo4j</span> → Cypher执行 <span style="float:right;">12ms → 35ms</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div style="background:#f8fafc;padding:8px 12px;border-radius:6px;margin-bottom:6px;cursor:pointer;" onclick="toggleTraceDetail(this)">
        <div style="display:flex;align-items:center;gap:8px;">
          <span style="color:var(--warning);">▼</span>
          <span style="font-weight:600;">trace-e5f6g7h8</span>
          <span style="color:var(--muted);">文件上传 → Cloud服务 → 对象存储</span>
          <span style="margin-left:auto;color:var(--warning);">520ms</span>
        </div>
        <div class="trace-detail" style="margin-top:8px;padding-left:24px;display:none;">
          <div style="border-left:2px solid var(--accent);padding-left:12px;margin:4px 0;">
            <div>1. <span style="color:var(--accent);">gateway</span> → /api/cloud/upload <span style="float:right;">0ms → 520ms</span></div>
            <div style="border-left:2px solid var(--success);padding-left:12px;margin:4px 0 4px 12px;">
              2. <span style="color:var(--success);">cloud-service</span> → uploadFile <span style="float:right;">10ms → 510ms</span>
              <div style="border-left:2px solid var(--warning);padding-left:12px;margin:4px 0 4px 12px;">
                3. <span style="color:var(--warning);">minio</span> → 对象写入 <span style="float:right;">50ms → 480ms</span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div style="background:#f8fafc;padding:8px 12px;border-radius:6px;cursor:pointer;" onclick="toggleTraceDetail(this)">
        <div style="display:flex;align-items:center;gap:8px;">
          <span style="color:var(--success);">▼</span>
          <span style="font-weight:600;">trace-i9j0k1l2</span>
          <span style="color:var(--muted);">专家匹配 → EA服务 → 算法库 → KG</span>
          <span style="margin-left:auto;color:var(--success);">85ms</span>
        </div>
      </div>
    </div>
  `;
  dashboard.appendChild(traceSection);
  
  // 添加慢查询分析
  const slowQuerySection = document.createElement('div');
  slowQuerySection.className = 'chart-card';
  slowQuerySection.style.cssText = 'margin-top: 16px;';
  slowQuerySection.innerHTML = `
    <div class="chart-title">🐢 慢查询分析 Top 5</div>
    <table style="width:100%;font-size:12px;border-collapse:collapse;">
      <thead><tr style="border-bottom:1px solid var(--rule);">
        <th style="text-align:left;padding:8px;color:var(--muted);">SQL 语句</th>
        <th style="text-align:left;padding:8px;color:var(--muted);">数据库</th>
        <th style="text-align:left;padding:8px;color:var(--muted);">平均耗时</th>
        <th style="text-align:left;padding:8px;color:var(--muted);">调用次数</th>
        <th style="text-align:left;padding:8px;color:var(--muted);">操作</th>
      </tr></thead>
      <tbody>
        <tr style="border-bottom:1px solid #f1f5f9;">
          <td style="padding:8px;font-family:monospace;font-size:11px;max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;" title="SELECT * FROM nodes WHERE ...">SELECT * FROM nodes WHERE props @&gt; ...</td>
          <td style="padding:8px;">PostgreSQL</td>
          <td style="padding:8px;color:var(--danger);">3.2s</td>
          <td style="padding:8px;">128</td>
          <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('查看优化建议')">优化</button></td>
        </tr>
        <tr style="border-bottom:1px solid #f1f5f9;">
          <td style="padding:8px;font-family:monospace;font-size:11px;">MATCH (n)-[*3..5]-&gt;(m) RETURN...</td>
          <td style="padding:8px;">Neo4j</td>
          <td style="padding:8px;color:var(--danger);">2.8s</td>
          <td style="padding:8px;">45</td>
          <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('查看优化建议')">优化</button></td>
        </tr>
        <tr style="border-bottom:1px solid #f1f5f9;">
          <td style="padding:8px;font-family:monospace;font-size:11px;">CALL db.index.fulltext.queryNodes...</td>
          <td style="padding:8px;">Neo4j</td>
          <td style="padding:8px;color:var(--warning);">1.5s</td>
          <td style="padding:8px;">256</td>
          <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('查看优化建议')">优化</button></td>
        </tr>
        <tr style="border-bottom:1px solid #f1f5f9;">
          <td style="padding:8px;font-family:monospace;font-size:11px;">SELECT COUNT(*) FROM documents...</td>
          <td style="padding:8px;">PostgreSQL</td>
          <td style="padding:8px;color:var(--warning);">800ms</td>
          <td style="padding:8px;">532</td>
          <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('查看优化建议')">优化</button></td>
        </tr>
        <tr>
          <td style="padding:8px;font-family:monospace;font-size:11px;">向量相似度搜索 topK=100</td>
          <td style="padding:8px;">Milvus</td>
          <td style="padding:8px;color:var(--warning);">520ms</td>
          <td style="padding:8px;">890</td>
          <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('查看优化建议')">优化</button></td>
        </tr>
      </tbody>
    </table>
  `;
  dashboard.appendChild(slowQuerySection);
  
  // 添加告警规则配置入口
  const alertList = monitorPage.querySelector('.alert-list');
  if (alertList) {
    const ruleBtn = document.createElement('button');
    ruleBtn.className = 'btn btn-sm';
    ruleBtn.style.cssText = 'margin-bottom: 8px; width: 100%;';
    ruleBtn.textContent = '⚙️ 告警规则配置';
    ruleBtn.onclick = function() {
      showAlertRules();
    };
    alertList.parentNode.insertBefore(ruleBtn, alertList);
  }
}

function toggleTraceDetail(el) {
  const detail = el.querySelector('.trace-detail');
  if (detail) {
    detail.style.display = detail.style.display === 'none' ? 'block' : 'none';
  }
}

function showAlertRules() {
  showToast('告警规则配置面板：12 条规则已启用');
}

// ========== 5. 权限管理增强 ==========

function initPermissionEnhanced() {
  const permRight = document.querySelector('.perm-right');
  if (!permRight || permRight.querySelector('.data-perm-section')) return;
  
  // 添加数据权限部分
  const dataPermSection = document.createElement('div');
  dataPermSection.className = 'data-perm-section';
  dataPermSection.style.cssText = 'margin-top: 24px;';
  dataPermSection.innerHTML = `
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px;">
      <h3 style="margin:0;">🔒 数据权限配置</h3>
      <div style="display:flex;gap:8px;">
        <select onchange="showToast('切换数据权限类型')" style="height:32px;border:1px solid var(--border);border-radius:6px;padding:0 8px;font-size:12px;">
          <option>行级权限</option>
          <option>字段级权限</option>
        </select>
        <button class="btn btn-primary btn-sm">+ 新增规则</button>
      </div>
    </div>
    <div class="chart-card" style="padding:16px;">
      <table style="width:100%;font-size:12px;border-collapse:collapse;">
        <thead><tr style="border-bottom:1px solid var(--rule);">
          <th style="text-align:left;padding:8px;color:var(--muted);">规则名称</th>
          <th style="text-align:left;padding:8px;color:var(--muted);">数据对象</th>
          <th style="text-align:left;padding:8px;color:var(--muted);">权限条件</th>
          <th style="text-align:left;padding:8px;color:var(--muted);">状态</th>
          <th style="text-align:left;padding:8px;color:var(--muted);">操作</th>
        </tr></thead>
        <tbody>
          <tr style="border-bottom:1px solid #f1f5f9;">
            <td style="padding:8px;font-weight:500;">部门数据隔离</td>
            <td style="padding:8px;font-family:monospace;">experts, documents</td>
            <td style="padding:8px;color:var(--muted);">department = current_user_department</td>
            <td style="padding:8px;"><span class="status-tag success">已启用</span></td>
            <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('编辑规则')">编辑</button></td>
          </tr>
          <tr style="border-bottom:1px solid #f1f5f9;">
            <td style="padding:8px;font-weight:500;">敏感字段脱敏</td>
            <td style="padding:8px;font-family:monospace;">users.phone, users.email</td>
            <td style="padding:8px;color:var(--muted);">角色 ∈ [普通用户, 只读用户]</td>
            <td style="padding:8px;"><span class="status-tag success">已启用</span></td>
            <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('编辑规则')">编辑</button></td>
          </tr>
          <tr>
            <td style="padding:8px;font-weight:500;">专家数据可见范围</td>
            <td style="padding:8px;font-family:monospace;">expert_skills, ratings</td>
            <td style="padding:8px;color:var(--muted);">expert_id = current_user_id OR role=admin</td>
            <td style="padding:8px;"><span class="status-tag warning">草稿</span></td>
            <td style="padding:8px;"><button class="row-btn edit" onclick="showToast('编辑规则')">编辑</button></td>
          </tr>
        </tbody>
      </table>
    </div>
    
    <div style="margin-top:20px;">
      <h3 style="margin-bottom:12px;">👥 角色继承关系</h3>
      <div class="chart-card" style="padding:20px;">
        <div style="display:flex;justify-content:center;align-items:flex-start;gap:40px;">
          <div style="text-align:center;">
            <div style="width:80px;height:80px;border-radius:50%;background:linear-gradient(135deg,#4f46e5,#7c3aed);color:white;display:flex;align-items:center;justify-content:center;font-weight:600;margin:0 auto 8px;">👑<br>超级管理员</div>
            <div style="font-size:11px;color:var(--muted);">继承所有权限</div>
          </div>
          <div style="display:flex;flex-direction:column;gap:15px;margin-top:25px;">
            <div style="color:var(--muted);font-size:11px;">↓ 继承</div>
            <div style="color:var(--muted);font-size:11px;">↓ 继承</div>
          </div>
          <div style="display:flex;flex-direction:column;gap:12px;">
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:50px;height:50px;border-radius:50%;background:#06b6d4;color:white;display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:600;">🔧 系统管理员</div>
              <div style="font-size:11px;color:var(--muted);">系统级配置</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:50px;height:50px;border-radius:50%;background:#10b981;color:white;display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:600;">🕸️ KG管理员</div>
              <div style="font-size:11px;color:var(--muted);">图谱模块管理</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:50px;height:50px;border-radius:50%;background:#f59e0b;color:white;display:flex;align-items:center;justify-content:center;font-size:12px;font-weight:600;">📚 知识库管理员</div>
              <div style="font-size:11px;color:var(--muted);">知识库模块管理</div>
            </div>
          </div>
          <div style="display:flex;flex-direction:column;gap:15px;margin-top:25px;">
            <div style="color:var(--muted);font-size:11px;">↓ 继承</div>
            <div style="color:var(--muted);font-size:11px;">↓ 继承</div>
            <div style="color:var(--muted);font-size:11px;">↓ 继承</div>
          </div>
          <div style="display:flex;flex-direction:column;gap:12px;">
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:45px;height:45px;border-radius:50%;background:#8b5cf6;color:white;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:600;">👤 认证专家</div>
              <div style="font-size:11px;color:var(--muted);">专家协作权限</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:45px;height:45px;border-radius:50%;background:#64748b;color:white;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:600;">👥 普通用户</div>
              <div style="font-size:11px;color:var(--muted);">基础使用权限</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:45px;height:45px;border-radius:50%;background:#94a3b8;color:white;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:600;">👁️ 只读用户</div>
              <div style="font-size:11px;color:var(--muted);">仅查看权限</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  `;
  
  permRight.appendChild(dataPermSection);
}

// ========== 6. 知识库云盘增强 ==========

let ragMode = false;

function initKbEnhanced() {
  const kbHeader = document.querySelector('.kb-header');
  if (!kbHeader || kbHeader.querySelector('.kb-rag-btn')) return;
  
  // 添加 RAG 问答按钮
  const ragBtn = document.createElement('button');
  ragBtn.className = 'btn kb-rag-btn';
  ragBtn.style.cssText = 'background: linear-gradient(135deg, #8b5cf6, #ec4899); color: white; border: none;';
  ragBtn.textContent = '🤖 知识问答';
  ragBtn.onclick = toggleRagPanel;
  kbHeader.insertBefore(ragBtn, kbHeader.querySelector('.btn:last-child'));
  
  // 添加语义检索开关
  const searchBox = document.querySelector('.kb-search input');
  if (searchBox) {
    searchBox.placeholder = '🔍 语义搜索：自然语言描述你想找的内容…';
  }
  
  // 添加版本对比功能
  // 在文件卡片右键或操作菜单中添加
}

function toggleRagPanel() {
  ragMode = !ragMode;
  
  const kbMain = document.querySelector('.kb-main');
  let ragPanel = document.getElementById('ragPanel');
  
  if (ragMode) {
    if (!ragPanel) {
      ragPanel = document.createElement('div');
      ragPanel.id = 'ragPanel';
      ragPanel.style.cssText = `
        position: absolute; top: 0; right: 0; bottom: 0; width: 400px;
        background: white; border-left: 1px solid var(--border);
        display: flex; flex-direction: column; z-index: 10;
      `;
      ragPanel.innerHTML = `
        <div style="padding:16px;border-bottom:1px solid var(--border);background:linear-gradient(135deg,#ede9fe,#fce7f3);">
          <div style="font-weight:600;font-size:14px;display:flex;align-items:center;gap:8px;">
            🤖 AI 知识问答 (RAG)
            <button onclick="toggleRagPanel()" style="margin-left:auto;background:none;border:none;cursor:pointer;font-size:18px;">×</button>
          </div>
          <div style="font-size:11px;color:#6d28d9;margin-top:4px;">基于知识库的智能问答，引用源文件可追溯</div>
        </div>
        <div id="ragMessages" style="flex:1;overflow-y:auto;padding:16px;display:flex;flex-direction:column;gap:12px;">
          <div style="display:flex;gap:8px;">
            <div style="width:28px;height:28px;border-radius:50%;background:linear-gradient(135deg,#8b5cf6,#ec4899);color:white;display:flex;align-items:center;justify-content:center;font-size:12px;flex-shrink:0;">AI</div>
            <div style="background:#f8fafc;padding:10px 14px;border-radius:0 10px 10px 10px;font-size:13px;line-height:1.6;">
              你好！我是知识助手，可以基于知识库中的文档回答你的问题。<br><br>
              你可以问我关于：<br>
              • 系统架构设计<br>
              • 图算法原理<br>
              • 知识库管理<br>
              • 专家联盟使用<br><br>
              试试问："什么是混合架构？"
            </div>
          </div>
        </div>
        <div style="padding:12px;border-top:1px solid var(--border);">
          <div style="display:flex;gap:8px;">
            <input type="text" id="ragInput" placeholder="输入问题…" style="flex:1;height:36px;border:1px solid var(--border);border-radius:8px;padding:0 12px;font-size:13px;" onkeypress="if(event.key==='Enter')sendRagQuestion()">
            <button class="btn btn-primary" onclick="sendRagQuestion()">发送</button>
          </div>
          <div style="font-size:11px;color:var(--muted);margin-top:6px;">💡 回答将引用知识库中的源文档</div>
        </div>
      `;
      kbMain.style.position = 'relative';
      kbMain.appendChild(ragPanel);
    }
    ragPanel.style.display = 'flex';
    showToast('已开启 AI 知识问答');
  } else {
    if (ragPanel) ragPanel.style.display = 'none';
  }
}

function sendRagQuestion() {
  const input = document.getElementById('ragInput');
  const question = input.value.trim();
  if (!question) return;
  
  const msgContainer = document.getElementById('ragMessages');
  
  // 用户消息
  msgContainer.insertAdjacentHTML('beforeend', `
    <div style="display:flex;gap:8px;flex-direction:row-reverse;">
      <div style="width:28px;height:28px;border-radius:50%;background:#4f46e5;color:white;display:flex;align-items:center;justify-content:center;font-size:12px;flex-shrink:0;">我</div>
      <div style="background:linear-gradient(135deg,#4f46e5,#6366f1);color:white;padding:10px 14px;border-radius:10px 0 10px 10px;font-size:13px;max-width:80%;">
        ${question}
      </div>
    </div>
  `);
  
  input.value = '';
  msgContainer.scrollTop = msgContainer.scrollHeight;
  
  // 模拟 AI 回答
  setTimeout(() => {
    const answer = generateRagAnswer(question);
    msgContainer.insertAdjacentHTML('beforeend', `
      <div style="display:flex;gap:8px;">
        <div style="width:28px;height:28px;border-radius:50%;background:linear-gradient(135deg,#8b5cf6,#ec4899);color:white;display:flex;align-items:center;justify-content:center;font-size:12px;flex-shrink:0;">AI</div>
        <div style="background:#f8fafc;padding:10px 14px;border-radius:0 10px 10px 10px;font-size:13px;line-height:1.6;max-width:85%;">
          ${answer}
        </div>
      </div>
    `);
    msgContainer.scrollTop = msgContainer.scrollHeight;
  }, 800);
}

function generateRagAnswer(q) {
  const qa = {
    '混合架构': `
      <strong>混合架构</strong>是我们推荐的最优方案：<br><br>
      <strong>前端：</strong>统一工作台，三栏式布局集成所有功能<br>
      <strong>后端：</strong>模块化微服务，7个服务独立部署<br>
      <strong>算法层：</strong>统一算法核心库，跨域复用 15+ 算法<br>
      <strong>通信：</strong>gRPC 高性能服务间调用<br><br>
      <span style="font-size:11px;color:var(--muted);">📚 参考：架构设计 V3.0.pdf · KG域架构规范.md</span>
    `,
    '图算法': `
      系统支持以下<strong>图算法</strong>：<br><br>
      1. <strong>中心性分析</strong> - 度中心性、介数中心性、接近中心性<br>
      2. <strong>社区发现</strong> - Louvain 算法，模块化优化<br>
      3. <strong>PageRank</strong> - 节点重要性排名<br>
      4. <strong>最短路径</strong> - Dijkstra / A* 算法<br>
      5. <strong>激活传播</strong> - 影响力传播模型<br><br>
      所有算法在 mox-unified-algo-core 中统一实现。<br>
      <span style="font-size:11px;color:var(--muted);">📚 参考：算法归一化方案.docx · GNN入门教程.md</span>
    `,
  };
  
  for (const key in qa) {
    if (q.includes(key)) return qa[key];
  }
  
  return `
    关于"<strong>${q}</strong>"的相关信息：<br><br>
    我在知识库中找到了 <strong>3</strong> 篇相关文档，基于这些文档整理如下：<br><br>
    1. 该功能在 V3.0 版本中已实现核心能力<br>
    2. 支持多种配置选项和扩展方式<br>
    3. 详细文档可在架构设计目录中查阅<br><br>
    <span style="font-size:11px;color:var(--muted);">
      📚 引用源：架构设计 V3.0.pdf (相似度 85%)<br>
      &nbsp;&nbsp;&nbsp;算法归一化方案.docx (相似度 72%)<br>
      &nbsp;&nbsp;&nbsp;KG域架构规范.md (相似度 65%)
    </span>
  `;
}

// ========== 初始化所有增强 ==========

function initAllEnhancements() {
  // 延迟初始化，确保DOM已加载
  setTimeout(() => {
    initKgEnhanced();
    initSqlEnhanced();
    initEaEnhanced();
    initMonitorEnhanced();
    initPermissionEnhanced();
    initKbEnhanced();
    console.log('[V3.1] 全维修复优化增强已加载');
  }, 300);
}

// 页面切换时重新初始化对应模块
const _origSwitchPage = window.switchPage;
window.switchPage = function(el, pageId) {
  _origSwitchPage(el, pageId);
  setTimeout(() => {
    if (pageId === 'knowledgegraph') initKgEnhanced();
    if (pageId === 'sqlconsole') initSqlEnhanced();
    if (pageId === 'expertalliance') initEaEnhanced();
    if (pageId === 'monitor') initMonitorEnhanced();
    if (pageId === 'permission') initPermissionEnhanced();
    if (pageId === 'knowledgebase') initKbEnhanced();
  }, 100);
};

// 自动初始化
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initAllEnhancements);
} else {
  initAllEnhancements();
}
