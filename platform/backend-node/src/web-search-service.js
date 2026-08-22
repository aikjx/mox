'use strict';

/**
 * 联网搜索服务（WebSearchService）
 * - 统一搜索入口，支持多搜索引擎，配置持久化到 data/web_search_config.json
 * - API Key 使用与 llm-gateway 相同的 AES-256-GCM 加密存储
 * - 结果统一为 { results: [{ title, url, snippet }], engine }
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const DATA_DIR = path.join(__dirname, '..', 'data');
const CIPHER_KEY = process.env.LLM_CIPHER_KEY || 'ous-llm-gateway-enterprise-key-2024';

const SEARCH_ENGINES = {
  bing: {
    name: 'Bing（必应）',
    needKey: false,
    description: '免费免 Key，国内可直接访问，中文结果质量好（推荐默认）'
  },
  duckduckgo: {
    name: 'DuckDuckGo',
    needKey: false,
    description: '免费免 Key（境内网络可能受限）'
  },
  tavily: {
    name: 'Tavily',
    needKey: true,
    description: 'AI 搜索 API，结果质量高，有免费额度（tavily.com）'
  },
  bocha: {
    name: '博查 Bocha',
    needKey: true,
    description: '国内可用的中文网页搜索 API（bochaai.com）'
  },
  searxng: {
    name: 'SearXNG（自托管）',
    needKey: false,
    description: '私有部署的元搜索引擎，需填写 Base URL'
  }
};

const DEFAULT_CONFIG = {
  enabled: true,
  engine: 'bing',
  api_key: '',
  base_url: '',
  max_results: 5,
  timeout_ms: 10000
};

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
    console.error('[web-search] writeJSON', file, e.message);
    return false;
  }
}

function encryptApiKey(apiKey) {
  if (!apiKey) return '';
  if (apiKey.startsWith('{')) return apiKey; // 已加密
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
  let encrypted = cipher.update(apiKey, 'utf8', 'hex');
  encrypted += cipher.final('hex');
  const tag = cipher.getAuthTag().toString('hex');
  return JSON.stringify({ iv: iv.toString('hex'), encrypted, tag });
}

function decryptApiKey(encryptedStr) {
  if (!encryptedStr) return '';
  try {
    if (!encryptedStr.startsWith('{')) return encryptedStr;
    const obj = JSON.parse(encryptedStr);
    const iv = Buffer.from(obj.iv, 'hex');
    const tag = Buffer.from(obj.tag, 'hex');
    const decipher = crypto.createDecipheriv('aes-256-gcm', Buffer.from(CIPHER_KEY.padEnd(32).slice(0, 32)), iv);
    decipher.setAuthTag(tag);
    let decrypted = decipher.update(obj.encrypted, 'hex', 'utf8');
    decrypted += decipher.final('utf8');
    return decrypted;
  } catch (e) {
    return '';
  }
}

function maskApiKey(encrypted) {
  const key = decryptApiKey(encrypted);
  if (!key) return '';
  if (key.length <= 8) return '****';
  return key.slice(0, 4) + '****' + key.slice(-4);
}

async function fetchWithTimeout(url, options = {}, timeoutMs = 10000) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...options, signal: controller.signal });
  } finally {
    clearTimeout(timeoutId);
  }
}

function decodeEntities(s) {
  return String(s || '')
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(parseInt(code, 10)))
    .replace(/&#x([0-9a-f]+);/gi, (_, code) => String.fromCodePoint(parseInt(code, 16)))
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#x27;|&#39;/g, "'")
    .replace(/&nbsp;|&ensp;|&emsp;/g, ' ')
    .replace(/&middot;/g, '·');
}

function stripTags(s) {
  return decodeEntities(String(s || '').replace(/<[^>]+>/g, '')).replace(/\s+/g, ' ').trim();
}

class WebSearchService {
  constructor() {
    this._loadConfig();
  }

  _loadConfig() {
    const stored = readJSON('web_search_config.json', {});
    this.config = { ...DEFAULT_CONFIG, ...stored };
    // 兼容：明文 key 加密落盘
    if (this.config.api_key && !String(this.config.api_key).startsWith('{')) {
      this.config.api_key = encryptApiKey(this.config.api_key);
      writeJSON('web_search_config.json', this._persistShape());
    }
  }

  _persistShape() {
    const { enabled, engine, base_url, max_results, timeout_ms } = this.config;
    return { enabled, engine, api_key: this.config.api_key, base_url, max_results, timeout_ms };
  }

  _saveConfig() {
    writeJSON('web_search_config.json', this._persistShape());
  }

  getConfig() {
    const { enabled, engine, base_url, max_results, timeout_ms } = this.config;
    return {
      enabled,
      engine,
      base_url,
      max_results,
      timeout_ms,
      has_key: !!decryptApiKey(this.config.api_key),
      api_key_masked: maskApiKey(this.config.api_key)
    };
  }

  getEngines() {
    return Object.entries(SEARCH_ENGINES).map(([id, e]) => ({ id, ...e }));
  }

  updateConfig(updates) {
    const allowed = ['enabled', 'engine', 'base_url', 'max_results', 'timeout_ms'];
    for (const k of allowed) {
      if (updates[k] !== undefined) this.config[k] = updates[k];
    }
    if (updates.engine && !SEARCH_ENGINES[updates.engine]) {
      throw new Error(`不支持的搜索引擎: ${updates.engine}`);
    }
    if (updates.api_key !== undefined) {
      this.config.api_key = updates.api_key ? encryptApiKey(updates.api_key) : '';
    }
    this._saveConfig();
    return this.getConfig();
  }

  isReady() {
    if (!this.config.enabled) return false;
    const engine = SEARCH_ENGINES[this.config.engine];
    if (!engine) return false;
    if (engine.needKey && !decryptApiKey(this.config.api_key)) return false;
    if (this.config.engine === 'searxng' && !this.config.base_url) return false;
    return true;
  }

  /**
   * 统一搜索入口
   * @returns {{ results: Array<{title,url,snippet}>, engine: string, duration_ms: number }}
   */
  async search(query) {
    const q = String(query || '').trim();
    if (!q) throw new Error('搜索词为空');
    if (!this.config.enabled) throw new Error('联网搜索未启用');
    const engine = SEARCH_ENGINES[this.config.engine];
    if (!engine) throw new Error(`不支持的搜索引擎: ${this.config.engine}`);
    if (engine.needKey && !decryptApiKey(this.config.api_key)) {
      throw new Error(`${engine.name} 需要配置 API Key`);
    }

    const start = Date.now();
    let results;
    switch (this.config.engine) {
      case 'bing':
        results = await this._searchBing(q);
        break;
      case 'tavily':
        results = await this._searchTavily(q);
        break;
      case 'bocha':
        results = await this._searchBocha(q);
        break;
      case 'searxng':
        results = await this._searchSearxng(q);
        break;
      case 'duckduckgo':
      default:
        results = await this._searchDuckDuckGo(q);
        break;
    }
    return {
      results: results.slice(0, this.config.max_results),
      engine: this.config.engine,
      engine_name: engine.name,
      duration_ms: Date.now() - start
    };
  }

  // ---------- Bing（免费无 Key，国内可访问） ----------
  async _searchBing(query) {
    const url = 'https://www.bing.com/search?q=' + encodeURIComponent(query) + '&mkt=zh-CN&count=' + Math.max(this.config.max_results, 10);
    const res = await fetchWithTimeout(url, {
      method: 'GET',
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36',
        'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8'
      }
    }, this.config.timeout_ms);
    if (!res.ok) throw new Error(`Bing HTTP ${res.status}`);
    const html = await res.text();

    const results = [];
    const seen = new Set();
    const blockRe = /<li class="b_algo"[\s\S]*?<\/li>/g;
    let m;
    while ((m = blockRe.exec(html)) !== null) {
      const block = m[0];
      const link = block.match(/<h2[^>]*>\s*<a[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/);
      if (!link || !/^https?:\/\//.test(link[1])) continue;
      if (seen.has(link[1])) continue;
      seen.add(link[1]);
      const snip = block.match(/<p[^>]*>([\s\S]*?)<\/p>/);
      results.push({
        title: stripTags(link[2]),
        url: link[1],
        snippet: snip ? stripTags(snip[1]) : ''
      });
    }
    if (!results.length) throw new Error('Bing 未返回可解析结果（可能被限流）');
    return results;
  }

  // ---------- DuckDuckGo（免费无 Key） ----------
  async _searchDuckDuckGo(query) {
    const url = 'https://html.duckduckgo.com/html/?q=' + encodeURIComponent(query);
    const res = await fetchWithTimeout(url, {
      method: 'GET',
      headers: {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36',
        'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8'
      }
    }, this.config.timeout_ms);
    if (!res.ok) throw new Error(`DuckDuckGo HTTP ${res.status}`);
    const html = await res.text();

    const results = [];
    const seen = new Set();
    // 结果块：<a rel="nofollow" class="result__a" href="...">title</a> ... <a class="result__snippet" ...>snippet</a>
    const linkRe = /<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/g;
    let m;
    while ((m = linkRe.exec(html)) !== null) {
      let href = decodeEntities(m[1]);
      // DDG 的跳转链接 uddg=-encoded 真实地址
      const uddg = href.match(/[?&]uddg=([^&]+)/);
      if (uddg) href = decodeURIComponent(uddg[1]);
      if (!/^https?:\/\//.test(href)) continue;
      if (seen.has(href)) continue;
      seen.add(href);
      results.push({ title: stripTags(m[2]), url: href, snippet: '' });
    }
    // snippet 与 title 按顺序配对
    const snipRe = /<a[^>]*class="result__snippet"[^>]*>([\s\S]*?)<\/a>/g;
    let i = 0;
    while ((m = snipRe.exec(html)) !== null) {
      if (i < results.length) results[i].snippet = stripTags(m[1]);
      i++;
    }
    if (!results.length) throw new Error('DuckDuckGo 未返回可解析结果（可能被限流或网络受限）');
    return results;
  }

  // ---------- Tavily ----------
  async _searchTavily(query) {
    const apiKey = decryptApiKey(this.config.api_key);
    const res = await fetchWithTimeout('https://api.tavily.com/search', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
      body: JSON.stringify({
        query,
        max_results: this.config.max_results,
        search_depth: 'basic',
        include_answer: false
      })
    }, this.config.timeout_ms);
    if (!res.ok) throw new Error(`Tavily HTTP ${res.status}`);
    const data = await res.json();
    const results = (data.results || []).map((r) => ({
      title: r.title || '',
      url: r.url || '',
      snippet: (r.content || '').slice(0, 500)
    }));
    if (!results.length) throw new Error('Tavily 未返回结果');
    return results;
  }

  // ---------- 博查 Bocha ----------
  async _searchBocha(query) {
    const apiKey = decryptApiKey(this.config.api_key);
    const res = await fetchWithTimeout('https://api.bochaai.com/v1/web-search', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
      body: JSON.stringify({ query, summary: true, count: this.config.max_results })
    }, this.config.timeout_ms);
    if (!res.ok) throw new Error(`Bocha HTTP ${res.status}`);
    const data = await res.json();
    const pages = (data.data && data.data.webPages) || [];
    const results = pages.map((p) => ({
      title: p.name || '',
      url: p.url || '',
      snippet: (p.summary || p.snippet || '').slice(0, 500)
    }));
    if (!results.length) throw new Error('Bocha 未返回结果');
    return results;
  }

  // ---------- SearXNG（自托管） ----------
  async _searchSearxng(query) {
    const base = String(this.config.base_url || '').replace(/\/+$/, '');
    if (!base) throw new Error('SearXNG 未配置 Base URL');
    const url = `${base}/search?q=${encodeURIComponent(query)}&format=json`;
    const res = await fetchWithTimeout(url, { method: 'GET' }, this.config.timeout_ms);
    if (!res.ok) throw new Error(`SearXNG HTTP ${res.status}`);
    const data = await res.json();
    const results = (data.results || []).map((r) => ({
      title: r.title || '',
      url: r.url || '',
      snippet: (r.content || '').slice(0, 500)
    }));
    if (!results.length) throw new Error('SearXNG 未返回结果');
    return results;
  }

  // 将搜索结果格式化为注入 LLM 的上下文
  buildSearchContext(query, searchResult) {
    const lines = [
      '【联网搜索结果】',
      `用户问题：「${query}」`,
      `搜索引擎：${searchResult.engine_name}（检索耗时 ${searchResult.duration_ms}ms）`,
      '以下为最新检索到的网页结果，请优先依据这些实时信息回答；若与你的训练知识冲突，以搜索结果为准。回答末尾请用 [1][2] 角标标注引用来源。'
    ];
    searchResult.results.forEach((r, idx) => {
      lines.push(`[${idx + 1}] ${r.title}\n    链接: ${r.url}\n    摘要: ${r.snippet || '（无摘要）'}`);
    });
    return lines.join('\n');
  }

  async test() {
    if (!this.config.enabled) {
      return { success: false, message: '联网搜索未启用，请先开启开关' };
    }
    try {
      const result = await this.search('今天是几号');
      return {
        success: true,
        message: `连接成功（${result.engine_name}），返回 ${result.results.length} 条结果，耗时 ${result.duration_ms}ms`,
        sample: result.results.slice(0, 3)
      };
    } catch (e) {
      return { success: false, message: `搜索失败: ${e.message}` };
    }
  }
}

let instance = null;

function getWebSearchService() {
  if (!instance) {
    instance = new WebSearchService();
  }
  return instance;
}

module.exports = { WebSearchService, getWebSearchService, SEARCH_ENGINES };
