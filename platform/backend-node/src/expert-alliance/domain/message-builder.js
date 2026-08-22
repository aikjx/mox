'use strict';

/**
 * 域层纯函数：咨询上下文消息构建（G2 尺寸治理 · 自 alliance-orchestrator 提取）
 * ------------------------------------------------------------------
 * 输入 expert/messages/options → 输出 LLM 网关消息数组。
 * 零 IO、无状态：背景上下文/业务约束注入 + 专家能力自描述。
 */

function buildContextMessages(expert, messages, options = {}) {
  const enhancedSystem = options.useCustomPrompt ? options.systemPrompt : expert.systemPrompt;

  const contextParts = [];
  if (options.problemContext) {
    contextParts.push(`## 背景上下文\n${options.problemContext}`);
  }
  if (options.businessConstraints) {
    contextParts.push(`## 业务约束\n${options.businessConstraints}`);
  }

  const enhancedMessages = [{
    role: 'system',
    content: contextParts.length > 0
      ? `${enhancedSystem}\n\n${contextParts.join('\n\n')}`
      : enhancedSystem
  }];

  if (options.includeExpertContext !== false) {
    enhancedMessages[0].content += `\n\n专家能力: ${expert.capabilities.join(', ')}`;
  }

  enhancedMessages.push(...messages);
  return enhancedMessages;
}

module.exports = { buildContextMessages };
