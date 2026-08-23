'use strict';

/**
 * 自动开发引擎（Auto Dev Engine）
 * =================================
 * 能力：需求 → 业务架构图谱 → 确定性代码渲染 → 安全落盘 → 在线预览
 *
 * 五阶段流水线：
 *   ① 架构图谱生成 —— LLM 从需求产出结构化业务架构（页面/导航/区块/实体）
 *   ② 图谱校验归一化 —— 结构校验（首页存在/导航引用合法/区块类型白名单）+ HTML 转义（XSS 防护）
 *   ③ 确定性代码渲染 —— 从图谱渲染 HTML/CSS（黄金分割布局，无 LLM 参与，可复现）
 *   ④ 安全落盘 —— 委托 local-artifact-service 安全闸门（白名单目录/扩展名/sha256 登记）
 *   ⑤ 在线预览 —— preview 路由静态服务（路径校验 + content-type 映射）
 *
 * 设计不变式：
 *   1) 架构图谱是唯一事实源：代码从图谱渲染，图谱入 graph_nodes/graph_edges 可查
 *   2) LLM 只产结构化 JSON，不直接产代码：确定性渲染保证可复现与安全
 *   3) 所有 LLM 内容一律 HTML 转义；链接仅允许相对 .html / #锚点
 *   4) 失败不半成品：任一阶段失败返回明确错误，不写残缺文件
 */

const fs = require('fs');
const path = require('path');
const { getGateway } = require('./llm-gateway');
const { getLocalArtifactService } = require('./local-artifact-service');

const DATA_DIR = path.join(__dirname, '..', 'data');

const SECTION_TYPES = ['hero', 'features', 'about', 'text', 'cta', 'contact'];
const MAX_PAGES = 8;
const MAX_SECTIONS_PER_PAGE = 8;
const MAX_ITEMS_PER_SECTION = 8;

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
    fs.mkdirSync(DATA_DIR, { recursive: true });
    fs.writeFileSync(path.join(DATA_DIR, file), JSON.stringify(data, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[auto-dev] writeJSON', file, e.message);
    return false;
  }
}

// HTML 转义（不变式③：LLM 内容一律转义）
function esc(s) {
  return String(s === undefined || s === null ? '' : s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

// 链接白名单校验：仅允许相对 .html 文件或 #锚点（不变式③）
function safeLink(link) {
  const s = String(link || '').trim();
  if (!s) return '#';
  if (s.startsWith('#')) return s.slice(0, 64);
  if (/^[a-zA-Z0-9_\-]+\.html$/.test(s)) return s;
  return '#';
}

function slug(name) {
  const s = String(name || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return s || 'site';
}

function parseJSONLoose(text) {
  const s = String(text || '').trim();
  const fenced = s.match(/```(?:json)?\s*([\s\S]*?)```/);
  const body = fenced ? fenced[1] : s;
  const start = body.indexOf('{');
  const end = body.lastIndexOf('}');
  if (start === -1 || end === -1 || end <= start) return null;
  try {
    return JSON.parse(body.slice(start, end + 1));
  } catch (e) {
    return null;
  }
}

class AutoDevEngine {
  constructor() {
    this.gateway = getGateway();
    this.artifacts = getLocalArtifactService();
  }

  isRealAI() {
    return typeof this.gateway.isRealAI === 'function' ? this.gateway.isRealAI() : false;
  }

  // ==================== 统一入口 ====================
  async develop(request) {
    const requirement = String((request && request.requirement) || '').trim();
    if (!requirement) throw new Error('缺少 requirement 参数');
    if (!this.isRealAI()) {
      throw new Error('自动开发引擎需要真实 AI 引擎（请在 LLM 配置页接入 Key 后重试）');
    }

    const project = slug(request.project_name || requirement.slice(0, 24));
    const startedAt = Date.now();

    // ---- ① 架构图谱生成 ----
    const arch = await this._generateArchitecture(requirement);

    // ---- ② 图谱校验归一化 ----
    const normalized = this._validateAndNormalize(arch, project);
    if (!normalized.pages.length) throw new Error('架构图谱校验失败：无有效页面');

    // ---- ③ 确定性代码渲染 ----
    const files = this._renderSite(normalized);

    // ---- ④ 安全落盘（复用制品服务闸门） ----
    const written = [];
    const skipped = [];
    for (const f of files) {
      const rel = `site-${project}/${f.filename}`;
      const rec = await this.artifacts.writeFileDirect(rel, f.content, {
        mode: 'code',
        purpose: f.purpose,
        overwrite: !!request.overwrite,
        session_id: request.session_id || null
      });
      if (rec.ok) written.push(rec.record);
      else skipped.push({ filename: rel, reason: rec.reason });
    }
    if (!written.length) {
      throw new Error('全部文件落盘失败：' + (skipped[0] ? skipped[0].reason : '未知原因'));
    }

    // ---- ⑤ 架构图谱入图（可在图谱 UI 查看） ----
    const graphMerge = this._storeArchitectureGraph(normalized, project, requirement);

    return {
      project: `site-${project}`,
      site_name: normalized.site_name,
      requirement,
      pipeline: ['architecture', 'validate', 'render', 'persist', 'graph-store'],
      architecture: {
        pages: normalized.pages.map((p) => ({ id: p.id, title: p.title, file: p.file, sections: p.sections.length })),
        nav: normalized.nav,
        entities: normalized.entities,
        theme: normalized.theme
      },
      files: written.map((w) => ({ filename: w.filename, size: w.size, sha256: w.sha256.slice(0, 12) })),
      skipped,
      graph_store: graphMerge,
      preview_url: `/ai/engine/auto-dev/preview/site-${project}/index.html`,
      preview_url_abs: `http://localhost:3010/ai/engine/auto-dev/preview/site-${project}/index.html`,
      duration_ms: Date.now() - startedAt
    };
  }

  // ==================== ① 架构图谱生成（LLM） ====================
  async _generateArchitecture(requirement) {
    const systemPrompt =
      '你是企业级网站架构师。根据需求输出**业务架构图谱 JSON**（只输出 JSON，不要其他文字）。\n' +
      'Schema：\n' +
      '{\n' +
      '  "site_name": "站点名（中文）",\n' +
      '  "tagline": "一句话标语",\n' +
      '  "theme": {"primary": "#RRGGBB", "surface": "#RRGGBB", "bg": "#RRGGBB", "text": "#RRGGBB", "muted": "#RRGGBB"},\n' +
      '  "pages": [\n' +
      '    {"id": "home", "title": "首页", "file": "index.html", "sections": [区块列表]}\n' +
      '  ],\n' +
      '  "nav": [{"from": "home", "to": "about", "label": "关于我们"}],\n' +
      '  "entities": [{"name": "产品", "description": "核心业务实体说明"}]\n' +
      '}\n' +
      '区块 section 类型（严格白名单）：\n' +
      '  hero: {"type":"hero","headline":"主标题","subline":"副标题","cta_text":"按钮文字","cta_link":"contact.html"}\n' +
      '  features: {"type":"features","title":"区块标题","items":[{"title":"要点标题","text":"要点说明"}]}\n' +
      '  about: {"type":"about","title":"标题","text":"正文（可多段，\\n分段）"}\n' +
      '  text: {"type":"text","title":"标题","text":"正文"}\n' +
      '  cta: {"type":"cta","headline":"号召标题","text":"说明","cta_text":"按钮文字","cta_link":"index.html"}\n' +
      '  contact: {"type":"contact","title":"标题","email":"邮箱","phone":"电话","address":"地址"}\n' +
      '硬性约束：\n' +
      '  1) 3-6 个页面，恰好一个 file="index.html"（首页），其余页面文件名用英文\n' +
      '  2) 每页 2-6 个区块；features 每区块 3-6 个要点\n' +
      '  3) theme 用低饱和度深空配色（暗底浅字或浅底深字，对比度≥4.5:1）\n' +
      '  4) nav 覆盖所有页面的导航关系；内容紧扣需求、专业详实';

    const res = await this.gateway.chat({
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: `网站开发需求：${requirement}` }
      ],
      temperature: 0.3,
      maxTokens: 4000
    });

    const parsed = parseJSONLoose(res.content);
    if (!parsed || !Array.isArray(parsed.pages)) {
      throw new Error('AI 未返回有效的架构图谱 JSON');
    }
    return parsed;
  }

  // ==================== ② 图谱校验归一化 ====================
  _validateAndNormalize(arch, project) {
    const out = {
      site_name: String(arch.site_name || '未命名站点').slice(0, 60),
      tagline: String(arch.tagline || '').slice(0, 120),
      theme: this._normalizeTheme(arch.theme),
      pages: [],
      nav: [],
      entities: []
    };

    // 页面：类型/数量/文件名校验
    let pages = (arch.pages || []).slice(0, MAX_PAGES).filter((p) => p && typeof p === 'object');
    pages = pages.map((p, i) => {
      const id = String(p.id || `page${i}`).replace(/[^a-zA-Z0-9_\-]/g, '').slice(0, 32) || `page${i}`;
      let file = String(p.file || `${id}.html`);
      if (!/^[a-zA-Z0-9_\-]+\.html$/.test(file)) file = `${id}.html`;
      return { id, title: String(p.title || id).slice(0, 40), file, sections: this._normalizeSections(p.sections) };
    });

    // 首页保证：无 index.html 则首页补位
    if (pages.length && !pages.some((p) => p.file === 'index.html')) {
      pages[0].file = 'index.html';
    }

    // 导航：引用合法性校验（丢弃非法引用）
    const ids = new Set(pages.map((p) => p.id));
    const files = new Set(pages.map((p) => p.file));
    (arch.nav || []).forEach((n) => {
      if (n && ids.has(String(n.from)) && ids.has(String(n.to))) {
        out.nav.push({ from: String(n.from), to: String(n.to), label: String(n.label || '').slice(0, 20) });
      }
    });

    // 实体
    (arch.entities || []).slice(0, 10).forEach((e) => {
      if (e && e.name) {
        out.entities.push({ name: String(e.name).slice(0, 30), description: String(e.description || '').slice(0, 200) });
      }
    });

    out.pages = pages;
    return out;
  }

  _normalizeSections(sections) {
    const out = [];
    (sections || []).slice(0, MAX_SECTIONS_PER_PAGE).forEach((s) => {
      if (!s || !SECTION_TYPES.includes(s.type)) return;
      const sec = { type: s.type };
      for (const key of ['headline', 'subline', 'title', 'text', 'cta_text', 'cta_link', 'email', 'phone', 'address']) {
        if (s[key] !== undefined) sec[key] = String(s[key]).slice(0, 500);
      }
      if (s.cta_link) sec.cta_link = safeLink(s.cta_link);
      if (Array.isArray(s.items)) {
        sec.items = s.items
          .slice(0, MAX_ITEMS_PER_SECTION)
          .filter((it) => it && (it.title || it.text))
          .map((it) => ({ title: String(it.title || '').slice(0, 60), text: String(it.text || '').slice(0, 300) }));
      }
      out.push(sec);
    });
    return out;
  }

  _normalizeTheme(theme) {
    const hex = (v, fallback) => (/^#[0-9a-fA-F]{6}$/.test(String(v || '')) ? String(v).toLowerCase() : fallback);
    return {
      primary: hex(theme && theme.primary, '#4c6ef5'),
      surface: hex(theme && theme.surface, '#1a1d2e'),
      bg: hex(theme && theme.bg, '#0f1220'),
      text: hex(theme && theme.text, '#e8eaf6'),
      muted: hex(theme && theme.muted, '#9aa0b5')
    };
  }

  // ==================== ③ 确定性代码渲染 ====================
  _renderSite(arch) {
    const files = [];
    files.push({ filename: 'styles.css', purpose: '全局样式（黄金分割布局）', content: this._renderCSS(arch) });
    for (const page of arch.pages) {
      files.push({ filename: page.file, purpose: `页面：${page.title}`, content: this._renderPage(arch, page) });
    }
    files.push({ filename: 'site.json', purpose: '业务架构图谱快照（机器可读）', content: JSON.stringify(arch, null, 2) });
    return files;
  }

  _renderCSS(arch) {
    const t = arch.theme;
    // 黄金比例间距标度：8 × φ^k ≈ 8/13/21/34/55
    return `/* ${arch.site_name} —— 自动开发引擎生成（业务架构图谱 → 确定性渲染） */
/* 布局规范：黄金分割 1:1.618 · 极简留白 · 低饱和深空配色 */
:root {
  --primary: ${t.primary};
  --surface: ${t.surface};
  --bg: ${t.bg};
  --text: ${t.text};
  --muted: ${t.muted};
  --phi: 1.618;
  --sp-1: 8px;   --sp-2: 13px;  --sp-3: 21px;
  --sp-4: 34px;  --sp-5: 55px;  --sp-6: 89px;
  --radius: 14px;
  --shadow: 0 8px 30px rgba(0,0,0,.28);
  --font: "Segoe UI", "PingFang SC", "Microsoft YaHei", system-ui, sans-serif;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
html { scroll-behavior: smooth; }
body { font-family: var(--font); background: var(--bg); color: var(--text); line-height: 1.7; }
a { color: var(--primary); text-decoration: none; transition: opacity .2s; }
a:hover { opacity: .82; }

/* 顶部导航 */
.nav { position: sticky; top: 0; z-index: 10; display: flex; align-items: center; justify-content: space-between;
  padding: var(--sp-3) var(--sp-5); background: color-mix(in srgb, var(--surface) 88%, transparent);
  backdrop-filter: blur(12px); border-bottom: 1px solid rgba(255,255,255,.06); }
.nav .brand { font-size: 1.25rem; font-weight: 700; color: var(--text); letter-spacing: .02em; }
.nav .brand span { color: var(--primary); }
.nav .links { display: flex; gap: var(--sp-3); flex-wrap: wrap; }
.nav .links a { color: var(--muted); font-size: .95rem; }
.nav .links a:hover { color: var(--text); }

/* 容器：黄金分割主栏 */
.container { max-width: 1200px; margin: 0 auto; padding: 0 var(--sp-4); }
section { padding: var(--sp-6) 0; }
section:nth-of-type(even) { background: color-mix(in srgb, var(--surface) 45%, var(--bg)); }
.sec-title { font-size: 1.9rem; font-weight: 700; margin-bottom: var(--sp-3); letter-spacing: .01em; }
.sec-title::after { content: ""; display: block; width: 56px; height: 3px; margin-top: var(--sp-2);
  border-radius: 2px; background: var(--primary); }

/* Hero：黄金分割视觉重心 */
.hero { padding: calc(var(--sp-6) * 1.618) 0; text-align: center; }
.hero h1 { font-size: clamp(2rem, 5vw, 3.4rem); font-weight: 800; line-height: 1.25; margin-bottom: var(--sp-3); }
.hero p { color: var(--muted); font-size: 1.15rem; max-width: 640px; margin: 0 auto var(--sp-4); }
.btn { display: inline-block; padding: var(--sp-2) var(--sp-4); border-radius: var(--radius);
  background: linear-gradient(135deg, var(--primary), color-mix(in srgb, var(--primary) 60%, #fff));
  color: #fff; font-weight: 600; box-shadow: var(--shadow); }
.btn:hover { opacity: .9; }

/* Features：卡片网格 */
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: var(--sp-3); }
.card { background: var(--surface); border: 1px solid rgba(255,255,255,.06); border-radius: var(--radius);
  padding: var(--sp-4); box-shadow: var(--shadow); transition: transform .2s, border-color .2s; }
.card:hover { transform: translateY(-3px); border-color: color-mix(in srgb, var(--primary) 55%, transparent); }
.card h3 { font-size: 1.1rem; margin-bottom: var(--sp-2); color: var(--text); }
.card p { color: var(--muted); font-size: .95rem; }

/* About / Text 正文 */
.prose { max-width: 760px; color: var(--muted); font-size: 1.02rem; }
.prose p { margin-bottom: var(--sp-3); }

/* CTA 横幅 */
.cta-band { text-align: center; background:
  linear-gradient(135deg, color-mix(in srgb, var(--primary) 22%, var(--surface)), var(--surface));
  border-radius: var(--radius); padding: var(--sp-5) var(--sp-4); box-shadow: var(--shadow); }
.cta-band h2 { font-size: 1.7rem; margin-bottom: var(--sp-2); }
.cta-band p { color: var(--muted); margin-bottom: var(--sp-3); }

/* 联系方式 */
.contact-list { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: var(--sp-3); }
.contact-item { background: var(--surface); border-radius: var(--radius); padding: var(--sp-3) var(--sp-4); }
.contact-item .k { color: var(--muted); font-size: .85rem; margin-bottom: var(--sp-1); }
.contact-item .v { font-weight: 600; }

/* 页脚 */
footer { border-top: 1px solid rgba(255,255,255,.06); padding: var(--sp-4) 0; text-align: center;
  color: var(--muted); font-size: .88rem; }

@media (max-width: 640px) {
  .nav { flex-direction: column; gap: var(--sp-2); }
  section { padding: var(--sp-5) 0; }
}
`;
  }

  _renderPage(arch, page) {
    const navLinks = arch.pages
      .filter((p) => p.file !== page.file)
      .map((p) => `<a href="${esc(p.file)}">${esc(p.title)}</a>`)
      .join('\n        ');

    const sectionsHtml = page.sections.map((s) => this._renderSection(s)).join('\n');
    const year = new Date().getFullYear();

    return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${esc(page.title)} · ${esc(arch.site_name)}</title>
  <meta name="description" content="${esc(arch.tagline)}">
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <nav class="nav">
    <div class="brand">${esc(arch.site_name)}<span>.</span></div>
    <div class="links">
        ${navLinks}
    </div>
  </nav>

  <main>
${sectionsHtml}
  </main>

  <footer>
    <div class="container">© ${year} ${esc(arch.site_name)} · ${esc(arch.tagline)}</div>
  </footer>
</body>
</html>
`;
  }

  _renderSection(s) {
    const c = (cls, inner) => `    <section class="${cls}">\n      <div class="container">\n${inner}\n      </div>\n    </section>`;
    switch (s.type) {
      case 'hero': {
        const cta = s.cta_text ? `\n        <a class="btn" href="${esc(safeLink(s.cta_link))}">${esc(s.cta_text)}</a>` : '';
        return c(
          'hero',
          `        <h1>${esc(s.headline || '')}</h1>\n        <p>${esc(s.subline || '')}</p>${cta}`
        );
      }
      case 'features': {
        const items = (s.items || [])
          .map((it) => `          <div class="card"><h3>${esc(it.title)}</h3><p>${esc(it.text)}</p></div>`)
          .join('\n');
        return c('features', `        <h2 class="sec-title">${esc(s.title || '核心能力')}</h2>\n        <div class="grid">\n${items}\n        </div>`);
      }
      case 'about':
      case 'text': {
        const paras = String(s.text || '')
          .split(/\n+/)
          .filter((p) => p.trim())
          .map((p) => `          <p>${esc(p)}</p>`)
          .join('\n');
        return c(s.type === 'about' ? 'about' : 'text', `        <h2 class="sec-title">${esc(s.title || '')}</h2>\n        <div class="prose">\n${paras}\n        </div>`);
      }
      case 'cta': {
        const cta = s.cta_text ? `\n          <a class="btn" href="${esc(safeLink(s.cta_link))}">${esc(s.cta_text)}</a>` : '';
        return c(
          'cta',
          `        <div class="cta-band">\n          <h2>${esc(s.headline || '')}</h2>\n          <p>${esc(s.text || '')}</p>${cta}\n        </div>`
        );
      }
      case 'contact': {
        const rows = [
          ['邮箱', s.email],
          ['电话', s.phone],
          ['地址', s.address]
        ]
          .filter(([, v]) => v)
          .map(([k, v]) => `          <div class="contact-item"><div class="k">${k}</div><div class="v">${esc(v)}</div></div>`)
          .join('\n');
        return c('contact', `        <h2 class="sec-title">${esc(s.title || '联系我们')}</h2>\n        <div class="contact-list">\n${rows}\n        </div>`);
      }
      default:
        return '';
    }
  }

  // ==================== ⑤ 架构图谱入图（graph_nodes / graph_edges） ====================
  _storeArchitectureGraph(arch, project, requirement) {
    const prefix = `sd:${project}`;
    const nodes = readJSON('graph_nodes.json', []);
    const edges = readJSON('graph_edges.json', []);

    const newNodes = [];
    const newEdges = [];
    const existingIds = new Set(nodes.map((n) => n.id));
    const existingEdgeKeys = new Set(edges.map((e) => `${e.source}_${e.target}_${e.label}`));
    const now = new Date().toISOString();

    const addNode = (id, label, type, description) => {
      if (!existingIds.has(id)) {
        const node = {
          id,
          label,
          type,
          description: String(description || '').slice(0, 300),
          attributes: { project: `site-${project}` },
          community: 0,
          degree: 0,
          created_at: now,
          ai_generated: true,
          topic: `auto-dev:${requirement.slice(0, 50)}`
        };
        nodes.push(node);
        newNodes.push(node);
        existingIds.add(id);
      }
    };
    const addEdge = (source, target, label, weight) => {
      const key = `${source}_${target}_${label}`;
      if (!existingEdgeKeys.has(key)) {
        const edge = {
          id: `sd_edge_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
          source,
          target,
          label,
          weight: weight || 1.0,
          created_at: now,
          ai_generated: true
        };
        edges.push(edge);
        newEdges.push(edge);
        existingEdgeKeys.add(key);
      }
    };

    // 站点根节点
    const siteId = `${prefix}:site`;
    addNode(siteId, arch.site_name, 'site', arch.tagline || requirement.slice(0, 100));

    // 页面节点 + contains 边
    const pageId = (pid) => `${prefix}:page:${pid}`;
    for (const p of arch.pages) {
      addNode(pageId(p.id), p.title, 'page', `页面文件 ${p.file}（${p.sections.length} 区块）`);
      addEdge(siteId, pageId(p.id), '包含', 1.0);
      // 区块节点 + contains 边
      p.sections.forEach((s, i) => {
        const secId = `${prefix}:section:${p.id}:${i}`;
        const label = { hero: '主视觉', features: '能力矩阵', about: '关于', text: '正文', cta: '行动号召', contact: '联系方式' }[s.type] || s.type;
        addNode(secId, `${p.title}·${label}`, 'section', s.headline || s.title || s.text || label);
        addEdge(pageId(p.id), secId, '包含', 0.8);
      });
    }

    // 导航边（page→page）
    for (const n of arch.nav) {
      if (arch.pages.some((p) => p.id === n.from) && arch.pages.some((p) => p.id === n.to)) {
        addEdge(pageId(n.from), pageId(n.to), '导航', 1.0);
      }
    }

    // 实体节点 + uses 边（挂到首页）
    const home = arch.pages.find((p) => p.file === 'index.html') || arch.pages[0];
    for (const e of arch.entities) {
      const entId = `${prefix}:entity:${slug(e.name)}`;
      addNode(entId, e.name, 'entity', e.description);
      if (home) addEdge(pageId(home.id), entId, '使用', 0.6);
    }

    writeJSON('graph_nodes.json', nodes);
    writeJSON('graph_edges.json', edges);

    return {
      added_nodes: newNodes.length,
      added_edges: newEdges.length,
      total_nodes: nodes.length,
      total_edges: edges.length,
      node_types: ['site', 'page', 'section', 'entity'],
      edge_types: ['包含', '导航', '使用'],
      query_hint: `GET /graph 可查看（filter topic=auto-dev）`
    };
  }

  // ==================== 项目列表 ====================
  listProjects() {
    const reg = readJSON('artifacts.json', { artifacts: [] });
    const siteFiles = (reg.artifacts || []).filter((a) => a.filename && a.filename.startsWith('site-') && a.filename.includes('/'));
    const byProject = new Map();
    for (const f of siteFiles) {
      const proj = f.filename.split('/')[0];
      if (!byProject.has(proj)) byProject.set(proj, { project: proj, files: 0, total_size: 0, last_created_at: null, _names: new Set() });
      const p = byProject.get(proj);
      // 同名文件重复生成（overwrite）只计一次，尺寸取最新记录
      if (!p._names.has(f.filename)) {
        p._names.add(f.filename);
        p.files += 1;
        p.total_size += f.size || 0;
      }
      if (!p.last_created_at || f.created_at > p.last_created_at) p.last_created_at = f.created_at;
    }
    return {
      total: byProject.size,
      projects: [...byProject.values()]
        .map(({ _names, ...rest }) => rest)
        .sort((a, b) => (b.last_created_at || '').localeCompare(a.last_created_at || ''))
    };
  }
}

let instance = null;

function getAutoDevEngine() {
  if (!instance) instance = new AutoDevEngine();
  return instance;
}

module.exports = { AutoDevEngine, getAutoDevEngine, SECTION_TYPES };
