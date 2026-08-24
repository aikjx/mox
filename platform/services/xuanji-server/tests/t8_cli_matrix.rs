//! Task 8 — CLI matrix integration tests.
//!
//! Exercices the `xuanji_server::cli_run` pure function with in-memory
//! `CliState`, validating JSON output shape for every subcommand:
//!   server, ec encode/decode/rebuild, mount attach/detach/list/enable/disable,
//!   legal-hold set/release/status, miji set/judge-read,
//!   bench ec/fshc/fusion, etl list-plugins/run/register.
//!
//! All tests stay in-process — no `Command::new("xuanji-server")` spawn,
//! no real TCP binding.

use serde_json::Value;
use std::sync::Arc;
use xuanji_server::cli::{
    BenchArgs, BenchEcArgs, BenchFshcArgs, BenchFusionArgs, BenchOp, Cli, CliState, Command,
    EcArgs, EcDecodeArgs, EcEncodeArgs, EcOp, EcRebuildArgs, EtlArgs, EtlOp, LegalHoldArgs,
    LegalHoldOp, MijiArgs, MijiLevelArg, MijiOp, MountArgs, MountOp, ServerArgs,
};
use xuanji_server::cli_run;

fn state() -> Arc<CliState> { CliState::new() }

fn run(cli: &Cli, s: &Arc<CliState>) -> Value {
    cli_run(cli, s).expect("cli_run must succeed")
}

// T8-01: server --single-node -> "single_node=true" and endpoints() uses TripleListener.
#[test]
fn t8_01_server_single_node_boolean() {
    let s = state();
    let cli = Cli {
        command: Command::Server(ServerArgs {
            single_node: true,
            public_port: 8080,
            ctrl_port: 9090,
            data_port: 9091,
            mountpaths: String::new(),
            bind_addr: "127.0.0.1".to_string(),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "server");
    assert_eq!(v["ok"], true);
    assert_eq!(v["single_node"], true);
    assert_eq!(v["public"], 8080);
    let eps = v["endpoints"].as_array().expect("endpoints array");
    assert_eq!(eps.len(), 3, "TripleListener must yield 3 endpoints");
    assert!(eps[0].as_str().unwrap().contains("8080"));
}

// T8-02: server --mountpaths /mnt/a,/mnt/b,/mnt/c -> mountpaths_count == 3
#[test]
fn t8_02_server_mountpaths_csv_count_3() {
    let s = state();
    let cli = Cli {
        command: Command::Server(ServerArgs {
            single_node: false,
            public_port: 8080,
            ctrl_port: 9080,
            data_port: 9081,
            mountpaths: "/mnt/a,/mnt/b,/mnt/c".to_string(),
            bind_addr: "127.0.0.1".to_string(),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "server");
    assert_eq!(v["mountpaths_count"], 3);
}

// T8-03: ec encode 4+2 --data deadbeef -> shard_count == 6, total_bytes == 4
#[test]
fn t8_03_ec_encode_4plus2_shardcount_6() {
    let s = state();
    let cli = Cli {
        command: Command::Ec(EcArgs {
            op: EcOp::Encode(EcEncodeArgs {
                profile: "4+2".to_string(),
                data: "deadbeef".to_string(),
                out: "/tmp/ec-out".to_string(),
            }),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "ec.encode");
    assert_eq!(v["ok"], true);
    assert_eq!(v["profile"], "4+2");
    assert_eq!(v["data_shards"], 4);
    assert_eq!(v["parity_shards"], 2);
    assert_eq!(v["shard_count"], 6);
    assert_eq!(v["total_bytes"], 4);
}

// T8-04: ec decode --manifest /no/such.json -> synthetic shard_count 6
#[test]
fn t8_04_ec_decode_missing_manifest_synthetic_count() {
    let s = state();
    let cli = Cli {
        command: Command::Ec(EcArgs {
            op: EcOp::Decode(EcDecodeArgs {
                manifest: "/tmp/xuanji-nosuch-manifest.json".to_string(),
            }),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "ec.decode");
    assert_eq!(v["shard_count"], 6);
    assert_eq!(v["data_shards"], 4);
    assert_eq!(v["parity_shards"], 2);
}

// T8-05: ec rebuild --faulty-list 0,3,5 -> faulty_count=3, expected_repaired_bytes=3*1048576
#[test]
fn t8_05_ec_rebuild_three_faulty_expected_bytes() {
    let s = state();
    let cli = Cli {
        command: Command::Ec(EcArgs {
            op: EcOp::Rebuild(EcRebuildArgs {
                faulty_list: "0,3,5".to_string(),
            }),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "ec.rebuild");
    assert_eq!(v["faulty_count"], 3);
    assert_eq!(v["expected_repaired_bytes"], 3 * 1_048_576);
}

// T8-06: mount attach /tmp/foo -> returns id; list count becomes 1
#[test]
fn t8_06_mount_attach_then_list_count_1() {
    let s = state();
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("mp");
    std::fs::create_dir_all(&p).unwrap();
    let attach_cli = Cli {
        command: Command::Mount(MountArgs {
            op: MountOp::Attach { path: p.clone() },
        }),
    };
    let a = run(&attach_cli, &s);
    assert_eq!(a["subcmd"], "mount.attach");
    assert_eq!(a["ok"], true);
    let id = a["id"].as_str().unwrap().to_string();
    assert_eq!(id.len(), 16, "mountpath id must be 16 char hex");

    let list_cli = Cli { command: Command::Mount(MountArgs { op: MountOp::List }) };
    let l = run(&list_cli, &s);
    assert_eq!(l["count"], 1);
    assert_eq!(l["items"].as_array().unwrap().len(), 1);

    // disable
    let disable_cli = Cli {
        command: Command::Mount(MountArgs { op: MountOp::Disable { id: id.clone() } }),
    };
    let d = run(&disable_cli, &s);
    assert_eq!(d["ok"], true);

    // enable
    let enable_cli = Cli {
        command: Command::Mount(MountArgs { op: MountOp::Enable { id: id.clone() } }),
    };
    let e = run(&enable_cli, &s);
    assert_eq!(e["ok"], true);

    // detach
    let detach_cli = Cli {
        command: Command::Mount(MountArgs { op: MountOp::Detach { id: id.clone() } }),
    };
    let dt = run(&detach_cli, &s);
    assert_eq!(dt["ok"], true);

    // list count should now be 0
    let l2 = run(&list_cli, &s);
    assert_eq!(l2["count"], 0);
}

// T8-07: miji judge-read: user=4(topsecret), obj=1(internal) -> allowed
//        user=1, obj=4 -> denied
#[test]
fn t8_07_miji_judge_read_both_sides() {
    let s = state();
    // Allowed: topsecret user reads internal doc.
    let allowed_cli = Cli {
        command: Command::Miji(MijiArgs {
            op: MijiOp::JudgeRead { user_level: 4, obj_level: 1 },
        }),
    };
    let a = run(&allowed_cli, &s);
    assert_eq!(a["subcmd"], "miji.judge-read");
    assert_eq!(a["allowed"], true);
    assert_eq!(a["verdict"], "allowed");

    // Denied: internal user reads topsecret doc.
    let denied_cli = Cli {
        command: Command::Miji(MijiArgs {
            op: MijiOp::JudgeRead { user_level: 1, obj_level: 4 },
        }),
    };
    let d = run(&denied_cli, &s);
    assert_eq!(d["allowed"], false);
    assert_eq!(d["verdict"], "denied");
    assert!(d["error"].is_string());
}

// T8-08: legal-hold set on URI, status shows active, release clears it.
#[test]
fn t8_08_legal_hold_set_status_release() {
    let s = state();
    let uri = "s3://bucket/obj.dat";
    let until = chrono::Utc::now().timestamp_millis() + 3600_000;
    let set_cli = Cli {
        command: Command::LegalHold(LegalHoldArgs {
            op: LegalHoldOp::Set { uri: uri.to_string(), until_ms: until },
        }),
    };
    let set = run(&set_cli, &s);
    assert_eq!(set["subcmd"], "legal-hold.set");
    assert_eq!(set["ok"], true);
    assert_eq!(set["was_new"], true);
    assert_eq!(set["active"], 1);

    let status_cli = Cli {
        command: Command::LegalHold(LegalHoldArgs {
            op: LegalHoldOp::Status { uri: uri.to_string() },
        }),
    };
    let st = run(&status_cli, &s);
    assert_eq!(st["active"], true);
    assert_eq!(st["hold"]["hold_until_ms"], until);

    let release_cli = Cli {
        command: Command::LegalHold(LegalHoldArgs {
            op: LegalHoldOp::Release { uri: uri.to_string() },
        }),
    };
    let rel = run(&release_cli, &s);
    assert_eq!(rel["released"], true);
    assert_eq!(rel["active"], 0);

    let st2 = run(&status_cli, &s);
    assert_eq!(st2["active"], false);
}

// T8-09: etl list-plugins -> md5, upper both present; etl run md5 -> 32 hex chars.
#[test]
fn t8_09_etl_list_and_run_md5() {
    let s = state();
    let list_cli = Cli { command: Command::Etl(EtlArgs { op: EtlOp::ListPlugins }) };
    let l = run(&list_cli, &s);
    assert_eq!(l["subcmd"], "etl.list-plugins");
    let ids = l["inline_get_ids"].as_array().unwrap();
    let ids: Vec<&str> = ids.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(ids.contains(&"md5"), "md5 must be listed, got {ids:?}");
    assert!(ids.contains(&"upper"), "upper must be listed, got {ids:?}");

    // etl run md5 data "abc" — md5("abc") is known = 900150983cd24fb0d6963f7d28e17f72
    let run_cli = Cli {
        command: Command::Etl(EtlArgs {
            op: EtlOp::Run { plugin: "md5".to_string(), data: "abc".to_string() },
        }),
    };
    let r = run(&run_cli, &s);
    assert_eq!(r["subcmd"], "etl.run");
    assert_eq!(r["output_len"], 16);
    assert_eq!(r["output"], "900150983cd24fb0d6963f7d28e17f72");
}

// T8-10: etl register inline-get custom noop -> ok=true; stub=true
#[test]
fn t8_10_etl_register_stub_custom() {
    let s = state();
    let reg_cli = Cli {
        command: Command::Etl(EtlArgs {
            op: EtlOp::Register {
                kind: xuanji_server::cli::PluginKindArg::InlineGet,
                id: "custom".to_string(),
                code: "noop".to_string(),
            },
        }),
    };
    let v = run(&reg_cli, &s);
    assert_eq!(v["subcmd"], "etl.register");
    assert_eq!(v["ok"], true);
    assert_eq!(v["id"], "custom");
    assert_eq!(v["stub"], true);
}

// T8-11: miji set --level topsecret on URI then inspect level_code=4.
#[test]
fn t8_11_miji_set_topsecret_level_code_4() {
    let s = state();
    let cli = Cli {
        command: Command::Miji(MijiArgs {
            op: MijiOp::Set {
                uri: "s3://b/secret.bin".to_string(),
                level: MijiLevelArg::Topsecret,
            },
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "miji.set");
    assert_eq!(v["ok"], true);
    assert_eq!(v["level"], "TopSecret");
    assert_eq!(v["level_code"], 4);
}

// T8-12: ec encode 12+4 profile -> shard_count 16, data_shards 12, parity 4
#[test]
fn t8_12_ec_encode_12plus4_profile_shape() {
    let s = state();
    let cli = Cli {
        command: Command::Ec(EcArgs {
            op: EcOp::Encode(EcEncodeArgs {
                profile: "12+4".to_string(),
                data: "aabbccdd00112233".to_string(),
                out: "/tmp/out12".to_string(),
            }),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["data_shards"], 12);
    assert_eq!(v["parity_shards"], 4);
    assert_eq!(v["shard_count"], 16);
    assert_eq!(v["total_bytes"], 8);
}

// T8-13: bench fusion n=100 -> total_edges >= 390 (obj+3tags=4/obj *100=400)
#[test]
fn t8_13_bench_fusion_n100_edges() {
    let s = state();
    let cli = Cli {
        command: Command::Bench(BenchArgs {
            op: BenchOp::Fusion(BenchFusionArgs { n_objects: 100 }),
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["subcmd"], "bench.fusion");
    assert_eq!(v["ok"], true);
    assert_eq!(v["n_objects"], 100);
    assert_eq!(v["events_emitted"], 100);
    let edges: u64 = v["total_edges"].as_u64().unwrap();
    assert!(edges >= 390, "fusion edges for 100 objs should be >= 390, got {edges}");
}

// T8-14: etl run upper "hElLo WoRlD" -> "HELLO WORLD"
#[test]
fn t8_14_etl_run_upper_transform() {
    let s = state();
    let cli = Cli {
        command: Command::Etl(EtlArgs {
            op: EtlOp::Run { plugin: "upper".to_string(), data: "hElLo WoRlD".to_string() },
        }),
    };
    let v = run(&cli, &s);
    assert_eq!(v["output"], "HELLO WORLD");
    assert_eq!(v["output_len"], 11);
}
