//! # OpenAPI 3.1 标准接口契约
//!
//! 算子统一系统对外 HTTP 接口的**机器可读标准契约**（OpenAPI 3.1）。
//! 任何 OpenAPI 工具链（Swagger UI / Redoc / openapi-generator / Postman /
//! 各语言 SDK 生成器）均可直接消费，实现「最规范标准接口对接」。
//!
//! 暴露端点：
//! - `GET /api/openapi.yaml` — OpenAPI 3.1 规范文档（YAML）
//! - `GET /api/docs`         — Swagger UI 交互式文档（CDN 加载，指向上述 YAML）

use axum::http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};

const OPENAPI_YAML: &str = r#"openapi: 3.1.0
info:
  title: 算子统一系统运行时 API
  version: 1.0.0
  description: |
    算子统一系统（operator-unified-system）v3.0 AI 驱动全维突破平台的对外 HTTP API。
    所有错误响应统一为 RFC 9457 application/problem+json 格式。
  license:
    name: MIT
servers:
  - url: http://localhost:3000
    description: 本地开发服务
tags:
  - name: system
    description: 健康检查 / 系统状态 / 插件 / 日志
  - name: operators
    description: 算子注册 / 执行
  - name: graph
    description: 知识图谱查询 / 推荐 / 社区发现
  - name: ai
    description: AI 对话 / 算法分析 / 资源 / 插件 / 工作流 / 浏览器 / 流程图
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: OUS_API_TOKEN
  schemas:
    Problem:
      type: object
      description: RFC 9457 Problem+JSON 统一错误体
      properties:
        type:
          type: string
          default: about:blank
        title:
          type: string
        status:
          type: integer
        detail:
          type: string
        instance:
          type: string
        code:
          type: string
          description: 业务错误码（扩展字段）
      required: [title, detail]
    OperatorInfo:
      type: object
      properties:
        id: { type: string }
        name: { type: string }
        description: { type: string }
        category: { type: string }
        input_type: { type: string }
        output_type: { type: string }
    ExecuteRequest:
      type: object
      required: [workflow, input]
      properties:
        workflow: { type: array, items: { type: string } }
        input: { type: array, items: { type: number } }
        parameters: { type: object, additionalProperties: { type: number } }
    ExecuteResponse:
      type: object
      properties:
        success: { type: boolean }
        output: { type: array, items: { type: number }, nullable: true }
        execution_time_ms: { type: integer }
        logs: { type: array, items: { type: string } }
        error: { type: string, nullable: true }
        metrics: { type: object, nullable: true }
    ChatRequest:
      type: object
      required: [message]
      properties:
        session_id: { type: string, nullable: true }
        message: { type: string }
    ChatResponse:
      type: object
      properties:
        session_id: { type: string }
        response: { type: string }
    FlowDefinition:
      type: object
      description: 流程图定义（FlowGraph IR）
    StatusSummary:
      type: object
      description: 系统状态聚合指标
  responses:
    Problem:
      description: RFC 9457 标准错误
      content:
        application/problem+json:
          schema:
            $ref: '#/components/schemas/Problem'
    BadRequest: { description: 请求参数错误, $ref: '#/components/responses/Problem' }
    Unauthorized: { description: 未授权, $ref: '#/components/responses/Problem' }
    Forbidden: { description: 禁止访问, $ref: '#/components/responses/Problem' }
    NotFound: { description: 资源不存在, $ref: '#/components/responses/Problem' }
    Internal: { description: 内部错误, $ref: '#/components/responses/Problem' }
security:
  - bearerAuth: []
paths:
  /api/health:
    get:
      tags: [system]
      summary: 健康检查
      security: []
      responses:
        '200':
          description: 服务存活
          content:
            application/json:
              schema:
                type: object
                properties:
                  status: { type: string }
  /api/status:
    get:
      tags: [system]
      summary: 系统状态概览
      responses:
        '200':
          description: 状态指标
          content:
            application/json:
              schema: { $ref: '#/components/schemas/StatusSummary' }
        '500': { $ref: '#/components/responses/Internal' }
  /api/status/full:
    get:
      tags: [system]
      summary: 全量系统状态（含资源 / 插件 / 审计 / 流程模板）
      responses:
        '200': { description: 全量状态 }
        '500': { $ref: '#/components/responses/Internal' }
  /api/plugins:
    get:
      tags: [system]
      summary: 已注册 WASM 插件列表
      responses:
        '200': { description: 插件列表 }
  /api/logs:
    get:
      tags: [system]
      summary: 执行日志
      responses:
        '200': { description: 日志列表 }
  /api/operators:
    get:
      tags: [operators]
      summary: 算子列表
      responses:
        '200':
          description: 算子清单
          content:
            application/json:
              schema:
                type: array
                items: { $ref: '#/components/schemas/OperatorInfo' }
  /api/operators/register:
    post:
      tags: [operators]
      summary: 注册自定义算子
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/OperatorInfo' }
      responses:
        '200': { description: 注册成功 }
        '400': { $ref: '#/components/responses/BadRequest' }
  /api/execute:
    post:
      tags: [operators]
      summary: 执行算子工作流链
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/ExecuteRequest' }
      responses:
        '200':
          description: 执行结果
          content:
            application/json:
              schema: { $ref: '#/components/schemas/ExecuteResponse' }
        '400': { $ref: '#/components/responses/BadRequest' }
        '500': { $ref: '#/components/responses/Internal' }
  /api/graph:
    get:
      tags: [graph]
      summary: 知识图谱（节点 + 边 + 指标）
      responses:
        '200': { description: 图谱数据 }
  /api/graph/stats:
    get: { tags: [graph], summary: 图谱统计指标, responses: { '200': { description: OK } } }
  /api/graph/centrality:
    get: { tags: [graph], summary: 中心性指标, responses: { '200': { description: OK } } }
  /api/graph/communities:
    get: { tags: [graph], summary: 社区发现, responses: { '200': { description: OK } } }
  /api/graph/pagerank:
    get: { tags: [graph], summary: PageRank, responses: { '200': { description: OK } } }
  /api/graph/neighbors/{id}:
    get:
      tags: [graph]
      summary: 节点邻居
      parameters: [{ name: id, in: path, required: true, schema: { type: string } }]
      responses:
        '200': { description: OK }
        '404': { $ref: '#/components/responses/NotFound' }
  /api/graph/path:
    get: { tags: [graph], summary: 最短路径, responses: { '200': { description: OK } } }
  /api/graph/recommend:
    post: { tags: [graph], summary: 节点推荐, responses: { '200': { description: OK } } }
  /api/ai/chat:
    post:
      tags: [ai]
      summary: AI 智能对话
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: '#/components/schemas/ChatRequest' }
      responses:
        '200':
          description: 对话响应
          content:
            application/json:
              schema: { $ref: '#/components/schemas/ChatResponse' }
        '400': { $ref: '#/components/responses/BadRequest' }
  /api/ai/chat/history/{session}:
    get:
      tags: [ai]
      summary: 对话历史
      parameters: [{ name: session, in: path, required: true, schema: { type: string } }]
      responses:
        '200': { description: OK }
        '404': { $ref: '#/components/responses/NotFound' }
  /api/ai/analyze-algorithm:
    post: { tags: [ai], summary: 算法分析归一化, responses: { '200': { description: OK }, '400': { $ref: '#/components/responses/BadRequest' } } }
  /api/ai/algorithm-types:
    get: { tags: [ai], summary: 算法类型列表, responses: { '200': { description: OK } } }
  /api/ai/resources:
    get: { tags: [ai], summary: 资源全景, responses: { '200': { description: OK } } }
  /api/ai/resources/health:
    get: { tags: [ai], summary: 资源健康, responses: { '200': { description: OK } } }
  /api/ai/plugins:
    get: { tags: [ai], summary: AI 插件列表, responses: { '200': { description: OK } } }
  /api/ai/plugins/register:
    post: { tags: [ai], summary: 注册 AI 插件, responses: { '200': { description: OK }, '400': { $ref: '#/components/responses/BadRequest' } } }
  /api/ai/plugins/send-message:
    post: { tags: [ai], summary: 插件消息路由, responses: { '200': { description: OK } } }
  /api/ai/workflows/templates:
    get: { tags: [ai], summary: 工作流模板, responses: { '200': { description: OK } } }
  /api/ai/workflows:
    get: { tags: [ai], summary: 工作流列表, responses: { '200': { description: OK } } }
    post: { tags: [ai], summary: 保存工作流, responses: { '200': { description: OK } } }
  /api/ai/workflows/execute:
    post: { tags: [ai], summary: 执行业务工作流, responses: { '200': { description: OK }, '400': { $ref: '#/components/responses/BadRequest' } } }
  /api/ai/workflows/save:
    post: { tags: [ai], summary: 保存工作流定义, responses: { '200': { description: OK } } }
  /api/ai/workflows/instances:
    get: { tags: [ai], summary: 工作流实例, responses: { '200': { description: OK } } }
  /api/ai/llm/config:
    get: { tags: [ai], summary: LLM 配置读取, responses: { '200': { description: OK } } }
    post: { tags: [ai], summary: LLM 配置更新, responses: { 'sq': { description: OK } } }
  /api/ai/llm/test:
    post: { tags: [ai], summary: LLM 连接测试, responses: { '200': { description: OK } } }
  /api/ai/browser/templates:
    get: { tags: [ai], summary: 浏览器任务模板, responses: { '200': { description: OK } } }
  /api/ai/browser/sessions:
    get: { tags: [ai], summary: 浏览器会话列表, responses: { '200': { description: OK } } }
  /api/ai/browser/sessions/{id}:
    get: { tags: [ai], summary: 浏览器会话详情, responses: { '200': { description: OK }, '404': { $ref: '#/components/responses/NotFound' } } }
    delete: { tags: [ai], summary: 关闭浏览器会话, responses: { '200': { description: OK } } }
  /api/ai/browser/execute-task:
    post: { tags: [ai], summary: 执行浏览器任务, responses: { '200': { description: OK } } }
  /api/ai/browser/execute-steps:
    post: { tags: [ai], summary: 执行浏览器步骤, responses: { '200': { description: OK } } }
  /api/ai/browser/execute-action:
    post: { tags: [ai], summary: 执行浏览器动作, responses: { '200': { description: OK } } }
  /api/ai/browser/natural:
    post: { tags: [ai], summary: 自然语言浏览器指令, responses: { '200': { description: OK } } }
  /api/ai/flows:
    get: { tags: [ai], summary: 流程图列表, responses: { '200': { description: OK } } }
    post: { tags: [ai], summary: 创建流程图, responses: { '200': { description: OK }, '400': { $ref: '#/components/responses/BadRequest' } } }
  /api/ai/flows/{id}:
    get: { tags: [ai], summary: 流程图详情, responses: { '200': { description: OK }, '404': { $ref: '#/components/responses/NotFound' } } }
    delete: { tags: [ai], summary: 删除流程图, responses: { '200': { description: OK } } }
  /api/ai/flows/validate:
    post: { tags: [ai], summary: 流程图静态校验, responses: { '200': { description: OK } } }
  /api/ai/flows/execute:
    post: { tags: [ai], summary: 执行流程图, responses: { '200': { description: OK } } }
  /api/ai/flows/node-types:
    get: { tags: [ai], summary: 流程节点类型, responses: { '200': { description: OK } } }
  /api/analyze/spiral:
    post: { tags: [ai], summary: 空间光速螺旋模型分析, responses: { '200': { description: OK } } }
"#;

/// 返回 OpenAPI 3.1 规范（YAML）。
pub async fn serve_openapi_yaml() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/yaml"),
    );
    (StatusCode::OK, headers, OPENAPI_YAML).into_response()
}

/// 返回 Swagger UI 交互式文档页面（CDN 加载，指向 `/api/openapi.yaml`）。
pub async fn serve_swagger_ui() -> Html<String> {
    Html(
        r##"<!DOCTYPE html>
<html lang="zh">
<head>
  <meta charset="UTF-8" />
  <title>算子统一系统 API · Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: "/api/openapi.yaml",
        dom_id: "#swagger-ui",
        deepLinking: true,
        presets: [SwaggerUIBundle.presets.apis],
        layout: "BaseLayout",
      });
    };
  </script>
</body>
</html>"##
            .to_string(),
    )
}
