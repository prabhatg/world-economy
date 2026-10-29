//! Forager dispersal (docs/04 §4.9, ADR-0048): bands in crowded cells bud off into less
//! crowded habitable neighbors, so populations spread as a wave of advance even where a
//! cell holds a single band and fission by group size never fires.

use oikos_sim::mechanisms::{self, Dispersal, Growth, Mechanism};
use oikos_sim::schedule::Schedule;
use oikos_sim::{Params, World, WorldConfig, checkpoint};

const REGION: WorldConfig = WorldConfig::base_cell(22, 4);

/// Every cell feeds 25 people (about one forager band per M1 cell), so toy fission (bands
/// of 60 or more) never fires.
fn one_band_cells() -> Params {
    Params { capacity_min: 25, capacity_max: 25, ..Params::default() }
}

fn occupied(w: &World) -> usize {
    w.population.iter().filter(|&&p| p > 0).count()
}

#[test]
fn crowded_bands_spread_as_a_wave() {
    let mut w = World::new(REGION, 3, one_band_cells()).unwrap();
    let start = occupied(&w);
    let mut fronts = vec![];
    for _ in 0..3 {
        w.run_years(400, 2, 400);
        fronts.push(occupied(&w));
    }
    assert!(start < fronts[0] && fronts[0] < fronts[1] && fronts[1] < fronts[2], "{start} {fronts:?}");
    assert!(fronts[2] > 10 * start, "the front keeps advancing: {fronts:?}");
    let pop: u64 = w.population.iter().sum();
    assert!(pop as f64 > 0.7 * 25.0 * fronts[2] as f64, "occupied cells fill toward capacity");
}

#[test]
fn without_dispersal_toy_fission_cannot_spread_one_band_cells() {
    let p = Params { dispersal_probability: 0.0, ..one_band_cells() };
    let mut w = World::new(REGION, 3, p).unwrap();
    let start = occupied(&w);
    w.run_years(1200, 2, 1200);
    assert!(occupied(&w) <= start + 2, "the M1 failure dispersal fixes (#14)");
}

#[test]
fn uncrowded_bands_stay() {
    let p = Params { capacity_min: 100_000, capacity_max: 100_000, ..Params::default() };
    let mut w = World::new(REGION, 5, p.clone()).unwrap();
    w.set_mechanisms(vec![
        Box::new(Growth::from_params(&p)),
        Box::new(Dispersal { after: vec!["growth"], ..Dispersal::from_params(&p) }),
    ])
    .unwrap();
    for _ in 0..50 {
        assert_eq!(w.step(2).migrants, 0, "no cell is near capacity");
    }
}

#[test]
fn dispersal_moves_people_toward_less_crowded_cells_and_conserves_them() {
    let p = one_band_cells();
    let mut w = World::new(REGION, 8, p.clone()).unwrap();
    w.run_years(600, 3, 600);
    // Every crowded cell tries every step: this test is about where people go, not the rate.
    w.set_mechanisms(vec![Box::new(Dispersal {
        after: vec![],
        probability: 1.0,
        ..Dispersal::from_params(&p)
    })])
    .unwrap();
    let before: u64 = w.population.iter().sum();
    let mut moved = 0;
    for _ in 0..20 {
        let s = w.step(3);
        assert_eq!((s.births, s.deaths), (0, 0));
        moved += s.migrants;
    }
    assert!(moved > 0);
    assert_eq!(w.population.iter().sum::<u64>(), before, "dispersal only moves people");
}

#[test]
fn dispersal_is_partition_independent() {
    let run = |parts| {
        let mut w = World::new(REGION, 11, one_band_cells()).unwrap();
        w.run_years(600, parts, 600);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(3));
    assert_eq!(run(1), run(8));
}

#[test]
fn dispersal_is_a_default_mechanism_after_fission() {
    let specs: Vec<_> = mechanisms::defaults(&Params::default()).iter().map(|m| m.spec()).collect();
    let order = Schedule::build(&specs).unwrap();
    let names: Vec<&str> = order.order().iter().map(|&i| specs[i].name).collect();
    assert_eq!(names, ["growth", "fission", "dispersal", "inundation", "cultivation"]);
    assert_eq!(Dispersal::from_params(&Params::default()).spec().name, "dispersal");
}

#[test]
fn groups_only_move_where_they_fit() {
    // Capacities from 1 to 200, so large bands sit next to cells that feed a few people.
    // Each group must fit in its destination as of last step; groups from several
    // neighbors can arrive in the same step, so a cell may overshoot by a little, which
    // logistic decline then corrects.
    let p = Params { capacity_min: 1, capacity_max: 200, ..Params::default() };
    let mut moved = 0;
    for seed in 0..4 {
        let mut w = World::new(REGION, seed, p.clone()).unwrap();
        w.run_years(1000, 2, 1000);
        w.set_mechanisms(vec![Box::new(Dispersal { after: vec![], ..Dispersal::from_params(&p) })]).unwrap();
        for _ in 0..200 {
            let before = w.population.clone();
            moved += w.step(2).migrants;
            for (c, (&now, &was)) in w.population.iter().zip(&before).enumerate() {
                let k = w.capacity[c];
                assert!(now <= was.max(k) + k / 2, "seed {seed} cell {c}: {was} -> {now}, capacity {k}");
            }
        }
    }
    assert!(moved > 0);
}
