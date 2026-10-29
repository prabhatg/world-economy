//! Cultivation and domestication (docs/04 §4.2, ADR-0049): bands shift effort into
//! cultivating wild cereals where they grow, driven by population pressure, climate
//! stress or abundance (competing hypotheses, nested as parameter weights); cultivation
//! selects for domestic traits, slowly or quickly depending on the selection hypothesis;
//! and domestic crops raise what the land can feed, which feeds back on pressure.
//! Practice travels with people.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
use oikos_sim::environment::{Farmland, cereal_suitability};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{self, Cultivation, Fission};
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

const AREA_KM2: f64 = 4_000.0;

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(vec![(AREA_KM2 * 1e6) as u32; h3.len()])).unwrap();
    b.build()
}

/// Temperature (degC) and precipitation (mm/yr) per (year, cell).
fn climate_pack(h3: &[u64], years: &[i64], climate: impl Fn(i64, usize) -> (f64, f64)) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(years.to_vec(), TimeRule::Linear);
    let (mut t, mut p) = (Vec::new(), Vec::new());
    for &y in years {
        for c in 0..h3.len() {
            let (tc, pm) = climate(y, c);
            t.push((tc * 100.0).round() as i16);
            p.push(pm.round() as u16);
        }
    }
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(t)).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(p)).unwrap();
    b.build()
}

fn id(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

fn world(packs: &[&[u8]], seed: u64, params: Params) -> World {
    World::with_inputs(WorldConfig::geometry(&id(packs[0])), seed, params, inputs(packs)).unwrap()
}

/// Only the named hypothesis drives cultivation; the world starts from a few founding
/// bands, so tests see the land fill. Toy fission is off: it sends half of a large band to
/// a random neighbor whatever its crowding, an overshoot that would read as pressure
/// (scalar-stress fission replaces it, #14); dispersal spreads people instead.
fn only(pressure: f64, stress: f64, abundance: f64) -> Params {
    Params {
        initial_occupancy: 0.0,
        fission_probability: 0.0,
        cultivation_pressure_weight: pressure,
        cultivation_stress_weight: stress,
        cultivation_abundance_weight: abundance,
        ..Params::default()
    }
}

/// Wild-cereal country everywhere: 15 degC, 500 mm.
fn cereal_country(_: i64, _: usize) -> (f64, f64) {
    (15.0, 500.0)
}

/// Mean cultivation effort over people.
fn mean_effort(w: &World) -> f64 {
    let (mut sum, mut people) = (0u128, 0u128);
    for a in w.agent_slices() {
        for i in 0..a.len() {
            sum += a.weight[i] as u128 * a.cultivation[i] as u128;
            people += a.weight[i] as u128;
        }
    }
    sum as f64 / (people as f64 * SHARE_ONE as f64)
}

#[test]
fn wild_cereals_grow_in_warm_country_with_winter_rain() {
    let p = Params::default();
    assert_eq!(cereal_suitability(15.0, 500.0, &p), 1.0, "oak-park woodland belt");
    assert_eq!(cereal_suitability(22.0, 80.0, &p), 0.0, "desert");
    assert_eq!(cereal_suitability(10.0, 1800.0, &p), 0.0, "wet forest");
    assert_eq!(cereal_suitability(-2.0, 500.0, &p), 0.0, "too cold");
    let edge = (p.cereal_precip_min + p.cereal_precip_full) / 2.0;
    assert!((cereal_suitability(15.0, edge, &p) - 0.5).abs() < 1e-12, "linear ramp at the dry edge");
    let cool = (p.cereal_temp_min + p.cereal_temp_full) / 2.0;
    assert!((cereal_suitability(cool, 500.0, &p) - 0.5).abs() < 1e-12, "linear ramp at the cold edge");
}

#[test]
fn hypotheses_set_the_target_effort() {
    let c = |p: Params| Cultivation::from_params(&p);
    // Pressure: none up to `crowding` times forager capacity, full weight one capacity beyond.
    let pressure = c(Params { cultivation_crowding: 1.0, ..only(0.6, 0.0, 0.0) });
    assert_eq!(pressure.target_effort(100, 100, 1.0, 0.0, 0.0), 0.0);
    assert!((pressure.target_effort(150, 100, 1.0, 0.0, 0.0) - 0.3).abs() < 1e-12);
    assert!((pressure.target_effort(400, 100, 1.0, 0.0, 0.0) - 0.6).abs() < 1e-12);
    // Stress: proportional to the decline of forager capacity, at any density.
    let stress = c(only(0.0, 2.0, 0.0));
    assert!((stress.target_effort(1, 100, 1.0, 0.2, 0.0) - 0.4).abs() < 1e-12);
    // Abundance: proportional to wild-cereal suitability, at any density.
    let abundance = c(only(0.0, 0.0, 0.5));
    assert!((abundance.target_effort(1, 100, 0.8, 0.0, 0.0) - 0.4).abs() < 1e-12);
    // No wild cereals, nothing to cultivate; effort never exceeds everything.
    let all = c(only(1.0, 5.0, 1.0));
    assert_eq!(all.target_effort(500, 100, 0.0, 0.5, 0.0), 0.0);
    assert_eq!(all.target_effort(500, 100, 1.0, 0.5, 0.0), 1.0);
}

/// Years of full-effort cultivation until half the crop is of the domestic type.
fn years_to_half(selection: f64) -> u32 {
    let c = Cultivation::from_params(&Params { selection_coefficient: selection, ..Params::default() });
    let mut d = 0.0;
    for year in 1..20_000 {
        d = c.domesticate(d, 1.0);
        if d >= 0.5 {
            return year;
        }
    }
    panic!("never domesticated at s = {selection}");
}

#[test]
fn domestication_is_protracted_under_weak_selection_and_rapid_under_strong() {
    // Purugganan and Fuller (2011): s ~ 0.003 from the archaeobotanical record, millennia;
    // Hillman and Davies (1990): sickle harvest and resowing, decades to two centuries.
    let slow = years_to_half(0.003);
    assert!((1500..4000).contains(&slow), "protracted: {slow} years");
    let fast = years_to_half(0.2);
    assert!(fast < 100, "rapid: {fast} years");
}

#[test]
fn domestic_traits_fade_without_cultivation() {
    let c = Cultivation::from_params(&Params::default());
    let mut d = 0.8;
    for _ in 0..3000 {
        d = c.domesticate(d, 0.0);
    }
    assert!(d < 0.05, "gene flow from wild stands and abandoned fields: {d}");
    // Light cultivation does not domesticate: selection loses to wild gene flow.
    let mut d = 0.0;
    for _ in 0..5000 {
        d = c.domesticate(d, 0.05);
    }
    assert!(d < 0.01, "{d}");
}

#[test]
fn toy_worlds_never_cultivate() {
    let mut w = World::new(WorldConfig::base_cell(22, 3), 4, Params::default()).unwrap();
    let start = w.capacity.clone();
    w.run_years(800, 2, 800);
    assert!(w.agent_slices().all(|a| a.cultivation.iter().all(|&e| e == 0)));
    assert!(w.agent_slices().all(|a| a.domestication.iter().all(|&d| d == 0)));
    assert_eq!(w.capacity, start, "the M0 toy keeps its capacity");
}

#[test]
fn without_any_driver_nobody_cultivates() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let mut w = world(&[&g, &c], 1, only(0.0, 0.0, 0.0));
    w.run_years(2500, 2, 2500);
    assert_eq!(mean_effort(&w), 0.0);
    assert_eq!(w.capacity, w.forage_capacity);
}

#[test]
fn pressure_brings_cultivation_where_people_outgrow_forager_capacity() {
    // A full land; in one world the rain falls from 500 to 280 mm after 11,000 BCE,
    // cutting forager capacity by 40%, in the other it does not.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let drying = climate_pack(&h3, &[-12_000, -11_000, -10_900, -9_000], |y, _| {
        (15.0, if y >= -10_900 { 280.0 } else { 500.0 })
    });
    let stable = climate_pack(&h3, &[-12_000], cereal_country);
    let p = Params { initial_occupancy: 0.9, ..only(0.8, 0.0, 0.0) };
    let mut dry = world(&[&g, &drying], 2, p.clone());
    let mut wet = world(&[&g, &stable], 2, p);
    dry.run_years(900, 2, 900);
    assert_eq!(mean_effort(&dry), 0.0, "foragers at equilibrium feel no pressure");
    dry.run_years(700, 2, 700);
    wet.run_years(1600, 2, 1600);
    assert_eq!(mean_effort(&wet), 0.0);
    assert!(mean_effort(&dry) > 0.2, "{}", mean_effort(&dry));
}

#[test]
fn stress_brings_cultivation_where_the_climate_worsens() {
    // Odd cells dry out from 500 to 300 mm between 11,000 and 10,800 BCE (a Younger Dryas
    // in miniature); even cells keep their climate.
    let h3 = h3_cells();
    let dry = |c: usize| c % 2 == 1;
    let climate = climate_pack(&h3, &[-12_000, -11_000, -10_800, -9_000], |y, c| {
        (15.0, if dry(c) && y >= -10_800 { 300.0 } else { 500.0 })
    });
    let g = geometry_pack(&h3);
    let mut w = world(&[&g, &climate], 3, only(0.0, 3.0, 0.0));
    w.run_years(1100, 2, 1100); // 10,900 BCE
    assert!(w.farmland.stress.iter().any(|&s| s > 0.1));
    assert!(w.farmland.stress.iter().enumerate().all(|(c, &s)| dry(c) || s == 0.0));
    w.run_years(200, 2, 200); // 10,700 BCE
    let effort = w.cultivation();
    let (mut stressed, mut stable) = (vec![], vec![]);
    for (c, &e) in effort.iter().enumerate() {
        if w.population[c] > 0 {
            if dry(c) { stressed.push(e) } else { stable.push(e) }
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    assert!(!stressed.is_empty() && !stable.is_empty());
    // Stable cells cultivate only through migrants from stressed neighbors.
    assert!(
        mean(&stressed) > 0.1 && mean(&stable) < mean(&stressed) / 4.0,
        "{} vs {}",
        mean(&stressed),
        mean(&stable)
    );
}

#[test]
fn abundance_brings_cultivation_where_wild_cereals_grow_at_any_density() {
    // Even cells grow wild cereals; odd cells are too wet (forest) for them.
    let h3 = h3_cells();
    let climate = climate_pack(&h3, &[-12_000], |_, c| (15.0, if c % 2 == 0 { 500.0 } else { 1900.0 }));
    let g = geometry_pack(&h3);
    let mut w = world(&[&g, &climate], 4, only(0.0, 0.0, 0.5));
    w.run_years(300, 2, 300);
    let effort = w.cultivation();
    let held = |even: bool| -> Vec<f64> {
        (0..h3.len()).filter(|&c| w.population[c] > 0 && (c % 2 == 0) == even).map(|c| effort[c]).collect()
    };
    let (rich, forest) = (held(true), held(false));
    // Bands that have lived in cereal country approach the target (0.5 x suitability 1);
    // newcomers from the forest are still catching up.
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    assert!(!rich.is_empty() && mean(&rich) > 0.4, "{rich:?}");
    // Forest cells cultivate only through migrants from cereal country, and stop.
    assert!(!forest.is_empty() && mean(&forest) < mean(&rich) / 3.0, "{forest:?}");
}

#[test]
fn cultivation_and_domestication_ratchet_up_capacity_and_population() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = Params { selection_coefficient: 0.02, ..only(0.8, 0.0, 0.2) };
    let mut w = world(&[&g, &c], 5, p);
    w.run_years(5000, 3, 5000);
    let occupied: Vec<usize> = (0..h3.len()).filter(|&c| w.population[c] > 0).collect();
    let forage: u64 = occupied.iter().map(|&c| w.forage_capacity[c]).sum();
    let people: u64 = occupied.iter().map(|&c| w.population[c]).sum();
    assert!(people > 2 * forage, "farmers outnumber what foraging could feed: {people} vs {forage}");
    let dom = w.domestication();
    let farmed: Vec<usize> = occupied.iter().copied().filter(|&c| w.cultivation()[c] > 0.5).collect();
    assert!(farmed.len() > occupied.len() / 2);
    assert!(farmed.iter().all(|&c| dom[c] > 0.5), "long cultivation domesticates the crop");
    assert!(farmed.iter().all(|&c| w.capacity[c] > 2 * w.forage_capacity[c]));
}

#[test]
fn practice_travels_with_people() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    // Fission's daughters push the front out fastest; pressure is off, so overshoot is harmless.
    let p = Params { fission_probability: Params::default().fission_probability, ..only(0.0, 0.0, 0.6) };
    let mut w = world(&[&g, &c], 6, p.clone());
    w.run_years(300, 2, 300);
    let before = w.cultivation();
    assert!(before.iter().any(|&e| e > 0.5));
    // From here nobody learns or unlearns: effort can only move with people.
    w.set_params(Params { adoption_rate: 0.0, ..p }).unwrap();
    w.run_years(1000, 2, 1000);
    let after = w.cultivation();
    let newly: Vec<usize> = (0..h3.len()).filter(|&c| w.population[c] > 0 && before[c] == 0.0).collect();
    let colonists: Vec<f64> = newly.iter().map(|&c| after[c]).collect();
    assert!(colonists.len() > 20, "the front moved: {} new cells", colonists.len());
    assert!(colonists.iter().all(|&e| e > 0.3), "colonists cultivate as at home: {colonists:?}");
}

#[test]
fn merging_keeps_practice() {
    // Without births or learning, splitting and merging bands move effort but keep its
    // people-weighted total (rounding down by less than one part per billion per merge).
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = only(0.0, 0.0, 0.6);
    let mut w = world(&[&g, &c], 7, p.clone());
    w.run_years(800, 2, 800);
    w.set_mechanisms(vec![Box::new(Fission {
        after: vec![],
        probability: 1.0,
        threshold: 2,
        ..Fission::from_params(&p)
    })])
    .unwrap();
    let mass = |w: &World| -> u128 {
        w.agent_slices()
            .map(|a| (0..a.len()).map(|i| a.weight[i] as u128 * a.cultivation[i] as u128).sum::<u128>())
            .sum()
    };
    let m0 = mass(&w);
    for _ in 0..10 {
        assert!(w.step(2).fissions > 0);
    }
    let m1 = mass(&w);
    assert!(m0 > 0 && m1 <= m0 && m0 - m1 < m0 / 1_000_000, "effort mass {m0} -> {m1}");
}

#[test]
fn cultivation_is_partition_independent_and_restores() {
    let h3 = h3_cells();
    let climate = climate_pack(&h3, &[-12_000, -11_000], |y, c| {
        (15.0, if y < -11_500 { 450.0 } else { 300.0 + 40.0 * (c % 7) as f64 })
    });
    let g = geometry_pack(&h3);
    let p = Params { selection_coefficient: 0.05, ..Params::default() };
    let run = |parts| {
        let mut w = world(&[&g, &climate], 9, p.clone());
        w.run_years(1500, parts, 1500);
        assert!(mean_effort(&w) > 0.0, "the test must exercise cultivation");
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let mut w = world(&[&g, &climate], 9, p.clone());
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &climate])).unwrap();
    assert_eq!(restored.capacity, w.capacity);
    assert_eq!(restored.cultivation(), w.cultivation());
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn cultivation_runs_after_bands_move() {
    let specs: Vec<_> = mechanisms::defaults(&Params::default()).iter().map(|m| m.spec()).collect();
    let order = Schedule::build(&specs).unwrap();
    let names: Vec<&str> = order.order().iter().map(|&i| specs[i].name).collect();
    assert_eq!(names, ["growth", "fission", "dispersal", "inundation", "cultivation"]);
}

#[test]
fn abandoned_cultivation_dies_out_completely() {
    // Bands that stop cultivating (or carry it into land without wild cereals) lose the
    // practice and the domestic crop entirely, not down to a rounding floor.
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = only(0.0, 0.0, 0.6);
    let mut w = world(&[&g, &c], 10, p.clone());
    w.run_years(400, 2, 400);
    assert!(mean_effort(&w) > 0.3);
    w.set_params(Params { introgression_rate: 0.05, ..only(0.0, 0.0, 0.0) }).unwrap();
    w.run_years(2000, 2, 2000);
    assert_eq!(mean_effort(&w), 0.0);
    assert!(w.agent_slices().all(|a| a.domestication.iter().all(|&d| d == 0)));
    assert_eq!(w.capacity, w.forage_capacity);
}

#[test]
fn sedentary_cultivators_grow_faster_than_mobile_foragers() {
    // The Neolithic demographic transition (Bocquet-Appel 2002, 2011): intrinsic growth
    // moves from the forager rate to the sedentary rate with cultivation effort.
    let p = Params { growth_rate: 0.002, sedentary_growth_rate: 0.03, ..Params::default() };
    let g = mechanisms::Growth::from_params(&p);
    assert_eq!(g.rate(0.0), 0.002);
    assert_eq!(g.rate(1.0), 0.03);
    assert!((g.rate(0.5) - 0.016).abs() < 1e-15);
}

#[test]
fn the_demographic_transition_follows_cultivation() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let run = |precip: f64, sedentary: f64| {
        let c = climate_pack(&h3, &[-12_000], |_, _| (15.0, precip));
        let p = Params { growth_rate: 0.002, sedentary_growth_rate: sedentary, ..only(0.0, 0.0, 0.8) };
        let mut w = world(&[&g, &c], 6, p);
        w.run_years(600, 2, 600);
        w.total_population()
    };
    // Cereal country: bands cultivate, settle and grow faster.
    let (transition, none) = (run(500.0, 0.03), run(500.0, 0.002));
    assert!(transition > 2 * none, "{transition} vs {none}");
    // Forest: nobody cultivates, so the sedentary rate changes nothing.
    assert_eq!(run(1900.0, 0.03), run(1900.0, 0.002));
}

#[test]
fn variability_lowers_the_target_effort() {
    // The stable-climate hypothesis (Richerson, Boyd and Bettinger 2001): sowing pays
    // only where harvests are reliable. Weight 0 switches it off.
    let p = Params { cultivation_variability_weight: 4.0, ..only(0.0, 0.0, 0.5) };
    let c = Cultivation::from_params(&p);
    assert!((c.target_effort(1, 100, 1.0, 0.0, 0.0) - 0.5).abs() < 1e-12);
    assert!((c.target_effort(1, 100, 1.0, 0.0, 0.1) - 0.3).abs() < 1e-12);
    assert_eq!(c.target_effort(1, 100, 1.0, 0.0, 0.3), 0.0, "too variable to sow at all");
    let off = Cultivation::from_params(&only(0.0, 0.0, 0.5));
    assert_eq!(off.target_effort(1, 100, 1.0, 0.0, 0.3), 0.5);
}

/// Every century from 12,000 BCE, rain swings between 500 mm (suitability 1) and 275 mm
/// (suitability 0.5) if `swinging`, else stays at 500 mm.
fn swinging_climate(h3: &[u64], swinging: bool) -> Vec<u8> {
    let years: Vec<i64> = (0..=40).map(|i| -12_000 + 100 * i).collect();
    climate_pack(h3, &years, move |y, _| (15.0, if swinging && (y / 100) % 2 != 0 { 275.0 } else { 500.0 }))
}

#[test]
fn climate_variability_is_the_mean_change_in_suitability_per_century() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let p = Params { stress_window_years: 300, ..Params::default() };
    for (swinging, expected) in [(true, 0.5), (false, 0.0)] {
        let c = swinging_climate(&h3, swinging);
        let f = Farmland::new(&inputs(&[&g, &c]), -11_500, &p, h3.len());
        assert!(
            f.variability.iter().all(|&v| (v - expected).abs() < 1e-9),
            "{swinging}: {:?}",
            &f.variability[..3]
        );
    }
}

#[test]
fn nobody_sows_where_the_climate_is_too_variable() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let run = |swinging: bool, weight: f64| {
        let c = swinging_climate(&h3, swinging);
        let p =
            Params { cultivation_variability_weight: weight, initial_occupancy: 0.5, ..only(0.0, 0.0, 0.8) };
        let mut w = world(&[&g, &c], 7, p);
        // The window fills over the first three centuries; effort sown then dies out.
        w.run_years(1000, 2, 1000);
        mean_effort(&w)
    };
    assert!(run(true, 0.0) > 0.3, "abundance alone brings cultivation");
    let swinging = run(true, 2.0);
    assert!(swinging < 0.01, "swinging harvests keep it away: {swinging}");
    assert_eq!(run(false, 2.0), run(false, 0.0), "a stable climate is unaffected");
}
