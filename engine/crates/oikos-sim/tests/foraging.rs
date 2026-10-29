//! Foraging carrying capacity from forcing (docs/04 §4.2, ADR-0047): with a climate pack,
//! each cell's capacity follows net primary productivity, dry-land area and access to
//! aquatic resources, and changes with the climate. Worlds without one keep the toy
//! capacity.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::environment::{forager_density, npp_miami};
use oikos_sim::inputs::Inputs;
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

/// 4,000 km2 per cell, stored like the geometry pack (u32 m2; any value works for the tests).
const AREA_KM2: f64 = 4_000.0;

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    let area = vec![(AREA_KM2 * 1e6) as u32; h3.len()];
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(area)).unwrap();
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

fn hydrology_pack(h3: &[u64], river: impl Fn(usize) -> f64) -> Vec<u8> {
    let mut b = PackBuilder::new("hydrology", region(h3));
    let q = (0..h3.len()).map(|c| (river(c) * 1000.0) as u32).collect();
    b.field(spec("river_discharge", Dims::Cell, 0.001), Data::U32(q)).unwrap();
    b.field(spec("lake_fraction", Dims::Cell, 1e-4), Data::U16(vec![0; h3.len()])).unwrap();
    b.build()
}

fn id(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn world(packs: &[&[u8]], seed: u64) -> World {
    let inputs =
        Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap();
    World::with_inputs(WorldConfig::geometry(&id(packs[0])), seed, Params::default(), inputs).unwrap()
}

/// Wet cells (even) get 800 mm, dry cells (odd) 250 mm, at 15 degC.
fn wet(c: usize) -> bool {
    c % 2 == 0
}

fn wet_dry(_: i64, c: usize) -> (f64, f64) {
    (15.0, if wet(c) { 800.0 } else { 250.0 })
}

fn expected(t: f64, p: f64, params: &Params, aquatic: bool) -> u64 {
    let water = if aquatic { params.aquatic_bonus * params.forager_density_ref } else { 0.0 };
    let k = AREA_KM2 * (forager_density(npp_miami(t, p, params), params) + water);
    (k as u64).max(1)
}

#[test]
fn capacity_follows_productivity_and_area() {
    let h3 = h3_cells();
    let w = world(&[&geometry_pack(&h3), &climate_pack(&h3, &[-12_000], wet_dry)], 1);
    let p = Params::default();
    for c in 0..h3.len() {
        let (t, pr) = wet_dry(0, c);
        assert_eq!(w.forage_capacity[c], expected(t, pr, &p, false), "cell {c}");
    }
    assert!(w.forage_capacity[0] > 2 * w.forage_capacity[1], "wetter cells feed more foragers");
}

#[test]
fn capacity_changes_with_the_climate() {
    // A cold, dry start warming and wetting by 11,000 BCE; capacity is recomputed every
    // `capacity_period_ticks`.
    let h3 = h3_cells();
    let climate =
        climate_pack(&h3, &[-12_000, -11_000], |y, _| if y < -11_500 { (2.0, 200.0) } else { (16.0, 600.0) });
    let mut w = world(&[&geometry_pack(&h3), &climate], 2);
    let p = Params::default();
    let start = w.forage_capacity[3];
    assert_eq!(start, expected(2.0, 200.0, &p, false));
    w.run_years(500, 2, 500); // -11,500: half way
    let mid = w.forage_capacity[3];
    w.run_years(600, 2, 600); // -10,900: past the last slice
    assert_eq!(w.forage_capacity[3], expected(16.0, 600.0, &p, false));
    assert!(start < mid && mid < w.forage_capacity[3]);
}

#[test]
fn perennial_rivers_raise_capacity() {
    let h3 = h3_cells();
    let w = world(
        &[
            &geometry_pack(&h3),
            &climate_pack(&h3, &[-12_000], |_, _| (15.0, 400.0)),
            &hydrology_pack(&h3, |c| if c % 3 == 0 { 50.0 } else { 0.2 }),
        ],
        3,
    );
    let p = Params::default();
    assert_eq!(w.forage_capacity[0], expected(15.0, 400.0, &p, true));
    assert_eq!(w.forage_capacity[1], expected(15.0, 400.0, &p, false));
}

#[test]
fn rivers_feed_foragers_in_deserts() {
    // No rain: no terrestrial productivity, but a perennial river (the Nile) still feeds people.
    let h3 = h3_cells();
    let w = world(
        &[
            &geometry_pack(&h3),
            &climate_pack(&h3, &[-12_000], |_, _| (24.0, 0.0)),
            &hydrology_pack(&h3, |c| if c == 0 { 2500.0 } else { 0.0 }),
        ],
        4,
    );
    let p = Params::default();
    assert_eq!(w.forage_capacity[1], 1, "rainless desert");
    assert_eq!(w.forage_capacity[0], expected(24.0, 0.0, &p, true));
    assert!(w.forage_capacity[0] > 100);
}

#[test]
fn people_concentrate_where_productivity_is_high() {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], wet_dry));
    let mut w = world(&[&g, &c], 7);
    w.run_years(1500, 3, 1500);
    let (wet_pop, dry_pop): (u64, u64) = (0..h3.len())
        .fold((0, 0), |(a, b), i| if wet(i) { (a + w.population[i], b) } else { (a, b + w.population[i]) });
    assert!(wet_pop > 2 * dry_pop, "wet {wet_pop}, dry {dry_pop}");
}

#[test]
fn climate_worlds_are_partition_independent_and_restore() {
    let h3 = h3_cells();
    let climate = climate_pack(&h3, &[-12_000, -11_000], |y, c| {
        (if y < -11_500 { 4.0 } else { 15.0 }, 200.0 + 50.0 * (c % 9) as f64)
    });
    let (g, cl) = (geometry_pack(&h3), climate);
    let run = |parts| {
        let mut w = world(&[&g, &cl], 9);
        w.run_years(700, parts, 700);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(5));
    let mut w = world(&[&g, &cl], 9);
    w.run_years(333, 2, 333);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), w.inputs.clone()).unwrap();
    assert_eq!(restored.capacity, w.capacity);
    restored.run_years(367, 4, 367);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn climate_worlds_start_with_foragers_on_all_the_land() {
    // At 12,000 BCE Epipaleolithic foragers lived across Southwest Asia: one band per
    // cell at `initial_occupancy` of its forager capacity.
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, &[-12_000], wet_dry));
    let w = world(&[&g, &c], 8);
    let p = Params::default();
    for cell in 0..h3.len() {
        let k = w.forage_capacity[cell];
        assert_eq!(w.population[cell], (p.initial_occupancy * k as f64) as u64, "cell {cell}");
    }
    assert_eq!(w.agent_count(), h3.len());
    // Zero occupancy keeps the founding bands of the toy start.
    let inputs =
        Inputs::from_packs([&g, &c].iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap();
    let few = Params { initial_occupancy: 0.0, ..Params::default() };
    let w = World::with_inputs(WorldConfig::geometry(&id(&g)), 8, few.clone(), inputs).unwrap();
    assert_eq!(w.total_population(), few.initial_bands as u64 * few.initial_weight);
}
