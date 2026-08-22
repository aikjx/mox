'use strict';

/**
 * 路由域：MCP 协议服务（Model Context Protocol · Streamable HTTP transport）
 * POST /mcp        JSON-RPC 2.0（单请求或批量）
 * GET  /mcp/tools  便捷工具清单（非协议端点，人读调试用）
 *
 * 兑现系统宣称的 MCP 兼容能力：专家联盟七大工具可供
 * Claude Code / Cursor / 任意 MCP 客户端标准调用。
 */
module.exports = function registerMCPRoutes(ctx) {
  const { alliance, getAllianceEngine, reg } = ctx;
  const { MCPOrchestrator, TOOLS, SERVER_INFO, PROTOCOL_VERSION } = require('../mcp');

  const orchestrator = new MCPOrchestrator({ alliance, allianceEngine: getAllianceEngine() });

  // 原始请求体（保留字符串）：MCP 需区分 Parse error(-32700) 与 Invalid Request(-32600)，
  // 共用 readBody 会吞掉解析失败（返回 {}），故此处按协议边界自读原文。
  function readRawBody(req) {
    return new Promise((resolve) => {
      let chunks = '';
      req.on('data', (c) => { chunks += c; });
      req.on('end', () => resolve(chunks));
      req.on('error', () => resolve(''));
    });
  }

  // 原生 JSON-RPC 响应（不走 ok 包装：MCP 协议体必须是标准 JSON-RPC 2.0）
  const rpcResponse = (res, payload, status = 200) => {
    res.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8' });
    res.end(JSON.stringify(payload));
  };

  reg('post', '/mcp', async (req, res) => {
    const raw = await readRawBody(req);

    let parsed;
    try {
      parsed = JSON.parse(raw || '');
    } catch (_e) {
      return rpcResponse(res, { jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } });
    }

    // 批量请求（数组）
    if (Array.isArray(parsed)) {
      if (parsed.length === 0) {
        return rpcResponse(res, { jsonrpc: '2.0', id: null, error: { code: -32600, message: 'Invalid Request' } });
      }
      const responses = await orchestrator.handleBatch(parsed);
      // 全为通知时按规范返回 202 无 body
      if (responses.length === 0) {
        res.writeHead(202);
        return res.end();
      }
      return rpcResponse(res, responses);
    }

    // 单请求
    const response = await orchestrator.handleMessage(parsed);
    if (response === null) {
      res.writeHead(202);
      return res.end();
    }
    return rpcResponse(res, response);
  });

  // 便捷端点（非 MCP 协议）：工具清单速览
  reg('get', '/mcp/tools', (req, res) => {
    const { ok } = ctx;
    ok(res, { server: SERVER_INFO, protocol_version: PROTOCOL_VERSION, tools: TOOLS });
  });
};
