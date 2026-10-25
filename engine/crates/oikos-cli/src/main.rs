//! `oikos` — run work units from the command line (also the headless volunteer
//! client's engine entry point, and runnable under wasmtime as wasm32-wasip1).
//!
//! Usage:
//!   oikos run <workunit.json> [-o <result.json>]
//!   oikos hash <checkpoint>
//!   oikos bench [--base-cell N] [--res N] [--bands N] [--years N] [--parts N]

use oikos_sim::{Params, WorkUnit, World, WorldConfig, checkpoint, run_work_unit};
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
        Some("bench") => bench(&args[1..]),
        _ => Err("usage: oikos run <workunit.json> [-o result.json] | oikos hash <checkpoint>".into()),
    }
}

/// Throughput benchmark on a world filled to carrying capacity. Prints JSON, including
/// the final state hash, which must be identical for every `--parts` and every host.
fn bench(args: &[String]) -> Result<(), String> {
    let opt = |name: &str, default: u64| -> Result<u64, String> {
        match args.iter().position(|a| a == name) {
            Some(i) => args
                .get(i + 1)
                .ok_or(format!("{name}: missing value"))?
                .parse()
                .map_err(|e| format!("{name}: {e}")),
            None => Ok(default),
        }
    };
    let config = WorldConfig { base_cell: opt("--base-cell", 22)? as u8, resolution: opt("--res", 6)? as u8 };
    let bands = opt("--bands", 4)? as u32;
    let years = opt("--years", 20)?;
    let parts = opt("--parts", 1)? as usize;

    let t0 = std::time::Instant::now();
    let mut world = World::filled(config, 1, Params::default(), bands)?;
    let setup = t0.elapsed().as_secs_f64();
    let agents_start = world.agent_count() as u64;

    let mut agent_steps = 0u64;
    let t1 = std::time::Instant::now();
    for _ in 0..years {
        agent_steps += world.agent_count() as u64;
        world.step(parts);
    }
    let secs = t1.elapsed().as_secs_f64();
    println!(
        "{{\"cells\": {}, \"partitions\": {parts}, \"steps\": {years}, \"agents_start\": {agents_start}, \
         \"agents_end\": {}, \"setup_s\": {setup:.3}, \"run_s\": {secs:.3}, \
         \"agent_steps_per_s\": {:.0}, \"ns_per_agent_step\": {:.2}, \"state_hash\": \"{}\"}}",
        world.grid.len(),
        world.agent_count(),
        agent_steps as f64 / secs,
        secs * 1e9 / agent_steps as f64,
        checkpoint::state_hash(&world),
    );
    Ok(())
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{path}: {e}"))
}
