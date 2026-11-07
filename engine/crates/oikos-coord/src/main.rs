//! `oikos-coord`: Coordinator v0 (docs/11 §11.3, ADR-0097).
//!
//!   oikos-coord serve --dir DIR [--addr 127.0.0.1:8650] [--client DIR]
//!   oikos-coord work --coordinator URL --host NAME [--oikos PATH] [--cache DIR]
//!                    [--scratch DIR] [--n 1] [--until-idle] [--poll SECS]

use oikos_coord::Policy;
use oikos_coord::server::Server;
use oikos_coord::worker::{Cli, Worker};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("oikos-coord: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The value after `--name`, if given.
fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("serve") => {
            let dir = PathBuf::from(opt(args, "--dir").ok_or("serve: --dir")?);
            let addr = opt(args, "--addr").unwrap_or("127.0.0.1:8650");
            let mut server = Server::open(&dir, Policy::default())?;
            if let Some(client) = opt(args, "--client") {
                server = server.with_client(PathBuf::from(client));
            }
            let http = tiny_http::Server::http(addr).map_err(|e| format!("{addr}: {e}"))?;
            let bound = http.server_addr().to_ip().ok_or("serve: not an IP address")?;
            eprintln!("oikos-coord: serving {} on http://{bound}", dir.display());
            server.serve(&http);
            Ok(())
        }
        Some("work") => {
            let base = opt(args, "--coordinator").ok_or("work: --coordinator")?.trim_end_matches('/').to_string();
            let host = opt(args, "--host").ok_or("work: --host")?.to_string();
            // Runs happen in their own directories: make every path given absolute.
            let absolute = |p: &str| std::path::absolute(p).map_err(|e| format!("{p}: {e}"));
            let oikos = match opt(args, "--oikos").unwrap_or("oikos") {
                p if p.contains(std::path::MAIN_SEPARATOR) => absolute(p)?,
                p => PathBuf::from(p), // found on PATH
            };
            let home = std::env::temp_dir().join(format!("oikos-coord-{host}"));
            let cache = opt(args, "--cache").map(absolute).transpose()?.unwrap_or_else(|| home.join("packs"));
            let scratch = opt(args, "--scratch").map(absolute).transpose()?.unwrap_or_else(|| home.join("runs"));
            let n: usize = opt(args, "--n").unwrap_or("1").parse().map_err(|_| "--n: a number")?;
            let poll: u64 = opt(args, "--poll").unwrap_or("30").parse().map_err(|_| "--poll: seconds")?;
            let runner = Cli(oikos);
            let worker = Worker { base, host, cache, runner: &runner };
            loop {
                if worker.round(n, &scratch)? == 0 {
                    if args.iter().any(|a| a == "--until-idle") {
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_secs(poll));
                }
            }
        }
        _ => Err("usage: oikos-coord serve --dir DIR [--addr HOST:PORT] [--client DIR] | oikos-coord work --coordinator URL --host NAME [--oikos PATH] [--n N] [--until-idle]".into()),
    }
}
