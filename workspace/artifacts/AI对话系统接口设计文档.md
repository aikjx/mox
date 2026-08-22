# AI 对话系统接口设计文档

**文档版本**：v1.0  
**编写日期**：2026年8月22日  
**文档状态**：正式发布  
**密级**：内部公开  

---

## 修订记录

| 版本 | 日期 | 修订人 | 修订说明 |
|------|------|--------|----------|
| v0.1 | 2026-08-15 | 系统架构组 | 初稿创建 |
| v0.2 | 2026-08-18 | 后端开发组 | 补充认证流程细节 |
| v0.3 | 2026-08-20 | 前端开发组 | 完善消息与会话接口 |
| v1.0 | 2026-08-22 | 项目管理办 | 评审通过，正式发布 |

---

## 目录

1. [引言](#1-引言)
2. [总体说明](#2-总体说明)
3. [认证接口设计](#3-认证接口设计)
4. [消息接口设计](#4-消息接口设计)
5. [会话管理接口设计](#5-会话管理接口设计)
6. [错误码与异常处理](#6-错误码与异常处理)
7. [安全与限流策略](#7-安全与限流策略)
8. [附录](#8-附录)

---

## 1. 引言

### 1.1 编写目的

本文档旨在定义 AI 对话系统的 RESTful API 接口规范，明确**认证（Authentication）**、**消息（Message）**、**会话管理（Conversation）** 三大核心模块的接口协议、数据格式及交互流程，为前端、后端、测试及第三方集成方提供统一的开发依据。

### 1.2 适用范围

本文档适用于所有接入 AI 对话系统的客户端（Web、移动端、桌面端）及服务端开发者。

### 1.3 术语与缩写

| 术语 | 说明 |
|------|------|
| API | 应用程序编程接口（Application Programming Interface） |
| JWT | JSON Web Token，用于身份认证的开放标准 |
| SSE | 服务器发送事件（Server-Sent Events），用于流式响应 |
| Token | 访问令牌，用于接口鉴权 |
| Refresh Token | 刷新令牌，用于获取新的访问令牌 |
| Conversation | 会话，即一组连续的消息上下文 |
| Message | 消息，会话中的单条对话记录 |

---

## 2. 总体说明

### 2.1 接口风格

- 采用 **RESTful** 架构风格，使用标准 HTTP 方法（GET、POST、PUT、DELETE、PATCH）。
- 所有请求与响应均使用 **JSON** 格式（`Content-Type: application/json; charset=utf-8`）。
- 流式消息使用 **SSE（text/event-stream）** 协议。

### 2.2 基础域名

| 环境 | 域名 |
|------|------|
| 开发环境 | `https://dev-api.aichat.example.com` |
| 测试环境 | `https://test-api.aichat.example.com` |
| 生产环境 | `https://api.aichat.example.com` |

### 2.3 通用请求头

| Header | 必填 | 说明 |
|--------|------|------|
| `Authorization` | 是（除认证接口） | `Bearer {access_token}` |
| `Content-Type` | 是 | `application/json` |
| `X-Request-ID` | 否 | 请求唯一标识（UUID），用于链路追踪 |
| `Accept-Language` | 否 | 语言偏好，如 `zh-CN`、`en-US` |

### 2.4 通用响应结构

所有非流式接口返回统一包装结构：

```json
{
  "code": 0,
  "message": "success",
  "data": {},
  "request_id": "a3f2c1e8-7b4d-4f6a-9e2c-1d5b8a0f3c7e",
  "timestamp": 1787382551091
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `code` | int | 业务状态码，`0` 表示成功，非 `0` 表示失败 |
| `message` | string | 状态描述信息 |
| `data` | object | 业务数据，可为 `null` |
| `request_id` | string | 请求追踪 ID |
| `timestamp` | long | 服务器时间戳（毫秒） |

### 2.5 分页说明

分页参数统一为 `page`（页码，从 1 开始）和 `page_size`（每页条数，默认 20，最大 100）。分页响应结构：

```json
{
  "list": [],
  "total": 100,
  "page": 1,
  "page_size": 20,
  "has_more": true
}
```

---

## 3. 认证接口设计

### 3.1 认证方式概述

系统采用 **JWT（JSON Web Token）** 双令牌机制：

- **Access Token**：短期有效（默认 2 小时），用于业务接口鉴权。
- **Refresh Token**：长期有效（默认 30 天），用于刷新 Access Token。

### 3.2 接口列表

| 序号 | 方法 | 路径 | 说明 |
|------|------|------|------|
| 1 | POST | `/api/v1/auth/register` | 用户注册 |
| 2 | POST | `/api/v1/auth/login` | 用户登录 |
| 3 | POST | `/api/v1/auth/logout` | 用户登出 |
| 4 | POST | `/api/v1/auth/refresh` | 刷新访问令牌 |
| 5 | GET | `/api/v1/auth/profile` | 获取当前用户信息 |
| 6 | PUT | `/api/v1/auth/password` | 修改密码 |
| 7 | POST | `/api/v1/auth/verify-email` | 发送邮箱验证邮件 |

---

### 3.3 用户注册

**POST** `/api/v1/auth/register`

#### 请求参数

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `username` | string | 是 | 用户名，3-32 位字母、数字或下划线 |
| `email` | string | 是 | 邮箱地址，需符合邮箱格式 |
| `password` | string | 是 | 密码，8-64 位，需包含大小写字母和数字 |
| `nickname` | string | 否 | 昵称，默认与用户名相同 |
| `captcha_id` | string | 是 | 图形验证码 ID |
| `captcha_code` | string | 是 | 图形验证码内容 |

#### 请求示例

```json
{
  "username": "alice_wang",
  "email": "alice@example.com",
  "password": "Passw0rd!2026",
  "nickname": "Alice",
  "captcha_id": "c4f2a1b8-9e3d-4f6a-8b2c-1d5e7a0f3c9e",
  "captcha_code": "8Kd2"
}
```

#### 响应参数（data 字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| `user_id` | string | 用户唯一 ID |
| `username` | string | 用户名 |
| `email` | string | 邮箱 |
| `nickname` | string | 昵称 |
| `created_at` | string | 注册时间（ISO 8601） |

#### 响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "user_id": "usr_8f3a2c1e",
    "username": "alice_wang",
    "email": "alice@example.com",
    "nickname": "Alice",
    "created_at": "2026-08-22T10:30:00+08:00"
  },
  "request_id": "b7d2e4f6-8a1c-4e3d-9f5a-2c6b8d0e1f3a",
  "timestamp": 1787382551091
}
```

#### 异常场景

| 场景 | HTTP 状态码 | code | message |
|------|------------|------|---------|
| 用户名已存在 | 409 | 10001 | 用户名已被注册 |
| 邮箱已注册 | 409 | 10002 | 邮箱已被注册 |
| 验证码错误 | 400 | 10003 | 验证码错误或已过期 |

---

### 3.4 用户登录

**POST** `/api/v1/auth/login`

#### 请求参数

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `account` | string | 是 | 用户名或邮箱 |
| `password` | string | 是 | 登录密码 |
| `captcha_id` | string | 否 | 验证码 ID（连续失败 3 次后必填） |
| `captcha_code` | string | 否 | 验证码内容 |

#### 请求示例

```json
{
  "account": "alice@example.com",
  "password": "Passw0rd!2026"
}
```

#### 响应参数（data 字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | string | 访问令牌（JWT） |
| `refresh_token` | string | 刷新令牌 |
| `expires_in` | int | Access Token 有效期（秒），默认 7200 |
| `token_type` | string | 令牌类型，固定为 `Bearer` |
| `user_info` | object | 用户基本信息 |

`user_info` 结构：

| 字段 | 类型 | 说明 |
|------|------|------|
| `user_id` | string | 用户 ID |
| `username` | string | 用户名 |
| `nickname` | string | 昵称 |
| `avatar_url` | string | 头像地址 |
| `role` | string | 角色（`user`/`admin`/`vip`） |

#### 响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 7200,
    "token_type": "Bearer",
    "user_info": {
      "user_id": "usr_8f3a2c1e",
      "username": "alice_wang",
      "nickname": "Alice",
      "avatar_url": "https://cdn.aichat.example.com/avatars/usr_8f3a2c1e.png",
      "role": "user"
    }
  },
  "request_id": "c8e3f5a7-9b2d-4f1e-8a4c-3d6e9f0a2b5c",
  "timestamp": 1787382551091
}
```

#### 异常场景

| 场景 | HTTP 状态码 | code | message |
|------|------------|------|---------|
| 账号不存在 | 404 | 10004 | 账号不存在 |
| 密码错误 | 401 | 10005 | 密码错误 |
| 账号已锁定 | 403 | 10006 | 账号已锁定，请 30 分钟后再试 |
| 需要验证码 | 400 | 10007 | 需要提供验证码 |

---

### 3.5 用户登出

**POST** `/api/v1/auth/logout`

#### 请求头

`Authorization: Bearer {access_token}`

#### 请求参数

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `refresh_token` | string | 是 | 需要吊销的刷新令牌 |

#### 请求示例

```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
}
```

#### 响应示例

```json
{
  "code": 0,
  "message": "success",
  "data": null,
  "request_id": "d9f4a6b8-0c3e-4f2a-9b5d-4e7f8a1c3d6e",
  "timestamp": 1787382551091
}
```

#### 说明

- 登出后，当前 Access Token 立即失效，Refresh Token 被吊销。
- 客户端需清除本地存储的令牌。

---

### 3.6 刷新访问令牌

**POST** `/api/v1/auth/refresh`

#### 请求参数

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `refresh_token` | string | 是 | 刷新令牌 |

#### 请求示例

```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6