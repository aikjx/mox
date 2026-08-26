//! Mox v2.0 AIS-grade fusion single-binary entry point.
//!
//! Parses argv via `clap` derive. When invoked as `mox-server server
//! --single-node` the binary starts a tokio runtime and binds the single-node
//! HTTP server (S3 + Graph + Metrics + Audit endpoints). All other
//! subcommands execute against the in-memory [`CliState`] and print the JSON
//! summary produced by [`mox_platform_gateway_svc::cli_run`].

use clap::Parser;
use std::process::ExitCode;
use std::sync::Arc;

use parking_lot::Mutex;
use mox_platform_gateway_svc::{cli_run, Cli, CliState, Command, ServerArgs, ServerState, serve_forever};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match &cli.command {
        Command::Server(args) if args.single_node => run_server_forever(args.clone()),
        _other => {
            let state = CliState::new();
            match cli_run(&cli, &state) {
                Ok(v) => {
                    println!("{}", serde_json::to_string_pretty(&v).expect("json format"));
                    ExitCode::from(0)
                }
                Err(e) => {
                    eprintln!("mox-server: error: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

/// Build tokio current-thread runtime, construct shared [`ServerState`] and
/// run [`serve_forever`] until Ctrl-C or binding error.
fn run_server_forever(args: ServerArgs) -> ExitCode {
    let rt = match tokio::mox_platform_orchestrator_svc::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mox-server: tokio runtime build error: {e}");
            return ExitCode::from(1);
        }
    };
    let state: Arc<Mutex<ServerState>> = Arc::new(Mutex::new(ServerState::new()));
    let public = args.public_port;
    let ctrl = args.ctrl_port;
    let data = args.data_port;
    let ctrl_bind = format!("{}:{}", args.bind_addr, args.ctrl_port);
    let data_bind = format!("{}:{}", args.bind_addr, args.data_port);
    eprintln!(
        "[mox-server] 🚀 entering single-node mode: public={public} ctrl={ctrl} data={data}"
    );
    eprintln!(
        "[mox-server] 🔌 ctrl & data endpoints piggyback on public listener in this build. \
         Reserved bind addresses: ctrl={ctrl_bind} data={data_bind}"
    );
    rt.block_on(async move {
        match serve_forever(args, state).await {
            Ok(()) => ExitCode::from(0),
            Err(e) => {
                eprintln!("mox-server: fatal: {e}");
                ExitCode::from(1)
            }
        }
    })
}
