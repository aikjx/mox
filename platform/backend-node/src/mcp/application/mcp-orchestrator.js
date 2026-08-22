'use strict';

/**
 * MCP 协议服务 · JSON-RPC 2.0 编排（application 层）
 * ------------------------------------------------------------------
 * Model Context Protocol Streamable HTTP transport：
 *   initialize / notifications/initialized / tools/list / tools/call / ping
 * 依赖注入：alliance（专家联盟编排）、allianceEngine（六阶段流水线引擎）。
 */

const { PROTOCOL_VERSION, SERVER_INFO, TOOLS } = require('../domain/tool-definitions');

// JSON-RPC 2.0 标准错误码
const RPC_ERRORS = {
  PARSE_ERROR: { code: -32700, message: 'Parse error' },
  INVALID_REQUEST: { code: -32600, message: 'Invalid Request' },
  METHOD_NOT_FOUND: { code: -32601, message: 'Method not found' },
  INVALID_PARAMS: { code: -32602, message: 'Invalid params' },
  INTERNAL_ERROR: { code: -32603, message: 'Internal error' }
};

class MCPOrchestrator {
  constructor({ alliance, allianceEngine } = {}) {
    this.alliance = alliance;
    this.allianceEngine = allianceEngine;
  }

  /** 处理单个 JSON-RPC 请求/通知，返回响应对象或 null（notification 无响应） */
  async handleMessage(msg) {
    if (!msg || typeof msg !== 'object' || msg.jsonrpc !== '2.0' || typeof msg.method !== 'string') {
      return this._error(msg && msg.id, RPC_ERRORS.INVALID_REQUEST);
    }

    const { id, method, params } = msg;
    const isNotification = id === undefined || id === null;

    try {
      // --- MCP 生命周期 ---
      if (method === 'initialize') {
        return this._result(id, {
          protocolVersion: PROTOCOL_VERSION,
          capabilities: { tools: { listChanged: false } },
          serverInfo: SERVER_INFO
        });
      }
      if (method === 'notifications/initialized' || method.startsWith('notifications/')) {
        return null; // 通知：无响应体
      }
      if (method === 'ping') {
        return this._result(id, {});
      }

      // --- 工具协议 ---
      if (method === 'tools/list') {
        return this._result(id, { tools: TOOLS });
      }
      if (method === 'tools/call') {
        const result = await this._callTool(params || {});
        return this._result(id, result);
      }

      return this._error(id, RPC_ERRORS.METHOD_NOT_FOUND);
    } catch (e) {
      if (isNotification) return null;
      return this._error(id, {
        code: RPC_ERRORS.INTERNAL_ERROR.code,
        message: e.message || RPC_ERRORS.INTERNAL_ERROR.message
      });
    }
  }

  /** 批量消息处理（JSON-RPC 2.0 批量规范） */
  async handleBatch(msgs) {
    const responses = [];
    for (const msg of msgs) {
      const r = await this.handleMessage(msg);
      if (r) responses.push(r);
    }
    return responses;
  }

  // ===================== 工具执行（application 编排） =====================

  async _callTool({ name, arguments: args }) {
    const tool = TOOLS.find(t => t.name === name);
    if (!tool) {
      return { content: [{ type: 'text', text: `未知工具: ${name}` }], isError: true };
    }
    const input = args || {};
    const missing = (tool.inputSchema.required || []).filter(k => input[k] === undefined);
    if (missing.length > 0) {
      return { content: [{ type: 'text', text: `缺少必填参数: ${missing.join(', ')}` }], isError: true };
    }

    let text;
    switch (name) {
      case 'list_experts':
        text = this._listExperts();
        break;
      case 'classify_intent':
        text = this._classifyIntent(input.question);
        break;
      case 'compose_team':
        text = this._composeTeam(input.question, input.team_size);
        break;
      case 'consult_expert':
        text = await this._consultExpert(input.expert_id, input.question);
        break;
      case 'alliance_deliberate':
        text = await this._deliberate(input.question, input.expert_ids, input.rounds);
        break;
      case 'alliance_process':
        text = await this._process(input.question, input.disable_retry);
        break;
      case 'alliance_traces_stats':
        text = this._tracesStats();
        break;
      default:
        text = `工具 ${name} 未实现执行器`;
    }
    return { content: [{ type: 'text', text }] };
  }

  _listExperts() {
    const experts = this.alliance ? this.alliance.listExperts() : [];
    return JSON.stringify(experts.map(e => ({
      id: e.id, name: e.name, type: e.type, status: e.status, capabilities: e.capabilities
    })), null, 2);
  }

  _classifyIntent(question) {
    if (!this.allianceEngine) return '意图识别引擎不可用';
    const intent = this.allianceEngine.classifyIntent(question);
    return JSON.stringify(intent, null, 2);
  }

  _composeTeam(question, teamSize) {
    if (!this.allianceEngine) return '组队引擎不可用';
    const intent = this.allianceEngine.classifyIntent(question);
    const plan = this.allianceEngine.composeTeam(question, intent, { teamSize: teamSize || 3 });
    return JSON.stringify({
      intent: intent.primary,
      team_size: plan.team_size,
      total_synergy: plan.total_synergy,
      security_note: plan.security_note || null,
      team: plan.team.map(m => ({ id: m.id, name: m.name, type: m.type }))
    }, null, 2);
  }

  async _consultExpert(expertId, question) {
    if (!this.alliance) return '专家联盟不可用';
    const result = await this.alliance.consult(expertId, [{ role: 'user', content: question }], {});
    return JSON.stringify({
      expert: result.expert, response: result.response,
      model: result.metadata && result.metadata.model
    }, null, 2);
  }

  async _deliberate(question, expertIds, rounds) {
    if (!this.alliance) return '专家联盟不可用';
    const result = await this.alliance.debate(question, expertIds, { rounds: rounds || 2 });
    return JSON.stringify({
      requested: expertIds.length, total: result.total, successful: result.successful,
      skipped: result.skipped || [],
      history: result.history
    }, null, 2);
  }

  async _process(question, disableRetry) {
    if (!this.allianceEngine) return '联盟流水线引擎不可用';
    const result = await this.allianceEngine.process(question, { disableRetry: !!disableRetry });
    return JSON.stringify({
      trace_id: result.trace_id,
      intent: result.intent.primary,
      team: result.team.map(m => m.type),
      gate: { level: result.gate.level, passed: result.gate.passed },
      consensus: result.consensus.score,
      synthesis: result.synthesis.synthesis || result.synthesis.answer,
      insights: result.synthesis.insights,
      recommendations: result.synthesis.recommendations
    }, null, 2);
  }

  _tracesStats() {
    if (!this.allianceEngine) return '审计引擎不可用';
    return JSON.stringify(this.allianceEngine.traceStats(), null, 2);
  }

  // ===================== JSON-RPC 响应构造 =====================

  _result(id, result) {
    return { jsonrpc: '2.0', id: id === undefined ? null : id, result };
  }

  _error(id, err) {
    return { jsonrpc: '2.0', id: id === undefined ? null : id, error: err };
  }
}

module.exports = { MCPOrchestrator };
