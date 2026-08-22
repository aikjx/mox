'use strict';

/**
 * 本地制品引擎（Local Artifact Service）
 * =========================================
 * 归一化设计依据：docs/modules/local-artifact-agent.md
 *
 * 三层收口：
 *   输入收口 —— 统一 ArtifactRequest { mode, message, session_id, overwrite }
 *   过程收口 —— 五步流水线：意图判定→制品规划→内容生成→安全闸门→落盘登记
 *   输出收口 —— 统一 ArtifactReport { created, skipped, reply }
 *
 * 四条不变式：
 *   1) 白名单落盘：只写 workspace/artifacts/，路径解析后必须仍在根内
 *   2) 扩展名白名单：文档 .md/.txt；代码 .js/.ts/.py/.rs/.vue/.html/.css/.json/.sql/.sh/.java/.go
 *   3) 失败不伤主链路：任何异常降级为普通对话，不阻塞 /ai/chat
 *   4) 登记可追溯：sha256 + 会话 + 时间；覆盖须显式 overwrite
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { getGateway } = require('./llm-gateway');

const REPO_ROOT = path.join(__dirname, '..', '..', '..');
const ARTIFACT_ROOT = path.join(REPO_ROOT, 'workspace', 'artifacts');
const DATA_DIR = path.join(__dirname, '..', 'data');
const REGISTRY_FILE = 'artifacts.json';

const ALLOWED_EXTENSIONS = {
  document: ['.md', '.txt'],
  code: ['.js', '.ts', '.py', '.rs', '.vue', '.html', '.css', '.json', '.sql', '.sh', '.java', '.go']
};

const MODE_META = {
  document: {
    label: '文档模式',
    planPrompt:
      '你是文件规划助手。判断用户消息是否要求创建/保存文档文件。若是，输出一个 JSON（只输出 JSON，不要其他文字）：\n' +
      '{"create": true, "files": [{"filename": "xxx.md", "purpose": "一句话说明"}]}\n' +
      '文件名必须以 .md 或 .txt 结尾，使用英文/数字/下划线/连字符命名，可含中文。\n' +
      '若用户没有创建文件的意图，输出 {"create": false}。\n\n用户消息：'
  },
  code: {
    label: '代码模式',
    planPrompt:
      '你是代码文件规划助手。判断用户消息是否要求创建/保存代码文件。若是，输出一个 JSON（只输出 JSON，不要其他文字）：\n' +
      '{"create": true, "files": [{"filename": "xxx.js", "purpose": "一句话说明"}]}\n' +
      '文件名扩展名必须是 .js/.ts/.py/.rs/.vue/.html/.css/.json/.sql/.sh/.java/.go 之一，根据用户意图选择最合适的语言。\n' +
      '若用户没有创建代码文件的意图，输出 {"create": false}。\n\n用户消息：'
  }
};

function sha256(content) {
  return crypto.createHash('sha256').update(content, 'utf8').digest('hex');
}

function readRegistry() {
  try {
    const fp = path.join(DATA_DIR, REGISTRY_FILE);
    if (!fs.existsSync(fp)) return { artifacts: [] };
    const raw = fs.readFileSync(fp, 'utf8');
    return raw ? JSON.parse(raw) : { artifacts: [] };
  } catch (e) {
    return { artifacts: [] };
  }
}

function writeRegistry(reg) {
  try {
    fs.mkdirSync(DATA_DIR, { recursive: true });
    fs.writeFileSync(path.join(DATA_DIR, REGISTRY_FILE), JSON.stringify(reg, null, 2), 'utf8');
    return true;
  } catch (e) {
    console.error('[artifact] writeRegistry:', e.message);
    return false;
  }
}

// 安全闸门②：路径逃逸校验（解析后必须仍在制品根目录内）
function safeJoin(filename) {
  const clean = String(filename || '').trim().replace(/\\/g, '/');
  if (!clean || clean.includes('..') || /^[a-zA-Z]:/.test(clean) || clean.startsWith('/')) {
    return null;
  }
  const target = path.resolve(ARTIFACT_ROOT, clean);
  const rootWithSep = ARTIFACT_ROOT + path.sep;
  if (target !== ARTIFACT_ROOT && !target.startsWith(rootWithSep)) return null;
  return target;
}

// 安全闸门①：扩展名白名单
function extAllowed(filename, mode) {
  const ext = path.extname(String(filename || '')).toLowerCase();
  return (ALLOWED_EXTENSIONS[mode] || []).includes(ext);
}

function parseJSONLoose(text) {
  const s = String(text || '').trim();
  // 剥离可能的 ```json 围栏
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

class LocalArtifactService {
  constructor(gateway) {
    this.gateway = gateway || getGateway();
  }

  getConfig() {
    return {
      artifact_root: ARTIFACT_ROOT,
      modes: Object.keys(MODE_META),
      allowed_extensions: ALLOWED_EXTENSIONS,
      registered: readRegistry().artifacts.length
    };
  }

  listArtifacts() {
    const reg = readRegistry();
    return {
      total: reg.artifacts.length,
      artifacts: reg.artifacts.slice().reverse().slice(0, 200)
    };
  }

  isRealAI() {
    return typeof this.gateway.isRealAI === 'function' ? this.gateway.isRealAI() : false;
  }

  /**
   * 统一入口：处理一次带制品模式的对话
   * @returns {Promise<{created: Array, skipped: Array, reply: string|null, mode: string}>}
   */
  async process(request) {
    const mode = MODE_META[request.mode] ? request.mode : null;
    const message = String(request.message || '').trim();
    const result = { created: [], skipped: [], reply: null, mode: request.mode || null };

    if (!mode) return result;
    if (!message) return result;
    if (!this.isRealAI()) {
      result.skipped.push({ filename: null, reason: '本地制品引擎需要真实 AI 引擎（请在 LLM 配置页接入）' });
      return result;
    }

    try {
      // ---- 第 1 步：意图判定 + 制品规划（合并为一次 LLM 调用，规划温度 0.2）----
      const planRes = await this.gateway.chat({
        messages: [{ role: 'user', content: MODE_META[mode].planPrompt + message }],
        sessionId: request.session_id,
        temperature: 0.2,
        maxTokens: 400
      });
      const plan = parseJSONLoose(planRes.content);
      if (!plan || !plan.create || !Array.isArray(plan.files) || !plan.files.length) {
        // 无创建意图：不产生任何制品，主链路继续普通回复
        return result;
      }

      // ---- 第 2 步：安全闸门（白名单）----
      const accepted = [];
      for (const f of plan.files.slice(0, 5)) {
        const filename = String(f.filename || '').trim();
        if (!filename) continue;
        if (!extAllowed(filename, mode)) {
          result.skipped.push({ filename, reason: `扩展名不在${MODE_META[mode].label}白名单内` });
          continue;
        }
        const absPath = safeJoin(filename);
        if (!absPath) {
          result.skipped.push({ filename, reason: '路径不合法（禁止逃逸制品根目录）' });
          continue;
        }
        const exists = fs.existsSync(absPath);
        if (exists && !request.overwrite) {
          result.skipped.push({ filename, reason: '文件已存在（需显式 overwrite 才覆盖）' });
          continue;
        }
        accepted.push({ filename, absPath, purpose: String(f.purpose || ''), exists });
      }
      if (!accepted.length) return result;

      // ---- 第 3 步：内容生成（生成温度 0.5，逐文件独立生成保证质量）----
      for (const item of accepted) {
        try {
          const content = await this._generateContent(mode, message, item);
          if (!content || !content.trim()) {
            result.skipped.push({ filename: item.filename, reason: 'AI 未生成有效内容' });
            continue;
          }
          // ---- 第 4 步：落盘 ----
          fs.mkdirSync(path.dirname(item.absPath), { recursive: true });
          fs.writeFileSync(item.absPath, content, 'utf8');

          // ---- 第 5 步：登记（sha256 + 会话 + 时间）----
          const record = {
            id: `art_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
            filename: item.filename,
            rel_path: path.relative(REPO_ROOT, item.absPath).replace(/\\/g, '/'),
            abs_path: item.absPath,
            mode,
            size: Buffer.byteLength(content, 'utf8'),
            sha256: sha256(content),
            session_id: request.session_id || null,
            purpose: item.purpose,
            overwritten: item.exists,
            created_at: new Date().toISOString()
          };
          const reg = readRegistry();
          reg.artifacts.push(record);
          writeRegistry(reg);
          result.created.push(record);
        } catch (e) {
          result.skipped.push({ filename: item.filename, reason: '生成/写盘失败: ' + e.message });
        }
      }
      return result;
    } catch (e) {
      // 不变式③：失败不伤主链路
      result.skipped.push({ filename: null, reason: '制品引擎异常（已降级为普通对话）: ' + e.message });
      return result;
    }
  }

  async _generateContent(mode, userMessage, item) {
    let prompt;
    if (mode === 'document') {
      prompt =
        `请为以下需求撰写完整文档，直接写入文件「${item.filename}」（用途：${item.purpose || '按用户需求'}）。\n` +
        `要求：Markdown 格式、结构完整（标题/章节/要点）、内容专业详实、直接输出文档正文（不要任何解释性前后缀）。\n\n用户需求：${userMessage}`;
    } else {
      prompt =
        `请根据以下需求编写完整代码文件「${item.filename}」（用途：${item.purpose || '按用户需求'}）。\n` +
        `要求：可直接运行的完整实现、含必要注释、遵循该语言最佳实践、只输出代码（不要 Markdown 围栏以外的解释文字）。\n\n用户需求：${userMessage}`;
    }
    const res = await this.gateway.chat({
      messages: [{ role: 'user', content: prompt }],
      sessionId: null,
      temperature: 0.5,
      maxTokens: 3000
    });
    let content = String(res.content || '').trim();
    // 剥离整体代码围栏（保留内部必要的围栏，如文档中的代码块）
    const fence = content.match(/^```[\w]*\s*\n([\s\S]*?)\n```$/);
    if (fence && mode === 'code') content = fence[1].trim();
    return content;
  }

  // 生成产物回执文案（附加在对话回复后）
  buildReplySuffix(result) {
    const parts = [];
    if (result.created.length) {
      parts.push(
        `\n\n---\n📦 **${MODE_META[result.mode] ? MODE_META[result.mode].label : '制品'}** 已创建 ${result.created.length} 个文件：` +
          result.created.map((c) => `\n- \`${c.rel_path}\`（${(c.size / 1024).toFixed(1)}KB${c.overwritten ? '，已覆盖' : ''}）`).join('')
      );
    }
    if (result.skipped.length) {
      parts.push('\n\n⚠️ 跳过 ' + result.skipped.length + ' 项：' + result.skipped.map((s) => `${s.filename || '(未命名)'}（${s.reason}）`).join('；'));
    }
    return parts.join('');
  }
}

let instance = null;

function getLocalArtifactService() {
  if (!instance) {
    instance = new LocalArtifactService();
  }
  return instance;
}

module.exports = { LocalArtifactService, getLocalArtifactService, ALLOWED_EXTENSIONS };
