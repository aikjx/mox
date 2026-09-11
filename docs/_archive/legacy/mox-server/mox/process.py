# -*- coding: utf-8 -*-
"""
mox-process 业务流程引擎
=========================
定义"无限发布系统"从需求到发布运行的全链路业务处理流程，每个节点明确
输入 / 处理逻辑 / 输出 / 责任组件 / 判定条件，供管理中心中台流程可视化与在线处理。

流程设计：需求 → 应用创建 → 数据源配置 → SQL 定义 → 页面装配 → 测试验收 → 发布上线 → 运行监控 → 下线归档
"""
from __future__ import annotations

import time
from typing import Any, Optional

# 应用状态机：draft(草稿) → prepared(就绪) → published(已发布) → running(运行中) → offline(已下线)
APP_STATES = ["draft", "prepared", "published", "running", "offline"]
APP_TRANSITIONS = {
    "draft": ["prepared"],
    "prepared": ["published", "draft"],
    "published": ["running", "prepared"],
    "running": ["offline", "prepared"],
    "offline": ["draft"],
}


def next_states(status: str) -> list[str]:
    return APP_TRANSITIONS.get(status, [])


def can_transition(current: str, target: str) -> bool:
    return target in APP_TRANSITIONS.get(current, [])


PROCESS_FLOW = [
    {
        "step": 0, "phase": "需求", "code": "req_analyze",
        "name": "需求分析与业务建模", "component": "AI 助手 + 配置台",
        "input": "业务方需求（自然语言 / 需求清单）",
        "process": "AI 助手解析需求 → 识别业务实体（产品/新闻/案例等）→ 生成数据模型与 SQL 骨架",
        "output": "需求基线 + 数据模型 + SQL 定义骨架",
        "check": "实体识别完成、SQL 骨架通过语法校验",
    },
    {
        "step": 1, "phase": "应用创建", "code": "app_create",
        "name": "应用创建与配置", "component": "管理中心中台（应用管理）",
        "input": "应用名 / 类型 / 域名 / 归属方",
        "process": "创建 app（app_key 全局唯一）→ 绑定数据源与模板 → 进入 prepared 状态",
        "output": "应用记录（status=draft→prepared）",
        "check": "app_key 唯一、必填字段完整",
    },
    {
        "step": 2, "phase": "数据源", "code": "ds_configure",
        "name": "数据源配置（中间层）", "component": "mox-db 适配器",
        "input": "driver + 连接配置（sqlite/mysql/pg/duckdb）",
        "process": "注册数据源 → 适配器连通性探测 → 加入中间层注册表",
        "output": "可用数据源（enabled=1）",
        "check": "连接成功、可执行只读查询",
    },
    {
        "step": 3, "phase": "SQL 定义", "code": "sql_define",
        "name": "业务 SQL 动态定义", "component": "mox-dsql-core",
        "input": "SQL 模板（{{param}}/{% if %}）+ 数据源 + 缓存 TTL",
        "process": "模板语法校验 → 只读白名单校验 → 落库（version++）→ status=published",
        "output": "已发布的 SQL 定义（code 全局唯一）",
        "check": "语法/安全校验通过；字段级权限可配置",
    },
    {
        "step": 4, "phase": "页面装配", "code": "page_assembly",
        "name": "页面与组件装配", "component": "低代码页面引擎",
        "input": "页面模板 + SQL code 映射",
        "process": "页面节点绑定 SQL code → 表单/列表/详情/搜索组件化装配",
        "output": "可预览页面",
        "check": "所有页面节点均有可用 SQL 绑定",
    },
    {
        "step": 5, "phase": "测试验收", "code": "test_accept",
        "name": "测试与验收", "component": "DSQL 试运行 + 浏览器验证",
        "input": "渲染 SQL + 测试参数",
        "process": "explain 渲染 → 试运行 → 校验结果行/权限/缓存命中 → 浏览器验证",
        "output": "测试报告（trace_id 可追踪）",
        "check": "接口全通、无 JS 错误、字段权限符合预期",
    },
    {
        "step": 6, "phase": "发布上线", "code": "publish",
        "name": "发布上线", "component": "发布中心",
        "input": "验收通过的应用 + SQL 集合",
        "process": "记录 publish_logs → publish_version++ → status=running → 开放访问入口",
        "output": "上线应用（含版本号与访问链接）",
        "check": "发布记录落库、访问入口可访问",
    },
    {
        "step": 7, "phase": "运行监控", "code": "run_monitor",
        "name": "运行监控与在线处理", "component": "缓存 + 审计 + AI 诊断",
        "input": "运行期请求（trace_id）",
        "process": "DSQL 执行（缓存优先）→ 审计落库 → 性能/命中率监控 → 异常 AI 诊断",
        "output": "监控指标 + 审计日志",
        "check": "缓存命中率、慢查询、错误率可视",
    },
    {
        "step": 8, "phase": "下线归档", "code": "offline_archive",
        "name": "下线与归档", "component": "发布中心",
        "input": "下线指令",
        "process": "记录下线日志 → status=offline → 关闭访问入口 → 数据归档",
        "output": "归档应用（可重新发布）",
        "check": "访问入口关闭、数据保留",
    },
]

STAGES = ["需求", "应用创建", "数据源", "SQL 定义", "页面装配", "测试验收", "发布上线", "运行监控", "下线归档"]


def get_process_flow() -> dict:
    return {
        "stages": STAGES,
        "app_states": APP_STATES,
        "transitions": APP_TRANSITIONS,
        "flow": PROCESS_FLOW,
        "total_steps": len(PROCESS_FLOW),
    }
