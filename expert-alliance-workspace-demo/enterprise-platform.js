// 企业级平台交互脚本

const pageNames = {
  dashboard: '数据看板', sqlconsole: 'SQL 控制台', datamanage: '数据管理',
  knowledgegraph: '知识图谱', knowledgebase: '知识库云盘', expertalliance: '专家联盟',
  monitor: '监控告警', permission: '权限管理', audit: '审计日志', settings: '系统设置',
};

function switchPage(el, pageId) {
  document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
  el.classList.add('active');
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.getElementById('page-' + pageId).classList.add('active');
  document.getElementById('breadcrumbCurrent').textContent = pageNames[pageId] || pageId;
  if (pageId === 'knowledgegraph') initKg();
  if (pageId === 'knowledgebase') initKb();
  if (pageId === 'expertalliance') initEa();
  if (pageId === 'datamanage') initDataManage();
  if (pageId === 'audit') initAudit();
}

// === SQL 控制台 ===
const sampleSQLResult = [
  { id: 'n001', label: '专家联盟', type: '系统', degree: 8, importance: 0.95 },
  { id: 'n002', label: '知识图谱', type: '核心能力', degree: 6, importance: 0.82 },
  { id: 'n003', label: '知识库', type: '核心能力', degree: 5, importance: 0.78 },
  { id: 'n004', label: '图算法', type: '算法', degree: 4, importance: 0.65 },
  { id: 'n005', label: '向量检索', type: '算法', degree: 3, importance: 0.58 },
  { id: 'n006', label: '专家匹配', type: '能力', degree: 3, importance: 0.52 },
  { id: 'n007', label: '云存储', type: '基础设施', degree: 2, importance: 0.45 },
  { id: 'n008', label: '任务编排', type: '能力', degree: 3, importance: 0.60 },
  { id: 'n009', label: '多专家辩论', type: '协作模式', degree: 3, importance: 0.55 },
  { id: 'n010', label: '知识融合', type: '能力', degree: 3, importance: 0.50 },
];

function runSQL() {
  const tbody = document.getElementById('sqlResultBody');
  tbody.innerHTML = sampleSQLResult.map(r => `
    <tr>
      <td>${r.id}</td><td>${r.label}</td>
      <td><span class="status-tag info">${r.type}</span></td>
      <td>${r.degree}</td><td>${r.importance}</td>
    </tr>
  `).join('');
  document.getElementById('sqlResultInfo').innerHTML = `
    <span>✓ 查询成功</span><span>返回 ${sampleSQLResult.length} 行</span>
    <span>耗时 0.023s</span><span style="margin-left:auto;">数据源：kg_prod</span>
  `;
  showToast(`SQL 执行成功，返回 ${sampleSQLResult.length} 行`);
}

function loadHistory(sql) {
  document.getElementById('sqlInput').value = sql;
  showToast('已加载历史 SQL');
}

function toggleDbItem(el) {
  event.stopPropagation();
  el.classList.toggle('expanded');
}

function changeDatasource() {
  showToast('已切换数据源');
}

function exportResult() {
  showToast('结果已导出为 CSV 文件');
}

// === 数据管理 ===
const tableSchemas = {
  experts: {
    columns: ['选择', 'ID', '姓名', '领域', '评分', '任务数', '状态', '创建时间', '操作'],
    data: [
      ['', 'E001', '璇玑算法', 'KG/AI', '4.8', '128', '在线', '2026-01-15', ''],
      ['', 'E002', '架构师', 'Platform', '4.7', '95', '在线', '2026-02-20', ''],
      ['', 'E003', '知识库管家', 'KG/Data', '4.6', '203', '忙碌', '2026-01-08', ''],
      ['', 'E004', '数据分析师', 'Data/AI', '4.5', '76', '在线', '2026-03-10', ''],
      ['', 'E005', 'GNN研究员', 'AI/KG', '4.9', '42', '在线', '2026-04-15', ''],
      ['', 'E006', '安全专家', 'Security', '4.4', '68', '在线', '2026-03-22', ''],
      ['', 'E007', '产品规划师', 'Product', '4.9', '54', '离线', '2026-02-01', ''],
      ['', 'E008', '运维工程师', 'DevOps', '4.3', '156', '忙碌', '2026-01-30', ''],
    ],
  },
  documents: {
    columns: ['选择', 'ID', '文件名', '类型', '大小', '版本', '作者', '更新时间', '操作'],
    data: [
      ['', 'D001', '架构设计 V3.0.pdf', 'PDF', '2.4 MB', 'v3.0', '架构师', '2026-08-28', ''],
      ['', 'D002', 'KG域架构规范.md', 'Markdown', '1.8 MB', 'v2.1', '璇玑算法', '2026-08-27', ''],
      ['', 'D003', '算法归一化方案.docx', 'Word', '986 KB', 'v1.2', '数据分析师', '2026-08-25', ''],
      ['', 'D004', '云存储选型报告.pdf', 'PDF', '3.1 MB', 'v2.0', '架构师', '2026-08-20', ''],
      ['', 'D005', 'GNN入门教程.md', 'Markdown', '1.2 MB', 'v1.0', 'GNN研究员', '2026-08-15', ''],
      ['', 'D006', '专家匹配白皮书.pdf', 'PDF', '2.0 MB', 'v1.5', '璇玑算法', '2026-08-10', ''],
    ],
  },
  nodes: {
    columns: ['选择', 'ID', '标签', '类型', '度数', '权重', '状态', '创建时间', '操作'],
    data: sampleSQLResult.map((r, i) => ['', r.id, r.label, r.type, r.degree, r.importance, 'active', '2026-0' + (i+1) + '-15', '']),
  },
  edges: {
    columns: ['选择', 'ID', '源节点', '目标节点', '关系类型', '权重', '方向', '创建时间', '操作'],
    data: [
      ['', 'L001', '专家联盟', '知识图谱', '包含', 0.9, '双向', '2026-01-01', ''],
      ['', 'L002', '专家联盟', '知识库', '包含', 0.85, '双向', '2026-01-01', ''],
      ['', 'L003', '知识图谱', '图算法', '依赖', 0.8, '正向', '2026-01-05', ''],
      ['', 'L004', '知识库', '向量检索', '依赖', 0.75, '正向', '2026-01-08', ''],
      ['', 'L005', '专家联盟', '多专家辩论', '支持', 0.7, '正向', '2026-02-10', ''],
    ],
  },
  sessions: {
    columns: ['选择', 'ID', '会话名称', '参与专家', '状态', '消息数', '创建时间', '最后活跃', '操作'],
    data: [
      ['', 'S001', '架构优化讨论', '3', '进行中', '128', '2026-08-28', '5分钟前', ''],
      ['', 'S002', 'KG融合策略', '2', '进行中', '86', '2026-08-27', '1小时前', ''],
      ['', 'S003', '性能瓶颈分析', '2', '已结束', '45', '2026-08-25', '2天前', ''],
    ],
  },
  users: {
    columns: ['选择', 'ID', '用户名', '角色', '部门', '状态', '最后登录', '注册时间', '操作'],
    data: [
      ['', 'U001', 'admin', '超级管理员', '平台部', '正常', '今天 09:30', '2025-01-01', ''],
      ['', 'U002', 'zhangsan', 'KG管理员', 'KG组', '正常', '今天 08:45', '2025-06-15', ''],
      ['', 'U003', 'lisi', '知识库管理员', '内容部', '正常', '昨天 17:20', '2025-08-20', ''],
    ],
  },
};

let currentTable = 'experts';

function initDataManage() {
  renderTable();
}

function changeTable() {
  currentTable = document.getElementById('tableSelect').value;
  renderTable();
}

function renderTable() {
  const schema = tableSchemas[currentTable];
  const head = document.getElementById('tableHead');
  const body = document.getElementById('tableBody');
  head.innerHTML = schema.columns.map(c => c === '选择' ? '<th><input type="checkbox" class="perm-check"></th>' : `<th>${c}</th>`).join('');
  body.innerHTML = schema.data.map(row => {
    let html = schema.columns.map((col, i) => {
      if (col === '选择') return '<td><input type="checkbox" class="perm-check"></td>';
      if (col === '操作') return `<td><div class="row-actions"><button class="row-btn edit" onclick="showToast('编辑')">编辑</button><button class="row-btn delete" onclick="showToast('已删除','warning')">删除</button></div></td>`;
      if (col === '状态') {
        const m = { '在线':'success', '正常':'success', 'active':'success', '进行中':'info', '忙碌':'warning', '离线':'danger', '已结束':'warning' };
        return `<td><span class="status-tag ${m[row[i]]||'info'}">${row[i]}</span></td>`;
      }
      return `<td>${row[i]}</td>`;
    }).join('');
    return `<tr>${html}</tr>`;
  }).join('');
  document.getElementById('tableCount').textContent = `共 ${schema.data.length} 条`;
}

function filterTable() {
  const q = document.getElementById('tableSearch').value.toLowerCase();
  renderTable();
  document.querySelectorAll('#tableBody tr').forEach(row => {
    row.style.display = row.textContent.toLowerCase().includes(q) ? '' : 'none';
  });
}

function openAddRowModal() {
  const schema = tableSchemas[currentTable];
  const fields = schema.columns.filter(c => c !== '选择' && c !== '操作' && c !== 'ID');
  document.getElementById('addRowBody').innerHTML = fields.map(f => `
    <div class="form-group"><label class="form-label">${f}</label><input class="form-input" placeholder="请输入${f}"></div>
  `).join('');
  document.getElementById('addRowModal').classList.add('show');
}

function saveRow() {
  closeModal('addRowModal');
  showToast('记录已新增');
}

function closeModal(id) {
  document.getElementById(id).classList.remove('show');
}

// === 知识图谱 ===
const kgNodes = [
  { id: 'n1', label: '专家联盟', x: 400, y: 220, size: 28, color: '#4f46e5', docs: 156, experts: 12 },
  { id: 'n2', label: '知识图谱', x: 220, y: 180, size: 24, color: '#06b6d4', docs: 203, experts: 8 },
  { id: 'n3', label: '知识库', x: 580, y: 180, size: 24, color: '#10b981', docs: 892, experts: 5 },
  { id: 'n4', label: '图算法', x: 140, y: 320, size: 20, color: '#8b5cf6', docs: 45, experts: 7 },
  { id: 'n5', label: '向量检索', x: 660, y: 320, size: 20, color: '#f59e0b', docs: 67, experts: 6 },
  { id: 'n6', label: '专家匹配', x: 280, y: 380, size: 18, color: '#ec4899', docs: 23, experts: 4 },
  { id: 'n7', label: '云存储', x: 520, y: 380, size: 18, color: '#14b8a6', docs: 34, experts: 3 },
  { id: 'n8', label: '任务编排', x: 200, y: 100, size: 16, color: '#f97316', docs: 28, experts: 5 },
  { id: 'n9', label: '多专家辩论', x: 400, y: 80, size: 18, color: '#ef4444', docs: 12, experts: 8 },
  { id: 'n10', label: '知识融合', x: 600, y: 100, size: 16, color: '#84cc16', docs: 31, experts: 4 },
];
const kgEdges = [
  ['n1','n2'],['n1','n3'],['n2','n4'],['n3','n5'],['n1','n9'],
  ['n2','n6'],['n3','n7'],['n4','n6'],['n5','n7'],['n2','n8'],
  ['n1','n8'],['n3','n10'],['n1','n10'],['n6','n9'],
];
let selectedKgNode = null;

function initKg() {
  renderKg();
  renderKgCentrality();
  renderKgCommunities();
}

function renderKg() {
  const eg = document.getElementById('kgEdges');
  const ng = document.getElementById('kgNodes');
  eg.innerHTML = kgEdges.map(e => {
    const s = kgNodes.find(n => n.id === e[0]);
    const t = kgNodes.find(n => n.id === e[1]);
    const hl = selectedKgNode && (e[0] === selectedKgNode.id || e[1] === selectedKgNode.id);
    return `<line x1="${s.x}" y1="${s.y}" x2="${t.x}" y2="${t.y}" stroke="${hl?'#4f46e5':'#cbd5e1'}" stroke-width="${hl?2.5:1.5}"/>`;
  }).join('');
  ng.innerHTML = kgNodes.map(n => `
    <g style="cursor:pointer;" onclick="selectKgNode('${n.id}')">
      <circle cx="${n.x}" cy="${n.y}" r="${n.size}" fill="${n.color}" opacity="0.85" stroke="white" stroke-width="2" ${selectedKgNode?.id===n.id?'stroke="#4f46e5" stroke-width="3"':''}/>
      <text x="${n.x}" y="${n.y+n.size+14}" text-anchor="middle" font-size="11" fill="#0f172a" font-weight="500">${n.label}</text>
    </g>
  `).join('');
}

function selectKgNode(id) {
  selectedKgNode = kgNodes.find(n => n.id === id);
  renderKg();
  const el = document.getElementById('kgNodeDetail');
  el.innerHTML = `
    <div style="margin-bottom:8px;">
      <div style="width:36px;height:36px;border-radius:8px;background:${selectedKgNode.color};color:white;display:inline-flex;align-items:center;justify-content:center;font-weight:600;">${selectedKgNode.label.charAt(0)}</div>
      <div style="font-weight:600;font-size:14px;">${selectedKgNode.label}</div>
    </div>
    <div style="font-size:12px;line-height:1.8;">
      <div>📄 关联文档：${selectedKgNode.docs} 篇</div>
      <div>👥 关联专家：${selectedKgNode.experts} 位</div>
    </div>
  `;
}

function renderKgCentrality() {
  const data = [
    { name: '专家联盟', v: 0.95 },
    { name: '知识图谱', v: 0.82 },
    { name: '知识库', v: 0.78 },
    { name: '图算法', v: 0.65 },
    { name: '任务编排', v: 0.60 },
  ];
  document.getElementById('kgCentrality').innerHTML = data.map(d => `
    <div class="analysis-row">
      <span>${d.name}</span>
      <div class="analysis-bar"><div class="analysis-fill" style="width:${d.v*100}%"></div></div>
      <span style="min-width:28px;text-align:right;font-size:11px;">${(d.v*100).toFixed(0)}</span>
    </div>
  `).join('');
}

function renderKgCommunities() {
  const comms = [
    { name: '图谱算法社区', count: 4, color: '#4f46e5' },
    { name: '知识管理社区', count: 3, color: '#10b981' },
    { name: '协作模式社区', count: 3, color: '#f59e0b' },
  ];
  document.getElementById('kgCommunities').innerHTML = comms.map(c => `
    <div class="analysis-row">
      <span style="display:flex;align-items:center;gap:6px;">
        <span style="width:8px;height:8px;border-radius:50%;background:${c.color};"></span>${c.name}
      </span>
      <span style="font-size:11px;color:var(--muted);">${c.count}节点</span>
    </div>
  `).join('');
}

function runKgAnalysis(type) {
  const msgs = { centrality:'中心性分析完成', community:'社区发现：3个社区', pagerank:'PageRank计算完成', shortest:'最短路径：2跳', propagation:'激活传播完成' };
  showToast(msgs[type] || '分析完成');
}

// === 知识库 ===
const kbFiles = [
  { icon: '📐', name: '架构设计 V3.0.pdf', size: '2.4 MB', time: '10分钟前' },
  { icon: '🕸️', name: 'KG域架构规范.md', size: '1.8 MB', time: '2小时前' },
  { icon: '📊', name: '算法归一化方案.docx', size: '986 KB', time: '昨天' },
  { icon: '☁️', name: '云存储选型报告.pdf', size: '3.1 MB', time: '3天前' },
  { icon: '🧠', name: 'GNN入门教程.md', size: '1.2 MB', time: '1周前' },
  { icon: '🎯', name: '专家匹配白皮书.pdf', size: '2.0 MB', time: '2周前' },
  { icon: '📚', name: '知识库运营手册.docx', size: '5.6 MB', time: '1个月前' },
  { icon: '🎼', name: '任务编排设计.pdf', size: '1.5 MB', time: '1个月前' },
  { icon: '🔒', name: '安全架构规范.pdf', size: '2.2 MB', time: '2个月前' },
  { icon: '📈', name: '性能优化报告.docx', size: '1.8 MB', time: '2个月前' },
  { icon: '💻', name: 'API接口文档.md', size: '890 KB', time: '3个月前' },
  { icon: '📋', name: '需求规格说明书.docx', size: '4.2 MB', time: '3个月前' },
];

function initKb() {
  document.getElementById('fileGrid').innerHTML = kbFiles.map(f => `
    <div class="file-card" onclick="showToast('打开：${f.name}')">
      <div class="file-card-icon">${f.icon}</div>
      <div class="file-card-name">${f.name}</div>
      <div class="file-card-meta">${f.size} · ${f.time}</div>
    </div>
  `).join('');
}

// === 专家联盟 ===
const eaExperts = [
  { id:'e1', name:'璇玑算法', avatar:'璇', role:'图算法专家', color:'#4f46e5', status:'online', skills:['知识图谱','图算法','RAG'], rating:4.8, tasks:128 },
  { id:'e2', name:'架构师', avatar:'架', role:'系统架构专家', color:'#06b6d4', status:'online', skills:['分布式','微服务','架构'], rating:4.7, tasks:95 },
  { id:'e3', name:'知识库管家', avatar:'知', role:'知识管理专家', color:'#10b981', status:'busy', skills:['知识管理','语义检索'], rating:4.6, tasks:203 },
  { id:'e4', name:'数据分析师', avatar:'数', role:'数据分析专家', color:'#f59e0b', status:'online', skills:['数据分析','Python'], rating:4.5, tasks:76 },
  { id:'e5', name:'GNN研究员', avatar:'G', role:'图神经网络专家', color:'#8b5cf6', status:'online', skills:['GNN','深度学习'], rating:4.9, tasks:42 },
  { id:'e6', name:'安全专家', avatar:'安', role:'安全架构专家', color:'#ef4444', status:'online', skills:['安全','渗透测试'], rating:4.4, tasks:68 },
];
let selectedExpert = eaExperts[0];
let eaChatMsgs = [];

function initEa() {
  renderExpertList();
  renderEaDetail();
  renderEaChat();
}

function renderExpertList() {
  document.getElementById('expertList').innerHTML = eaExperts.map(e => `
    <div class="expert-card ${selectedExpert.id===e.id?'active':''}" onclick="selectEaExpert('${e.id}')">
      <div class="expert-avatar" style="background:${e.color}">${e.avatar}</div>
      <div class="expert-info">
        <div class="expert-name">${e.name} <span style="font-size:10px;color:${e.status==='online'?'#10b981':'#f59e0b'};">●</span></div>
        <div class="expert-role">${e.role}</div>
        <div class="expert-skills">${e.skills.slice(0,2).map(s=>`<span class="expert-skill">${s}</span>`).join('')}</div>
      </div>
    </div>
  `).join('');
}

function filterExperts(q) {
  const filtered = eaExperts.filter(e => e.name.includes(q)||e.role.includes(q)||e.skills.some(s=>s.includes(q)));
  document.getElementById('expertList').innerHTML = filtered.map(e => `
    <div class="expert-card" onclick="selectEaExpert('${e.id}')">
      <div class="expert-avatar" style="background:${e.color}">${e.avatar}</div>
      <div class="expert-info">
        <div class="expert-name">${e.name}</div>
        <div class="expert-role">${e.role}</div>
      </div>
    </div>
  `).join('');
}

function selectEaExpert(id) {
  selectedExpert = eaExperts.find(e => e.id === id);
  eaChatMsgs = [{ sender:selectedExpert.name, avatar:selectedExpert.avatar, color:selectedExpert.color, text:`您好！我是${selectedExpert.name}，${selectedExpert.role}。`, self:false }];
  renderExpertList();
  renderEaDetail();
  renderEaChat();
}

function renderEaDetail() {
  const e = selectedExpert;
  document.getElementById('eaDetail').innerHTML = `
    <div class="ea-detail-avatar" style="background:${e.color}">${e.avatar}</div>
    <div class="ea-detail-info">
      <div class="ea-detail-name">${e.name} <span class="status-tag success" style="margin-left:8px;">● 在线</span></div>
      <div class="ea-detail-role">${e.role}</div>
      <div style="margin-top:8px;">${e.skills.map(s=>`<span class="expert-skill" style="font-size:11px;padding:2px 8px;">${s}</span>`).join(' ')}</div>
    </div>
    <div class="ea-detail-stats">
      <div class="stat-item"><div class="stat-value">${e.rating}</div><div class="stat-label">评分</div></div>
      <div class="stat-item"><div class="stat-value">${e.tasks}</div><div class="stat-label">任务</div></div>
      <div class="stat-item"><div class="stat-value">98%</div><div class="stat-label">好评率</div></div>
    </div>
    <div class="ea-detail-actions">
      <button class="btn btn-primary" onclick="showToast('已发起协作邀请')">+ 协作</button>
      <button class="btn" onclick="showToast('专家主页')">主页</button>
    </div>
  `;
}

function renderEaChat() {
  const c = document.getElementById('eaChatMessages');
  c.innerHTML = eaChatMsgs.map(m => `
    <div class="chat-msg ${m.self?'self':''}">
      <div class="chat-avatar-sm" style="background:${m.color}">${m.avatar}</div>
      <div class="chat-bubble">${m.text}</div>
    </div>
  `).join('');
  c.scrollTop = c.scrollHeight;
}

function handleEaChatKeypress(e) {
  if (e.key === 'Enter') sendEaMessage();
}

function sendEaMessage() {
  const input = document.getElementById('eaChatInput');
  const text = input.value.trim();
  if (!text) return;
  eaChatMsgs.push({ sender:'我', avatar:'我', color:'#64748b', text, self:true });
  input.value = '';
  renderEaChat();
  setTimeout(() => {
    const replies = ['这个问题我来分析一下。','从知识图谱角度，建议混合架构。','算法层归一化能减少重复实现。','后端模块化可独立扩缩容。','调用统一算法核心库即可完成。'];
    eaChatMsgs.push({ sender:selectedExpert.name, avatar:selectedExpert.avatar, color:selectedExpert.color, text:replies[Math.floor(Math.random()*replies.length)], self:false });
    renderEaChat();
  }, 700);
}

function selectRole(el) {
  document.querySelectorAll('.role-item').forEach(r => r.classList.remove('active'));
  el.classList.add('active');
  showToast('已切换角色');
}

// === 审计日志 ===
const auditLogs = [
  ['2026-08-30 14:32:15','admin','login','系统','用户登录系统','192.168.1.100','成功'],
  ['2026-08-30 14:28:03','zhangsan','query','知识图谱','执行SQL查询','192.168.1.105','成功'],
  ['2026-08-30 14:15:42','lisi','insert','知识库','上传文档《GNN进阶教程.pdf》','192.168.1.108','成功'],
  ['2026-08-30 14:02:18','wangwu','update','专家联盟','更新专家技能标签','192.168.1.120','成功'],
  ['2026-08-30 13:55:09','admin','update','权限管理','修改角色权限配置','192.168.1.100','成功'],
  ['2026-08-30 13:42:33','zhangsan','delete','知识图谱','删除节点 n12345','192.168.1.105','成功'],
  ['2026-08-30 13:30:21','qianqi','query','审计日志','查询近7天审计日志','192.168.1.130','成功'],
  ['2026-08-30 13:18:47','lisi','update','知识库','移动文档至架构设计目录','192.168.1.108','成功'],
  ['2026-08-30 13:05:12','wangwu','login','系统','登录失败：密码错误','192.168.1.120','失败'],
  ['2026-08-30 12:58:36','admin','update','系统设置','修改密码策略配置','192.168.1.100','成功'],
  ['2026-08-30 12:45:20','zhangsan','query','知识图谱','中心性分析查询','192.168.1.105','成功'],
  ['2026-08-30 12:30:08','admin','insert','权限管理','新增用户 zhaoliu','192.168.1.100','成功'],
];

function initAudit() {
  const opClasses = { login:'op-login', query:'op-query', insert:'op-insert', update:'op-update', delete:'op-delete' };
  const opNames = { login:'登录', query:'查询', insert:'新增', update:'修改', delete:'删除' };
  document.getElementById('auditBody').innerHTML = auditLogs.map(row => `
    <tr>
      <td><input type="checkbox" class="perm-check"></td>
      <td>${row[0]}</td><td>${row[1]}</td>
      <td><span class="op-type ${opClasses[row[2]]}">${opNames[row[2]]}</span></td>
      <td>${row[3]}</td><td>${row[4]}</td><td>${row[5]}</td>
      <td><span class="status-tag ${row[6]==='成功'?'success':'danger'}">${row[6]}</span></td>
      <td><button class="row-btn edit" onclick="showToast('查看详情')">详情</button></td>
    </tr>
  `).join('');
}

// === Toast ===
function showToast(msg, type='success') {
  const c = document.getElementById('toastContainer');
  const t = document.createElement('div');
  t.className = `toast ${type}`;
  const icon = type==='success'?'✓':type==='error'?'✕':'⚠';
  t.innerHTML = `<span>${icon}</span>${msg}`;
  c.appendChild(t);
  setTimeout(() => { t.style.opacity='0'; t.style.transform='translateX(100%)'; t.style.transition='all 0.3s'; setTimeout(()=>t.remove(),300); }, 2500);
}

// 初始化
document.addEventListener('DOMContentLoaded', function() {
  initDataManage();
  initAudit();
});
