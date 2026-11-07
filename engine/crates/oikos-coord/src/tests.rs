use super::*;
use serde_json::{Value, json};

const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// No spot checks unless a test asks for them.
fn coordinator(spot_check_one_in: u64) -> Coordinator {
    let mut c =
        Coordinator::new(Policy { lease_secs: 100, trusted_after: 2, spot_check_one_in, max_replicas: 3 });
    c.submit("e", (0..4).map(|i| (format!("u{i}"), json!({ "seed": i }))).collect()).unwrap();
    c
}

fn take(c: &mut Coordinator, host: &str, now: u64) -> Lease {
    c.lease(host, 1, now).pop().expect("a unit to lease").0
}

/// Gives `host` a clean record of `n` agreements on units that sort before the rest.
fn earn_trust(c: &mut Coordinator, host: &str, n: u32) {
    for i in 0..n {
        let id = format!("trust-{host}-{i}");
        c.submit("trust", vec![(id.clone(), json!({}))]).unwrap();
        for h in [host, "witness"] {
            let l = take(c, h, 0);
            assert_eq!(l.unit_id, id);
            c.report(l.id, h, H1, json!({}), 0).unwrap();
        }
    }
}

#[test]
fn a_unit_is_done_when_two_hosts_agree() {
    let mut c = coordinator(0);
    let a = take(&mut c, "a", 0);
    assert_eq!(a.unit_id, "u0");
    let b = take(&mut c, "b", 0);
    assert_eq!(b.unit_id, "u0", "a new host's result needs a second host");
    assert_eq!(take(&mut c, "c", 0).unit_id, "u1", "two hold u0, enough");
    assert_eq!(c.report(a.id, "a", H1, json!({"x": 1}), 5).unwrap(), Status::Open);
    assert_eq!(c.report(b.id, "b", H1, json!({"x": 2}), 6).unwrap(), Status::Done);
    assert_eq!(c.result("u0").unwrap().result, json!({"x": 1}), "the first agreeing result stands");
    assert_eq!(c.host("a"), HostRecord { agreed: 1, outvoted: 0 });
}

#[test]
fn a_host_never_holds_two_replicas_of_one_unit() {
    let mut c = coordinator(0);
    let leases: Vec<String> = c.lease("a", 10, 0).into_iter().map(|(l, _)| l.unit_id).collect();
    assert_eq!(leases, ["u0", "u1", "u2", "u3"]);
    assert!(c.lease("a", 10, 0).is_empty());
}

#[test]
fn a_disagreement_asks_for_another_host_and_the_majority_stands() {
    let mut c = coordinator(0);
    let (a, b) = (take(&mut c, "a", 0), take(&mut c, "b", 0));
    c.report(a.id, "a", H1, json!({}), 1).unwrap();
    assert_eq!(c.report(b.id, "b", H2, json!({}), 1).unwrap(), Status::Open);
    let third = take(&mut c, "c", 2);
    assert_eq!(third.unit_id, "u0", "a third host breaks the tie");
    assert_eq!(c.report(third.id, "c", H2, json!({}), 3).unwrap(), Status::Done);
    assert_eq!(c.result("u0").unwrap().host, "b");
    assert_eq!(c.host("a"), HostRecord { agreed: 0, outvoted: 1 });
    assert_eq!(c.host("c"), HostRecord { agreed: 1, outvoted: 0 });
}

#[test]
fn units_without_a_quorum_fail_after_the_most_replicas() {
    let mut c = coordinator(0);
    for (h, hash) in [("a", H1), ("b", H2), ("c", "3".repeat(64).as_str())] {
        let l = take(&mut c, h, 0);
        assert_eq!(l.unit_id, "u0");
        c.report(l.id, h, hash, json!({}), 0).unwrap();
    }
    assert_eq!(c.status("u0"), Some(Status::Failed));
    assert_eq!(take(&mut c, "d", 0).unit_id, "u1");
}

#[test]
fn a_trusted_host_stands_alone_but_for_spot_checks() {
    let mut c = coordinator(0);
    earn_trust(&mut c, "t", 2);
    let l = take(&mut c, "t", 0);
    assert_eq!(take(&mut c, "n", 0).unit_id, "u1", "u0 is held by a trusted host");
    assert_eq!(c.report(l.id, "t", H1, json!({}), 1).unwrap(), Status::Done);
    assert_eq!(c.host("t").agreed, 2, "unchecked results earn nothing");

    let mut spot = coordinator(1); // every unit is checked
    earn_trust(&mut spot, "t", 2);
    let l = take(&mut spot, "t", 0);
    assert_eq!(take(&mut spot, "n", 0).unit_id, l.unit_id, "a spot check replicates it anyway");
    assert_eq!(spot.report(l.id, "t", H1, json!({}), 1).unwrap(), Status::Open);
}

#[test]
fn an_outvoted_host_is_no_longer_trusted() {
    let mut c = coordinator(1); // every unit is checked, so t's result meets others
    earn_trust(&mut c, "t", 2);
    assert!(c.trusted("t"));
    let (t, n) = (take(&mut c, "t", 0), take(&mut c, "n", 0));
    c.report(t.id, "t", H1, json!({}), 0).unwrap();
    c.report(n.id, "n", H2, json!({}), 0).unwrap();
    let m = take(&mut c, "m", 0);
    assert_eq!(c.report(m.id, "m", H2, json!({}), 0).unwrap(), Status::Done);
    assert_eq!(c.host("t"), HostRecord { agreed: 2, outvoted: 1 });
    assert!(!c.trusted("t"), "one outvoted result ends trust");
}

#[test]
fn an_expired_lease_returns_its_unit_and_a_late_result_still_counts() {
    let mut c = coordinator(0);
    let a = take(&mut c, "a", 0);
    let b = take(&mut c, "b", 0);
    assert_eq!(take(&mut c, "c", 50).unit_id, "u1");
    let c2 = take(&mut c, "c", 100);
    assert_eq!(c2.unit_id, "u0", "a's and b's leases expired at 100");
    c.report(a.id, "a", H1, json!({}), 150).unwrap();
    assert_eq!(c.report(c2.id, "c", H1, json!({}), 160).unwrap(), Status::Done);
    assert_eq!(c.report(b.id, "b", H1, json!({}), 170).unwrap(), Status::Done, "late, but credited");
    assert_eq!(c.host("b").agreed, 1);
}

#[test]
fn reports_are_checked() {
    let mut c = coordinator(0);
    let a = take(&mut c, "a", 0);
    assert!(c.report(a.id, "b", H1, json!({}), 0).unwrap_err().contains("another host"));
    assert!(c.report(99, "a", H1, json!({}), 0).unwrap_err().contains("unknown lease"));
    assert!(c.report(a.id, "a", "abc", json!({}), 0).unwrap_err().contains("64 hex"));
    c.report(a.id, "a", H1, json!({}), 0).unwrap();
    assert!(c.report(a.id, "a", H1, json!({}), 0).is_err(), "a lease is used once");
    assert!(c.submit("e", vec![("u0".into(), json!({}))]).unwrap_err().contains("already"));
}

#[test]
fn the_state_is_a_function_of_the_calls() {
    let play = || {
        let mut c = coordinator(3);
        for (i, h) in ["a", "b", "c", "a", "b"].iter().enumerate() {
            for (l, _) in c.lease(h, 2, i as u64 * 40) {
                let hash = if l.id % 4 == 0 { H2 } else { H1 };
                c.report(l.id, h, hash, json!({ "lease": l.id }), i as u64 * 40 + 10).unwrap();
            }
        }
        serde_json::to_string(&c).unwrap()
    };
    let a = play();
    assert_eq!(a, play());
    let back: Coordinator = serde_json::from_str(&a).unwrap();
    assert_eq!(serde_json::to_string(&back).unwrap(), a, "a snapshot restores the state exactly");
    assert_eq!(c_counts(&back).open + c_counts(&back).done + c_counts(&back).failed, 4);
}

fn c_counts(c: &Coordinator) -> Counts {
    c.counts(Some("e"))
}

#[test]
fn a_late_disagreement_reopens_a_result_accepted_alone() {
    let mut c = coordinator(0);
    earn_trust(&mut c, "t", 2);
    let t = take(&mut c, "t", 0);
    assert_eq!(t.unit_id, "u0");
    let (a, b) = (take(&mut c, "a", 100), take(&mut c, "b", 100));
    assert_eq!((a.unit_id.as_str(), b.unit_id.as_str()), ("u0", "u0"), "t's lease expired at 100");
    assert_eq!(c.report(t.id, "t", H1, json!({}), 101).unwrap(), Status::Done, "late, trusted, alone");
    assert_eq!(c.report(a.id, "a", H2, json!({}), 102).unwrap(), Status::Open, "unchecked: reopened");
    assert_eq!(c.report(b.id, "b", H2, json!({}), 103).unwrap(), Status::Done);
    assert_eq!(c.result("u0").unwrap().host, "a");
    assert_eq!(c.host("t").outvoted, 1);
    assert!(!c.trusted("t"));
}

#[test]
fn a_late_agreement_with_a_result_accepted_alone_credits_both() {
    let mut c = coordinator(0);
    earn_trust(&mut c, "t", 2);
    let t = take(&mut c, "t", 0);
    let a = take(&mut c, "a", 100);
    assert_eq!(c.report(t.id, "t", H1, json!({}), 101).unwrap(), Status::Done);
    assert_eq!(c.report(a.id, "a", H1, json!({}), 102).unwrap(), Status::Done);
    assert_eq!((c.host("t").agreed, c.host("a").agreed), (3, 1));
}

#[test]
fn a_result_reported_before_its_host_was_trusted_still_needs_a_second_host() {
    let mut c = coordinator(0);
    let a = take(&mut c, "a", 0);
    assert_eq!(c.report(a.id, "a", H1, json!({}), 0).unwrap(), Status::Open);
    earn_trust(&mut c, "a", 2);
    assert!(c.trusted("a"));
    let n = take(&mut c, "n", 0);
    assert_eq!(n.unit_id, "u0", "u0 is not stranded");
    assert_eq!(c.report(n.id, "n", H1, json!({}), 0).unwrap(), Status::Done);
}

#[test]
fn a_host_whose_lease_expired_is_not_given_the_unit_again() {
    let mut c = coordinator(0);
    let first = take(&mut c, "a", 0);
    assert_eq!(take(&mut c, "a", 100).unit_id, "u1");
    assert_eq!(c.report(first.id, "a", H1, json!({}), 101).unwrap(), Status::Open, "its late result counts");
}

#[test]
fn a_unit_whose_runs_fail_on_enough_hosts_is_failed() {
    let mut c = coordinator(0); // max_replicas 3
    let (a, b) = (take(&mut c, "a", 0), take(&mut c, "b", 0));
    assert_eq!(c.fail(a.id, "a", "bad unit", 1).unwrap(), Status::Open);
    assert!(c.fail(a.id, "a", "again", 1).is_err(), "a lease is used once");
    assert_eq!(c.fail(b.id, "b", "bad unit", 1).unwrap(), Status::Open);
    assert_eq!(take(&mut c, "a", 2).unit_id, "u1", "a host that failed a unit is not given it again");
    let third = take(&mut c, "c", 2);
    assert_eq!(c.fail(third.id, "c", "bad unit", 3).unwrap(), Status::Failed);
    assert_eq!(c.failures("u0").len(), 3);
}

#[test]
fn hashes_compare_whatever_their_case() {
    let mut c = coordinator(0);
    let (a, b) = (take(&mut c, "a", 0), take(&mut c, "b", 0));
    c.report(a.id, "a", &"AB".repeat(32), json!({}), 0).unwrap();
    assert_eq!(c.report(b.id, "b", &"ab".repeat(32), json!({}), 0).unwrap(), Status::Done);
    assert_eq!(c.host("a").outvoted, 0);
}

#[test]
fn units_must_keep_their_outputs_inside_the_run_directory() {
    let unit = |extra: Value| {
        let mut u = json!({ "start": { "kind": "initial" } });
        u.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        u
    };
    assert!(
        confined(&unit(json!({ "activity": { "dir": "world" }, "outputs": { "dir": "./out/x" } }))).is_ok()
    );
    for bad in [
        json!({ "checkpoint_out": "/tmp/x" }),
        json!({ "outputs": { "dir": "../x" } }),
        json!({ "activity": { "dir": "a/../../x" } }),
        json!({ "activity": { "dir": "" } }),
        json!({ "checkpoint_out": 3 }),
    ] {
        assert!(confined(&unit(bad.clone())).is_err(), "{bad}");
    }
    assert!(confined(&json!({ "start": { "kind": "checkpoint", "path": "x" } })).is_err());
}
