//! Pluggable mechanisms and multi-rate scheduling (docs/03-engine.md §3.4).

use oikos_sim::mechanisms::{Fission, Growth, Mechanism};
use oikos_sim::{Params, World, WorldConfig};

const DENSE: WorldConfig = WorldConfig { base_cell: 22, resolution: 5 };

#[test]
fn growth_alone_never_creates_agents() {
    let p = Params::default();
    let mut w = World::filled(DENSE, 4, p.clone(), 3).unwrap();
    w.set_mechanisms(vec![Box::new(Growth::from_params(&p))]).unwrap();
    let mut count = w.agent_count();
    for _ in 0..10 {
        let s = w.step(3);
        assert_eq!(s.fissions, 0);
        assert!(w.agent_count() <= count, "agents can only merge or die out");
        count = w.agent_count();
    }
}

#[test]
fn slower_mechanism_runs_on_its_own_period() {
    let p = Params::default();
    let mut w = World::filled(DENSE, 4, p.clone(), 3).unwrap();
    let mut fission = Fission::from_params(&p);
    fission.period_ticks = 2 * p.period_ticks;
    w.set_mechanisms(vec![Box::new(Growth::from_params(&p)), Box::new(fission)]).unwrap();
    for step in 0..8 {
        let ran_fission = w.tick.0 % (2 * p.period_ticks) == 0;
        let s = w.step(2);
        assert_eq!(s.fissions > 0, ran_fission, "step {step}");
    }
}

#[test]
fn ambiguous_order_is_rejected() {
    let p = Params::default();
    let mut w = World::new(DENSE, 1, p.clone()).unwrap();
    let mut fission = Fission::from_params(&p);
    fission.after.clear();
    let mechs: Vec<Box<dyn Mechanism>> = vec![Box::new(Growth::from_params(&p)), Box::new(fission)];
    let e = w.set_mechanisms(mechs).unwrap_err();
    assert!(e.contains("growth") && e.contains("fission"), "{e}");
}
