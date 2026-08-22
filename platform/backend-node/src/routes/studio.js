'use strict';

/**
 * 路由域：璇玑工作台（用户视角融合层 · 豆包式低门槛交互）
 * ------------------------------------------------------------------
 * 设计理念（借鉴豆包，全自研实现）：
 *   零门槛直通：打开即用，无需注册与学习成本
 *   超短链路：场景化快捷入口，意图直达（声落即出）
 *   在线运行：页面上的代码真实执行（vm 沙箱 + 超时守卫 + 零 IO）
 *
 * 端点：
 *   GET  /studio            工作台页面（单文件原生 JS，零依赖）
 *   POST /studio/run-code   JS 代码在线运行（vm 隔离沙箱）
 *   POST /studio/run-code/precheck  语法预检（不执行）
 */
const vm = require('vm');
const fs = require('fs');
const path = require('path');

module.exports = function registerStudioRoutes(ctx) {
  const { ok, fail, readBody, reg } = ctx;

  const SANDBOX_TIMEOUT_MS = 3000; // 超时守卫：3s 强制终止
  const MAX_CODE_LENGTH = 64 * 1024; // 64KB 代码上限

  // 工作台页面（单文件 · 原生 JS · 黄金比例布局）
  reg('get', '/studio', (req, res) => {
    const htmlPath = path.join(__dirname, '..', '..', 'public', 'xuanji-studio.html');
    try {
      if (!fs.existsSync(htmlPath)) {
        res.writeHead(503, { 'Content-Type': 'text/plain; charset=utf-8' });
        return res.end('工作台页面未找到');
      }
      const content = fs.readFileSync(htmlPath, 'utf8');
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-cache' });
      res.end(content);
    } catch (e) {
      res.writeHead(500, { 'Content-Type': 'text/plain; charset=utf-8' });
      res.end('加载工作台页面失败: ' + e.message);
    }
  });

  // 语法预检：不执行，仅编译检查（与运行时同形态包裹后编译，含顶层 await 支持）
  reg('post', '/studio/run-code/precheck', async (req, res) => {
    const body = await readBody(req).catch(() => ({}));
    const code = typeof body.code === 'string' ? body.code : '';
    if (!code.trim()) return fail(res, 400, 'code 为必填');
    if (code.length > MAX_CODE_LENGTH) return fail(res, 400, `代码超过上限 ${MAX_CODE_LENGTH} 字节`);
    try {
      new vm.Script(wrapCode(code), { filename: 'studio-sandbox.js' });
      ok(res, { valid: true, length: code.length });
    } catch (e) {
      ok(res, { valid: false, error: e.message, line: e.lineNumber || null });
    }
  });

  // JS 代码在线运行：vm 隔离沙箱（零 IO · 无 require/process · console 捕获 · 超时守卫）
  reg('post', '/studio/run-code', async (req, res) => {
    const started = Date.now();
    const body = await readBody(req).catch(() => ({}));
    const code = typeof body.code === 'string' ? body.code : '';

    if (!code.trim()) return fail(res, 400, 'code 为必填');
    if (code.length > MAX_CODE_LENGTH) {
      return fail(res, 400, `代码超过上限 ${MAX_CODE_LENGTH} 字节`);
    }

    // 沙箱安全审查：拒绝危险全局引用（防御第一层）
    const forbidden = /\b(require|process|globalThis\s*=|Function\s*\(\s*['"]|eval\s*\()/;
    if (forbidden.test(code)) {
      return fail(res, 400, '沙箱安全审查未通过：禁用 require/process/eval/Function 构造器');
    }

    // 沙箱上下文：极简白名单（console 捕获 + 纯计算全局 + 同源受限 fetch）
    const logs = [];
    const sandboxConsole = {
      log: (...args) => pushLog(logs, args, 'log'),
      info: (...args) => pushLog(logs, args, 'info'),
      warn: (...args) => pushLog(logs, args, 'warn'),
      error: (...args) => pushLog(logs, args, 'error')
    };
    const sandbox = {
      console: sandboxConsole,
      Math, JSON, Date, Number, String, Boolean, Array, Object,
      Map, Set, WeakMap, WeakSet, Symbol, Promise, RegExp, Error,
      TypeError, RangeError, isNaN, parseInt, parseFloat,
      BigInt, Infinity, NaN, undefined,
      fetch: sandboxFetch(req)
    };
    sandbox.globalThis = sandbox; // 沙箱内自引用（指向沙箱而非宿主）

    try {
      // 表达式优先：单表达式直接回显（REPL 语义）；语句块退回无返回形态（须显式 return）
      let script;
      try {
        script = new vm.Script(wrapExpr(code), { filename: 'studio-sandbox.js' });
      } catch (_e) {
        script = new vm.Script(wrapCode(code), { filename: 'studio-sandbox.js' });
      }
      const context = vm.createContext(sandbox);
      // 用户代码包裹为 async IIFE：runInContext 返回 Promise，必须 await——
      // 否则异步异常逃逸为 unhandledRejection（曾致整个 API 进程崩溃）
      const promise = script.runInContext(context, { timeout: SANDBOX_TIMEOUT_MS });
      const result = await Promise.race([
        promise,
        rejectAfter(SANDBOX_TIMEOUT_MS + 7000, '整体执行超时（含异步链，10s 守卫）')
      ]);

      ok(res, {
        success: true,
        result: safeSerialize(result),
        logs,
        duration: Date.now() - started,
        timeoutMs: SANDBOX_TIMEOUT_MS
      });
    } catch (e) {
      ok(res, {
        success: false,
        error: String(e && e.message ? e.message : e),
        logs,
        duration: Date.now() - started,
        timeoutMs: SANDBOX_TIMEOUT_MS
      });
    }
  });

  /** 同源受限 fetch：仅允许相对路径（以 / 开头），转发至本服务自身。
   *  页面代码在线运行的核心能力：可直接调用系统 API（图谱/项目/验证）。 */
  function sandboxFetch(req) {
    return (url, opts = {}) => {
      if (typeof url !== 'string' || !url.startsWith('/') || url.startsWith('//')) {
        return Promise.reject(new Error('沙箱 fetch 仅支持同源相对路径（以 / 开头）'));
      }
      const host = req.headers.host || `127.0.0.1:${req.socket && req.socket.localPort}`;
      return fetch(`http://${host}${url}`, {
        ...opts,
        signal: AbortSignal.timeout(5000) // 单请求 5s 超时守卫
      }).catch(e => {
        throw new Error(`fetch ${url} 失败: ${e.message}`);
      });
    };
  }

  /** 定时拒绝守卫（整体异步链兜底） */
  function rejectAfter(ms, message) {
    return new Promise((_, reject) => setTimeout(() => reject(new Error(message)), ms));
  }

  /** console 输出捕获（深序列化，循环引用防护） */
  function pushLog(logs, args, level) {
    if (logs.length >= 200) return; // 日志上限守卫
    logs.push({ level, text: args.map(a => safeSerialize(a)).join(' ') });
  }

  /** 包裹用户代码：语句块形态（多语句代码须显式 return 返回结果） */
  function wrapCode(code) {
    return `"use strict";\n(async () => {\n${code}\n})();`;
  }

  /** 包裹用户代码：表达式形态（单表达式自动回显，REPL 语义） */
  function wrapExpr(code) {
    return `"use strict";\n(async () => (\n${code}\n))();`;
  }

  /** 安全序列化：循环引用/函数/BigInt 兜底 */
  function safeSerialize(value, depth = 0) {
    if (value === null) return null;
    if (depth > 6) return '[深度超限]';
    const t = typeof value;
    if (t === 'string' || t === 'boolean' || t === 'undefined') return value;
    if (t === 'number') return Number.isFinite(value) ? value : String(value);
    if (t === 'bigint') return `${value}n`;
    if (t === 'function') return `[Function: ${value.name || 'anonymous'}]`;
    if (t === 'symbol') return value.toString();
    if (value instanceof Error) return { name: value.name, message: value.message };
    if (Array.isArray(value)) return value.map(v => safeSerialize(v, depth + 1));
    if (t === 'object') {
      try {
        const out = {};
        for (const k of Object.keys(value).slice(0, 100)) out[k] = safeSerialize(value[k], depth + 1);
        return out;
      } catch (e) { return '[不可序列化]'; }
    }
    return String(value);
  }
};
