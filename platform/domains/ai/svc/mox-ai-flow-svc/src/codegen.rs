//! 流程图 ⇄ 代码 双向映射
//!
//! 正向：优化后的 FlowGraph → 分层 Python 工程（调度层 / 业务层 / 工具层 / 异常层）
//! 逆向：现有 RPA Python 代码 → 反解析出 FlowGraph（补全缺失的异常分支）
//!
//! 生成的代码按并行层组织，直接体现调度决策：同层用 ThreadPoolExecutor 并发下发。

use crate::conflict::ConflictReport;
use crate::dataflow::ParallelPlan;
use crate::model::{EdgeKind, FlowEdge, FlowGraph, FlowNode, NodeKind, ToolKind};
use crate::schedule::Schedule;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// 生成的工程文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// 代码生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBundle {
    pub files: Vec<GeneratedFile>,
    /// 是否因阻断级冲突而拒绝生成
    pub rejected: bool,
    pub reject_reasons: Vec<String>,
}

impl CodeBundle {
    pub fn file(&self, path: &str) -> Option<&GeneratedFile> {
        self.files.iter().find(|f| f.path == path)
    }
    pub fn total_lines(&self) -> usize {
        self.files.iter().map(|f| f.content.lines().count()).sum()
    }
}

pub fn py_ident(id: &str) -> String {
    let mut s: String = id
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    // 内部注入的节点（如 __guard_x / __error_handler）以双下划线开头，
    // 而 Python 在类体内会对 __name 做名称修饰(name mangling)，导致引用失败。
    // 统一改写为 op_ 前缀，保证生成的标识符在任何上下文都可安全引用。
    let trimmed = s.trim_start_matches('_');
    if s.starts_with("__") && !trimmed.is_empty() {
        s = format!("op_{}", trimmed);
    }
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true) {
        s.insert(0, '_');
    }
    s.to_lowercase()
}

fn tool_module(t: ToolKind) -> &'static str {
    match t {
        ToolKind::Compute => "compute",
        ToolKind::Llm => "llm",
        ToolKind::File => "file_io",
        ToolKind::Browser => "browser",
        ToolKind::Database => "database",
        ToolKind::Http => "http",
        ToolKind::Shell => "shell",
        ToolKind::Human => "human",
    }
}

/// 全栈生成别名（供 mox_platform_orchestrator_svc / 草莓多平台调用）：等价于 `generate`
///
/// 返回的 `CodeBundle` 现包含后端骨架（tools/tasks/errors/scheduler/main）+
/// 数据库 DDL（`schema.sql`）+ 前端 Vue 骨架（`App.vue`），即对一张流程图
/// 一次性产出「后端 + 数据库 + 前端」三部分代码。
pub fn generate_full_stack(
    graph: &FlowGraph,
    plan: &ParallelPlan,
    schedule: &Schedule,
    conflicts: &ConflictReport,
) -> CodeBundle {
    generate(graph, plan, schedule, conflicts)
}

/// 正向生成：流程图 → 分层 Python 工程
pub fn generate(
    graph: &FlowGraph,
    plan: &ParallelPlan,
    schedule: &Schedule,
    conflicts: &ConflictReport,
) -> CodeBundle {
    let blocking = conflicts.blocking();
    if !blocking.is_empty() {
        return CodeBundle {
            files: Vec::new(),
            rejected: true,
            reject_reasons: blocking.iter().map(|c| c.message.clone()).collect(),
        };
    }

    let files = vec![
        GeneratedFile {
            path: "generated/__init__.py".into(),
            content: String::new(),
        },
        GeneratedFile {
            path: "generated/tools.py".into(),
            content: gen_tools(graph),
        },
        GeneratedFile {
            path: "generated/tasks.py".into(),
            content: gen_tasks(graph),
        },
        GeneratedFile {
            path: "generated/errors.py".into(),
            content: gen_errors(graph),
        },
        GeneratedFile {
            path: "generated/scheduler.py".into(),
            content: gen_scheduler(graph, plan, schedule),
        },
        GeneratedFile {
            path: "generated/main.py".into(),
            content: gen_main(graph),
        },
        // ↓↓↓ 草莓多平台：全栈生成器扩展 ↓↓↓
        GeneratedFile {
            path: "generated/schema.sql".into(),
            content: gen_db_schema(graph),
        },
        GeneratedFile {
            path: "generated/App.vue".into(),
            content: gen_frontend(graph),
        },
    ];

    CodeBundle {
        files,
        rejected: false,
        reject_reasons: Vec::new(),
    }
}

/// 工具层：每类工具一个受控封装，统一超时/重试/资源池
fn gen_tools(graph: &FlowGraph) -> String {
    let used: BTreeSet<ToolKind> = graph.nodes.iter().filter_map(|n| n.tool).collect();
    let mut s = String::new();
    let _ = writeln!(s, "\"\"\"工具层 —— 由流程图自动生成，勿手改。\n\n每个工具封装资源池信号量，与流程图的 pools 配置一一对应。\n\"\"\"");
    let _ = writeln!(
        s,
        "import threading\nimport time\nfrom contextlib import contextmanager\n"
    );
    let _ = writeln!(s, "_POOLS = {{");
    for t in &used {
        let pool = t.resource_pool();
        let _ = writeln!(
            s,
            "    \"{}\": threading.Semaphore({}),",
            pool,
            graph.capacity_of(pool)
        );
    }
    let _ = writeln!(s, "}}\n");
    let _ = writeln!(
        s,
        "@contextmanager\ndef acquire(pool: str):\n    sem = _POOLS.get(pool)\n    if sem is None:\n        yield\n        return\n    sem.acquire()\n    try:\n        yield\n    finally:\n        sem.release()\n"
    );
    for t in &used {
        let m = tool_module(*t);
        let _ = writeln!(
            s,
            "class {}Tool:\n    \"\"\"{:?} 工具适配器\"\"\"\n    POOL = \"{}\"\n\n    @classmethod\n    def call(cls, action: str, ctx: dict, **kwargs):\n        with acquire(cls.POOL):\n            started = time.time()\n            result = cls._invoke(action, ctx, **kwargs)\n            ctx.setdefault(\"_timings\", {{}})[action] = time.time() - started\n            return result\n\n    @classmethod\n    def _invoke(cls, action: str, ctx: dict, **kwargs):\n        raise NotImplementedError(\"接入真实 {:?} SDK\")\n",
            capitalize(m),
            t,
            t.resource_pool(),
            t
        );
    }
    s
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// 业务层：每个流程节点一个函数，签名带显式读写集
fn gen_tasks(graph: &FlowGraph) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "\"\"\"业务层 —— 每个函数对应流程图一个节点。\n\n读写集由流程图声明推导，供调度层做依赖校验。\n\"\"\"");
    let _ = writeln!(
        s,
        "from .tools import *  # noqa\nfrom .errors import GuardFailed\n"
    );
    for n in graph.nodes.iter().filter(|n| n.kind.is_executable()) {
        let fname = py_ident(&n.id);
        let reads: Vec<String> = n.read_set().iter().map(|r| format!("\"{}\"", r)).collect();
        let writes: Vec<String> = n.write_set().iter().map(|r| format!("\"{}\"", r)).collect();
        let _ = writeln!(s, "def {}(ctx: dict):", fname);
        let _ = writeln!(
            s,
            "    \"\"\"{}\n\n    节点: {} | 类型: {:?} | 预估: {}ms",
            n.name, n.id, n.kind, n.duration_ms
        );
        let _ = writeln!(s, "    reads : [{}]", reads.join(", "));
        let _ = writeln!(s, "    writes: [{}]", writes.join(", "));
        let _ = writeln!(s, "    \"\"\"");
        if n.kind == NodeKind::Guard {
            let _ = writeln!(
                s,
                "    # 前置拦截：reads 声明的键必须存在于 ctx，缺失即拦截，避免无效工具调用与 LLM 消耗\n    _missing = [k for k in [{}] if k not in ctx]\n    if _missing:\n        raise GuardFailed(f\"{} 缺少输入: {{_missing}}\")\n    return ctx",
                reads.join(", "),
                n.name
            );
        } else if let Some(t) = n.tool {
            if n.transactional {
                let _ = writeln!(
                    s,
                    "    with {}Tool.transaction(ctx):",
                    capitalize(tool_module(t))
                );
                let _ = writeln!(
                    s,
                    "        return {}Tool.call(\"{}\", ctx)",
                    capitalize(tool_module(t)),
                    n.id
                );
            } else {
                let _ = writeln!(
                    s,
                    "    return {}Tool.call(\"{}\", ctx)",
                    capitalize(tool_module(t)),
                    n.id
                );
            }
        } else {
            let _ = writeln!(s, "    return ctx");
        }
        let _ = writeln!(s);
    }
    s
}

/// 异常层
fn gen_errors(graph: &FlowGraph) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "\"\"\"异常层 —— 统一异常类型与兜底处理。\"\"\"\n");
    let _ = writeln!(
        s,
        "class FlowError(Exception):\n    \"\"\"流程基类异常\"\"\"\n\n\nclass GuardFailed(FlowError):\n    \"\"\"前置校验失败（合规/路径/参数）\"\"\"\n\n\nclass ToolFailed(FlowError):\n    \"\"\"工具调用失败\"\"\"\n\n\nclass ComplianceViolation(FlowError):\n    \"\"\"业务合规规则违反\"\"\"\n"
    );
    let handlers: Vec<&FlowEdge> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Exception)
        .collect();
    let _ = writeln!(s, "\n# 流程图声明的异常路由表: 源节点 -> 处理节点");
    let _ = writeln!(s, "EXCEPTION_ROUTES = {{");
    for e in handlers {
        let _ = writeln!(s, "    \"{}\": \"{}\",", e.from, e.to);
    }
    let _ = writeln!(s, "}}\n");
    let _ = writeln!(
        s,
        "def handle(node_id: str, exc: Exception, ctx: dict):\n    \"\"\"路由到异常处理节点；无路由则原样抛出。\n\n    返回处理节点 id，由调度层负责实际执行。\n    \"\"\"\n    target = EXCEPTION_ROUTES.get(node_id)\n    ctx.setdefault(\"_errors\", []).append({{\"node\": node_id, \"error\": repr(exc), \"route\": target}})\n    if target is None:\n        raise exc\n    return target\n"
    );
    s
}

/// 异常处理节点：入边全为 Exception 边。这类节点**不得**进入正常执行层，
/// 否则会在无错误时被无条件触发。
fn is_exception_handler(graph: &FlowGraph, id: &str) -> bool {
    let incoming: Vec<&FlowEdge> = graph.edges.iter().filter(|e| e.to == id).collect();
    !incoming.is_empty() && incoming.iter().all(|e| e.kind == EdgeKind::Exception)
}

/// 调度层：按并行层下发，体现资源受限排程
fn gen_scheduler(graph: &FlowGraph, plan: &ParallelPlan, schedule: &Schedule) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "\"\"\"调度层 —— 由数据流分析 + 资源受限调度自动生成。\n\n串行耗时 : {}ms\n并行下界 : {}ms (关键路径)\n资源受限 : {}ms (实际排程)\n加速比   : {:.2}x\n\"\"\"",
        plan.sequential_ms,
        plan.parallel_ms,
        schedule.makespan_ms,
        plan.speedup()
    );
    let _ = writeln!(
        s,
        "from concurrent.futures import ThreadPoolExecutor, as_completed\n"
    );
    let _ = writeln!(s, "from . import tasks, errors\n");

    let _ = writeln!(s, "# 执行层次：同层节点无数据依赖，可并发下发");
    let _ = writeln!(
        s,
        "# （异常处理节点已排除，仅在失败时由 errors.handle 路由触发）"
    );
    let _ = writeln!(s, "LAYERS = [");
    for layer in &plan.layers {
        let exec: Vec<&String> = layer
            .iter()
            .filter(|id| {
                graph
                    .node(id)
                    .map(|n| n.kind.is_executable())
                    .unwrap_or(false)
            })
            .filter(|id| !is_exception_handler(graph, id))
            .collect();
        if exec.is_empty() {
            continue;
        }
        let items: Vec<String> = exec.iter().map(|id| format!("\"{}\"", id)).collect();
        let _ = writeln!(s, "    [{}],", items.join(", "));
    }
    let _ = writeln!(s, "]\n");

    let _ = writeln!(s, "DISPATCH = {{");
    for n in graph.nodes.iter().filter(|n| n.kind.is_executable()) {
        let _ = writeln!(s, "    \"{}\": tasks.{},", n.id, py_ident(&n.id));
    }
    let _ = writeln!(s, "}}\n");

    let _ = writeln!(s, "# 资源池峰值占用（来自调度分析）");
    let _ = writeln!(s, "POOL_PEAK = {{");
    for p in &schedule.pools {
        let _ = writeln!(
            s,
            "    \"{}\": {{\"capacity\": {}, \"peak\": {}, \"utilization\": {:.3}}},",
            p.pool, p.capacity, p.peak, p.utilization
        );
    }
    let _ = writeln!(s, "}}\n");

    // 迭代上限护栏：图中 LoopStart 节点若声明了 props["max_iter"]（由编排层从
    // LoopGuard(Bounded) 桥接而来），则整个 LAYERS 执行需受迭代上限约束，
    // 避免有界循环被展开后失去上限保护（企业级不可无限重放）。多循环取最大上限。
    let loop_bound: Option<u32> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::LoopStart)
        .filter_map(|n| n.props.get("max_iter"))
        .filter_map(|v| v.parse::<u32>().ok())
        .max();
    let loop_guard_line = match loop_bound {
        Some(n) => format!("    for _loop_iter in range({}):\n", n),
        None => String::new(),
    };
    let loop_indent = if loop_bound.is_some() {
        "        "
    } else {
        "    "
    };

    let _ = writeln!(
        s,
        "def run(ctx: dict | None = None, max_workers: int = {}):\n    ctx = ctx if ctx is not None else {{}}\n{}for layer in LAYERS:\n{}if len(layer) == 1:\n{}node_id = layer[0]\n{}__run_one(node_id, ctx)\n{}continue\n{}with ThreadPoolExecutor(max_workers=min(max_workers, len(layer))) as pool:\n{}futures = {{pool.submit(_run_one, nid, ctx): nid for nid in layer}}\n{}for fut in as_completed(futures):\n{}fut.result()\n    return ctx\n",
        schedule.max_concurrency.max(1),
        loop_guard_line, loop_indent, loop_indent, loop_indent, loop_indent, loop_indent, loop_indent, loop_indent, loop_indent
    );
    let _ = writeln!(
        s,
        "\ndef _run_one(node_id: str, ctx: dict):\n    fn = DISPATCH[node_id]\n    try:\n        return fn(ctx)\n    except Exception as exc:  # noqa: BLE001\n        target = errors.handle(node_id, exc, ctx)\n        handler = DISPATCH.get(target)\n        return handler(ctx) if handler else None\n"
    );
    s
}

fn gen_main(graph: &FlowGraph) -> String {
    format!(
        "\"\"\"入口 —— 流程 `{}` ({})\"\"\"\nimport json\n\nfrom .scheduler import run\n\n\ndef main():\n    ctx = run()\n    print(json.dumps({{\"errors\": ctx.get(\"_errors\", []), \"timings\": ctx.get(\"_timings\", {{}})}}, ensure_ascii=False, indent=2))\n\n\nif __name__ == \"__main__\":\n    main()\n",
        graph.name, graph.id
    )
}

// ==================== 草莓多平台：数据库 DDL 生成 ====================
//
// 从流程图中所有 `db:` 前缀的访问声明自动推导表结构：
//   - 写访问（Write/ReadWrite）⇒ 建表；读访问 ⇒ 仅引用（不建表）
//   - 每个表收集其写入字段（var: 前缀归并到表字段）
//   - 事务性节点 ⇒ 该表标记需要事务
// 这是「草莓多」对话生成系统模板后产出数据库骨架的核心能力。

/// 从 `db:表名.字段` 解析出 (表, 字段)
fn parse_db_access(res: &str) -> Option<(String, String)> {
    let r = res.strip_prefix("db:")?;
    let (table, field) = match r.split_once('.') {
        Some((t, f)) => (t.to_string(), f.to_string()),
        None => (r.to_string(), "id".to_string()),
    };
    Some((table, field))
}

/// 流程图 → 数据库 DDL（PostgreSQL 方言，企业级默认）
pub fn gen_db_schema(graph: &FlowGraph) -> String {
    // 表 -> 字段集合（去重保序）
    let mut tables: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    let mut transactional_tables: BTreeSet<String> = BTreeSet::new();

    for n in &graph.nodes {
        for a in &n.accesses {
            if let Some((table, field)) = parse_db_access(&a.resource) {
                let entry = tables.entry(table.clone()).or_default();
                entry.entry(field.clone()).or_insert(true);
                if n.transactional && a.mode.writes() {
                    transactional_tables.insert(table.clone());
                }
            }
        }
    }

    if tables.is_empty() {
        return "-- 本流程图未声明任何 db: 资源访问，无数据库结构需要生成。\n".to_string();
    }

    let mut s = String::new();
    let _ = writeln!(
        s,
        "-- 数据库 Schema —— 由流程图「{}」自动生成，勿手改。",
        graph.name
    );
    let _ = writeln!(s, "-- 生成时间无关，可重复执行（幂等 DDL）。\n");

    for (table, fields) in &tables {
        // 安全加固：表名来自 `db:<table>.<field>` 资源解析，必须在生成 DDL 前做严格标识符消毒，
        // 防止名称中夹带 `; DROP TABLE ...` / 引号等注入（与列名 sql_ident 同规）。
        let table_ident = sql_ident(table);
        if table_ident.is_empty() {
            continue; // 表名全为非法字符：跳过，绝不注入未消毒标识符
        }
        let _ = writeln!(
            s,
            "-- 表: {} {}",
            table_ident,
            if transactional_tables.contains(table) {
                "(事务表)"
            } else {
                ""
            }
        );
        let _ = writeln!(s, "CREATE TABLE IF NOT EXISTS {} (", table_ident);
        let _ = writeln!(s, "    id          BIGSERIAL PRIMARY KEY,");
        // 由流程字段推导业务列（不含 id，已单独声明）
        let cols: Vec<&String> = fields.keys().filter(|f| *f != "id").collect();
        for (i, col) in cols.iter().enumerate() {
            let comma = if i + 1 == cols.len() { "" } else { "," };
            let _ = writeln!(
                s,
                "    {}   TEXT NOT NULL DEFAULT ''{}",
                sql_ident(col),
                comma
            );
        }
        let _ = writeln!(s, "    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),");
        let _ = writeln!(s, "    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()");
        let _ = writeln!(s, ");\n");
        let _ = writeln!(
            s,
            "CREATE INDEX IF NOT EXISTS idx_{}_updated ON {} (updated_at);\n",
            table_ident, table_ident
        );
    }

    let _ = writeln!(s, "-- 行级注释：企业级数据治理留痕");
    let _ = writeln!(s, "-- 共生成 {} 张表。", tables.len());
    s
}

fn sql_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

// ==================== 草莓多平台：前端 Vue 生成 ====================
//
// 从流程图的控制/任务节点推导一个最小可用的 Vue3 视图骨架：
//   - 每个 Guard/Task 节点的读写字段 ⇒ 表单字段（响应式 ref）
//   - Guard 节点 ⇒ 前端校验规则（required）
//   - Start/End ⇒ 页面说明
// 目标是「对话生成模板 → 直接给出可运行前端骨架」。

/// 从流程节点收集前端需要展示的字段（var:/db: 前缀的业务字段）
fn collect_form_fields(graph: &FlowGraph) -> Vec<(String, bool)> {
    let mut seen: BTreeMap<String, bool> = BTreeMap::new();
    for n in &graph.nodes {
        for a in &n.accesses {
            let key = if let Some(r) = a.resource.strip_prefix("var:") {
                Some(r.to_string())
            } else if let Some((_, f)) = parse_db_access(&a.resource) {
                Some(f)
            } else {
                None
            };
            if let Some(k) = key {
                if k == "id" {
                    continue;
                }
                let required = n.kind == NodeKind::Guard && a.mode.writes();
                let entry = seen.entry(k).or_insert(false);
                *entry = *entry || required;
            }
        }
    }
    seen.into_iter().collect()
}

/// 流程图 → 前端 Vue3 单文件组件（组合式 API 骨架）
pub fn gen_frontend(graph: &FlowGraph) -> String {
    let fields = collect_form_fields(graph);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "<!-- 前端视图骨架 —— 由流程图「{}」自动生成，勿手改。 -->",
        graph.name
    );
    let _ = writeln!(s, "<template>");
    let _ = writeln!(s, "  <div class=\"caomei-flow\">");
    let _ = writeln!(s, "    <h2>{}</h2>", graph.name);
    let _ = writeln!(s, "    <form @submit.prevent=\"submit\">");
    for (field, required) in &fields {
        let label = field.replace('_', " ");
        let _ = writeln!(s, "      <label>{}:", label);
        let _ = writeln!(
            s,
            "        <input v-model=\"form.{}\" {} />",
            field,
            if *required { "required" } else { "" }
        );
        let _ = writeln!(s, "      </label>");
    }
    let _ = writeln!(s, "      <button type=\"submit\">提交</button>");
    let _ = writeln!(s, "    </form>");
    let _ = writeln!(s, "    <pre v-if=\"result\">{{ result }}</pre>");
    let _ = writeln!(s, "  </div>");
    let _ = writeln!(s, "</template>\n");

    let _ = writeln!(s, "<script setup>");
    let _ = writeln!(s, "// 组合式 API：字段与提交流程由流程图推导");
    let _ = writeln!(s, "import {{ reactive, ref }} from 'vue'");
    let _ = writeln!(s, "const form = reactive({{");
    for (field, _) in &fields {
        let _ = writeln!(s, "  {}: '',", field);
    }
    let _ = writeln!(s, "}})");
    let _ = writeln!(s, "const result = ref(null)");
    let _ = writeln!(s, "async function submit() {{");
    let _ = writeln!(s, "  // 真实提交：调用流程图「{}」对应后端入口", graph.name);
    let name_json = serde_json::to_string(&graph.name).unwrap_or_else(|_| "\"flow\"".to_string());
    let _ = writeln!(s, "  const res = await fetch(`/api/flow/${{encodeURIComponent({})}}`, {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: JSON.stringify(form) }})", name_json);
    let _ = writeln!(s, "  result.value = await res.json()");
    let _ = writeln!(s, "}}");
    let _ = writeln!(s, "</script>\n");

    let _ = writeln!(s, "<style scoped>");
    let _ = writeln!(s, ".caomei-flow {{ max-width: 720px; margin: 2rem auto; }}");
    let _ = writeln!(s, "label {{ display: block; margin: .6rem 0; }}");
    let _ = writeln!(s, "input {{ width: 100%; padding: .4rem; }}");
    let _ = writeln!(s, "</style>");
    s
}

// ==================== 逆向：代码 → 流程图 ====================

/// 逆向解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseResult {
    pub graph: FlowGraph,
    /// 自动补全的缺陷（原代码里缺失的分支/异常处理）
    pub gaps: Vec<String>,
}

/// 从 Python RPA 源码反生成流程图
///
/// 采用缩进敏感的轻量结构解析：识别 def / if / else / for / while / try / except
/// 以及常见 RPA 调用（open / requests / selenium / cursor.execute / subprocess）。
/// 目标不是完整 Python parser，而是把**控制结构与外部副作用**抽出成可视化流程。
pub fn reverse_from_python(src: &str, flow_id: &str) -> ReverseResult {
    let mut g = FlowGraph::new(flow_id, format!("{} (逆向解析)", flow_id));
    let mut gaps = Vec::new();
    g.add_node(FlowNode::new("start", "开始", NodeKind::Start));

    // (indent, node_id, kind) 栈
    let mut stack: Vec<(usize, String, NodeKind)> = vec![(0, "start".into(), NodeKind::Start)];
    let mut prev = "start".to_string();
    let mut counter = 0usize;
    let mut has_try = false;
    let mut in_docstring = false;

    for raw in src.lines() {
        let trimmed = raw.trim();

        // 跨行文档字符串：整段忽略，否则注释文本会被误识为节点
        let quote_hits = trimmed.matches("\"\"\"").count() + trimmed.matches("'''").count();
        if in_docstring {
            if quote_hits % 2 == 1 {
                in_docstring = false;
            }
            continue;
        }
        if quote_hits % 2 == 1 {
            in_docstring = true;
            continue;
        }
        if quote_hits >= 2 && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
            continue; // 单行文档字符串
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();

        // 退栈到当前缩进
        while stack.len() > 1 && stack.last().map(|(i, _, _)| *i >= indent).unwrap_or(false) {
            let (_, id, kind) = stack.pop().unwrap();
            if kind == NodeKind::LoopStart {
                counter += 1;
                let end_id = format!("loop_end_{}", counter);
                g.add_node(FlowNode::new(end_id.clone(), "循环结束", NodeKind::LoopEnd));
                g.add_edge(FlowEdge::seq(prev.clone(), end_id.clone()));
                g.add_edge(FlowEdge::seq(end_id.clone(), id.clone()));
                prev = end_id;
            }
        }

        counter += 1;
        let nid = format!("n{}", counter);

        if let Some(cond) = strip_kw(trimmed, "if ") {
            let node = FlowNode::new(nid.clone(), format!("判断: {}", cond), NodeKind::Decision);
            g.add_node(node);
            g.add_edge(FlowEdge::seq(prev.clone(), nid.clone()));
            stack.push((indent, nid.clone(), NodeKind::Decision));
            prev = nid;
        } else if trimmed.starts_with("elif ") || trimmed == "else:" {
            // 挂回最近的 Decision
            if let Some((_, did, _)) = stack
                .iter()
                .rev()
                .find(|(_, _, k)| *k == NodeKind::Decision)
            {
                prev = did.clone();
            }
        } else if let Some(cond) = strip_kw(trimmed, "for ").or_else(|| strip_kw(trimmed, "while "))
        {
            let node = FlowNode::new(nid.clone(), format!("循环: {}", cond), NodeKind::LoopStart);
            g.add_node(node);
            g.add_edge(FlowEdge::seq(prev.clone(), nid.clone()));
            stack.push((indent, nid.clone(), NodeKind::LoopStart));
            prev = nid;
        } else if trimmed == "try:" {
            has_try = true;
            let node = FlowNode::new(nid.clone(), "异常保护段", NodeKind::Guard).with_tag("try");
            g.add_node(node);
            g.add_edge(FlowEdge::seq(prev.clone(), nid.clone()));
            stack.push((indent, nid.clone(), NodeKind::Guard));
            prev = nid;
        } else if trimmed.starts_with("except") {
            let node = FlowNode::new(nid.clone(), "异常处理", NodeKind::Guard).with_tag("except");
            g.add_node(node);
            if let Some((_, tid, _)) = stack.iter().rev().find(|(_, _, k)| *k == NodeKind::Guard) {
                g.add_edge(FlowEdge::exception(tid.clone(), nid.clone()));
            }
            prev = nid;
        } else if trimmed.starts_with("def ") {
            continue;
        } else if let Some((tool, label)) = detect_tool_call(trimmed) {
            let mut node = FlowNode::task(nid.clone(), label, tool, default_duration(tool));
            node.idempotent = matches!(tool, ToolKind::Compute | ToolKind::File);
            g.add_node(node);
            g.add_edge(FlowEdge::seq(prev.clone(), nid.clone()));
            prev = nid;
        }
    }

    // 收尾循环
    while stack.len() > 1 {
        let (_, id, kind) = stack.pop().unwrap();
        if kind == NodeKind::LoopStart {
            counter += 1;
            let end_id = format!("loop_end_{}", counter);
            g.add_node(FlowNode::new(end_id.clone(), "循环结束", NodeKind::LoopEnd));
            g.add_edge(FlowEdge::seq(prev.clone(), end_id.clone()));
            g.add_edge(FlowEdge::seq(end_id.clone(), id.clone()));
            prev = end_id;
        }
    }

    g.add_node(FlowNode::new("end", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq(prev, "end"));

    // 缺陷补全：外部调用无 try 保护
    let risky: Vec<String> = g
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.tool,
                Some(ToolKind::Browser)
                    | Some(ToolKind::Database)
                    | Some(ToolKind::Http)
                    | Some(ToolKind::Shell)
            )
        })
        .map(|n| n.name.clone())
        .collect();
    if !has_try && !risky.is_empty() {
        gaps.push(format!(
            "原代码 {} 处外部调用（{}）无 try/except 保护，已在流程图标记待补异常分支",
            risky.len(),
            risky.join("、")
        ));
        let handler = FlowNode::new("__error_handler", "统一异常处理", NodeKind::Guard)
            .with_tag("error_handler");
        g.add_node(handler);
        let ids: Vec<String> = g
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.tool,
                    Some(ToolKind::Browser)
                        | Some(ToolKind::Database)
                        | Some(ToolKind::Http)
                        | Some(ToolKind::Shell)
                )
            })
            .map(|n| n.id.clone())
            .collect();
        for id in ids {
            g.add_edge(FlowEdge::exception(id, "__error_handler"));
        }
    }

    // 缺陷补全：判断节点缺 else
    let decisions: Vec<String> = g
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Decision)
        .map(|n| n.id.clone())
        .collect();
    for d in decisions {
        let outs = g.edges.iter().filter(|e| e.from == d).count();
        if outs < 2 {
            let name = g.node(&d).map(|n| n.name.clone()).unwrap_or_default();
            gaps.push(format!("判断节点「{}」缺少 else 分支，已标记", name));
        }
    }

    ReverseResult { graph: g, gaps }
}

fn strip_kw<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    line.strip_prefix(kw)
        .map(|r| r.trim_end_matches(':').trim())
        .filter(|r| !r.is_empty())
}

fn default_duration(t: ToolKind) -> u64 {
    match t {
        ToolKind::Compute => 20,
        ToolKind::Llm => 1500,
        ToolKind::File => 200,
        ToolKind::Browser => 800,
        ToolKind::Database => 300,
        ToolKind::Http => 400,
        ToolKind::Shell => 250,
        ToolKind::Human => 60_000,
    }
}

fn detect_tool_call(line: &str) -> Option<(ToolKind, String)> {
    // `return XxxTool.call(...)` 也是一次工具调用，先剥离 return 前缀
    let body = line.strip_prefix("return ").unwrap_or(line).trim();
    let l = body.to_lowercase();
    let label = body.trim_end_matches(':').trim().to_string();

    // 识别本库自己生成的工具层调用，支持 代码→流程图 回稻
    if let Some(pos) = l.find("tool.call(") {
        let prefix = &l[..pos];
        let kind = match prefix
            .rsplit(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap_or("")
        {
            "browser" => ToolKind::Browser,
            "database" => ToolKind::Database,
            "http" => ToolKind::Http,
            "shell" => ToolKind::Shell,
            "fileio" | "file_io" | "file" => ToolKind::File,
            "llm" => ToolKind::Llm,
            "human" => ToolKind::Human,
            _ => ToolKind::Compute,
        };
        return Some((kind, label));
    }
    let table: &[(&[&str], ToolKind)] = &[
        (
            &[
                "driver.",
                "selenium",
                "page.",
                "playwright",
                "webdriver",
                "browser.",
            ],
            ToolKind::Browser,
        ),
        (
            &[
                "cursor.execute",
                "conn.",
                "session.query",
                "sqlalchemy",
                "cursor.",
            ],
            ToolKind::Database,
        ),
        (
            &["requests.", "httpx.", "urlopen", "aiohttp"],
            ToolKind::Http,
        ),
        (&["subprocess.", "os.system", "popen"], ToolKind::Shell),
        (
            &[
                "open(",
                "pandas.read",
                "pd.read",
                "to_excel",
                "to_csv",
                "openpyxl",
                "workbook",
            ],
            ToolKind::File,
        ),
        (
            &["openai.", "llm.", "chat.completions", "invoke_model"],
            ToolKind::Llm,
        ),
        (&["input(", "confirm(", "approve("], ToolKind::Human),
    ];
    for (pats, kind) in table {
        if pats.iter().any(|p| l.contains(p)) {
            return Some((*kind, label));
        }
    }
    // 普通赋值/函数调用视为计算节点（过滤纯语法行）
    let noise = l.starts_with("print(")
        || l.starts_with("import ")
        || l.starts_with("from ")
        || l.starts_with("class ")
        || l.starts_with("@")
        || l.starts_with("raise ")
        || l == "pass";
    if l.contains('(') && !noise {
        return Some((ToolKind::Compute, label));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict;
    use crate::dataflow;
    use crate::model::{Access, ToolKind};
    use crate::schedule;

    fn pipeline() -> FlowGraph {
        let mut g = FlowGraph::new("p", "报表流水线");
        g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::task("read", "读取Excel", ToolKind::File, 300)
                .with_access(Access::read("file:in.xlsx"))
                .with_access(Access::write("var:rows"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("query", "查库", ToolKind::Database, 400)
                .with_access(Access::read("db:orders"))
                .with_access(Access::write("var:orders"))
                .idempotent(true),
        );
        g.add_node(
            FlowNode::task("save", "落库", ToolKind::Database, 300)
                .with_access(Access::write("db:orders.order_no"))
                .with_access(Access::write("db:orders.amount"))
                .transactional(true),
        );
        g.add_node(
            FlowNode::task("merge", "汇总", ToolKind::Compute, 100)
                .with_access(Access::read("var:rows"))
                .with_access(Access::read("var:orders"))
                .with_access(Access::write("file:out.xlsx")),
        );
        g.add_node(FlowNode::new("end", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "read"));
        g.add_edge(FlowEdge::seq("read", "query"));
        g.add_edge(FlowEdge::seq("query", "merge"));
        g.add_edge(FlowEdge::seq("merge", "end"));
        g.add_edge(FlowEdge::seq("merge", "save"));
        g.add_edge(FlowEdge::seq("save", "end"));
        g.add_edge(FlowEdge::exception("query", "merge"));
        g
    }

    fn bundle_of(g: &FlowGraph) -> CodeBundle {
        let plan = dataflow::analyze(g);
        let sc = schedule::schedule(g, &plan.dependencies);
        let cf = conflict::detect(g, &plan.layers);
        generate(g, &plan, &sc, &cf)
    }

    #[test]
    fn generates_layered_project() {
        let g = pipeline();
        let b = bundle_of(&g);
        assert!(!b.rejected, "{:?}", b.reject_reasons);
        for p in [
            "generated/tools.py",
            "generated/tasks.py",
            "generated/scheduler.py",
            "generated/errors.py",
            "generated/main.py",
        ] {
            assert!(b.file(p).is_some(), "缺少 {}", p);
        }
        assert!(b.total_lines() > 60);
    }

    #[test]
    fn scheduler_encodes_parallel_layers() {
        let g = pipeline();
        let b = bundle_of(&g);
        let sched = &b.file("generated/scheduler.py").unwrap().content;
        assert!(sched.contains("ThreadPoolExecutor"));
        // read 与 query 无数据依赖，应在同一并行层（同一层数组中同时出现）
        let layer_line = sched
            .lines()
            .find(|l| l.contains("\"query\"") && l.contains("\"read\""))
            .unwrap_or("");
        assert!(
            layer_line.contains("\"query\"")
                && layer_line.contains("\"read\"")
                && layer_line.contains('['),
            "调度层未将 read/query 放入同一并行层:\n{}",
            sched
        );
    }

    #[test]
    fn tools_layer_has_semaphore_capacity() {
        let g = pipeline();
        let b = bundle_of(&g);
        let tools = &b.file("generated/tools.py").unwrap().content;
        assert!(tools.contains("threading.Semaphore"));
        assert!(tools.contains("\"db\""));
    }

    #[test]
    fn rejects_on_blocking_conflict() {
        let mut g = pipeline();
        // 制造双写文件冲突并让二者并行
        g.nodes.push(
            FlowNode::task("merge2", "汇总2", ToolKind::File, 100)
                .with_access(Access::write("file:out.xlsx")),
        );
        let plan = dataflow::analyze(&g);
        let sc = schedule::schedule(&g, &plan.dependencies);
        let mut cf = conflict::detect(&g, &plan.layers);
        // 强制注入一个阻断冲突以验证拒绝路径
        cf.conflicts.push(crate::conflict::Conflict {
            kind: crate::conflict::ConflictKind::FileLock,
            severity: crate::model::Severity::Blocking,
            nodes: vec!["merge".into(), "merge2".into()],
            resource: Some("file:out.xlsx".into()),
            message: "并发写同一文件".into(),
            remedy: None,
        });
        let b = generate(&g, &plan, &sc, &cf);
        assert!(b.rejected);
        assert!(b.files.is_empty());
    }

    #[test]
    fn generated_identifiers_avoid_name_mangling() {
        assert_eq!(py_ident("__error_handler"), "op_error_handler");
        assert_eq!(
            py_ident("__guard_desensitize_db"),
            "op_guard_desensitize_db"
        );
        assert_eq!(py_ident("read"), "read");
        assert_eq!(py_ident("9bad"), "_9bad");
        // 生成的 tasks.py 不得出现 `def __` 开头的函数
        let src = "driver.get(\"http://a\")\n";
        let g = reverse_from_python(src, "lg").graph;
        let b = bundle_of(&g);
        let tasks = &b.file("generated/tasks.py").unwrap().content;
        assert!(
            !tasks.contains("def __"),
            "不应生成双下划线函数名:\n{}",
            tasks
        );
    }

    #[test]
    fn exception_handler_excluded_from_normal_layers() {
        // 逆向解析会注入 __error_handler（仅有异常入边），它不得出现在正常执行层，
        // 否则无错误时也会被无条件执行。
        let src = "driver.get(\"http://a\")\ncursor.execute(\"select 1\")\n";
        let g = reverse_from_python(src, "legacy3").graph;
        assert!(g.node("__error_handler").is_some());
        let b = bundle_of(&g);
        let sched = &b.file("generated/scheduler.py").unwrap().content;
        let layers_block = sched
            .split("LAYERS = [")
            .nth(1)
            .and_then(|s| s.split(']').next())
            .unwrap_or("");
        assert!(
            !layers_block.contains("__error_handler"),
            "异常处理节点泄漏到执行层:\n{}",
            layers_block
        );
        // 但它仍须可被路由调度
        assert!(
            sched.contains("__error_handler"),
            "处理器应仍在 DISPATCH 中可被路由"
        );
        assert!(sched.contains("handler(ctx) if handler else None"));
    }

    #[test]
    fn reverse_parses_control_structures() {
        let src = r#"
def run():
    rows = pd.read_excel("in.xlsx")
    for row in rows:
        if row.valid:
            driver.get(row.url)
            cursor.execute("insert into t values (?)", row.id)
    requests.post("http://api/report")
"#;
        let r = reverse_from_python(src, "legacy");
        let kinds: Vec<NodeKind> = r.graph.nodes.iter().map(|n| n.kind).collect();
        assert!(kinds.contains(&NodeKind::LoopStart));
        assert!(kinds.contains(&NodeKind::Decision));
        assert!(r
            .graph
            .nodes
            .iter()
            .any(|n| n.tool == Some(ToolKind::Browser)));
        assert!(r
            .graph
            .nodes
            .iter()
            .any(|n| n.tool == Some(ToolKind::Database)));
        assert!(r.graph.nodes.iter().any(|n| n.tool == Some(ToolKind::File)));
    }

    #[test]
    fn reverse_reports_missing_exception_handling() {
        let src = "driver.get(\"http://a\")\ncursor.execute(\"select 1\")\n";
        let r = reverse_from_python(src, "legacy2");
        assert!(!r.gaps.is_empty());
        assert!(r.graph.node("__error_handler").is_some());
        assert!(r.graph.edges.iter().any(|e| e.kind == EdgeKind::Exception));
    }

    #[test]
    fn roundtrip_flow_to_code_to_flow() {
        let g = pipeline();
        let b = bundle_of(&g);
        let tasks = &b.file("generated/tasks.py").unwrap().content;
        let back = reverse_from_python(tasks, "roundtrip");
        // 反解析应至少恢复出可执行节点，且图保持有效拓扑序
        assert!(back.graph.nodes.len() > 2);
        let topo = back.graph.topo_order();
        assert!(
            topo.is_ok(),
            "roundtrip 反解析图应满足拓扑序，实际: {:?}",
            topo.err()
        );
    }

    // ============ 草莓多平台：全栈生成扩展测试 ============

    #[test]
    fn generates_db_schema_from_db_access() {
        let g = pipeline();
        let b = bundle_of(&g);
        let sql = &b.file("generated/schema.sql").unwrap().content;
        // 落库节点写入 db:orders.order_no/amount ⇒ 应建 orders 表
        assert!(
            sql.contains("CREATE TABLE IF NOT EXISTS orders"),
            "未生成 orders 表:\n{}",
            sql
        );
        assert!(sql.contains("order_no"), "字段推导缺失:\n{}", sql);
        assert!(sql.contains("amount"), "字段推导缺失:\n{}", sql);
        // 读访问 db:orders 不应重复建表
        assert_eq!(sql.matches("CREATE TABLE IF NOT EXISTS orders").count(), 1);
        // 事务表标记
        assert!(sql.contains("事务表"), "事务性节点未标注:\n{}", sql);
    }

    #[test]
    fn db_schema_is_idempotent_ddl() {
        let g = pipeline();
        let b = bundle_of(&g);
        let sql = &b.file("generated/schema.sql").unwrap().content;
        // 全部用 IF NOT EXISTS，可重复执行
        assert!(sql.contains("IF NOT EXISTS"));
        assert!(!sql.contains("DROP TABLE"));
    }

    #[test]
    fn generates_vue_frontend_skeleton() {
        let g = pipeline();
        let b = bundle_of(&g);
        let vue = &b.file("generated/App.vue").unwrap().content;
        assert!(vue.contains("<template>"));
        assert!(vue.contains("<script setup>"));
        assert!(vue.contains("import { reactive, ref }"));
        // 表单字段来自流程读写集
        assert!(
            vue.contains("v-model=\"form.order_no\"") || vue.contains("v-model=\"form.amount\"")
        );
    }

    #[test]
    fn frontend_marks_guard_fields_required() {
        // Guard 节点的写入字段应在前端标记为 required
        let mut g = FlowGraph::new("signup", "注册");
        g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
        g.add_node(
            FlowNode::new("validate", "校验手机号", NodeKind::Guard)
                .with_access(Access::write("var:phone")),
        );
        g.add_node(FlowNode::new("end", "结束", NodeKind::End));
        g.add_edge(FlowEdge::seq("start", "validate"));
        g.add_edge(FlowEdge::seq("validate", "end"));
        let plan = dataflow::analyze(&g);
        let sc = schedule::schedule(&g, &plan.dependencies);
        let cf = conflict::detect(&g, &plan.layers);
        let b = generate(&g, &plan, &sc, &cf);
        let vue = &b.file("generated/App.vue").unwrap().content;
        assert!(vue.contains("required"), "Guard 写入字段应为必填:\n{}", vue);
        assert!(
            vue.contains("v-model=\"form.phone\""),
            "字段未出现在表单:\n{}",
            vue
        );
    }
}
