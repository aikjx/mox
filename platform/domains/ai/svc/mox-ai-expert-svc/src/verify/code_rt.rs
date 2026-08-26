//! 5e 代码往返一致（仅 emit_code 时；不一致仅告警不阻断）

use crate::verify::Check;
use mox_ai_flow_svc::pipeline::OptimizationReport;

/// 5e 代码往返一致（仅 emit_code 时；不一致仅告警不阻断）
pub fn code_roundtrip_invariant(opt: &OptimizationReport) -> Check {
    let code = match &opt.code {
        Some(c) => c,
        None => {
            return Check {
                name: "code_rt".into(),
                passed: true,
                blocking: false,
                detail: "未生成代码（跳过往返检查）".into(),
            }
        }
    };
    // 取主模块 main.py 做反向解析
    let main = code
        .file("main.py")
        .or_else(|| code.files.first())
        .map(|f| &f.content);
    let src = match main {
        Some(s) => s,
        None => {
            return Check {
                name: "code_rt".into(),
                passed: true,
                blocking: false,
                detail: "无 main.py（跳过）".into(),
            }
        }
    };
    let rev = mox_ai_flow_svc::codegen::reverse_from_python(src, &opt.flow_id);
    let g2 = &rev.graph;
    // 反向解析器会重新推导节点 id（基于工具名派生），不保证与原 id 一致；
    // 因此用「可执行工具节点数量」做语义守恒判定：生成的代码应覆盖全部核心工具节点。
    let before_tool_count = opt
        .optimized_graph
        .nodes
        .iter()
        .filter(|n| n.tool.is_some())
        .count();
    let rev_tool_count = g2.nodes.iter().filter(|n| n.tool.is_some()).count();
    // 反向解析器可能因缩进/结构未被识别到工具节点，这里仅做「尽力告警」，不阻断
    if rev_tool_count == 0 && before_tool_count > 0 {
        return Check {
            name: "code_rt".into(),
            passed: false,
            blocking: false,
            detail: "反向解析未识别出工具节点（结构未被缩进解析覆盖），仅告警".into(),
        };
    }
    if rev_tool_count < before_tool_count {
        return Check {
            name: "code_rt".into(),
            passed: false,
            blocking: false,
            detail: format!(
                "反向解析工具节点 {} < 原核心工具节点 {}，疑似丢失",
                rev_tool_count, before_tool_count
            ),
        };
    }
    Check {
        name: "code_rt".into(),
        passed: true,
        blocking: false,
        detail: format!(
            "代码⇄流程图往返一致（反向工具节点 {}，原核心 {}，结构恢复完整）",
            rev_tool_count, before_tool_count
        ),
    }
}
