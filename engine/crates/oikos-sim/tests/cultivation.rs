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
use oikos_sim::mechanisms::{self, Aggregation, Cultivation, Fission};
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
///
/// The other hypotheses are off and demography, densities and domestication rates are
/// fixed at the values these tests were written for (the registry defaults before M1
/// calibration, ADR-0057), so a calibration round that moves the defaults never moves
/// what the tests check.
fn only(pressure: f64, stress: f64, abundance: f64) -> Params {
    Params {
        initial_occupancy: 0.0,
        fission_probability: 0.0,
        cultivation_pressure_weight: pressure,
        cultivation_stress_weight: stress,
        cultivation_abundance_weight: abundance,
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        soil_formation_rate: 0.0001,
        growth_rate: 0.01,
        sedentary_growth_rate: 0.01,
        dispersal_probability: 0.1,
        forager_density_ref: 0.1,
        farm_density: 2.0,
        selection_coefficient: 0.005,
        introgression_rate: 0.002,
        domestic_wild_frequency: 0.001,
        dispersal_crowding: 0.9,
        npp_elasticity: 1.0,
        soil_critical: 0.5,
        wild_yield_ratio: 0.5,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        aggregation_rate: 0.0,
        disease_mortality: 0.0,
        neighbor_learning: 0.0,
        success_bias: 0.0,
        settlement_integration: 1.0,
        settlement_hierarchy: 0.0,
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

/// Years of full-effort cultivation until half the crop is of the domestic type, from the
/// wild mutant frequency and introgression `only` pins.
fn years_to_half(selection: f64) -> u32 {
    let c = Cultivation::from_params(&Params { selection_coefficient: selection, ..only(0.0, 0.0, 0.0) });
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
    let c = Cultivation::from_params(&only(1.0, 1.0, 1.0));
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
fn each_wheat_domesticates_at_its_own_pace() {
    // Allaby et al. (2017): under the same cultivation, emmer and einkorn gained domestic
    // traits more slowly than barley, at a fraction of its selection.
    let c = Cultivation::from_params(&Params {
        selection_coefficient: 0.01,
        emmer_selection_ratio: 0.5,
        einkorn_selection_ratio: 0.0,
        ..only(0.0, 0.0, 0.0)
    });
    let (mut barley, mut emmer, mut einkorn) = (0.0, 0.0, 0.0);
    for _ in 0..1500 {
        barley = c.domesticate(barley, 1.0);
        [emmer, einkorn] = c.domesticate_wheats([emmer, einkorn], 1.0);
    }
    assert!(barley > 0.99 && emmer > 0.2 && emmer < barley / 2.0, "{barley} {emmer}");
    assert_eq!(einkorn, 0.0, "without selection the wild type stays");
    // At barley's own selection a wheat follows it exactly.
    let same = Cultivation::from_params(&Params {
        emmer_selection_ratio: 1.0,
        einkorn_selection_ratio: 1.0,
        ..only(0.0, 0.0, 0.0)
    });
    let d = same.domesticate(0.3, 0.4);
    assert_eq!(same.domesticate_wheats([0.3, 0.3], 0.4), [d, d]);
}

#[test]
fn the_wheats_ride_on_the_same_fields_without_feeding_back() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = Params { selection_coefficient: 0.02, ..only(0.8, 0.0, 0.2) };
    let run = |emmer, einkorn| {
        let p = Params { emmer_selection_ratio: emmer, einkorn_selection_ratio: einkorn, ..p.clone() };
        let mut w = world(&[&g, &c], 5, p);
        w.run_years(3000, 3, 3000);
        w
    };
    let same = run(1.0, 1.0);
    assert!(
        same.agent_slices().all(|a| a
            .wheat_domestication
            .iter()
            .zip(&a.domestication)
            .all(|(w, &d)| *w == [d, d]))
    );
    let slower = run(0.5, 0.2);
    assert_eq!(slower.population, same.population, "only the crop barley stands for feeds people");
    let (dom, emmer, einkorn) =
        (slower.domestication(), slower.emmer_domestication(), slower.einkorn_domestication());
    let farmed: Vec<usize> = (0..h3.len()).filter(|&c| slower.cultivation()[c] > 0.5).collect();
    assert!(!farmed.is_empty());
    for &c in &farmed {
        assert!(
            dom[c] >= emmer[c] && emmer[c] >= einkorn[c] && einkorn[c] > 0.0,
            "{} {} {}",
            dom[c],
            emmer[c],
            einkorn[c]
        );
    }
    // Fields worked fully keep every crop domestic; the rest lag by their selection.
    let mean = |x: &[f64]| farmed.iter().map(|&c| x[c]).sum::<f64>() / farmed.len() as f64;
    assert!(
        mean(&dom) > mean(&emmer) && mean(&emmer) > mean(&einkorn),
        "{} {} {}",
        mean(&dom),
        mean(&emmer),
        mean(&einkorn)
    );
}

#[test]
fn seed_exchange_moves_a_bands_seed_toward_what_is_sown_around_it() {
    // Farmers renew and replace seed from kin and neighbours (Pautasso et al. 2013), as
    // much as they sow.
    let c = Cultivation::from_params(&Params { seed_exchange: 0.2, ..only(0.0, 0.0, 0.0) });
    assert_eq!(c.exchange_seed(0.1, 0.5, 0.9), 0.1 + 0.2 * 0.5 * (0.9 - 0.1));
    assert_eq!(c.exchange_seed(0.1, 0.0, 0.9), 0.1, "a band that sows nothing takes no seed");
    assert_eq!(c.exchange_seed(0.9, 1.0, 0.1), 0.9 - 0.2 * 0.8, "wild seed comes too");
    let off = Cultivation::from_params(&Params { seed_exchange: 0.0, ..only(0.0, 0.0, 0.0) });
    assert_eq!(off.exchange_seed(0.1, 1.0, 0.9), 0.1, "off at zero");
    assert_eq!(Params::default().seed_exchange, 0.3, "calibrated by profile on M1 (ADR-0105)");
    let all = Cultivation::from_params(&Params { seed_exchange: 1.0, ..only(0.0, 0.0, 0.0) });
    assert_eq!(all.exchange_seed(0.1, 1.0, 0.9), 0.9, "never past what it sees");
}

/// Core cells (even) grow wild cereals fully; margin cells (odd), at 230 mm, a fifth as
/// well, so few there sow.
fn core_and_margin(_: i64, c: usize) -> (f64, f64) {
    if c % 2 == 0 { (15.0, 500.0) } else { (15.0, 230.0) }
}

#[test]
fn seed_exchange_carries_domestic_seed_to_the_margins() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], core_and_margin));
    let p = Params {
        selection_coefficient: 0.02,
        emmer_selection_ratio: 0.5,
        einkorn_selection_ratio: 0.2,
        neighbor_learning: 1.0,
        ..only(0.0, 0.0, 1.0)
    };
    let run = |exchange| {
        let mut w = world(&[&g, &c], 5, Params { seed_exchange: exchange, ..p.clone() });
        w.run_years(2500, 3, 2500);
        w
    };
    let (alone, exchanging) = (run(0.0), run(0.3));
    // Sown-weighted shares of the margin's crops, and of the core's.
    let sown = |w: &World, margin: bool, share: fn(&World) -> Vec<f64>| {
        let (d, e) = (share(w), w.cultivation());
        let cells = (0..h3.len()).filter(|&c| (c % 2 == 1) == margin && e[c] > 0.0);
        let (x, y) = cells.fold((0.0, 0.0), |(x, y), c| (x + e[c] * d[c], y + e[c]));
        assert!(y > 0.0, "some sow in the {}", if margin { "margin" } else { "core" });
        x / y
    };
    let crops: [fn(&World) -> Vec<f64>; 3] =
        [World::domestication, World::emmer_domestication, World::einkorn_domestication];
    for crop in crops {
        let (margin, core) = (sown(&alone, true, crop), sown(&alone, false, crop));
        let traded = sown(&exchanging, true, crop);
        assert!(margin < core, "alone, the margin's few plots lag: {margin} {core}");
        assert!(traded > margin + 0.05, "seed from the core: {traded} against {margin}");
    }
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
        village: 0,
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
    // Seed exchange reads the sums of neighbouring cells, which partitions split (#48).
    let p = Params { selection_coefficient: 0.05, seed_exchange: 0.2, ..Params::default() };
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
    assert_eq!(
        names,
        [
            "growth",
            "disease",
            "fission",
            "dispersal",
            "inundation",
            "aggregation",
            "cultivation",
            "inequality"
        ]
    );
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
fn births_and_deaths_split_the_logistic_change() {
    // Fertility moves with cultivation effort as growth does (Bocquet-Appel 2002); mortality
    // takes the rest, so the net change is still logistic and bands still settle at capacity.
    let p = Params {
        growth_rate: 0.01,
        sedentary_growth_rate: 0.02,
        fertility: 0.03,
        sedentary_fertility: 0.05,
        ..Params::default()
    };
    let g = mechanisms::Growth::from_params(&p);
    assert_eq!((g.fertility(0.0), g.fertility(1.0)), (0.03, 0.05));
    let (b, d) = g.vital_rates(0.5, 50.0, 100.0);
    assert!((b - 0.04).abs() < 1e-15);
    assert!((b - d - 0.015 * 0.5).abs() < 1e-15, "net change r (1 - N/K)");
    let (b, d) = g.vital_rates(1.0, 100.0, 100.0);
    assert_eq!(b, d, "at capacity births and deaths balance");
    let (b, d) = g.vital_rates(0.0, 150.0, 100.0);
    assert!(d > b && (d - b - 0.005).abs() < 1e-15, "beyond capacity deaths exceed births");
    // A population cannot grow faster than it bears children.
    let fast = mechanisms::Growth::from_params(&Params { growth_rate: 0.05, fertility: 0.03, ..p });
    assert_eq!(fast.vital_rates(0.0, 0.0, 100.0), (0.03, 0.0));
}

#[test]
fn cultivators_bear_more_children_at_the_same_numbers() {
    // The transition's signature in the skeletal record (Bocquet-Appel 2002) is fertility:
    // in a world at capacity, higher sedentary fertility means more births and as many
    // more deaths, not more people.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let c = climate_pack(&h3, &[-12_000], |_, _| (15.0, 500.0));
    let run = |sedentary: f64| {
        let p = Params { fertility: 0.0325, sedentary_fertility: sedentary, ..only(0.0, 0.0, 0.8) };
        let mut w = world(&[&g, &c], 6, p);
        w.run_years(600, 2, 600);
        let (mut births, before) = (0, w.total_population());
        w.run_years_observed(100, 2, 100, &mut |_, s| {
            births += s.births;
            Ok(())
        })
        .unwrap();
        (births as f64 / (100.0 * (before + w.total_population()) as f64 / 2.0), w.total_population())
    };
    let ((none, people_none), (transition, people)) = (run(0.0325), run(0.06));
    assert!((none - 0.0325).abs() < 0.002, "births per person-year at forager fertility: {none}");
    assert!(transition > none + 0.01, "{transition} vs {none}");
    assert!((people as f64 / people_none as f64 - 1.0).abs() < 0.05, "{people} vs {people_none}");
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

#[test]
fn soil_erodes_under_fields_and_forms_slowly() {
    use oikos_sim::environment::{erode, soil_yield};
    let p = Params {
        soil_erosion_rate: 0.0004,
        soil_formation_rate: 0.0001,
        soil_critical: 0.5,
        ..Params::default()
    };
    // Erosion takes depth at a steady rate under fields; formation refills toward full.
    assert!((erode(0.5, 1.0, 10.0, 1.0, &p) - (0.5 - 0.004 + 0.0005)).abs() < 1e-15);
    assert!((erode(0.5, 0.0, 10.0, 1.0, &p) - 0.5005).abs() < 1e-15, "fallow land re-forms slowly");
    assert_eq!(erode(0.001, 1.0, 10.0, 1.0, &p), 0.0, "never below bare rock");
    assert_eq!(erode(1.0, 0.0, 10.0, 1.0, &p), 1.0);
    // The same depth is a larger share of a thin soil (#24): half as deep, twice the share.
    assert!((erode(0.5, 1.0, 10.0, 0.5, &p) - (0.5 - 0.008 + 0.0005)).abs() < 1e-15);
    // Yields hold while roots have enough soil, then fall linearly to nothing.
    assert_eq!(soil_yield(0.8, &p), 1.0);
    assert_eq!(soil_yield(0.5, &p), 1.0);
    assert!((soil_yield(0.25, &p) - 0.5).abs() < 1e-15);
    assert_eq!(soil_yield(0.0, &p), 0.0);
    // Rate zero switches the hypothesis off: full soil stays full.
    assert_eq!(erode(1.0, 1.0, 10.0, 0.5, &Params { soil_erosion_rate: 0.0, ..p }), 1.0);
}

/// A cereal-country world that ratchets into farming (as in the ratchet test).
fn farming_world(seed: u64, extra: impl Fn(Params) -> Params) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    // Foragers everywhere from the start, as in M1, so the land does not fill meanwhile.
    let p = extra(Params { selection_coefficient: 0.02, initial_occupancy: 0.5, ..only(0.8, 0.0, 0.2) });
    (world(&[&g, &c], seed, p), g, c)
}

#[test]
fn eroding_fields_turn_a_farming_boom_into_a_bust() {
    let trajectory = |erosion: f64| {
        let (mut w, ..) =
            farming_world(5, |p| Params { soil_erosion_rate: erosion, soil_critical: 0.5, ..p });
        let mut people = Vec::new();
        for _ in 0..32 {
            w.run_years(250, 3, 250);
            people.push(w.total_population());
        }
        (people, w.soil().iter().sum::<f64>() / w.soil().len() as f64)
    };
    let (eroding, soil) = trajectory(0.0004);
    let peak = *eroding.iter().max().unwrap();
    let at = eroding.iter().position(|&n| n == peak).unwrap();
    assert!(at > 2 && at < eroding.len() - 4, "a boom first: peak at {at} of {eroding:?}");
    assert!(*eroding.last().unwrap() < peak * 7 / 10, "then a bust: {eroding:?}");
    assert!(soil < 0.5, "fields have worn the soil down: {soil}");
    let (lasting, soil) = trajectory(0.0);
    let peak = *lasting.iter().max().unwrap();
    assert!(*lasting.last().unwrap() > peak * 95 / 100, "without erosion farming lasts: {lasting:?}");
    assert_eq!(soil, 1.0);
}

/// Depth to bedrock as a share of 2 m (#24): `depth(c)`, or no data where `None`.
fn bedrock_pack(h3: &[u64], depth: impl Fn(usize) -> Option<f64>) -> Vec<u8> {
    let mut b = PackBuilder::new("bedrock", region(h3));
    let v = (0..h3.len()).map(|c| depth(c).map_or(u16::MAX, |d| (d * 1e4).round() as u16)).collect();
    b.field(
        FieldSpec { nodata: Some(u16::MAX as i64), ..spec("depth_share", Dims::Cell, 1e-4) },
        Data::U16(v),
    )
    .unwrap();
    b.build()
}

#[test]
fn thin_soils_wear_down_to_rock_first() {
    // Fields take the same depth everywhere, so a soil a quarter as deep loses its share
    // four times as fast (Montgomery 2007); cells without data are deep.
    let (_, g, c) = farming_world(5, |p| p);
    let b = bedrock_pack(&h3_cells(), |c| match c % 3 {
        0 => Some(0.25),
        1 => Some(1.0),
        _ => None,
    });
    let p = Params {
        selection_coefficient: 0.02,
        initial_occupancy: 0.5,
        soil_erosion_rate: 0.0002,
        ..only(0.8, 0.0, 0.2)
    };
    let mut w = world(&[&g, &c, &b], 5, p);
    w.run_years(2000, 3, 2000);
    let soil = w.soil();
    let mean = |k: usize| {
        let v: Vec<f64> = soil.iter().enumerate().filter(|(c, _)| c % 3 == k).map(|(_, &s)| s).collect();
        v.iter().sum::<f64>() / v.len() as f64
    };
    let (thin, deep, unknown) = (mean(0), mean(1), mean(2));
    assert!(deep < 1.0, "the test must exercise erosion: {deep}");
    assert!(1.0 - thin > 2.5 * (1.0 - deep), "thin soils lose more of their share: {thin} against {deep}");
    assert!(
        (1.0 - unknown) < 1.5 * (1.0 - deep) && (1.0 - deep) < 1.5 * (1.0 - unknown),
        "no data reads as deep"
    );
}

#[test]
fn soil_is_partition_independent_and_restores() {
    let erosion = |p| Params { soil_erosion_rate: 0.001, ..p };
    let run = |parts| {
        let (mut w, ..) = farming_world(9, erosion);
        w.run_years(1500, parts, 1500);
        assert!(w.soil().iter().any(|&s| s < 1.0), "the test must exercise erosion");
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(9, erosion);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    assert_eq!(restored.soil(), w.soil());
    assert_eq!(restored.capacity, w.capacity);
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

/// A checkpoint's agents as layouts before ADR-0074 wrote them (no inequality, nor
/// settlements), and the checkpoint its restored world encodes to (every group at
/// foragers' inequality, in a settlement of its own).
fn before_inequality(now: &[u8], agents: usize, params: &Params) -> (Vec<u8>, Vec<u8>) {
    let forager = oikos_sim::mechanisms::Inequality::foragers(params).inequality.to_le_bytes();
    let at = now.len() - 52 * agents;
    let (mut before, mut restored) = (now[..at].to_vec(), now[..at].to_vec());
    for r in now[at..].chunks(52) {
        before.extend_from_slice(&r[..28]);
        restored.extend_from_slice(&r[..28]);
        restored.extend_from_slice(&forager);
        restored.extend_from_slice(&0u32.to_le_bytes());
        restored.extend_from_slice(&[0; 8]); // no herding (#37)
        restored.extend_from_slice(&r[24..28]); // wheats as domestic as the crop (#38)
        restored.extend_from_slice(&r[24..28]);
    }
    (before, restored)
}

#[test]
fn checkpoints_from_before_soil_restore_with_whole_soil() {
    let (mut w, g, c) = farming_world(9, |p| Params { soil_erosion_rate: 0.0, ..p });
    w.run_years(300, 2, 300);
    let now = checkpoint::encode(&w);
    // The layout before soil: magic OIKOSCK2 and nothing between birth counters and
    // agents (neither soil nor the obsidian source count that followed it).
    let cells = w.population.len();
    let params_len = serde_json::to_vec(&w.params).unwrap().len();
    let header = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8 + 4 + params_len + 4;
    let soil_at = header + 16 * cells;
    let (mut before, zeroed) = before_inequality(&now, w.agent_count(), &w.params);
    before[..8].copy_from_slice(b"OIKOSCK2");
    before.drain(soil_at..soil_at + 4 * cells + 4);
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    assert!(restored.soil().iter().all(|&s| s == 1.0));
    assert_eq!(checkpoint::encode(&restored), zeroed, "nothing eroded, so nothing is lost");
    let mut bad = now.clone();
    bad[soil_at..soil_at + 4].copy_from_slice(&(SHARE_ONE + 1).to_le_bytes());
    let err = checkpoint::decode_with(&bad, inputs(&[&g, &c])).err().expect("refused");
    assert!(err.contains("soil"), "{err}");
}

#[test]
fn bands_that_learn_from_others_move_at_the_pace_of_what_they_see() {
    // Social learning (Bass 1969; Henrich 2001): a band closes the gap to its target at
    // adoption_rate * (i + (1 - i) * seen), `seen` being the mean effort of the people in
    // its cell and i the innovation share. i = 1 (as `only` pins it) is individual learning.
    let p = Params { innovation_share: 0.01, adoption_rate: 0.1, ..only(0.0, 0.0, 0.5) };
    let c = Cultivation::from_params(&p);
    assert!(
        (c.adopt(0.0, 0.5, 0.0) - 0.1 * 0.01 * 0.5).abs() < 1e-12,
        "nobody around plants: only innovation"
    );
    assert!((c.adopt(0.0, 0.5, 1.0) - 0.1 * 0.5).abs() < 1e-12, "everyone around plants: the full rate");
    assert!((c.adopt(0.4, 0.0, 0.5) - (0.4 - 0.1 * 0.505 * 0.4)).abs() < 1e-12, "unlearning too");
    let alone = Cultivation::from_params(&only(0.0, 0.0, 0.5));
    assert_eq!(alone.adopt(0.0, 0.5, 0.0), alone.adopt(0.0, 0.5, 1.0), "individual learning ignores others");
}

/// Mean effort after each of `years` (cumulative), in a full land of wild cereals where
/// abundance alone drives cultivation.
fn effort_path(innovation_share: f64, years: &[u64]) -> Vec<f64> {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = Params { initial_occupancy: 0.9, innovation_share, ..only(0.0, 0.0, 0.5) };
    let mut w = world(&[&g, &c], 3, p);
    let mut done = 0;
    years
        .iter()
        .map(|&y| {
            w.run_years(y - done, 2, y - done);
            done = y;
            mean_effort(&w)
        })
        .collect()
}

#[test]
fn practice_learned_from_others_takes_off_late_and_then_fast() {
    let years = [300, 600, 900, 1200, 1500, 1800, 2100, 2400];
    let alone = effort_path(1.0, &years);
    assert!(alone[0] > 0.4, "individual learning reaches the target within centuries: {alone:?}");
    let social = effort_path(1e-3, &years);
    assert!(social[0] < 0.05, "an innovation few have seen spreads slowly at first: {social:?}");
    assert!(social[7] > 0.4, "then it reaches the same target: {social:?}");
    let steps: Vec<f64> = social.windows(2).map(|w| w[1] - w[0]).collect();
    let fastest = steps.iter().cloned().fold(0.0, f64::max);
    assert!(fastest > 3.0 * social[0], "an S-curve: the rise is fastest after the start: {social:?}");
}

#[test]
fn without_neighbors_or_success_a_band_sees_its_cells_mean_effort() {
    // The learning of ADR-0065, nested: the people-weighted mean effort of its own cell.
    let c = Cultivation::from_params(&only(0.0, 0.0, 0.5));
    let seen = c.example([(1.0, 100, 0.3, 2.0), (0.0, 50, 0.9, 1.0)].into_iter()).unwrap();
    assert!((seen - 0.3).abs() < 1e-12, "{seen}");
    assert_eq!(c.example(std::iter::empty()), None, "nobody to learn from");
}

#[test]
fn success_biased_learners_copy_farming_more_where_it_pays() {
    // Payoff-biased learning (Boyd and Richerson 1985; McElreath et al. 2008): each model
    // counts in proportion to its payoff to the power success_bias (1: linear payoff
    // weighting); a forager's payoff is 1.
    let c = Cultivation::from_params(&Params { success_bias: 1.0, ..only(0.0, 0.0, 0.5) });
    let seen = |payoff| c.example([(1.0, 100, 0.5, payoff)].into_iter()).unwrap();
    assert!((seen(2.0) - 2.0 / 3.0).abs() < 1e-12, "farming that feeds twice as many is copied more");
    assert!((seen(0.5) - 1.0 / 3.0).abs() < 1e-12, "farming that pays less than foraging is copied less");
    assert!((seen(1.0) - 0.5).abs() < 1e-12);
}

#[test]
fn bands_learn_from_farmers_in_neighboring_cells() {
    // Kin and marriage ties reach beyond a band's own country (Wobst 1974): it sees its
    // neighbors' practice, each neighboring cell's people counting neighbor_learning as
    // much as its own cell's.
    let c = Cultivation::from_params(&Params { neighbor_learning: 0.1, ..only(0.0, 0.0, 0.5) });
    let models = [(1.0, 100, 0.0, 1.0), (0.1, 100, 1.0, 1.0)];
    let seen = c.example(models.into_iter()).unwrap();
    assert!((seen - 10.0 / 110.0).abs() < 1e-12, "{seen}");
}

#[test]
fn farmings_payoff_is_what_a_unit_of_effort_yields_against_foraging() {
    // farmed_capacity: effort e yields fields of farm * (w e + (1 - w) crop) against the
    // forage * e it gives up, w the wild yield ratio and crop / e the domestic share.
    let c = Cultivation::from_params(&Params { wild_yield_ratio: 0.5, ..only(0.0, 0.0, 0.5) });
    let effort = 100 * SHARE_ONE as u64;
    assert!((c.payoff((600.0, 0.0), 200, effort, effort / 2) - 2.25).abs() < 1e-9);
    assert!((c.payoff((600.0, 0.0), 200, 0, 0) - 1.5).abs() < 1e-9, "wild crops where nobody farms yet");
    // A floodplain feeds only domestic crops (ADR-0085): 400 more at full domestication.
    assert!((c.payoff((600.0, 400.0), 200, effort, effort / 2) - 3.25).abs() < 1e-9);
    assert!((c.payoff((600.0, 400.0), 200, 0, 0) - 1.5).abs() < 1e-9, "no wild cereals on the floodplain");
}

/// Mean effort after each of `years` in a full land where abundance alone drives
/// cultivation, few innovate and the rest learn from what they see; `climate` per cell.
fn learned_path(p: Params, climate: impl Fn(i64, usize) -> (f64, f64), years: &[u64]) -> Vec<f64> {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], climate));
    let p = Params { initial_occupancy: 0.9, innovation_share: 1e-3, ..p };
    let mut w = world(&[&g, &c], 3, p);
    let mut done = 0;
    years
        .iter()
        .map(|&y| {
            w.run_years(y - done, 2, y - done);
            done = y;
            mean_effort(&w)
        })
        .collect()
}

/// Rich wild stands in one cell of seven, thin ones (a third as suitable) elsewhere.
fn patchy_country(_: i64, cell: usize) -> (f64, f64) {
    (15.0, if cell % 7 == 0 { 500.0 } else { 250.0 })
}

/// Mean effort of the rich and of the thin cells of patchy country after `years`.
fn patchy_efforts(p: Params, years: u64) -> (f64, f64) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], patchy_country));
    let mut w = world(&[&g, &c], 3, Params { initial_occupancy: 0.9, innovation_share: 1e-3, ..p });
    w.run_years(years, 2, years);
    let mean = |rich: bool| {
        let cells: Vec<usize> =
            (0..h3.len()).filter(|&c| (c % 7 == 0) == rich && w.population[c] > 0).collect();
        cells.iter().map(|&c| w.cultivation()[c]).sum::<f64>() / cells.len() as f64
    };
    (mean(true), mean(false))
}

#[test]
fn learning_from_neighbors_carries_cultivation_from_rich_patches_into_thin_country() {
    // Cultivation takes off first in the rich patches. Bands around them that see it there
    // take it up sooner, and the leaders, seeing their slower neighbors too, less fast:
    // wider learning evens practice out across a landscape.
    let years = 900;
    let (rich, thin) = patchy_efforts(only(0.0, 0.0, 0.5), years);
    let (rich_n, thin_n) = patchy_efforts(Params { neighbor_learning: 0.3, ..only(0.0, 0.0, 0.5) }, years);
    assert!(rich > 0.0 && thin > 0.0, "the test needs planting: {rich} {thin}");
    assert!(
        thin_n / rich_n > 1.2 * (thin / rich),
        "thin/rich {} with neighbors, {} without",
        thin_n / rich_n,
        thin / rich
    );
}

#[test]
fn success_bias_speeds_farming_where_it_pays_and_slows_it_where_it_does_not() {
    let years = [600, 900, 1200];
    let pays = only(0.0, 0.0, 0.5);
    let poor = Params { farm_density: 0.05, ..only(0.0, 0.0, 0.5) };
    for (p, faster) in [(pays, true), (poor, false)] {
        let plain = learned_path(p.clone(), cereal_country, &years);
        let biased = learned_path(Params { success_bias: 1.0, ..p }, cereal_country, &years);
        assert_eq!(biased[1] > plain[1], faster, "biased {biased:?}, plain {plain:?}");
    }
}

#[test]
fn bands_give_cultivation_up_by_copying_foragers() {
    // A band copies the practice it moves toward (ADR-0076): farmers' when taking
    // cultivation up, foragers' when giving it up, so a band that carried farming into
    // country where it pays nothing drops it at the pace of the foragers around it.
    let p = Params { innovation_share: 0.01, adoption_rate: 0.1, success_bias: 1.0, ..only(0.0, 0.0, 0.5) };
    let c = Cultivation::from_params(&p);
    let rate = 0.1 * (0.01 + 0.99 * 1.0);
    assert!((c.adopt(0.4, 0.0, 0.0) - (0.4 - rate * 0.4)).abs() < 1e-12, "among foragers: the full rate");
    assert!(
        (c.adopt(0.0, 0.5, 1.0) - rate * 0.5).abs() < 1e-12,
        "among farmers, taking it up: the full rate"
    );
    let before = Cultivation::from_params(&Params {
        innovation_share: 0.01,
        adoption_rate: 0.1,
        ..only(0.0, 0.0, 0.5)
    });
    let slow = 0.1 * 0.01;
    assert!(
        (before.adopt(0.4, 0.0, 0.0) - (0.4 - slow * 0.4)).abs() < 1e-12,
        "ADR-0065: paced by farmers seen"
    );
}

#[test]
fn full_effort_is_all_farming_whatever_its_payoff() {
    // Everyone farms: the share is 1 even when farming's weight is far below rounding.
    let c = Cultivation::from_params(&Params { success_bias: 5.0, ..only(0.0, 0.0, 0.5) });
    let seen = c.example([(1.0, 100, 1.0, 0.0008)].into_iter()).unwrap();
    assert!((seen - 1.0).abs() < 1e-12, "{seen}");
}

/// Patchy country where few innovate, so what bands see from neighbors matters.
fn patchy_world(p: Params) -> World {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], patchy_country));
    world(&[&g, &c], 4, Params { initial_occupancy: 0.9, innovation_share: 1e-3, ..p })
}

#[test]
fn learning_from_neighbors_is_partition_independent() {
    // Neighbors across partition boundaries are read from the previous step's snapshot.
    let p = Params { neighbor_learning: 0.3, success_bias: 1.5, ..only(0.0, 0.0, 0.5) };
    let run = |p: &Params, parts| {
        let mut w = patchy_world(p.clone());
        w.run_years(900, parts, 900);
        (w.cultivation(), checkpoint::state_hash(&w))
    };
    let (own, _) = run(&only(0.0, 0.0, 0.5), 1);
    let (seen, hash) = run(&p, 1);
    assert_ne!(own, seen, "the test needs neighbors to matter");
    assert_eq!(hash, run(&p, 5).1);
}

#[test]
fn a_registered_cultivation_learns_by_its_own_settings() {
    // The world tabulates its cells as models for its own parameters; a cultivation
    // mechanism set up with others computes them itself, with the same result.
    let p = Params { neighbor_learning: 0.3, success_bias: 1.5, ..only(0.0, 0.0, 0.5) };
    let alone = |w: &mut World, c: &Params| {
        w.set_mechanisms(vec![Box::new(Cultivation { after: vec![], ..Cultivation::from_params(c) })])
            .unwrap();
        w.run_years(300, 3, 300);
        w.cultivation()
    };
    let tabulated = alone(&mut patchy_world(p.clone()), &p);
    let computed = alone(&mut patchy_world(only(0.0, 0.0, 0.5)), &p);
    assert!(tabulated.iter().any(|&e| e > 0.0), "the test needs planting");
    assert_eq!(tabulated, computed);
}

#[test]
fn checkpoints_from_before_neighbors_and_success_restore_without_them() {
    let (mut w, g, c) = farming_world(9, |p| p);
    w.run_years(100, 2, 100);
    let now = checkpoint::encode(&w);
    let at = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8;
    let len = u32::from_le_bytes(now[at..at + 4].try_into().unwrap()) as usize;
    let mut params: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&now[at + 4..at + 4 + len]).unwrap();
    for k in ["neighbor_learning", "success_bias"] {
        assert!(params.remove(k).is_some(), "{k}");
    }
    let old = serde_json::to_vec(&params).unwrap();
    let mut before = now[..at].to_vec();
    before.extend_from_slice(&(old.len() as u32).to_le_bytes());
    before.extend_from_slice(&old);
    before.extend_from_slice(&now[at + 4 + len..]);
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    assert_eq!((restored.params.neighbor_learning, restored.params.success_bias), (0.0, 0.0));
}

#[test]
fn checkpoints_from_before_social_learning_restore_with_individual_learning() {
    let (mut w, g, c) = farming_world(9, |p| Params { innovation_share: 1.0, ..p });
    w.run_years(100, 2, 100);
    let now = checkpoint::encode(&w);
    // The parameters before ADR-0065 had no innovation share: drop it from the JSON.
    let at = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8;
    let len = u32::from_le_bytes(now[at..at + 4].try_into().unwrap()) as usize;
    let mut params: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&now[at + 4..at + 4 + len]).unwrap();
    assert!(params.remove("innovation_share").is_some());
    let old = serde_json::to_vec(&params).unwrap();
    let mut before = now[..at].to_vec();
    before.extend_from_slice(&(old.len() as u32).to_le_bytes());
    before.extend_from_slice(&old);
    before.extend_from_slice(&now[at + 4 + len..]);
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    assert_eq!(restored.params.innovation_share, 1.0, "whatever the registry default");
    assert_eq!(checkpoint::encode(&restored), now);
}

#[test]
fn the_size_limit_rises_from_camp_to_village_with_cultivation() {
    // Scalar stress (Johnson 1982): a group splits when it outgrows how its members
    // decide together; farming villages hold more people before they split (Alberti 2014).
    let f = Fission::from_params(&Params {
        fission_threshold: 60,
        village_fission_size: 127,
        ..Params::default()
    });
    assert_eq!(f.limit(0.0), 60.0);
    assert_eq!(f.limit(1.0), 127.0);
    assert!((f.limit(0.5) - (60.0_f64 * 127.0).sqrt()).abs() < 1e-9, "in between, in proportion");
    assert!(f.splits(127, 1.0) && !f.splits(126, 1.0) && f.splits(60, 0.0) && !f.splits(59, 0.0));
    let one =
        Fission::from_params(&Params { fission_threshold: 2, village_fission_size: 1, ..Params::default() });
    assert!(!one.splits(1, 1.0), "one person cannot split");
    let off =
        Fission::from_params(&Params { fission_threshold: 60, village_fission_size: 0, ..Params::default() });
    assert!(off.splits(60, 1.0) && !off.splits(59, 1.0), "no villages: one limit");
}

/// Sizes of the groups whose effort is mostly cultivation.
fn farming_groups(w: &World) -> Vec<u64> {
    w.agent_slices()
        .flat_map(|a| {
            a.weight.iter().zip(a.cultivation.iter()).filter(|(_, e)| **e > SHARE_ONE / 2).map(|(w, _)| *w)
        })
        .collect()
}

#[test]
fn farmers_live_in_villages_and_foragers_in_camps() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let c = climate_pack(&h3, &[-12_000], |_, _| (15.0, 500.0));
    let run = |village: u64| {
        let p = Params { fission_probability: 0.2, village_fission_size: village, ..only(0.0, 0.0, 0.8) };
        let mut w = world(&[&g, &c], 6, p);
        w.run_years(800, 2, 800);
        farming_groups(&w)
    };
    let (villages, camps) = (run(127), run(0));
    assert!(!villages.is_empty() && !camps.is_empty());
    let largest = |x: &[u64]| *x.iter().max().unwrap();
    assert!(largest(&villages) > 100 && largest(&villages) < 150, "villages: {}", largest(&villages));
    assert!(largest(&camps) < 80, "without villages farmers split as camps do: {}", largest(&camps));
}

#[test]
fn village_daughters_settle_nearby_while_the_land_has_room() {
    // A village that splits founds another in its own territory while the land feeds
    // more; only a full one sends its daughter away. The fission before villages
    // (village size 0) placed it by crowding, often in a neighbor.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let c = climate_pack(&h3, &[-12_000], |_, _| (15.0, 500.0));
    let run = |village: u64| {
        let p = Params {
            fission_probability: 0.2,
            fission_threshold: 40,
            village_fission_size: village,
            dispersal_probability: 0.0,
            ..only(0.0, 0.0, 0.8)
        };
        let mut w = world(&[&g, &c], 6, p);
        let start = w.agent_count();
        let mut migrants = 0;
        w.run_years_observed(150, 2, 150, &mut |_, s| {
            migrants += s.migrants;
            Ok(())
        })
        .unwrap();
        (migrants, w.agent_count() - start)
    };
    let (moved, more) = run(127);
    assert!(more > 0, "villages split");
    assert_eq!(moved, 0, "into their own land");
    assert!(run(0).0 > 0, "the fission before villages sent daughters to neighbors");
}

fn median(mut x: Vec<u64>) -> u64 {
    x.sort_unstable();
    x[x.len() / 2]
}

#[test]
fn farming_groups_gather_into_villages() {
    // Sedentary farmers draw together (Kuijt 2000) up to the size at which villages split
    // (ADR-0070), so villages fill toward that limit instead of staying split in halves.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let c = climate_pack(&h3, &[-12_000], |_, _| (15.0, 500.0));
    // Cells that fill: about 200 farmers each, as M1's do; in a full cell nobody grows, so
    // only joining makes villages larger.
    let run = |rate: f64| {
        let p = Params {
            fission_probability: 0.2,
            aggregation_rate: rate,
            farm_density: 0.05,
            forager_density_ref: 0.01,
            growth_rate: 0.03,
            sedentary_growth_rate: 0.03,
            ..only(0.0, 0.0, 0.8)
        };
        let mut w = world(&[&g, &c], 6, p);
        w.run_years(800, 2, 800);
        farming_groups(&w)
    };
    let (gathered, apart) = (run(0.1), run(0.0));
    assert!(
        median(gathered.clone()) > 3 * median(apart.clone()) / 2,
        "{} vs {}",
        median(gathered),
        median(apart)
    );
    assert!(*gathered.iter().max().unwrap() < 150, "within the village limit, give or take a year's growth");
}

#[test]
fn foragers_never_gather() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let forest = climate_pack(&h3, &[-12_000], |_, _| (15.0, 1900.0));
    let run = |rate: f64| {
        let p = Params { fission_probability: 0.2, aggregation_rate: rate, ..only(0.0, 0.0, 0.8) };
        let mut w = world(&[&g, &forest], 6, p);
        w.run_years(300, 2, 300);
        w.agent_slices().flat_map(|a| a.weight.to_vec()).collect::<Vec<_>>()
    };
    assert_eq!(run(0.5), run(0.0));
}

#[test]
fn gathering_keeps_people_and_practice() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let p = Params { fission_probability: 0.2, ..only(0.0, 0.0, 0.6) };
    let mut w = world(&[&g, &c], 7, p.clone());
    w.run_years(600, 2, 600);
    w.set_mechanisms(vec![Box::new(Aggregation {
        rate: 1.0,
        after: vec![],
        ..Aggregation::from_params(&p)
    })])
    .unwrap();
    let mass = |w: &World| -> (u64, u128) {
        let people = w.agent_slices().map(|a| a.weight.iter().sum::<u64>()).sum();
        let effort = w
            .agent_slices()
            .map(|a| (0..a.len()).map(|i| a.weight[i] as u128 * a.cultivation[i] as u128).sum::<u128>())
            .sum();
        (people, effort)
    };
    let ((p0, m0), agents) = (mass(&w), w.agent_count());
    for _ in 0..10 {
        w.step(2);
    }
    let (p1, m1) = mass(&w);
    assert!(w.agent_count() < agents, "groups joined");
    assert_eq!(p1, p0, "people are conserved");
    assert!(m0 > 0 && m1 <= m0 && m0 - m1 < m0 / 1_000_000, "effort mass {m0} -> {m1}");
}

#[test]
fn a_joined_village_stays_within_the_limit_of_its_pooled_practice() {
    // 40 people farming little lower a village's effort, and so its limit, when they join.
    let agg = Aggregation::from_params(&Params {
        fission_threshold: 60,
        village_fission_size: 127,
        ..Params::default()
    });
    let share = |x: f64| (x * SHARE_ONE as f64) as u32;
    assert!(!agg.fits(70, share(1.0), 40, share(0.1)), "pooled effort 0.67 allows about 99, not 110");
    assert!(agg.fits(70, share(1.0), 20, share(1.0)));
    assert!(!agg.fits(100, share(1.0), 27, share(1.0)), "127 is the limit itself");
}

#[test]
fn settlements_count_groups_by_how_settled_they_are() {
    // What surveys find (ADR-0072): each group as far as it is settled.
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let mut w = world(&[&g, &c], 7, only(0.0, 0.0, 0.6));
    w.run_years(600, 2, 600);
    let mut expected = vec![0u64; w.grid.len()];
    for a in w.agent_slices() {
        for i in 0..a.len() {
            expected[a.cell[i] as usize] += a.cultivation[i] as u64;
        }
    }
    let got = w.settlements();
    assert!(got.iter().any(|&x| x > 1.0), "some cells hold more than one settled group");
    for (x, e) in got.iter().zip(&expected) {
        assert_eq!(*x, *e as f64 / SHARE_ONE as f64);
    }
}

#[test]
fn village_sizes_weigh_each_settlement_by_the_log_of_its_people() {
    // The village-size record (ADR-0078): summed over a cell's groups, each as far as it
    // is settled, the log of its people; divided by `settlements`, their mean log size.
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], cereal_country));
    let mut w = world(&[&g, &c], 7, only(0.0, 0.0, 0.6));
    w.run_years(600, 2, 600);
    let mut expected = vec![0.0f64; w.grid.len()];
    for a in w.agent_slices() {
        for i in 0..a.len() {
            if a.cultivation[i] > 0 && a.weight[i] > 0 {
                expected[a.cell[i] as usize] +=
                    a.cultivation[i] as f64 / SHARE_ONE as f64 * oikos_math::ln(a.weight[i] as f64);
            }
        }
    }
    let got = w.village_sizes();
    assert!(got.iter().any(|&x| x > 0.0), "some cells hold settled groups");
    for (x, e) in got.iter().zip(&expected) {
        assert!((x - e).abs() <= 1e-9 * e.abs().max(1.0), "{x} against {e}");
    }
    let mut four = world(&[&g, &c], 7, only(0.0, 0.0, 0.6));
    four.run_years(600, 4, 600);
    assert_eq!(four.village_sizes(), got, "the same at any partition count");
}

/// A climate_anomaly pack (#41): the regional index at `years`.
fn anomaly_pack(h3: &[u64], years: &[i64], index: &[f64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate_anomaly", region(h3));
    b.time(years.to_vec(), TimeRule::Linear);
    let stored = index.iter().map(|i| (i * 1000.0).round() as i16).collect();
    b.field(spec("index", Dims::Time, 0.001), Data::I16(stored)).unwrap();
    b.build()
}

#[test]
fn the_anomaly_index_warms_and_wets_by_its_scales() {
    use oikos_sim::environment::anomalous;
    let p = Params { climate_anomaly_temp: 1.5, climate_anomaly_precip: 0.25, ..Params::default() };
    assert_eq!(anomalous(15.0, 400.0, 0.0, &p), (15.0, 400.0));
    assert_eq!(anomalous(15.0, 400.0, -2.0, &p), (12.0, 200.0));
    assert_eq!(anomalous(15.0, 400.0, -8.0, &p).1, 0.0, "never less than no rain");
    let off = Params { climate_anomaly_temp: 0.0, climate_anomaly_precip: 0.0, ..p };
    assert_eq!(anomalous(15.0, 400.0, -2.0, &off), (15.0, 400.0));
}

/// A drought of 150 years in a steady wild-cereal country: the index falls to -1 from
/// 10,990 to 10,850 BCE (rain 40% lower at the default test scale).
fn drought(h3: &[u64]) -> Vec<u8> {
    anomaly_pack(
        h3,
        &[-12_000, -11_000, -10_990, -10_850, -10_840, -9_000],
        &[0.0, 0.0, -1.0, -1.0, 0.0, 0.0],
    )
}

#[test]
fn a_decadal_drought_stresses_foragers_into_cultivating() {
    // The climate pack's century means never change; only the index does.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let climate = climate_pack(&h3, &[-12_000, -9_000], cereal_country);
    let event = drought(&h3);
    let p = Params { climate_anomaly_temp: 0.0, climate_anomaly_precip: 0.4, ..only(0.0, 3.0, 0.0) };
    let run = |packs: &[&[u8]], p: Params| {
        let mut w = world(packs, 3, p);
        w.run_years(1100, 2, 1100); // 10,900 BCE
        let stress = w.farmland.stress.iter().sum::<f64>() / w.farmland.stress.len() as f64;
        w.run_years(50, 2, 50); // 10,850 BCE
        (stress, mean_effort(&w), w.population.clone())
    };
    let (stress, effort, _) = run(&[&g, &climate, &event], p.clone());
    let (calm, never, steady) = run(&[&g, &climate], p.clone());
    assert!(stress > 0.1 && calm == 0.0, "{stress} {calm}");
    assert!(effort > 0.05 && never == 0.0, "{effort} {never}");
    let off = Params { climate_anomaly_precip: 0.0, ..p };
    assert_eq!(
        run(&[&g, &climate, &event], off),
        (calm, never, steady),
        "at zero scales the index changes nothing"
    );
}

#[test]
fn variability_sees_century_means_of_the_index_not_decadal_swings() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let climate = climate_pack(&h3, &[-12_000, -9_000], cereal_country);
    let p = Params {
        stress_window_years: 300,
        climate_anomaly_temp: 0.0,
        climate_anomaly_precip: 0.4,
        ..Params::default()
    };
    let years: Vec<i64> = (0..=300).map(|i| -12_000 + 10 * i).collect();
    let swings: Vec<f64> = (0..=300).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
    let dip: Vec<f64> =
        years.iter().map(|&y| if (-11_690..=-11_600).contains(&y) { -1.0 } else { 0.0 }).collect();
    let variability = |packs: &[&[u8]]| Farmland::new(&inputs(packs), -11_500, &p, h3.len()).variability;
    let steady = variability(&[&g, &climate]);
    assert_eq!(variability(&[&g, &climate, &anomaly_pack(&h3, &years, &swings)]), steady);
    let dipped = variability(&[&g, &climate, &anomaly_pack(&h3, &years, &dip)]);
    assert!(dipped.iter().zip(&steady).all(|(d, s)| d > s), "{:?} {:?}", &dipped[..2], &steady[..2]);
}

#[test]
fn an_anomaly_index_must_be_a_time_series() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let mut b = PackBuilder::new("climate_anomaly", region(&h3));
    b.field(spec("index", Dims::Cell, 1.0), Data::U8(vec![0; h3.len()])).unwrap();
    let packs = vec![Pack::parse(g).unwrap(), Pack::parse(b.build()).unwrap()];
    assert!(Inputs::from_packs(packs).unwrap_err().contains("time series"));
}
