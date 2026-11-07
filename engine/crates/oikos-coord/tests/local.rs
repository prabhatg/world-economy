//! A coordinator and two workers on one machine (ADR-0097): units go out, results come
//! back, quorum validates them, archives keep each run's outputs.

use oikos_coord::Policy;
use oikos_coord::server::Server;
use oikos_coord::worker::{Runner, Worker};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oikos-coord-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A coordinator on a free local port; returns its base URL.
fn start(dir: &Path) -> String {
    let server = Arc::new(
        Server::open(
            dir,
            Policy { lease_secs: 600, trusted_after: 100, spot_check_one_in: 0, max_replicas: 3 },
        )
        .unwrap(),
    );
    let http = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", http.server_addr().to_ip().unwrap());
    std::thread::spawn(move || server.serve(&http));
    base
}

fn post(url: &str, body: Value) -> Value {
    let r = ureq::post(url).set("Content-Type", "application/json").send_string(&body.to_string()).unwrap();
    serde_json::from_str(&r.into_string().unwrap()).unwrap()
}

fn get(url: &str) -> Vec<u8> {
    let mut b = Vec::new();
    ureq::get(url).call().unwrap().into_reader().read_to_end(&mut b).unwrap();
    b
}

/// The engine in this process, writing into the run directory.
struct InProcess;

impl Runner for InProcess {
    fn run(&self, unit: &Value, dir: &Path) -> Result<Value, String> {
        let wu: oikos_sim::WorkUnit = serde_json::from_value(unit.clone()).map_err(|e| e.to_string())?;
        let result = oikos_sim::run_work_unit(
            &wu,
            |p| std::fs::read(p).map_err(|e| e.to_string()),
            |p, b| {
                let path = dir.join(p);
                std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                std::fs::write(path, b).map_err(|e| e.to_string())
            },
        )?;
        Ok(serde_json::to_value(result).unwrap())
    }
}

#[test]
fn two_workers_compute_an_experiment_that_quorum_validates() {
    let dir = scratch("quorum");
    let base = start(&dir.join("coord"));
    let units: Vec<Value> = (0..3)
        .map(|i| {
            json!({ "id": format!("u{i}"), "unit": {
                "format": 1, "world": { "base_cell": 22, "resolution": 3 }, "seed": i, "start": { "kind": "initial" },
                "years": 200, "activity": { "dir": "rec", "every_years": 10 } } })
        })
        .collect();
    assert_eq!(post(&format!("{base}/experiments/toy"), json!(units))["submitted"], 3);
    for host in ["a", "b"] {
        let w = Worker {
            base: base.clone(),
            host: host.into(),
            cache: dir.join(host).join("packs"),
            runner: &InProcess,
        };
        assert_eq!(w.round(10, &dir.join(host).join("runs")).unwrap(), 3, "{host} takes every unit once");
    }
    let exp: Value = serde_json::from_slice(&get(&format!("{base}/experiments/toy"))).unwrap();
    assert_eq!(exp["counts"]["done"], 3, "{exp}");
    let unit: Value = serde_json::from_slice(&get(&format!("{base}/units/u1"))).unwrap();
    assert_eq!(unit["report"]["host"], "a");
    assert_eq!(unit["report"]["result"]["state_hash"].as_str().unwrap().len(), 64);
    let tar = get(&format!("{base}/archives/u1/a.tar"));
    let mut names = Vec::new();
    for e in tar::Archive::new(&tar[..]).entries().unwrap() {
        names.push(e.unwrap().path().unwrap().to_string_lossy().to_string());
    }
    assert!(names.iter().any(|n| n.ends_with("rec/activity.zarr/zarr.json")), "{names:?}");
}

/// Checks the pack it was given and returns a made-up result.
struct PackReader;

impl Runner for PackReader {
    fn run(&self, unit: &Value, _dir: &Path) -> Result<Value, String> {
        let input = &unit["inputs"][0];
        let bytes = std::fs::read(input["path"].as_str().unwrap()).map_err(|e| e.to_string())?;
        assert_eq!(blake3::hash(&bytes).to_hex().as_str(), input["id"].as_str().unwrap());
        Ok(json!({ "state_hash": "ab".repeat(32) }))
    }
}

/// A toy work unit with `extra` fields.
fn toy(extra: Value) -> Value {
    let mut u = json!({ "format": 1, "world": { "base_cell": 22, "resolution": 3 }, "seed": 1,
                        "start": { "kind": "initial" }, "years": 100 });
    u.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    u
}

fn refused(base: &str, experiment: &str, units: Value) -> String {
    let r = ureq::post(&format!("{base}/experiments/{experiment}"))
        .set("Content-Type", "application/json")
        .send_string(&units.to_string());
    match r {
        Err(ureq::Error::Status(400, r)) => r.into_string().unwrap(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn workers_fetch_packs_by_id_and_the_coordinator_keeps_its_own_copy() {
    let dir = scratch("packs");
    let base = start(&dir.join("coord"));
    let pack = dir.join("forcing.oikospack");
    std::fs::write(&pack, b"pack bytes").unwrap();
    let id = blake3::hash(b"pack bytes").to_hex().to_string();
    let unit = |pid: &str| json!([{ "id": "p0", "unit": toy(json!({ "inputs": [{ "id": pid, "path": pack.to_string_lossy() }] })) }]);
    assert!(
        refused(&base, "bad", unit(&"0".repeat(64))).contains("has id"),
        "a pack whose content is not its id"
    );
    post(&format!("{base}/experiments/packs"), unit(&id));
    std::fs::write(&pack, b"changed since").unwrap();
    let w = Worker { base: base.clone(), host: "a".into(), cache: dir.join("cache"), runner: &PackReader };
    assert_eq!(w.round(1, &dir.join("runs")).unwrap(), 1, "the pack submitted is the pack served");
    assert!(dir.join("cache").join(format!("{id}.oikospack")).exists());
}

#[test]
fn workers_fetch_a_units_zone_file_like_its_packs() {
    let dir = scratch("zones");
    let base = start(&dir.join("coord"));
    let config = serde_json::from_value(json!({ "base_cell": 22, "resolution": 3 })).unwrap();
    let world = oikos_sim::World::new(config, 1, Default::default()).unwrap();
    let h3: Vec<u64> = (0..world.grid.len() as u32).map(|c| world.grid.h3(c)).collect();
    let zones = serde_json::to_vec(&json!({
        "format": "oikos-zones/1", "cells_hash": oikos_io::pack::cells_hash(&h3), "cells": h3.len(),
        "first_year": world.tick.calendar_year(), "centuries": 1, "zones": [[0, 1]], "weighted": [],
        "weights": { "cells": [], "row": [], "rows": [] } }))
    .unwrap();
    let path = dir.join("zones.json");
    std::fs::write(&path, &zones).unwrap();
    let id = blake3::hash(&zones).to_hex().to_string();
    let activity =
        json!({ "dir": "rec", "every_years": 10, "zones": { "id": id, "path": path.to_string_lossy() } });
    post(
        &format!("{base}/experiments/zones"),
        json!([{ "id": "z0", "unit": toy(json!({ "activity": activity })) }]),
    );
    std::fs::remove_file(&path).unwrap();
    let w = Worker { base: base.clone(), host: "a".into(), cache: dir.join("cache"), runner: &InProcess };
    assert_eq!(w.round(1, &dir.join("runs")).unwrap(), 1);
    let unit: Value = serde_json::from_slice(&get(&format!("{base}/units/z0"))).unwrap();
    assert_eq!(unit["failures"], json!([]), "{unit}");
    let tar = get(&format!("{base}/archives/z0/a.tar"));
    let mut group = String::new();
    for e in tar::Archive::new(&tar[..]).entries().unwrap() {
        let mut e = e.unwrap();
        if e.path().unwrap().ends_with("rec/activity.zarr/zarr.json") {
            e.read_to_string(&mut group).unwrap();
        }
    }
    assert!(group.contains(&id), "the record is summed over the zones: {group}");
}

#[test]
fn file_references_must_be_objects_so_hosts_can_rewrite_their_paths() {
    let dir = scratch("refs");
    let base = start(&dir.join("coord"));
    let id = "a".repeat(64);
    let zones = json!({ "activity": { "dir": "rec", "every_years": 10, "zones": [id, "/etc/hosts"] } });
    assert!(refused(&base, "z", json!([{ "id": "z0", "unit": toy(zones) }])).contains("{id, path}"));
    let inputs = json!({ "inputs": [[id, "/etc/hosts"]] });
    assert!(refused(&base, "i", json!([{ "id": "i0", "unit": toy(inputs) }])).contains("{id, path}"));
}

#[test]
fn units_that_are_not_work_units_or_write_outside_their_run_are_refused() {
    let dir = scratch("refused");
    let base = start(&dir.join("coord"));
    assert!(refused(&base, "e", json!([{ "id": "x", "unit": null }])).contains("unit x"));
    assert!(refused(&base, "e", json!([{ "id": "x", "unit": { "seed": 1 } }])).contains("unit x"));
    let escape = toy(json!({ "activity": { "dir": "../../elsewhere" } }));
    assert!(refused(&base, "e", json!([{ "id": "x", "unit": escape }])).contains("inside the run directory"));
    let status: Value = serde_json::from_slice(&get(&format!("{base}/status"))).unwrap();
    assert_eq!(status["open"], 0, "nothing was submitted");
}

/// Fails every run.
struct Broken;

impl Runner for Broken {
    fn run(&self, _unit: &Value, _dir: &Path) -> Result<Value, String> {
        Err("engine exploded".into())
    }
}

#[test]
fn failed_runs_are_reported_and_a_unit_failing_everywhere_is_given_up() {
    let dir = scratch("broken");
    let base = start(&dir.join("coord"));
    post(&format!("{base}/experiments/e"), json!([{ "id": "u0", "unit": toy(json!({})) }]));
    for host in ["a", "b", "c"] {
        let w = Worker { base: base.clone(), host: host.into(), cache: dir.join("cache"), runner: &Broken };
        assert_eq!(w.round(1, &dir.join(host)).unwrap(), 1, "{host} reports the failure and carries on");
    }
    let unit: Value = serde_json::from_slice(&get(&format!("{base}/units/u0"))).unwrap();
    assert_eq!(unit["status"], "failed", "max_replicas 3: {unit}");
    assert_eq!(unit["failures"][0]["error"], "engine exploded");
}

#[test]
fn the_coordinator_serves_the_browser_client_from_its_own_origin() {
    let dir = scratch("client");
    let client = dir.join("client-files");
    std::fs::create_dir_all(client.join("assets")).unwrap();
    std::fs::write(client.join("volunteer.html"), "<!doctype html>").unwrap();
    std::fs::write(client.join("assets").join("volunteer-a1.js"), "export {}").unwrap();
    std::fs::write(client.join("oikos.wasm"), b"\0asm").unwrap();
    let server = Server::open(&dir.join("state"), Policy::default()).unwrap().with_client(client.clone());
    let get = |url: &str| server.handle("GET", url, b"");
    let (status, bytes, kind) = get("/client/volunteer.html").unwrap();
    assert_eq!(
        (status, bytes.as_slice(), kind),
        (200, b"<!doctype html>".as_slice(), "text/html; charset=utf-8")
    );
    assert_eq!(get("/client/assets/volunteer-a1.js").unwrap().2, "text/javascript");
    assert_eq!(get("/client/oikos.wasm").unwrap().2, "application/wasm", "for WebAssembly.compileStreaming");
    assert_eq!(get("/client/missing.html").unwrap().0, 404);
    assert!(get("/client/../state/state.json").is_err(), "no way out of the client directory");
    assert!(get("/client/.hidden").is_err());
    #[cfg(unix)]
    {
        // A link inside the client directory must not reach files outside it.
        std::fs::write(dir.join("secret.json"), "{}").unwrap();
        std::os::unix::fs::symlink(&dir, client.join("link")).unwrap();
        assert!(
            !matches!(get("/client/link/secret.json"), Ok((200, _, _))),
            "followed a link out of the client"
        );
    }
    let none = Server::open(&dir.join("other"), Policy::default()).unwrap();
    assert_eq!(
        none.handle("GET", "/client/volunteer.html", b"").unwrap().0,
        404,
        "no client unless one is given"
    );
}
