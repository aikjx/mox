#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
agentic_loop_minimal.py
========================
L4 编排层 Agentic 闭环 —— 可真实运行的最小示例（纯标准库，无第三方依赖）。

落地了文档 §5.6–§5.9 的核心机制：
  - 形式化有限状态机（FSM）：IDLE→PERCEIVE→RECALL→PLAN→ACT→OBSERVE→REFLECT→{HITL}→GENERATE→CONSOLIDATE→DONE/ABORT
  - 三条循环守卫：步数/预算熔断、进展检测、高风险 HITL
  - 模型路由：规划/反思用强模型，执行用轻模型（成本模拟）
  - 每步检查点（落盘 JSON，可续跑）
  - HITL 钩子（高风险动作人工确认）
  - 成本追踪
  - 内存向量检索（词袋 + 余弦相似度，作为 RECALL 的记忆机制）
  - 一个真实工具：calculator（基于 ast 的安全算术求值）

说明：LLM 调用在此用确定性 stub 替代，因此无需任何 API Key 即可真实跑通；
      但 FSM / 守卫 / 路由 / 检查点 / HITL / 成本 等“系统心脏”均为真实可执行的代码路径。
"""

import ast
import json
import math
import os
import re
import time
from collections import Counter
from dataclasses import dataclass, field, asdict
from typing import Callable, Optional

# ----------------------------------------------------------------------------
# 1. 形式化状态机
# ----------------------------------------------------------------------------

class State:
    IDLE = "IDLE"
    PERCEIVE = "PERCEIVE"
    RECALL = "RECALL"
    PLAN = "PLAN"
    ACT = "ACT"
    OBSERVE = "OBSERVE"
    REFLECT = "REFLECT"
    HITL = "HITL"
    GENERATE = "GENERATE"
    CONSOLIDATE = "CONSOLIDATE"
    DONE = "DONE"
    ABORT = "ABORT"


# 状态转移表（显式，便于审计与校验）
TRANSITIONS = {
    State.IDLE: [State.PERCEIVE],
    State.PERCEIVE: [State.RECALL],
    State.RECALL: [State.PLAN],
    State.PLAN: [State.ACT],
    State.ACT: [State.OBSERVE, State.HITL],
    State.OBSERVE: [State.REFLECT],
    State.REFLECT: [State.HITL, State.GENERATE],
    State.HITL: [State.PLAN, State.ABORT],
    State.GENERATE: [State.CONSOLIDATE],
    State.CONSOLIDATE: [State.DONE],
    State.DONE: [],
    State.ABORT: [],
}

RISK_HIGH = ["delete", "rm ", "remove", "drop", "shutdown", "format", "清空"]


# ----------------------------------------------------------------------------
# 2. 内存向量检索（RECALL 的记忆机制，词袋 + 余弦）
# ----------------------------------------------------------------------------

def _tokenize(text: str):
    return re.findall(r"[a-zA-Z0-9\u4e00-\u9fff]+", text.lower())


def _embed(text: str) -> Counter:
    return Counter(_tokenize(text))


def _cosine(a: Counter, b: Counter) -> float:
    if not a or not b:
        return 0.0
    common = set(a) & set(b)
    num = sum(a[t] * b[t] for t in common)
    na = math.sqrt(sum(v * v for v in a.values()))
    nb = math.sqrt(sum(v * v for v in b.values()))
    return num / (na * nb) if na and nb else 0.0


class VectorStore:
    """最简单的内存向量库：句袋向量 + 余弦检索。"""
    def __init__(self):
        self.docs = []          # list of (id, text, embedding)
        self._next_id = 0

    def add(self, text: str) -> int:
        did = self._next_id
        self._next_id += 1
        self.docs.append((did, text, _embed(text)))
        return did

    def retrieve(self, query: str, k: int = 3):
        q = _embed(query)
        scored = [(did, text, _cosine(q, emb)) for did, text, emb in self.docs]
        scored = [s for s in scored if s[2] > 0.0]
        scored.sort(key=lambda x: x[2], reverse=True)
        return scored[:k]


# ----------------------------------------------------------------------------
# 3. 真实工具：calculator（基于 ast 的安全算术）
# ----------------------------------------------------------------------------

_ALLOWED_BINOPS = (ast.Add, ast.Sub, ast.Mult, ast.Div, ast.Pow, ast.Mod)
_ALLOWED_UNARY = (ast.UAdd, ast.USub)


def calculator(expr: str) -> str:
    """仅允许数字与 + - * / ** % 和括号的安全算术求值。"""
    def _eval(node):
        if isinstance(node, ast.Expression):
            return _eval(node.body)
        if isinstance(node, ast.Constant):
            if isinstance(node.value, (int, float)):
                return node.value
            raise ValueError("仅支持数值常量")
        if isinstance(node, ast.BinOp) and isinstance(node.op, _ALLOWED_BINOPS):
            return _binop(node.op, _eval(node.left), _eval(node.right))
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, _ALLOWED_UNARY):
            v = _eval(node.operand)
            return v if isinstance(node.op, ast.UAdd) else -v
        raise ValueError(f"不支持的语法: {ast.dump(node)}")

    def _binop(op, a, b):
        if isinstance(op, ast.Add): return a + b
        if isinstance(op, ast.Sub): return a - b
        if isinstance(op, ast.Mult): return a * b
        if isinstance(op, ast.Div): return a / b
        if isinstance(op, ast.Pow): return a ** b
        if isinstance(op, ast.Mod): return a % b

    try:
        val = _eval(ast.parse(expr, mode="eval"))
        return f"{expr} = {val}"
    except Exception as e:  # noqa: BLE001
        return f"calculator 错误: {e}"


# ----------------------------------------------------------------------------
# 4. 模型路由 + 成本追踪（模拟）
# ----------------------------------------------------------------------------

MODEL_COST = {            # 每千 token 的模拟单价（美元）
    "strong": 0.03,
    "small": 0.002,
}


@dataclass
class CostTracker:
    total_usd: float = 0.0
    calls: int = 0

    def charge(self, model: str, prompt_tokens: int, completion_tokens: int):
        t = (prompt_tokens + completion_tokens) / 1000.0
        self.total_usd += MODEL_COST[model] * t
        self.calls += 1


# 确定性 stub：在没有 API Key 时，用规则产出“规划/反思/生成”结果，
# 真实部署时替换为 LLM 调用即可（签名保持一致）。
def _llm_strong(system: str, user: str) -> str:
    return f"[strong] plan/reflect for: {user[:60]}"


def _llm_small(system: str, user: str) -> str:
    return f"[small] act for: {user[:60]}"


# ----------------------------------------------------------------------------
# 5. 循环守卫
# ----------------------------------------------------------------------------

@dataclass
class Guards:
    max_steps: int = 8
    budget_usd: float = 0.05
    no_progress_limit: int = 3          # 连续 N 步进展分数不升则熔断
    progress_threshold: float = 0.01


# ----------------------------------------------------------------------------
# 6. 可运行闭环
# ----------------------------------------------------------------------------

@dataclass
class LoopState:
    task: str = ""
    state: str = State.IDLE
    step: int = 0
    trace: list = field(default_factory=list)        # 情景记忆（情景 Trace）
    progress: float = 0.0
    no_progress_count: int = 0
    result: str = ""
    log: list = field(default_factory=list)


class AgenticLoop:
    def __init__(
        self,
        store: VectorStore,
        tools: dict[str, Callable[[str], str]],
        guards: Guards | None = None,
        hitl_fn: Optional[Callable[[str], bool]] = None,
        checkpoint_path: str = "agentic_checkpoint.json",
    ):
        self.store = store
        self.tools = tools
        self.guards = guards or Guards()
        self.hitl_fn = hitl_fn or (lambda action: True)   # 默认自动通过
        self.checkpoint_path = checkpoint_path
        self.cost = CostTracker()
        self.s = LoopState()

    # ---- 检查点（每步落盘，可续跑）----
    def _checkpoint(self):
        try:
            with open(self.checkpoint_path, "w", encoding="utf-8") as f:
                json.dump(asdict(self.s), f, ensure_ascii=False, indent=2)
        except OSError:
            pass

    def _transition(self, nxt: str):
        if nxt not in TRANSITIONS[self.s.state]:
            raise RuntimeError(f"非法转移: {self.s.state} -> {nxt}")
        self.s.state = nxt
        self.s.log.append(f"step {self.s.step}: {self.s.state}")

    def _guard_pass(self) -> bool:
        if self.s.step >= self.guards.max_steps:
            self.s.log.append("守卫触发: 步数上限")
            return False
        if self.cost.total_usd >= self.guards.budget_usd:
            self.s.log.append("守卫触发: 预算熔断")
            return False
        if self.s.no_progress_count >= self.guards.no_progress_limit:
            self.s.log.append("守卫触发: 进展停滞（错误方向勤奋）")
            return False
        return True

    def run(self, task: str, recall_top_k: int = 3) -> str:
        self.s = LoopState(task=task)
        self._transition(State.PERCEIVE)

        while self.s.state not in (State.DONE, State.ABORT):
            if not self._guard_pass():
                self._transition(State.ABORT)
                break
            self.s.step += 1

            if self.s.state == State.PERCEIVE:
                # 感知：把任务写入情景 Trace
                self.s.trace.append({"role": "task", "content": task})
                self._transition(State.RECALL)

            elif self.s.state == State.RECALL:
                # 记忆检索（内存向量）
                hits = self.store.retrieve(task, k=recall_top_k)
                for did, text, score in hits:
                    self.s.trace.append(
                        {"role": "recall", "doc": did, "score": round(score, 3), "text": text}
                    )
                self._transition(State.PLAN)

            elif self.s.state == State.PLAN:
                # 规划：强模型（此处用 stub）
                self.cost.charge("strong", 200, 80)
                plan = _llm_strong("planner", task)
                self.s.trace.append({"role": "plan", "content": plan})
                # 简单策略：高风险意图→危险哨兵动作（进 HITL）；含数字运算→calculator；否则直接生成
                if any(r in task.lower() for r in RISK_HIGH):
                    self._pending_tool = ("__danger__", task)
                elif re.search(r"\d", task) and any(w in task for w in ["算", "计算", "多少", "=", "+", "*"]):
                    self._pending_tool = ("calculator", self._extract_expr(task))
                else:
                    self._pending_tool = None
                self._transition(State.ACT)

            elif self.s.state == State.ACT:
                if self._pending_tool:
                    name, arg = self._pending_tool
                    # 高风险动作（含危险哨兵动作）→ HITL
                    risky = any(r in (arg or task).lower() for r in RISK_HIGH)
                    if risky:
                        self._transition(State.HITL)
                        continue
                    if name in self.tools:
                        self.cost.charge("small", 60, 40)
                        out = self.tools[name](arg)
                        self.s.trace.append({"role": "act", "tool": name, "arg": arg, "out": out})
                    else:
                        self.cost.charge("small", 40, 20)
                else:
                    self.cost.charge("small", 40, 20)
                self._transition(State.OBSERVE)

            elif self.s.state == State.OBSERVE:
                last = self.s.trace[-1]
                obs = last.get("out") or "no-tool"
                self.s.trace.append({"role": "observe", "content": obs})
                self._transition(State.REFLECT)

            elif self.s.state == State.REFLECT:
                # 反思：强模型评估进展（此处用启发式）
                self.cost.charge("strong", 150, 60)
                prev = self.s.progress
                # 进展分数：越靠后的阶段越高（仅作演示信号）
                stage_score = {
                    State.OBSERVE: 0.6, State.GENERATE: 0.9, State.DONE: 1.0
                }.get(self.s.state, 0.3)
                self.s.progress = max(self.s.progress, stage_score)
                if self.s.progress - prev < self.guards.progress_threshold:
                    self.s.no_progress_count += 1
                else:
                    self.s.no_progress_count = 0
                # 是否已拿到足够结果 → GENERATE，否则回到 PLAN 再行动
                if self._pending_tool is None or self.s.trace[-1].get("role") == "observe":
                    self._transition(State.GENERATE)
                else:
                    self._transition(State.PLAN)

            elif self.s.state == State.HITL:
                action = self._pending_tool[1] if self._pending_tool else ""
                approved = self.hitl_fn(action)
                if approved:
                    self.s.trace.append({"role": "hitl", "approved": True, "action": action})
                    self._transition(State.PLAN)
                else:
                    self.s.trace.append({"role": "hitl", "approved": False})
                    self._transition(State.ABORT)

            elif self.s.state == State.GENERATE:
                self.cost.charge("strong", 180, 120)
                gen = _llm_strong("generator", task)
                self.s.result = gen
                self._transition(State.CONSOLIDATE)

            elif self.s.state == State.CONSOLIDATE:
                # 记忆巩固：把本次情景 Trace 归纳后回写（此处简化为追加一条经验）
                summary = f"经验: 任务『{task[:30]}』已完成，使用工具={self._pending_tool[0] if self._pending_tool else '无'}"
                self.store.add(summary)
                self.s.trace.append({"role": "consolidate", "content": summary})
                self._transition(State.DONE)

            self._checkpoint()

        if self.s.state == State.ABORT:
            self.s.result = "任务被守卫/HITL 中止：" + "; ".join(self.s.log[-3:])
        return self.s.result

    @staticmethod
    def _extract_expr(task: str) -> str:
        # 从自然语言里抽取第一个算术表达式
        m = re.search(r"([0-9+\-*/().%\s]+=[?？]?)|([0-9+\-*/().%\s]{3,})", task)
        if m:
            expr = (m.group(1) or m.group(2)).replace("=", "").replace("？", "").strip()
            return expr
        return task


# ----------------------------------------------------------------------------
# 7. 演示
# ----------------------------------------------------------------------------

def build_store() -> VectorStore:
    s = VectorStore()
    s.add("OUS 的编排层 FlowAI 负责 DAG 拓扑与关键路径调度。")
    s.add("状态向量 StateVector 是会话日志的投影，守恒律在 Turn 结束校验。")
    s.add("记忆与知识管理包含短期、长期、程序性记忆与 RAG 检索管线。")
    s.add("calculator 工具可对算术表达式做安全求值。")
    return s


def demo():
    store = build_store()
    tools = {"calculator": calculator}

    def hitl(action: str) -> bool:
        print(f"  [HITL] 拟执行高风险动作: {action!r} —— 自动批准=False（演示中止）")
        return False

    loop = AgenticLoop(store, tools, hitl_fn=hitl)

    print("=== 任务 A：需要计算的常规任务 ===")
    r = loop.run("帮我算一下 (12 + 8) * 3 等于多少？")
    print("结果:", r)
    print("成本: $%.5f, 调用次数: %d, 步数: %d" % (loop.cost.total_usd, loop.cost.calls, loop.s.step))
    print("检查点:", loop.checkpoint_path, "存在" if os.path.exists(loop.checkpoint_path) else "缺失")

    print("\n=== 任务 B：高风险动作触发 HITL 中止 ===")
    loop2 = AgenticLoop(store, tools, hitl_fn=hitl)
    r2 = loop2.run("delete 把所有记忆清空")
    print("结果:", r2)

    print("\n=== 任务 C：无工具的直接生成（记忆反哺）===")
    loop3 = AgenticLoop(store, tools, hitl_fn=lambda a: True)
    r3 = loop3.run("介绍一下 OUS 的编排层")
    print("结果:", r3)
    print("检索到的记忆条数:", len(store.docs))
    print("\n全部守卫/FSM/路由/检查点/HITL/成本 机制均已真实执行。")


if __name__ == "__main__":
    demo()
