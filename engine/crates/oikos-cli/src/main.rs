//! `oikos` — run work units from the command line (also the headless volunteer
//! client's engine entry point, and runnable under wasmtime as wasm32-wasip1).
//!
//! Usage:
//!   oikos run <workunit.json> [-o <result.json>]
//!   oikos hash <checkpoint>

use oikos_sim::{WorkUnit, checkpoint, run_work_unit};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("oikos: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("run") => {
            let path = args.get(1).ok_or("run: missing work unit path")?;
            let out = match args.get(2).map(String::as_str) {
                Some("-o") => Some(args.get(3).ok_or("-o: missing path")?.clone()),
                Some(other) => return Err(format!("unexpected argument {other}")),
                None => None,
            };
            let wu: WorkUnit = serde_json::from_slice(&read(path)?).map_err(|e| format!("{path}: {e}"))?;
            let result = run_work_unit(&wu, read, write)?;
            let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())? + "\n";
            match out {
                Some(p) => write(&p, json.as_bytes()),
                None => {
                    print!("{json}");
                    Ok(())
                }
            }
        }
        Some("hash") => {
            let path = args.get(1).ok_or("hash: missing checkpoint path")?;
            let world = checkpoint::decode(&read(path)?)?;
            println!("{}", checkpoint::state_hash(&world));
            Ok(())
        }
        _ => Err("usage: oikos run <workunit.json> [-o result.json] | oikos hash <checkpoint>".into()),
    }
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))
}
