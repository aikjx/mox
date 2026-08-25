//! 业务全景目录 CLI：把系统所有业务建模为流程图 + 六维关系网，并演示"使用中不断优化"。
//!
//! DIP 版：本文件内不再直接引用 mox_expert 的 GovernanceReport/algo/gate 等 concrete 字段。
//! 统一通过 business_catalog 对外的 ConsultReport（投影）类型展示结果。
//!
//! 用法：
//!   cargo run -p business-catalog --bin catalog
//!   cargo run -p business-catalog --bin catalog -- --simulate    # 模拟多轮使用后的权重衰减/复用

use business_catalog::{all_businesses, build_topology};
use mox_expert::types::ConsultReport;

fn summarize(rep: &ConsultReport) -> (String, bool, f64) {
    // 从 steps/score/vetoed 提取摘要（旧版 GovernanceReport 的 algo/gate/optimization 已投影到 steps 文本）
    let summary = rep.reason.clone().unwrap_or_else(|| {
        if !rep.steps.is_empty() {
            rep.steps.join(" | ")
        } else if rep.vetoed {
            "治理闸门驳回".into()
        } else {
            "璇玑已优化".into()
        }
    });
    (summary, rep.vetoed, rep.score)
}

fn main() {
    let simulate = std::env::args().any(|a| a == "--simulate");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   璇玑 · 业务全景目录  (流程图 + 六维关系网 + 使用中优化)      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ── 1. 把每个业务建模成流程图，交给璇玑优化（DIP：通过 ExpertConsultant trait，不再暴露内部字段） ──
    println!("【一】用璇玑优化全部业务（流程图 IR → 并行/冲突/验证/治理）");
    let biz = all_businesses();
    let mut topo = build_topology();
    let mut report_lines: Vec<String> = Vec::new();
    for b in &biz {
        let flow = (b.build)();
        let nodes_count = flow.nodes.len();
        let rep: ConsultReport = b.optimize();
        let (summary, vetoed, score) = summarize(&rep);

        if vetoed {
            report_lines.push(format!(
                "       └ ⛨ 否决（score={:.2}）: {}",
                score, summary
            ));
        }
        topo.ingest_flow(&flow); // 把该业务汇入共享六维关系网
        report_lines.push(format!(
            "  {:<10} 节点{:>3}  健康分{:>5.2}  步骤数{:>3}  闸门:{}  算法否决:{}",
            b.name,
            nodes_count,
            score,
            rep.steps.len(),
            if vetoed { "Blocked" } else { "Approved" },
            vetoed,
        ));
    }
    for l in &report_lines {
        println!("{}", l);
    }

    // ── 2. 六维关系网统计 ──
    println!("\n【二】六维关系网（跨业务共享知识网）");
    let mut by_kind: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &topo.entities {
        *by_kind.entry(format!("{:?}", e.kind)).or_insert(0) += 1;
    }
    println!(
        "  实体 {} 个 / 关系 {} 条  维度分布: {}",
        topo.entities.len(),
        topo.relations.len(),
        by_kind
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join("  ")
    );

    // ── 3. 跨业务复用最短路径（命中历史 Skill → 跳过完整 ReAct）──
    println!("\n【三】指令路由：复用最短路径（命中 Skill 即跳过完整推理）");
    let queries = [
        ("政务 PII 脱敏后入库", "skill:desensitize"),
        ("客服意图分类路由", "skill:intent-route"),
        ("ETL 字段映射抽取", "skill:etl-map"),
        ("财务对账数据库拉取", "skill:db-pull"),
    ];
    for (q, expect_skill) in queries {
        let plan = topo.route(q, 0.4);
        let hit = plan
            .entry
            .as_ref()
            .map(|e| e.entity_id.clone())
            .unwrap_or_default();
        println!(
            "  指令「{}」 → 入口 {} | fast_path={} | 路径长 {}",
            q,
            hit,
            plan.fast_path,
            plan.path.len()
        );
        assert_eq!(hit, expect_skill, "路由应命中 {}", expect_skill);
    }

    // ── 4. 级联影响分析（改一节点，全链路同步）──
    println!("\n【四】级联影响：修改脱敏节点 → 哪些实体需同步更新");
    let impact = topo.impact_of("flow:gov-pii:guard");
    println!("  改动 flow:gov-pii:guard 影响 {} 个实体：", impact.total);
    for (k, v) in &impact.affected {
        println!("    {}: {}", k, v.join(", "));
    }

    // ── 5. 使用中不断学习（权重衰减 + 高频提权）──
    if simulate {
        println!("\n【五】模拟 100 轮使用后：记录命中 + 衰减归档");
        for _ in 0..100 {
            topo.record_hit("skill:desensitize");
            topo.record_hit("skill:intent-route");
        }
        topo.record_hit("mem:kb_vec");
        let archived = topo.decay(0.95, 0.3);
        println!(
            "  高频 Skill 权重提升，低频实体归档 {} 个；活跃实体 {} 个",
            archived,
            topo.active_count()
        );
        // 衰减后复用路径依然可达（高频 Skill 被保留）
        let plan = topo.route("客服意图分类路由", 0.4);
        println!(
            "  衰减后路由「客服意图分类路由」→ fast_path={}（高频 Skill 仍存活）",
            plan.fast_path
        );
    } else {
        println!("\n【五】提示：加 --simulate 可模拟多轮使用后的权重衰减与复用学习");
    }

    println!("\n✅ 业务全景目录分析完成：所有业务统一为流程图 + 六维关系网，由璇玑持续优化。");
}
