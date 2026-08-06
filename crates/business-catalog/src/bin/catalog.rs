//! 业务全景目录 CLI：把系统所有业务建模为流程图 + 六维关系网，并演示"使用中不断优化"。
//!
//! 用法：
//!   cargo run -p business-catalog --bin catalog
//!   cargo run -p business-catalog --bin catalog -- --simulate    # 模拟多轮使用后的权重衰减/复用

use business_catalog::{all_businesses, build_topology};

fn main() {
    let simulate = std::env::args().any(|a| a == "--simulate");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   璇玑 · 业务全景目录  (流程图 + 六维关系网 + 使用中优化)      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ── 1. 把每个业务建模成流程图，交给开发专家联盟优化 ──
    println!("【一】用开发专家联盟优化全部业务（流程图 IR → 并行/冲突/验证/治理）");
    let biz = all_businesses();
    let mut topo = build_topology();
    let mut report_lines: Vec<String> = Vec::new();
    for b in &biz {
        let rep = b.optimize();
        let opt = &rep.optimization;
        let g = &rep.gate;
        let sp = opt.gains.speedup;
        if rep.algo.vetoed {
            // 真实否定：算法验证网关抓到语义破坏/阻断冲突，治理强制 BLOCK
            report_lines.push(format!("       └ ⛨ 算法否决: {}", rep.algo.summary));
            for cf in &rep.optimization.conflicts.conflicts {
                if cf.severity == flow_ai::model::Severity::Blocking {
                    report_lines.push(format!(
                        "          └ 阻断冲突: {:?} nodes={:?} {}",
                        cf.kind, cf.nodes, cf.message
                    ));
                }
            }
            for c in &rep.algo.checks {
                if !c.passed {
                    report_lines.push(format!(
                        "          └ 检查 {} 未过(blocking={}): {}",
                        c.name, c.blocking, c.detail
                    ));
                }
            }
        }
        topo.ingest_flow(&(b.build)()); // 把该业务汇入共享六维关系网
        report_lines.push(format!(
            "  {:<10} 节点{:>3}  加速{:>5.2}×  省时{:>5.1}%  算力压{:>5.1}%  闸门:{}  算法否决:{}",
            b.name,
            opt.optimized_graph.nodes.len(),
            sp,
            opt.gains.time_saved_pct,
            opt.gains.compute_saved_pct,
            if g.approved { "Approved" } else { "Blocked" },
            rep.algo.vetoed,
        ));
        if !g.approved {
            report_lines.push(format!("       └ 原因: {}", g.reason));
        }
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
    println!(
        "  改动 flow:gov-pii:guard 影响 {} 个实体：",
        impact.total
    );
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

    println!("\n✅ 业务全景目录分析完成：所有业务统一为流程图 + 六维关系网，由开发专家联盟持续优化。");
}
