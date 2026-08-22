'use strict';

/**
 * HTTP 响应与请求体解析（跨域共享基础设施）
 * 供 api-server.js 组合根与全部路由域共用，经 ctx 注入。
 */

function send(res, status, payload, headers, opts) {
  opts = opts || {};
  const pretty = opts.pretty || res._pretty;
  const body = pretty ? JSON.stringify(payload, null, 2) : JSON.stringify(payload);
  res.writeHead(status, Object.assign({
    'Content-Type': 'application/json; charset=utf-8',
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET,POST,PUT,DELETE,OPTIONS,PATCH',
    'Access-Control-Allow-Headers': 'Content-Type,Authorization,Accept,X-Requested-With,Origin'
  }, headers || {}));
  res.end(body);
}

function ok(res, data, extra, opts) {
  send(res, 200, Object.assign({ success: true, data: data }, extra || {}), null, opts);
}

function fail(res, status, message, extra) {
  send(res, status, Object.assign({ success: false, error: message }, extra || {}));
}

function readBody(req) {
  return new Promise((resolve) => {
    let chunks = '';
    req.on('data', (c) => { chunks += c; });
    req.on('end', () => {
      if (!chunks) return resolve({});
      try { resolve(JSON.parse(chunks)); } catch (e) { resolve({}); }
    });
    req.on('error', () => resolve({}));
  });
}

module.exports = { send, ok, fail, readBody };
