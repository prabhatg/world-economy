//! `oikos-coord work`: a host that pulls work units, runs them and returns the results.
//!
//! For each lease: fetch the unit's forcing packs into a cache (by id, checked against
//! their content hash), point the unit at them, run it in a fresh directory, upload that
//! directory as a tar archive and report the engine's result, or report why the run
//! failed. A unit must keep its outputs inside the run directory ([`crate::confined`]):
//! the worker checks that itself rather than trust its coordinator. The engine runs through a
//! [`Runner`]: the `oikos` CLI as a child process (docs/11 §11.4: native clients keep the
//! engine in its own process), or in-process for tests.

use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Runs a work unit in `dir` (its relative output paths land there); returns the
/// engine's `WorkResult` as JSON.
pub trait Runner {
    fn run(&self, unit: &Value, dir: &Path) -> Result<Value, String>;
}

/// The `oikos` CLI at `path`.
pub struct Cli(pub PathBuf);

impl Runner for Cli {
    fn run(&self, unit: &Value, dir: &Path) -> Result<Value, String> {
        let wu = dir.join("unit.json");
        std::fs::write(&wu, serde_json::to_vec_pretty(unit).expect("serializes"))
            .map_err(|e| e.to_string())?;
        let out = std::process::Command::new(&self.0)
            .args(["run", "unit.json", "-o", "result.json"])
            .current_dir(dir)
            .output()
            .map_err(|e| format!("{}: {e}", self.0.display()))?;
        if !out.status.success() {
            return Err(format!("oikos run failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        let bytes = std::fs::read(dir.join("result.json")).map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("result.json: {e}"))
    }
}

pub struct Worker<'a> {
    pub base: String,
    pub host: String,
    pub cache: PathBuf,
    pub runner: &'a dyn Runner,
}

fn http_err(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, r) => format!("HTTP {code}: {}", r.into_string().unwrap_or_default()),
        other => other.to_string(),
    }
}

impl Worker<'_> {
    fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let r = ureq::post(&format!("{}{path}", self.base))
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(http_err)?;
        serde_json::from_str(&r.into_string().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn get_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let r = ureq::get(&format!("{}{path}", self.base)).call().map_err(http_err)?;
        let mut bytes = Vec::new();
        r.into_reader().read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    /// The pack's path in the cache, fetched and checked if it is not there yet.
    fn pack(&self, id: &str) -> Result<PathBuf, String> {
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("pack id {id:?} is not a content hash"));
        }
        let path = self.cache.join(format!("{id}.oikospack"));
        if !path.exists() {
            let bytes = self.get_bytes(&format!("/packs/{id}"))?;
            if blake3::hash(&bytes).to_hex().as_str() != id {
                return Err(format!("pack {id}: content does not match its id"));
            }
            let tmp = self.cache.join(format!("{id}.tmp"));
            std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        }
        Ok(path)
    }

    /// The unit ready to run in `dir`: confined, with its packs fetched.
    fn prepare(&self, mut unit: Value) -> Result<Value, String> {
        crate::confined(&unit)?;
        for input in crate::files_mut(&mut unit)? {
            let id = input["id"].as_str().ok_or("input id")?.to_string();
            input["path"] = json!(self.pack(&id)?.to_string_lossy());
        }
        Ok(unit)
    }

    /// Leases, runs and reports up to `n` units; returns how many it was given.
    pub fn round(&self, n: usize, scratch: &Path) -> Result<usize, String> {
        std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
        let leases = self.post("/lease", &json!({ "host": self.host, "n": n }))?;
        let leases = leases.as_array().ok_or("lease: expected a list")?.clone();
        for l in &leases {
            let lease = l["lease"].as_u64().ok_or("lease id")?;
            let dir = scratch.join(format!("lease-{lease}"));
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
            }
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            match self.prepare(l["unit"].clone()).and_then(|unit| self.runner.run(&unit, &dir)) {
                Ok(result) => {
                    let mut tar = tar::Builder::new(Vec::new());
                    tar.append_dir_all(".", &dir).map_err(|e| e.to_string())?;
                    let archive = tar.into_inner().map_err(|e| e.to_string())?;
                    ureq::put(&format!("{}/archives/{lease}?host={}", self.base, self.host))
                        .set("Content-Type", "application/x-tar")
                        .send_bytes(&archive)
                        .map_err(http_err)?;
                    self.post(&format!("/report/{lease}"), &json!({ "host": self.host, "result": result }))?;
                }
                Err(error) => {
                    eprintln!("oikos-coord: lease {lease} failed: {error}");
                    self.post(&format!("/fail/{lease}"), &json!({ "host": self.host, "error": error }))?;
                }
            }
            std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        Ok(leases.len())
    }
}
