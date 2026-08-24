//! cdc_100k_harness: generate 100k CDC events (70k Vertex + 30k Edge),
//! stream through FlinkCdcSource next_blocking(), then IdempotentWriter.
//!
//! Writes JSON report to: projects/t11-graph-artifacts/cdc_100k_report.json
//! Exit 0 iff { lost==0 && duplicates_in_upsert==0 && total_in==total_out==100000 }
//!
//! Run: cargo run -p xuanji-graph-streams --bin cdc_100k_harness

use std::sync::Arc;
use std::time::{Duration, Instant};
use xuanji_graph_streams::{FlinkCdcSource, IdempotentWriter};
use xuanji_graph_storage::cdc_source::{CdcEventType, CdcSource};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let src = Arc::new(CdcSource::new("graph"));
    let fs = FlinkCdcSource::new(src.clone());
    let writer = IdempotentWriter::new();

    // Step 1: emit 70k vertex + 30k edge in interleaved fashion (so raft_index is monotonic mix)
    const V: u64 = 70_000;
    const E: u64 = 30_000;
    const N: u64 = V + E;
    let start = Instant::now();
    let mut v_done = 0u64;
    let mut e_done = 0u64;
    for i in 1..=N {
        let pick_vertex = ((i * 7) % 10) < 7; // ~70% vertex
        if pick_vertex && v_done < V {
            let id = v_done + 1;
            src.emit(
                "graph",
                CdcEventType::VertexCreated,
                format!(
                    "{{\"id\":{id},\"label\":\"user_{id}\",\"type\":\"Person\",\"attr\":{{\"age\":{},\"city\":\"SH\"}}}}",
                    (id * 3) % 80 + 18
                ),
            );
            v_done += 1;
        } else if e_done < E {
            let id = e_done + 1;
            let s = (id * 3) % V + 1;
            let t = (id * 7) % V + 1;
            src.emit(
                "graph",
                CdcEventType::EdgeCreated,
                format!("{{\"src\":{s},\"tgt\":{t},\"label\":\"knows\",\"weight\":{id}}}"),
            );
            e_done += 1;
        } else {
            // Fallback vertex
            src.emit(
                "graph",
                CdcEventType::VertexCreated,
                format!("{{\"id\":{i},\"label\":\"extra\"}}"),
            );
        }
        // Flush every 256 events to force pipeline behavior
        if i % 256 == 0 {
            let _ = src.flush();
        }
    }
    // Ensure final flush
    let _ = src.flush();
    let emit_ms = start.elapsed().as_millis();

    // Step 2: consume via next_blocking, 5ms timeout, until 1s idle
    let stream_t0 = Instant::now();
    let mut idle_count = 0u32;
    loop {
        match fs.next_blocking(Duration::from_millis(5)) {
            Some(ev) => {
                writer.upsert(ev);
                idle_count = 0;
            }
            None => {
                idle_count += 1;
                if idle_count >= 200 {
                    // 200 * 5ms = 1s idle
                    break;
                }
            }
        }
    }
    let stream_ms = stream_t0.elapsed().as_millis();

    // Step 3: produce report
    let report = writer.report(N, t0);
    let json = serde_json::json!({
        "harness": "cdc_100k",
        "generated_at_ms": t0.elapsed().as_millis(),
        "emit_duration_ms": emit_ms,
        "stream_duration_ms": stream_ms,
        "report": report,
        "targets": {
            "expected_total_in": 100_000,
            "expected_total_out": 100_000,
            "expected_lost": 0,
            "expected_duplicates_in_upsert": 0,
            "vertex_expected": 70_000,
            "edge_expected": 30_000,
        }
    });
    let out_dir = "projects/t11-graph-artifacts";
    std::fs::create_dir_all(out_dir).ok();
    let path = format!("{out_dir}/cdc_100k_report.json");
    std::fs::write(&path, serde_json::to_string_pretty(&json)?)?;
    println!("{}", serde_json::to_string_pretty(&json)?);

    // Quality gate
    let mut ok = true;
    if report.total_in != 100_000 {
        eprintln!("[FAIL] total_in={} != 100000", report.total_in);
        ok = false;
    }
    if report.total_out != 100_000 {
        eprintln!("[FAIL] total_out={} != 100000", report.total_out);
        ok = false;
    }
    if report.lost != 0 {
        eprintln!("[FAIL] lost={} != 0", report.lost);
        ok = false;
    }
    if report.duplicates_in_upsert != 0 {
        eprintln!("[FAIL] duplicates={} != 0", report.duplicates_in_upsert);
        ok = false;
    }
    if !report.monotonic_raft {
        eprintln!("[FAIL] raft_index not monotonic");
        ok = false;
    }
    if ok {
        println!("[PASS] 100k CDC integrity: lost=0 duplicates=0 total=100000");
        Ok(())
    } else {
        std::process::exit(2);
    }
}
