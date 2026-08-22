'use strict';

/**
 * Melody2Score 企业级转谱引擎模块
 * 
 * 注册 Node.js 后端路由，代理请求到 Python FastAPI 服务（port 3008）。
 * 所有路由前缀为 /melody2score/（Vite 代理会剥离 /api 前缀）
 */

const { registerModule, BaseModule } = require('./index');

const routes = [
  // 健康检查
  { method: 'get', path: '/melody2score/health', handler: async (req, res) => {
    try {
      const data = await _proxy('GET', '/health');
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.ok(res, { status: 'offline', service: 'melody2score', error: e.message });
    }
  }},

  // 服务状态
  { method: 'get', path: '/melody2score/status', handler: async (req, res) => {
    try {
      const data = await _proxy('GET', '/status');
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.fail(res, 503, 'Python 服务不可用: ' + e.message);
    }
  }},

  // 列出内置样例
  { method: 'get', path: '/melody2score/samples', handler: async (req, res) => {
    try {
      const data = await _proxy('GET', '/samples');
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.ok(res, []); // 降级返回空列表
    }
  }},

  // 获取样例音频
  { method: 'get', path: '/melody2score/sample-audio', handler: (req, res) => {
    const parsed = require('url').parse(req.url, true);
    const file = parsed.query.file;
    if (!file) return BaseModule.fail(res, 400, '缺少 file 参数');
    _proxyStream(req, res, 'GET', `/sample-audio?file=${encodeURIComponent(file)}`);
  }},

  // 识别上传的音频文件
  { method: 'post', path: '/melody2score/recognize', handler: (req, res) => {
    _proxyFormData(req, res, '/recognize');
  }},

  // 识别内置样例
  { method: 'post', path: '/melody2score/recognize-sample', handler: (req, res) => {
    _proxyFormData(req, res, '/recognize-sample');
  }},

  // 识别录音（base64 wav）
  { method: 'post', path: '/melody2score/recognize-record', handler: async (req, res) => {
    try {
      const body = await BaseModule.readBody(req);
      const data = await _proxy('POST', '/recognize-record', body);
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.fail(res, 502, '识别失败: ' + e.message);
    }
  }},

  // 导出歌谱图片
  { method: 'post', path: '/melody2score/export-sheet', handler: async (req, res) => {
    try {
      const body = await BaseModule.readBody(req);
      const data = await _proxy('POST', '/export-sheet', body);
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.fail(res, 502, '导出失败: ' + e.message);
    }
  }},

  // 保存报告
  { method: 'post', path: '/melody2score/save-report', handler: async (req, res) => {
    try {
      const body = await BaseModule.readBody(req);
      const data = await _proxy('POST', '/save-report', body);
      BaseModule.ok(res, data);
    } catch (e) {
      BaseModule.fail(res, 502, '保存报告失败: ' + e.message);
    }
  }},

  // 下载文件
  { method: 'get', path: '/melody2score/download', handler: (req, res) => {
    const parsed = require('url').parse(req.url, true);
    const fname = parsed.query.file;
    if (!fname) return BaseModule.fail(res, 400, '缺少 file 参数');
    _proxyStream(req, res, 'GET', `/download/${encodeURIComponent(fname)}`);
  }},

  // 批量识别
  { method: 'post', path: '/melody2score/batch-recognize', handler: (req, res) => {
    _proxyFormData(req, res, '/batch-recognize');
  }},
];

// ========== 代理工具 ==========

const PYTHON_HOST = process.env.MELODY2SCORE_HOST || '127.0.0.1';
const PYTHON_PORT = parseInt(process.env.MELODY2SCORE_PORT || '3008', 10);

const http = require('http');

function _proxy(method, path, body) {
  return new Promise((resolve, reject) => {
    const opts = {
      hostname: PYTHON_HOST,
      port: PYTHON_PORT,
      path: `/api/melody2score${path}`,
      method: method,
      timeout: 120000,
      headers: {}
    };
    if (body) {
      const data = JSON.stringify(body);
      opts.headers['Content-Type'] = 'application/json';
      opts.headers['Content-Length'] = Buffer.byteLength(data);
    }
    const req = http.request(opts, (res) => {
      let chunks = '';
      res.on('data', (c) => { chunks += c; });
      res.on('end', () => {
        try { resolve(JSON.parse(chunks)); }
        catch (e) { reject(new Error('Python 响应解析失败')); }
      });
    });
    req.on('error', (e) => reject(new Error(`Python 服务连接失败: ${e.message}`)));
    req.on('timeout', () => { req.destroy(); reject(new Error('Python 服务超时')); });
    if (body) req.write(JSON.stringify(body));
    req.end();
  });
}

function _proxyFormData(req, res, path) {
  const opts = {
    hostname: PYTHON_HOST,
    port: PYTHON_PORT,
    path: `/api/melody2score${path}`,
    method: 'POST',
    headers: { ...req.headers },
    timeout: 120000
  };
  delete opts.headers['host'];
  const proxyReq = http.request(opts, (proxyRes) => {
    res.writeHead(proxyRes.statusCode, {
      ...proxyRes.headers,
      'Access-Control-Allow-Origin': '*'
    });
    proxyRes.pipe(res);
  });
  proxyReq.on('error', (e) => {
    res.writeHead(502, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
    res.end(JSON.stringify({ success: false, error: `Python 服务转发失败: ${e.message}` }));
  });
  req.pipe(proxyReq);
}

function _proxyStream(req, res, method, path) {
  const opts = {
    hostname: PYTHON_HOST,
    port: PYTHON_PORT,
    path: `/api/melody2score${path}`,
    method: method,
    timeout: 30000
  };
  const proxyReq = http.request(opts, (proxyRes) => {
    res.writeHead(proxyRes.statusCode, {
      ...proxyRes.headers,
      'Access-Control-Allow-Origin': '*'
    });
    proxyRes.pipe(res);
  });
  proxyReq.on('error', () => {
    res.writeHead(502, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
    res.end(JSON.stringify({ success: false, error: '文件下载失败' }));
  });
  proxyReq.end();
}

// 注册模块
registerModule('melody2score', routes, {
  version: '2.0.0',
  description: '企业级旋律转谱引擎集成模块'
});