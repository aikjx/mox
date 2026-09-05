// =============================================================================
// 缓存统计指标（CacheStats）
// =============================================================================

use serde::{Deserialize, Serialize};

/// 缓存运行时统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// 缓存名称
    pub name: String,
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 淘汰次数
    pub evictions: u64,
    /// 总操作数
    pub total_ops: u64,
    /// 总延迟（纳秒）
    pub total_latency_ns: u64,
    /// 当前条目数
    pub entry_count: u64,
}

impl CacheStats {
    /// 命中率（0.0 ~ 1.0）
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 平均延迟（毫秒）
    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_ops == 0 {
            0.0
        } else {
            self.total_latency_ns as f64 / self.total_ops as f64 / 1_000_000.0
        }
    }

    /// 格式化为 Prometheus 指标文本
    pub fn to_prometheus(&self) -> String {
        format!(
            "# HELP mox_cache_hits Total cache hits\n\
             # TYPE mox_cache_hits counter\n\
             mox_cache_hits{{name=\"{name}\"}} {hits}\n\
             # HELP mox_cache_misses Total cache misses\n\
             # TYPE mox_cache_misses counter\n\
             mox_cache_misses{{name=\"{name}\"}} {misses}\n\
             # HELP mox_cache_evictions Total cache evictions\n\
             # TYPE mox_cache_evictions counter\n\
             mox_cache_evictions{{name=\"{name}\"}} {evictions}\n\
             # HELP mox_cache_entries Current cache entry count\n\
             # TYPE mox_cache_entries gauge\n\
             mox_cache_entries{{name=\"{name}\"}} {entries}\n\
             # HELP mox_cache_hit_rate Cache hit rate (0-1)\n\
             # TYPE mox_cache_hit_rate gauge\n\
             mox_cache_hit_rate{{name=\"{name}\"}} {rate:.4}\n",
            name = self.name,
            hits = self.hits,
            misses = self.misses,
            evictions = self.evictions,
            entries = self.entry_count,
            rate = self.hit_rate(),
        )
    }
}

impl std::fmt::Display for CacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] hits={} misses={} hit_rate={:.2}% evictions={} entries={} avg_latency={:.3}ms",
            self.name,
            self.hits,
            self.misses,
            self.hit_rate() * 100.0,
            self.evictions,
            self.entry_count,
            self.avg_latency_ms(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_rate() {
        let stats = CacheStats { name: "test".into(), hits: 80, misses: 20, ..Default::default() };
        assert!((stats.hit_rate() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_zero_hit_rate() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    #[test]
    fn test_prometheus_output() {
        let stats = CacheStats { name: "test".into(), hits: 10, misses: 5, entry_count: 3, ..Default::default() };
        let output = stats.to_prometheus();
        assert!(output.contains("mox_cache_hits{name=\"test\"} 10"));
        assert!(output.contains("mox_cache_entries{name=\"test\"} 3"));
    }
}
