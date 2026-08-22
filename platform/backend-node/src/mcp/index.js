'use strict';

/**
 * MCP 协议服务 · 域门面（组合根装配点）
 * ------------------------------------------------------------------
 * 对外唯一导出：工具定义 + JSON-RPC 编排器。
 * 依赖注入约定：{ alliance, allianceEngine } 由 routes/mcp.js 从 ctx 提供。
 */

const { PROTOCOL_VERSION, SERVER_INFO, TOOLS } = require('./domain/tool-definitions');
const { MCPOrchestrator } = require('./application/mcp-orchestrator');

module.exports = { PROTOCOL_VERSION, SERVER_INFO, TOOLS, MCPOrchestrator };
