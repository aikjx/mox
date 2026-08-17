//! 5d 收益可信：speedup≥1 且并行不慢于串行

use crate::verify::Check;
use flow_ai::pipeline::OptimizationReport;

/// 5d 收益可信：speedup≥1 且并行不慢于串行
pub fn credible_gains_invariant(opt: &OptimizationReport) -> Check {
    let g = &opt.gains;
    if g.speedup < 1.0 {
        return Check {
            name: "gains".into(),
            passed: false,
            blocking: false,
            detail: format!("speedup={:.2} < 1.0，收益虚假", g.speedup),
        };
    }
    // 并行调度耗时不应超过串行（允许 5% 调度误差）
    let eps = (g.sequential_ms as f64 * 0.05).max(1.0);
    if g.scheduled_ms as f64 > g.sequential_ms as f64 + eps {
        return Check {
            name: "gains".into(),
            passed: false,
            blocking: false,
            detail: format!(
                "scheduled_ms={} > sequential_ms={}（+eps {}），并行反而更慢",
                g.scheduled_ms, g.sequential_ms, eps as u64
            ),
        };
    }
    Check {
        name: "gains".into(),
        passed: true,
        blocking: false,
        detail: format!(
            "speedup={:.2}×，scheduled {}ms ≤ sequential {}ms",
            g.speedup, g.scheduled_ms, g.sequential_ms
        ),
    }
}
