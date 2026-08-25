'use strict';

/**
 * MCP 协议服务 · 工具定义（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * Model Context Protocol 标准工具 schema：name / description / inputSchema（JSON Schema）。
 * 暴露专家联盟核心能力，供 Claude Code / Cursor / 任意 MCP 客户端调用。
 */

const PROTOCOL_VERSION = '2025-06-18';
const SERVER_INFO = { name: 'mox-expert-alliance', version: '1.0.0' };

const TOOLS = [
  {
    name: 'list_experts',
    description: '列出专家联盟全部可用专家（ID、姓名、类型、状态、能力清单）',
    inputSchema: { type: 'object', properties: {}, additionalProperties: false }
  },
  {
    name: 'classify_intent',
    description: '对问题做多标签意图识别（返回主意图、置信度、全部命中标签）',
    inputSchema: {
      type: 'object',
      properties: {
        question: { type: 'string', description: '待识别的问题文本' }
      },
      required: ['question'], additionalProperties: false
    }
  },
  {
    name: 'compose_team',
    description: '按问题意图组建最优专家团队（能力匹配 + 协同增益 + 负载均衡多目标选队）',
    inputSchema: {
      type: 'object',
      properties: {
        question: { type: 'string', description: '业务问题' },
        team_size: { type: 'integer', description: '团队规模（1-4，默认 3）', minimum: 1, maximum: 4 }
      },
      required: ['question'], additionalProperties: false
    }
  },
  {
    name: 'consult_expert',
    description: '向指定专家发起咨询（真实 LLM 单专家直答）',
    inputSchema: {
      type: 'object',
      properties: {
        expert_id: { type: 'string', description: '专家 ID（见 list_experts）' },
        question: { type: 'string', description: '咨询问题' }
      },
      required: ['expert_id', 'question'], additionalProperties: false
    }
  },
  {
    name: 'alliance_deliberate',
    description: '多专家辩论：指定专家团队进行多轮辩论收敛（真实 LLM，返回轮次历史）',
    inputSchema: {
      type: 'object',
      properties: {
        question: { type: 'string', description: '辩论议题' },
        expert_ids: { type: 'array', items: { type: 'string' }, description: '参辩专家 ID 列表（2-4 位）' },
        rounds: { type: 'integer', description: '辩论轮数（默认 2）', minimum: 1, maximum: 4 }
      },
      required: ['question', 'expert_ids'], additionalProperties: false
    }
  },
  {
    name: 'alliance_process',
    description: '专家联盟六阶段全流水线：意图识别→最优组队→并行咨询与辩论→综合→质量门禁→技能沉淀（真实 LLM，含 trace 审计）',
    inputSchema: {
      type: 'object',
      properties: {
        question: { type: 'string', description: '业务问题（安全类问题将强制选入安全专家）' },
        disable_retry: { type: 'boolean', description: '禁用质量门禁 C 级重试（默认 false）' }
      },
      required: ['question'], additionalProperties: false
    }
  },
  {
    name: 'alliance_traces_stats',
    description: '查询联盟处理审计统计（窗口内总数、成功率、P95 延迟、门禁分布、意图分布）',
    inputSchema: { type: 'object', properties: {}, additionalProperties: false }
  }
];

module.exports = { PROTOCOL_VERSION, SERVER_INFO, TOOLS };
