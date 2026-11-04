//! Reproducibility contract tests (docs/03-engine.md §3.5).

use oikos_sim::checkpoint;
use oikos_sim::workunit::Start;
use oikos_sim::{Params, WorkUnit, World, WorldConfig, run_work_unit};

const GOLDEN: &str = include_str!("../../../golden/m0-toy.json");

fn golden_unit() -> WorkUnit {
    serde_json::from_str(include_str!("../../../golden/m0-toy.workunit.json")).unwrap()
}

#[test]
fn identical_across_partition_counts() {
    let hashes: Vec<String> = [1, 2, 3, 4, 8]
        .iter()
        .map(|&p| {
            let mut w = World::new(WorldConfig::SOUTHWEST_ASIA, 7, Params::default()).unwrap();
            w.run_years(1_500, p, 500);
            checkpoint::state_hash(&w)
        })
        .collect();
    assert!(hashes.windows(2).all(|h| h[0] == h[1]), "{hashes:?}");
}

#[test]
fn people_are_conserved_exactly() {
    let mut w = World::new(WorldConfig::SOUTHWEST_ASIA, 11, Params::default()).unwrap();
    for _ in 0..1_000 {
        let before = w.total_population();
        let s = w.step(4);
        assert_eq!(w.total_population(), before + s.births - s.deaths);
        assert_eq!(w.total_agent_weight(), w.total_population());
    }
}

#[test]
fn checkpoint_restore_continues_identically() {
    let mut straight = World::new(WorldConfig::SOUTHWEST_ASIA, 3, Params::default()).unwrap();
    straight.run_years(1_000, 2, 1_000);

    let mut first = World::new(WorldConfig::SOUTHWEST_ASIA, 3, Params::default()).unwrap();
    first.run_years(400, 4, 1_000);
    let bytes = checkpoint::encode(&first);
    let mut resumed = checkpoint::decode(&bytes).unwrap();
    assert_eq!(checkpoint::encode(&resumed), bytes, "encode(decode(x)) == x");
    resumed.run_years(600, 1, 1_000);

    assert_eq!(checkpoint::state_hash(&straight), checkpoint::state_hash(&resumed));
}

#[test]
fn fork_diverges_from_parent() {
    let mut w = World::new(WorldConfig::SOUTHWEST_ASIA, 5, Params::default()).unwrap();
    w.run_years(300, 1, 300);
    let ckpt = checkpoint::encode(&w);
    let unit = |seed| WorkUnit {
        format: 1,
        id: None,
        world: WorldConfig::SOUTHWEST_ASIA,
        seed,
        params: None,
        start: Start::Checkpoint { path: "parent".into(), fork: true },
        years: 300,
        partitions: 2,
        summary_every_years: 300,
        checkpoint_out: None,
        outputs: None,
        inputs: vec![],
    };
    let read = |_: &str| Ok(ckpt.clone());
    let write = |_: &str, _: &[u8]| Ok(());
    let a = run_work_unit(&unit(5), read, write).unwrap();
    let b = run_work_unit(&unit(6), read, write).unwrap();
    assert_ne!(a.state_hash, b.state_hash);
}

/// The committed golden result must be reproduced bit-for-bit on every target
/// (x86_64, aarch64, wasm32). Regenerate only for intentional model changes:
/// `cargo run -p oikos-cli -- run golden/m0-toy.workunit.json -o golden/m0-toy.json`
#[test]
fn golden_result() {
    let result = run_work_unit(&golden_unit(), |p| Err(p.to_string()), |_, _| Ok(())).unwrap();
    let expected: oikos_sim::WorkResult = serde_json::from_str(GOLDEN).unwrap();
    assert_eq!(result.state_hash, expected.state_hash);
    assert_eq!(result.summaries, expected.summaries);
}

#[test]
fn the_hypotheses_switched_off_are_the_engine_before_them() {
    // Fertility 0 keeps the net change rounded once (ADR-0068 off) and village size 0 the
    // fission before villages (ADR-0070 off), so checkpoints from before them continue as
    // they ran (ADR-0069): the golden summaries of that engine, at the forager growth and
    // dispersal it ran with (the round-8 defaults, ADR-0066).
    let mut wu = golden_unit();
    let p = wu.params.clone().unwrap_or_default();
    wu.params = Some(Params {
        fertility: 0.0,
        sedentary_fertility: 0.0,
        village_fission_size: 0,
        growth_rate: 0.02316,
        dispersal_probability: 0.3026,
        ..p
    });
    let result = run_work_unit(&wu, |p| Err(p.to_string()), |_, _| Ok(())).unwrap();
    let before: Vec<(i64, u64, u64)> = vec![
        (-12000, 360, 12),
        (-11750, 9276, 400),
        (-11500, 45310, 1882),
        (-11250, 92709, 3770),
        (-11000, 151120, 6038),
        (-10750, 224442, 8863),
        (-10500, 300268, 11624),
        (-10250, 373245, 14268),
        (-10000, 442222, 16749),
        (-9750, 490618, 18424),
        (-9500, 506307, 18942),
        (-9250, 506596, 18934),
        (-9000, 506594, 18934),
    ];
    let got: Vec<(i64, u64, u64)> =
        result.summaries.iter().map(|s| (s.calendar_year, s.population, s.agents)).collect();
    assert_eq!(got, before);
}

/// Dense world (every cell occupied, many cross-partition migrants).
#[test]
fn dense_world_identical_across_partition_counts() {
    let config = WorldConfig::base_cell(22, 5);
    let hashes: Vec<String> = [1, 3, 4, 7]
        .iter()
        .map(|&p| {
            let mut w = World::filled(config.clone(), 1, Params::default(), 3).unwrap();
            for _ in 0..15 {
                w.step(p);
            }
            checkpoint::state_hash(&w)
        })
        .collect();
    assert!(hashes.windows(2).all(|h| h[0] == h[1]), "{hashes:?}");
}
