//! `oikos-coord serve`: the coordinator over HTTP and JSON, for one trusted network.
//!
//! v0 has no accounts or authentication: run it where only your own workers reach it.
//! The state is snapshot to `state.json` after every change (written to a temporary file
//! and renamed, so a crash leaves the last whole snapshot). Forcing packs an experiment
//! names are checked against their content hash and copied into `packs/`, then served
//! by id; workers' run directories come back as tar archives, kept per unit and host.
//!
//! Routes (bodies are JSON unless said):
//! - `POST /experiments/<name>` `[{"id", "unit"}]`: submit work units; their `inputs`
//!   paths must be readable here (or the pack already copied), and workers receive them
//!   by id only. Each unit must parse as a work unit and run inside its run directory.
//! - `GET /experiments/<name>`: counts and each unit's status.
//! - `POST /lease` `{"host", "n"}`: up to `n` leases, each `{"lease", "unit_id", "deadline", "unit"}`.
//! - `PUT /archives/<lease>?host=<host>` (tar bytes), then `POST /report/<lease>`
//!   `{"host", "result"}` with the engine's `WorkResult`; or `POST /fail/<lease>`
//!   `{"host", "error"}` when the run failed.
//! - `GET /units/<id>`: status and the accepted report; `GET /archives/<unit>/<host>.tar`.
//! - `GET /packs/<id>`: a pack's bytes. `GET /status`: counts.

use crate::{Coordinator, Policy, Status};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Server {
    dir: PathBuf,
    state: Mutex<Coordinator>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A name safe as one path component: letters, digits, `-`, `_` and `.` (not leading).
fn safe(name: &str) -> Result<&str, String> {
    let ok = !name.is_empty()
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.');
    if ok { Ok(name) } else { Err(format!("{name:?} is not a safe name")) }
}

/// A pack id: a BLAKE3 hash in lowercase hex.
fn pack_id(id: &str) -> Result<&str, String> {
    let ok = id.len() == 64 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if ok { Ok(id) } else { Err(format!("pack id {id:?} is not a content hash")) }
}

type Reply = (u16, Vec<u8>, &'static str);

fn json_reply(status: u16, v: Value) -> Reply {
    (status, serde_json::to_vec_pretty(&v).expect("serializes"), "application/json")
}

impl Server {
    /// Opens (or creates) the coordinator kept in `dir`.
    pub fn open(dir: &Path, policy: Policy) -> Result<Server, String> {
        for sub in ["archives", "packs"] {
            std::fs::create_dir_all(dir.join(sub)).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let state = match std::fs::read(dir.join("state.json")) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| format!("state.json: {e}"))?,
            Err(_) => Coordinator::new(policy),
        };
        Ok(Server { dir: dir.to_path_buf(), state: Mutex::new(state) })
    }

    fn save(&self, c: &Coordinator) -> Result<(), String> {
        let tmp = self.dir.join("state.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(c).expect("serializes")).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.dir.join("state.json")).map_err(|e| e.to_string())
    }

    fn pack_path(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self.dir.join("packs").join(format!("{}.oikospack", pack_id(id)?)))
    }

    /// Serves until the process ends.
    pub fn serve(&self, http: &tiny_http::Server) {
        for mut req in http.incoming_requests() {
            let mut body = Vec::new();
            let reply = match req.as_reader().read_to_end(&mut body) {
                Ok(_) => self.handle(req.method().as_str(), req.url(), &body),
                Err(e) => Err(e.to_string()),
            };
            let (status, bytes, kind) = reply.unwrap_or_else(|e| json_reply(400, json!({ "error": e })));
            let header = tiny_http::Header::from_bytes("Content-Type", kind).expect("valid header");
            let _ = req
                .respond(tiny_http::Response::from_data(bytes).with_status_code(status).with_header(header));
        }
    }

    /// One request: method, URL (path and query) and body.
    pub fn handle(&self, method: &str, url: &str, body: &[u8]) -> Result<Reply, String> {
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        let param = |k: &str| query.split('&').find_map(|kv| kv.strip_prefix(k)?.strip_prefix('='));
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let parse = || serde_json::from_slice::<Value>(body).map_err(|e| format!("body: {e}"));
        match (method, parts.as_slice()) {
            ("GET", ["status"]) => Ok(json_reply(200, json!(self.state.lock().unwrap().counts(None)))),
            ("POST", ["experiments", name]) => self.submit(safe(name)?, parse()?),
            ("GET", ["experiments", name]) => {
                let c = self.state.lock().unwrap();
                let units: BTreeMap<String, Status> = c
                    .units(name)
                    .into_iter()
                    .map(|id| (id.clone(), c.status(&id).expect("listed")))
                    .collect();
                Ok(json_reply(200, json!({ "counts": c.counts(Some(name)), "units": units })))
            }
            ("POST", ["lease"]) => {
                let v = parse()?;
                let host = safe(v["host"].as_str().ok_or("lease: host")?)?.to_string();
                let n = v["n"].as_u64().unwrap_or(1).min(64) as usize;
                let mut c = self.state.lock().unwrap();
                let leases = c.lease(&host, n, now());
                self.save(&c)?;
                let out: Vec<Value> = leases
                    .into_iter()
                    .map(|(l, unit)| json!({ "lease": l.id, "unit_id": l.unit_id, "deadline": l.deadline, "unit": unit }))
                    .collect();
                Ok(json_reply(200, json!(out)))
            }
            ("PUT", ["archives", lease]) => {
                let lease: u64 = lease.parse().map_err(|_| "archive: lease id")?;
                let host = safe(param("host").ok_or("archive: ?host=")?)?;
                let unit =
                    self.state.lock().unwrap().lease_unit(lease, host).ok_or("archive: no such lease")?;
                let dir = self.dir.join("archives").join(safe(&unit)?);
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                std::fs::write(dir.join(format!("{host}.tar")), body).map_err(|e| e.to_string())?;
                Ok(json_reply(200, json!({ "stored": body.len() })))
            }
            ("POST", ["report", lease]) => {
                let lease: u64 = lease.parse().map_err(|_| "report: lease id")?;
                let v = parse()?;
                let host = safe(v["host"].as_str().ok_or("report: host")?)?;
                let hash = v["result"]["state_hash"].as_str().ok_or("report: result.state_hash")?;
                let mut c = self.state.lock().unwrap();
                let status = c.report(lease, host, hash, v["result"].clone(), now())?;
                self.save(&c)?;
                Ok(json_reply(200, json!({ "status": status })))
            }
            ("POST", ["fail", lease]) => {
                let lease: u64 = lease.parse().map_err(|_| "fail: lease id")?;
                let v = parse()?;
                let host = safe(v["host"].as_str().ok_or("fail: host")?)?;
                let error = v["error"].as_str().ok_or("fail: error")?;
                let mut c = self.state.lock().unwrap();
                let status = c.fail(lease, host, error, now())?;
                self.save(&c)?;
                Ok(json_reply(200, json!({ "status": status })))
            }
            ("GET", ["units", id]) => {
                let c = self.state.lock().unwrap();
                let status = c.status(id).ok_or("no such unit")?;
                Ok(json_reply(
                    200,
                    json!({ "status": status, "report": c.result(id), "failures": c.failures(id) }),
                ))
            }
            ("GET", ["archives", unit, file]) => {
                let path = self.dir.join("archives").join(safe(unit)?).join(safe(file)?);
                let bytes = std::fs::read(&path).map_err(|_| format!("no archive {unit}/{file}"))?;
                Ok((200, bytes, "application/x-tar"))
            }
            ("GET", ["packs", id]) => {
                let bytes = std::fs::read(self.pack_path(id)?).map_err(|_| format!("no pack {id}"))?;
                Ok((200, bytes, "application/octet-stream"))
            }
            _ => Ok(json_reply(404, json!({ "error": format!("no route {method} {path}") }))),
        }
    }

    /// Checks each unit, copies the packs it names into `packs/` (checking their ids
    /// against their content) and strips their paths.
    fn submit(&self, experiment: &str, body: Value) -> Result<Reply, String> {
        let items = body.as_array().ok_or("submit: a list of {id, unit}")?;
        let mut units = Vec::new();
        for item in items {
            let id = safe(item["id"].as_str().ok_or("submit: id")?)?.to_string();
            let mut unit = item["unit"].clone();
            serde_json::from_value::<oikos_sim::WorkUnit>(unit.clone())
                .map_err(|e| format!("unit {id}: {e}"))?;
            crate::confined(&unit).map_err(|e| format!("unit {id}: {e}"))?;
            if let Some(inputs) = unit.get_mut("inputs").and_then(Value::as_array_mut) {
                for input in inputs {
                    let pid = pack_id(input["id"].as_str().ok_or("submit: input id")?)?.to_string();
                    let kept = self.pack_path(&pid)?;
                    if !kept.exists() {
                        let path = input["path"].as_str().filter(|p| !p.is_empty());
                        let path =
                            path.ok_or_else(|| format!("pack {pid} is unknown here; give its path"))?;
                        let bytes = std::fs::read(path).map_err(|e| format!("pack {path}: {e}"))?;
                        let actual = blake3::hash(&bytes).to_hex().to_string();
                        if actual != pid {
                            return Err(format!("pack {path} has id {actual}, not {pid}"));
                        }
                        let tmp = kept.with_extension("tmp");
                        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
                        std::fs::rename(&tmp, &kept).map_err(|e| e.to_string())?;
                    }
                    input["path"] = json!("");
                }
            }
            units.push((id, unit));
        }
        let n = units.len();
        let mut c = self.state.lock().unwrap();
        c.submit(experiment, units)?;
        self.save(&c)?;
        Ok(json_reply(200, json!({ "submitted": n })))
    }
}
