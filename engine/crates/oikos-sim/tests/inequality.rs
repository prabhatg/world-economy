//! Wealth inequality within groups (ADR-0074): households pass on part of their wealth
//! and meet chance shocks (Borgerhoff Mulder et al. 2009), and land that is farmed and
//! closed in becomes property, so farmers in a filled landscape inherit more and grow
//! unequal while foragers and farmers on an open frontier stay as equal as foragers.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{self, Inequality};
use oikos_sim::schedule::Schedule;
use oikos_sim::{Params, World, WorldConfig, checkpoint};

fn h3_cells() -> Vec<u64> {
    let g = Grid::from_base_cell(22, 3).unwrap();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

fn region(h3: &[u64]) -> Region {
    Region { name: "test".into(), resolution: 3, cells: h3.len() as u32, cells_hash: cells_hash(h3) }
}

fn spec(name: &str, dims: Dims, scale: f64) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        unit: "".into(),
        description: name.into(),
        dims,
        scale,
        offset: 0.0,
        nodata: None,
    }
}

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(vec![4_000_000_000; h3.len()])).unwrap();
    b.build()
}

/// Warm country with winter rain, where wild cereals grow everywhere.
fn climate_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![500; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

/// Farming spreads from a few founding bands by population pressure, so the land fills
/// with farmers over a few thousand years.
fn farming_world(seed: u64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        fission_probability: 0.0,
        cultivation_pressure_weight: 0.8,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.2,
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        growth_rate: 0.01,
        sedentary_growth_rate: 0.01,
        dispersal_probability: 0.1,
        forager_density_ref: 0.1,
        farm_density: 2.0,
        selection_coefficient: 0.02,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// Gini of a lognormal distribution with log variance `v`: erf(sigma / 2), by
/// Abramowitz and Stegun 7.1.26 (error below 1.5e-7).
fn gini(v: f64) -> f64 {
    let x = v.sqrt() / 2.0;
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly =
        t * (0.254829592 + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    1.0 - poly * (-x * x).exp()
}

/// Settled-people-weighted mean variance of log wealth, and the people-weighted mean.
fn mean_inequality(w: &World) -> (f64, f64) {
    let (mut settled, mut s_sum, mut people, mut p_sum) = (0.0, 0.0, 0.0, 0.0);
    for a in w.agent_slices() {
        for i in 0..a.len() {
            let (n, e, v) = (a.weight[i] as f64, a.cultivation[i] as f64 / SHARE_ONE as f64, a.inequality[i]);
            let v = v as f64 / SHARE_ONE as f64;
            settled += n * e;
            s_sum += n * e * v;
            people += n;
            p_sum += n * v;
        }
    }
    (s_sum / settled.max(1e-300), p_sum / people)
}

#[test]
fn a_generation_of_yearly_steps_is_the_generational_recursion() {
    // var(w_t) = beta^2 var(w_{t-1}) + sigma^2 per generation (Borgerhoff Mulder et al.
    // 2009, eq. 2), whatever the period and generation lengths.
    let p = Params { generation_years: 25.0, ..Params::default() };
    let m = Inequality::from_params(&p);
    for (owned, beta, shock) in [
        (0.0, p.wealth_transmission, p.wealth_shock),
        (1.0, p.landed_wealth_transmission, p.landed_wealth_shock),
    ] {
        let mut v = 1.3;
        for _ in 0..25 {
            v = m.next(v, owned);
        }
        let want = beta * beta * 1.3 + shock;
        assert!((v - want).abs() < 1e-12, "owned {owned}: {v} after a generation, want {want}");
    }
}

#[test]
fn partial_ownership_follows_the_generational_recursion_whatever_the_period() {
    // Transmission and shocks move linearly with ownership per generation; a generation
    // of periods gives that generation's recursion, for yearly and seven-year periods.
    let owned = 0.3;
    let p = Params::default();
    let beta = p.wealth_transmission + owned * (p.landed_wealth_transmission - p.wealth_transmission);
    let shock = p.wealth_shock + owned * (p.landed_wealth_shock - p.wealth_shock);
    let want = beta * beta * 1.3 + shock;
    for (years, periods) in [(1, 28), (7, 4)] {
        let m = Inequality::from_params(&Params { period_ticks: 365 * years, ..p.clone() });
        let mut v = 1.3;
        for _ in 0..periods {
            v = m.next(v, owned);
        }
        assert!((v - want).abs() < 1e-6 * want, "{years}-year periods: {v}, want {want}");
        let stationary = shock / (1.0 - beta * beta);
        assert!((m.stationary(owned) - stationary).abs() < 1e-6 * stationary);
    }
}

#[test]
fn transmission_of_all_wealth_is_refused() {
    for p in [
        Params { wealth_transmission: 1.0, ..Params::default() },
        Params { landed_wealth_transmission: 1.0, ..Params::default() },
    ] {
        let err = p.validate().expect_err("refused");
        assert!(err.contains("transmission"), "{err}");
    }
}

#[test]
fn foragers_and_landowners_settle_at_the_inequality_of_their_economies() {
    // Hunter-gatherers: Gini 0.25 with transmission 0.19; agriculturalists 0.48 with 0.36
    // (Borgerhoff Mulder et al. 2009, Table 2), under lognormal wealth.
    let m = Inequality::from_params(&Params::default());
    assert!((gini(m.stationary(0.0)) - 0.25).abs() < 0.002, "{}", gini(m.stationary(0.0)));
    assert!((gini(m.stationary(1.0)) - 0.48).abs() < 0.002, "{}", gini(m.stationary(1.0)));
    let half = m.stationary(0.5);
    assert!(m.stationary(0.0) < half && half < m.stationary(1.0));
}

#[test]
fn settled_foragers_are_skipped_only_where_the_update_would_keep_them() {
    for p in
        [Params::default(), Params { wealth_transmission: 0.05, wealth_shock: 0.29, ..Params::default() }]
    {
        let m = Inequality::from_params(&p);
        let fixed = m.forager_fixed.expect("rounding keeps the forager level");
        assert_eq!(m.next_stored(fixed, 0.0), fixed);
        assert!((fixed as f64 / SHARE_ONE as f64 - m.stationary(0.0)).abs() < 1e-9);
    }
}

#[test]
fn land_is_closed_where_neighbors_farm_it_or_nobody_can_live() {
    // Ownership needs farming and land closed in on every side (Carneiro 1970): open
    // forager country is not, farmed neighbors and sea or desert are.
    assert_eq!(Inequality::closure([(true, 0.0), (true, 0.0)].into_iter()), 0.0);
    assert_eq!(Inequality::closure([(true, 1.0), (true, 1.0)].into_iter()), 1.0);
    assert_eq!(Inequality::closure([(true, 1.0), (false, 0.0)].into_iter()), 1.0, "sea or desert closes");
    assert_eq!(Inequality::closure([(true, 0.5), (true, 0.0)].into_iter()), 0.25);
    assert_eq!(Inequality::closure(std::iter::empty()), 1.0, "an island closed on every side");
}

#[test]
fn foragers_start_and_stay_as_equal_as_foragers() {
    let mut w = World::new(WorldConfig::base_cell(22, 3), 4, Params::default()).unwrap();
    w.run_years(1, 1, 1);
    let want = Inequality::from_params(&w.params).stationary(0.0);
    for a in w.agent_slices() {
        for &v in &a.inequality {
            let v = v as f64 / SHARE_ONE as f64;
            assert!((v - want).abs() < 1e-6 * want, "forager inequality {v}, stationary {want}");
        }
    }
}

#[test]
fn farmers_grow_unequal_as_farmland_closes_in() {
    // On the open frontier the first farmers stay about as equal as foragers
    // (horticulturalists, Borgerhoff Mulder et al. 2009); once farmers hold the land
    // around them, land is property and inequality rises toward the landed level.
    let (mut w, ..) = farming_world(3);
    let m = Inequality::from_params(&w.params);
    let (forager, landed) = (m.stationary(0.0), m.stationary(1.0));
    let mut early = None;
    let mut late = 0.0;
    for _ in 0..40 {
        w.run_years(100, 2, 100);
        let (settled, _) = mean_inequality(&w);
        let farmers = w.cultivation().iter().zip(&w.population).filter(|&(&e, &n)| n > 0 && e > 0.5).count();
        if early.is_none() && farmers > 0 {
            early = Some(settled);
        }
        late = settled;
    }
    let early = early.expect("farming began");
    assert!(early < forager + 0.3 * (landed - forager), "first farmers near the forager level: {early}");
    assert!(late > forager + 0.6 * (landed - forager), "a farmed landscape is unequal: {late}");
}

#[test]
fn inequality_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = farming_world(9);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(9);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    assert_eq!(restored.inequality(), w.inequality());
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn checkpoints_from_before_inequality_restore_with_foragers_inequality() {
    // The layout before ADR-0074: magic OIKOSCK4 and agents without their inequality.
    let (mut w, g, c) = farming_world(9);
    w.run_years(300, 2, 300);
    let now = checkpoint::encode(&w);
    let agents = w.agent_count();
    let at = now.len() - 32 * agents;
    let mut before = now[..at].to_vec();
    before[..8].copy_from_slice(b"OIKOSCK4");
    for r in now[at..].chunks(32) {
        before.extend_from_slice(&r[..28]);
    }
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    let forager = Inequality::foragers(&w.params).inequality;
    assert!(restored.agent_slices().all(|a| a.inequality.iter().all(|&v| v == forager)));
    assert_eq!(restored.cultivation(), w.cultivation());
}

#[test]
fn inequality_follows_cultivation() {
    let specs: Vec<_> = mechanisms::defaults(&Params::default()).iter().map(|m| m.spec()).collect();
    let order = Schedule::build(&specs).unwrap();
    let names: Vec<&str> = order.order().iter().map(|&i| specs[i].name).collect();
    assert_eq!(names.last(), Some(&"inequality"));
}

#[test]
fn settled_inequality_is_the_settled_people_weighted_mean() {
    let (mut w, ..) = farming_world(5);
    w.run_years(2500, 2, 2500);
    let per_cell = w.inequality();
    let (settled, _) = mean_inequality(&w);
    let (mut num, mut den) = (0.0, 0.0);
    for (c, &v) in per_cell.iter().enumerate() {
        let s = w.cultivation()[c] * w.population[c] as f64;
        num += s * v;
        den += s;
    }
    assert!(den > 0.0, "the test needs farmers");
    assert!((num / den - settled).abs() < 1e-6 * settled, "{} vs {settled}", num / den);
}
