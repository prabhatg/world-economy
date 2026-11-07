//! Coordinator v0 (docs/11 §11.3-11.4, ADR-0097): work units in, validated results out.
//!
//! This is the dispatcher's and validator's state machine, without I/O, so its rules are
//! tested exactly and any server (`oikos-coord serve`, later a cloud service) wraps the
//! same rules. Clients pull: a host asks for units and gets leases with deadlines; a
//! lease that expires returns its unit to the pool, and a result that arrives late still
//! counts. Results are validated by quorum on the state hash, which the engine's
//! bit-for-bit determinism makes exact: a unit is done when two distinct hosts report
//! the same hash, or one trusted host does (adaptive replication). A host is trusted
//! after `trusted_after` results that agreed with others and none that were outvoted;
//! one unit in `spot_check_one_in` (by its id) is replicated anyway. A disagreement asks
//! for one more replica, up to `max_replicas`, and counts against the hosts outvoted; a
//! later result that disagrees with one accepted alone reopens it. Whether a result may
//! stand alone is fixed when it arrives, by its host's trust then. A host is never given
//! a unit twice, and a unit whose run fails on `max_replicas` hosts is failed.
//! Every decision is a function of the calls made and the times passed in, so the
//! state can be snapshot and replayed.

pub mod server;
pub mod worker;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path};

/// The content-addressed files `unit` names, each `{id, path}`: its forcing packs and
/// its zone file (`activity.zones`, #47). Hosts fetch them by id and rewrite their paths,
/// so a reference in any other form, which the engine would read where it points, is
/// refused.
pub fn files_mut(unit: &mut Value) -> Result<Vec<&mut Value>, String> {
    let mut out = Vec::new();
    for (key, v) in unit.as_object_mut().into_iter().flatten() {
        match key.as_str() {
            "inputs" => out.extend(v.as_array_mut().into_iter().flatten()),
            "activity" => out.extend(v.get_mut("zones").filter(|z| !z.is_null())),
            _ => {}
        }
    }
    if out.iter().any(|r| !r.is_object()) {
        return Err("file references (inputs, activity.zones) must be {id, path} objects".into());
    }
    Ok(out)
}

/// Checks that a host can run `unit` inside its run directory: an initial start (v0
/// ships no checkpoints), and every output path relative, without `..`.
pub fn confined(unit: &Value) -> Result<(), String> {
    if unit["start"]["kind"] != "initial" {
        return Err("v0 runs only work units with an initial start".into());
    }
    let paths = [
        ("checkpoint_out", &unit["checkpoint_out"]),
        ("outputs.dir", &unit["outputs"]["dir"]),
        ("activity.dir", &unit["activity"]["dir"]),
    ];
    for (field, path) in paths {
        let inside = |p: &str| {
            !p.is_empty()
                && Path::new(p).components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        };
        match path {
            Value::Null => {}
            Value::String(p) if inside(p) => {}
            _ => return Err(format!("{field} {path} must be a relative path inside the run directory")),
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    /// Seconds a host has to return a unit before it is issued again.
    pub lease_secs: u64,
    /// Agreeing results, and none outvoted, before a host's result stands alone.
    pub trusted_after: u32,
    /// One unit in this many (by id) is replicated even when a trusted host takes it.
    pub spot_check_one_in: u64,
    /// Results a unit may gather without a quorum before it is given up as failed.
    pub max_replicas: u32,
}

impl Default for Policy {
    fn default() -> Policy {
        Policy { lease_secs: 2 * 3600, trusted_after: 10, spot_check_one_in: 10, max_replicas: 5 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    Done,
    Failed,
}

/// One host's result for a unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub host: String,
    pub state_hash: String,
    /// The engine's `WorkResult`, as the host sent it.
    pub result: Value,
    pub received: u64,
    /// Whether the host was trusted when it reported: whether this result may stand alone.
    pub trusted: bool,
}

/// A host's run of a unit that ended in an error instead of a result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Failure {
    pub host: String,
    pub error: String,
    pub received: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub id: u64,
    pub unit_id: String,
    pub host: String,
    pub deadline: u64,
}

/// What a host's results have shown: agreements with other hosts, and times outvoted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HostRecord {
    pub agreed: u32,
    pub outvoted: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Unit {
    experiment: String,
    body: Value,
    reports: Vec<Report>,
    #[serde(default)]
    failures: Vec<Failure>,
    status: Status,
    /// Index into `reports` of the accepted result.
    canonical: Option<usize>,
}

/// Counts of units by status.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    pub open: usize,
    pub done: usize,
    pub failed: usize,
    pub leased: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coordinator {
    policy: Policy,
    units: BTreeMap<String, Unit>,
    /// Leases out now.
    leases: BTreeMap<u64, Lease>,
    /// Leases past their deadline, whose results are still accepted.
    expired: BTreeMap<u64, Lease>,
    hosts: BTreeMap<String, HostRecord>,
    next_lease: u64,
}

impl Coordinator {
    pub fn new(policy: Policy) -> Coordinator {
        Coordinator {
            policy,
            units: BTreeMap::new(),
            leases: BTreeMap::new(),
            expired: BTreeMap::new(),
            hosts: BTreeMap::new(),
            next_lease: 1,
        }
    }

    /// Adds `units` (id, work unit) of `experiment`; ids must be new.
    pub fn submit(&mut self, experiment: &str, units: Vec<(String, Value)>) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for (id, _) in &units {
            if id.is_empty() || self.units.contains_key(id) || !seen.insert(id) {
                return Err(format!("unit id {id:?} is empty or already submitted"));
            }
        }
        for (id, body) in units {
            let unit = Unit {
                experiment: experiment.into(),
                body,
                reports: Vec::new(),
                failures: Vec::new(),
                status: Status::Open,
                canonical: None,
            };
            self.units.insert(id, unit);
        }
        Ok(())
    }

    fn trusted(&self, host: &str) -> bool {
        self.hosts.get(host).is_some_and(|h| h.agreed >= self.policy.trusted_after && h.outvoted == 0)
    }

    fn spot_checked(&self, unit_id: &str) -> bool {
        // FNV-1a: stable across hosts and versions, unlike std's hasher.
        let h = unit_id
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3));
        self.policy.spot_check_one_in > 0 && h % self.policy.spot_check_one_in == 0
    }

    /// Results a unit has or has coming: its reports and the leases out on it.
    fn holders(&self, unit_id: &str) -> usize {
        self.units[unit_id].reports.len() + self.leases.values().filter(|l| l.unit_id == unit_id).count()
    }

    /// Whether `host` ever had `unit_id`: a report, a failure or a lease, current or expired.
    fn had(&self, unit_id: &str, host: &str) -> bool {
        let u = &self.units[unit_id];
        u.reports.iter().any(|r| r.host == host)
            || u.failures.iter().any(|f| f.host == host)
            || self
                .leases
                .values()
                .chain(self.expired.values())
                .any(|l| l.unit_id == unit_id && l.host == host)
    }

    /// Results a unit wants in all: one more than it has after a disagreement; else two,
    /// or one while every result may stand alone, every lease is a trusted host's and the
    /// unit is not spot-checked.
    fn wanted(&self, unit_id: &str) -> usize {
        let u = &self.units[unit_id];
        let mut hashes: Vec<&str> = u.reports.iter().map(|r| r.state_hash.as_str()).collect();
        hashes.sort_unstable();
        hashes.dedup();
        if hashes.len() > 1 {
            return u.reports.len() + 1;
        }
        let alone = u.reports.iter().all(|r| r.trusted)
            && self.leases.values().filter(|l| l.unit_id == unit_id).all(|l| self.trusted(&l.host));
        if alone && !self.spot_checked(unit_id) { 1 } else { 2 }
    }

    /// Moves leases past their deadline to `expired`, returning their units to the pool.
    pub fn expire(&mut self, now: u64) {
        let due: Vec<u64> = self.leases.values().filter(|l| l.deadline <= now).map(|l| l.id).collect();
        for id in due {
            let lease = self.leases.remove(&id).expect("listed");
            self.expired.insert(id, lease);
        }
    }

    /// Up to `n` leases for `host`: open units, in id order, that want more results and
    /// that the host never had.
    pub fn lease(&mut self, host: &str, n: usize, now: u64) -> Vec<(Lease, Value)> {
        self.expire(now);
        let mut out = Vec::new();
        let ids: Vec<String> =
            self.units.iter().filter(|(_, u)| u.status == Status::Open).map(|(id, _)| id.clone()).collect();
        for id in ids {
            if out.len() >= n {
                break;
            }
            if self.had(&id, host) || self.holders(&id) >= self.wanted(&id) {
                continue;
            }
            let lease = Lease {
                id: self.next_lease,
                unit_id: id.clone(),
                host: host.into(),
                deadline: now + self.policy.lease_secs,
            };
            self.next_lease += 1;
            self.leases.insert(lease.id, lease.clone());
            out.push((lease, self.units[&id].body.clone()));
        }
        out
    }

    /// A host's result for its lease (current or expired). Returns the unit's status.
    pub fn report(
        &mut self,
        lease_id: u64,
        host: &str,
        state_hash: &str,
        result: Value,
        now: u64,
    ) -> Result<Status, String> {
        if state_hash.len() != 64 || !state_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("state hash {state_hash:?} is not 64 hex digits"));
        }
        let state_hash = state_hash.to_ascii_lowercase();
        let unit_id = self.take_lease(lease_id, host)?;
        let status = self.units[&unit_id].status;
        let trusted = self.trusted(host);
        let report =
            Report { host: host.into(), state_hash: state_hash.clone(), result, received: now, trusted };
        let u = self.units.get_mut(&unit_id).expect("leases name submitted units");
        u.reports.push(report);
        match status {
            Status::Open => self.validate(&unit_id),
            Status::Done => {
                let u = &self.units[&unit_id];
                let accepted = &u.reports[u.canonical.expect("done units have one")];
                let earlier = &u.reports[..u.reports.len() - 1];
                let alone = earlier.iter().filter(|r| r.state_hash == accepted.state_hash).count() == 1;
                let accepted_host = accepted.host.clone();
                if accepted.state_hash == state_hash {
                    if alone {
                        self.credit(&accepted_host, true);
                    }
                    self.credit(host, true);
                } else if alone {
                    // Nobody had checked it: reopen, and let a quorum decide.
                    let u = self.units.get_mut(&unit_id).expect("exists");
                    u.status = Status::Open;
                    u.canonical = None;
                } else {
                    self.credit(host, false);
                }
            }
            Status::Failed => {}
        }
        Ok(self.units[&unit_id].status)
    }

    /// A host's run of its lease ended in `error`. A unit that fails on `max_replicas`
    /// hosts is failed. Returns the unit's status.
    pub fn fail(&mut self, lease_id: u64, host: &str, error: &str, now: u64) -> Result<Status, String> {
        let unit_id = self.take_lease(lease_id, host)?;
        let failure = Failure { host: host.into(), error: error.chars().take(2000).collect(), received: now };
        let u = self.units.get_mut(&unit_id).expect("leases name submitted units");
        u.failures.push(failure);
        if u.status == Status::Open && u.failures.len() >= self.policy.max_replicas as usize {
            u.status = Status::Failed;
            self.drop_leases(&unit_id);
        }
        Ok(self.units[&unit_id].status)
    }

    /// Ends `host`'s lease `lease_id` (current or expired) once it has a report or a
    /// failure; returns its unit.
    fn take_lease(&mut self, lease_id: u64, host: &str) -> Result<String, String> {
        let lease = self.leases.get(&lease_id).or_else(|| self.expired.get(&lease_id));
        let lease = lease.ok_or_else(|| format!("unknown lease {lease_id}"))?;
        if lease.host != host {
            return Err(format!("lease {lease_id} belongs to another host"));
        }
        let unit_id = lease.unit_id.clone();
        let u = &self.units[&unit_id];
        if u.reports.iter().any(|r| r.host == host) || u.failures.iter().any(|f| f.host == host) {
            return Err(format!("{host} already returned {unit_id}"));
        }
        self.leases.remove(&lease_id);
        self.expired.remove(&lease_id);
        Ok(unit_id)
    }

    fn credit(&mut self, host: &str, agreed: bool) {
        let h = self.hosts.entry(host.into()).or_default();
        if agreed { h.agreed += 1 } else { h.outvoted += 1 }
    }

    /// Accepts a hash two hosts agree on, or one trusted host's alone when no other
    /// hash was reported and the unit is not spot-checked; gives up after too many.
    fn validate(&mut self, unit_id: &str) {
        let u = &self.units[unit_id];
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for r in &u.reports {
            *counts.entry(r.state_hash.as_str()).or_default() += 1;
        }
        let alone = counts.len() == 1 && !self.spot_checked(unit_id) && u.reports.iter().all(|r| r.trusted);
        let accepted = counts
            .iter()
            .find(|(_, n)| **n >= 2)
            .map(|(h, _)| h.to_string())
            .or_else(|| alone.then(|| u.reports[0].state_hash.clone()));
        match accepted {
            Some(hash) => {
                let canonical = u.reports.iter().position(|r| r.state_hash == hash).expect("reported");
                let votes: Vec<(String, bool)> =
                    u.reports.iter().map(|r| (r.host.clone(), r.state_hash == hash)).collect();
                let unit = self.units.get_mut(unit_id).expect("exists");
                unit.status = Status::Done;
                unit.canonical = Some(canonical);
                // A trusted host alone gains no agreement: nobody checked it.
                if votes.len() > 1 {
                    for (host, agreed) in votes {
                        self.credit(&host, agreed);
                    }
                }
                self.drop_leases(unit_id);
            }
            None if u.reports.len() >= self.policy.max_replicas as usize => {
                self.units.get_mut(unit_id).expect("exists").status = Status::Failed;
                self.drop_leases(unit_id);
            }
            None => {}
        }
    }

    /// Leases on a settled unit stay valid for credit but no longer hold it.
    fn drop_leases(&mut self, unit_id: &str) {
        let ids: Vec<u64> = self.leases.values().filter(|l| l.unit_id == unit_id).map(|l| l.id).collect();
        for id in ids {
            let lease = self.leases.remove(&id).expect("listed");
            self.expired.insert(id, lease);
        }
    }

    /// Why a unit's runs failed, by host.
    pub fn failures(&self, unit_id: &str) -> &[Failure] {
        self.units.get(unit_id).map_or(&[], |u| &u.failures)
    }

    /// The unit of `host`'s lease `lease_id`, current or expired.
    pub fn lease_unit(&self, lease_id: u64, host: &str) -> Option<String> {
        let lease = self.leases.get(&lease_id).or_else(|| self.expired.get(&lease_id))?;
        (lease.host == host).then(|| lease.unit_id.clone())
    }

    pub fn status(&self, unit_id: &str) -> Option<Status> {
        self.units.get(unit_id).map(|u| u.status)
    }

    /// The accepted result of a done unit.
    pub fn result(&self, unit_id: &str) -> Option<&Report> {
        let u = self.units.get(unit_id)?;
        u.canonical.map(|i| &u.reports[i])
    }

    pub fn host(&self, host: &str) -> HostRecord {
        self.hosts.get(host).cloned().unwrap_or_default()
    }

    /// Unit counts, of all units or of one experiment's.
    pub fn counts(&self, experiment: Option<&str>) -> Counts {
        let mut c = Counts { leased: self.leases.len(), ..Counts::default() };
        for u in self.units.values().filter(|u| experiment.is_none_or(|e| u.experiment == e)) {
            match u.status {
                Status::Open => c.open += 1,
                Status::Done => c.done += 1,
                Status::Failed => c.failed += 1,
            }
        }
        c
    }

    /// The ids of an experiment's units, in order.
    pub fn units(&self, experiment: &str) -> Vec<String> {
        self.units.iter().filter(|(_, u)| u.experiment == experiment).map(|(id, _)| id.clone()).collect()
    }
}

#[cfg(test)]
mod tests;
