//! Moving coastlines: a `coastline` pack's `land_fraction` decides which cells are
//! habitable in each year. Bands start on land, never move into the sea, and leave
//! cells the sea floods (docs/04 §4.1, ADR-0043).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::inputs::Inputs;
use oikos_sim::{Params, World, WorldConfig, checkpoint};

fn h3_cells() -> Vec<u64> {
    let g = Grid::from_base_cell(22, 3).unwrap();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

fn region(h3: &[u64]) -> Region {
    Region { name: "test".into(), resolution: 3, cells: h3.len() as u32, cells_hash: cells_hash(h3) }
}

fn spec(name: &str, unit: &str, dims: Dims, scale: f64) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        unit: unit.into(),
        description: name.into(),
        dims,
        scale,
        offset: 0.0,
        nodata: None,
    }
}

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", "h3 index", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    b.build()
}

/// Land fraction per (year, cell) from `land(year, cell)`.
fn coastline_pack(h3: &[u64], years: &[i64], land: impl Fn(i64, usize) -> f64) -> Vec<u8> {
    let mut b = PackBuilder::new("coastline", region(h3));
    b.time(years.to_vec(), TimeRule::Linear);
    let land = &land;
    let v = years
        .iter()
        .flat_map(|&y| (0..h3.len()).map(move |c| (land(y, c) * 10_000.0) as u16))
        .collect::<Vec<_>>();
    b.field(spec("land_fraction", "1", Dims::TimeCell, 1e-4), Data::U16(v)).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

fn id(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn sea(c: usize) -> bool {
    c % 5 == 0
}

fn flooded(c: usize) -> bool {
    c % 5 == 1
}

/// Cells `sea` are always sea; cells `flooded` are land until 11,500 BCE and sea from
/// 11,400 BCE; a few cells are half land.
fn world(seed: u64) -> World {
    world_with(seed, Params::default())
}

fn world_with(seed: u64, params: Params) -> World {
    let h3 = h3_cells();
    let coast = coastline_pack(&h3, &[-12_000, -11_500, -11_400], |y, c| {
        if sea(c) || (flooded(c) && y >= -11_400) {
            0.0
        } else if c % 5 == 2 {
            0.5
        } else {
            1.0
        }
    });
    let g = geometry_pack(&h3);
    let inputs = inputs(&[&g, &coast]);
    World::with_inputs(WorldConfig::geometry(&id(&g)), seed, params, inputs).unwrap()
}

fn people(w: &World, cells: impl Fn(usize) -> bool) -> u64 {
    (0..w.grid.len()).filter(|&c| cells(c)).map(|c| w.population[c]).sum()
}

#[test]
fn bands_start_on_land_and_never_enter_the_sea() {
    for seed in 0..8 {
        let mut w = world(seed);
        assert_eq!(people(&w, sea), 0, "seed {seed}: initial bands in the sea");
        assert!(w.total_population() > 0);
        w.run_years_observed(400, 3, 100, &mut |w, _| {
            assert_eq!(people(w, sea), 0, "people in the sea at year {}", w.tick.calendar_year());
            Ok(())
        })
        .unwrap();
    }
}

#[test]
fn people_leave_cells_the_sea_floods() {
    let mut w = world(4);
    w.run_years(490, 2, 490);
    let before = (w.total_population(), people(&w, flooded));
    assert!(before.1 > 0, "people must have reached the cells that will flood");
    let (mut deaths, mut births) = (0, 0);
    w.run_years_observed(120, 2, 120, &mut |w, s| {
        deaths += s.deaths;
        births += s.births;
        if w.tick.calendar_year() > -11_400 {
            assert_eq!(people(w, flooded), 0, "people in flooded cells in {}", w.tick.calendar_year());
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(people(&w, flooded), 0, "flooded cells are empty from 11,400 BCE");
    assert_eq!(people(&w, sea), 0);
    assert_eq!(w.total_population(), before.0 + births - deaths, "people are conserved");
}

#[test]
fn coastline_worlds_are_partition_independent_and_restore() {
    let run = |parts| {
        let mut w = world(9);
        w.run_years(650, parts, 650);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    assert_eq!(run(1), run(7));

    // Restored mid-flood, the habitable mask is rebuilt from the packs and the tick.
    let mut w = world(9);
    w.run_years(550, 3, 550);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), w.inputs.clone()).unwrap();
    w.run_years(100, 3, 100);
    restored.run_years(100, 2, 100);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
    assert_eq!(checkpoint::state_hash(&w), run(1));
}

#[test]
fn a_coastline_without_land_is_rejected() {
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let inputs = inputs(&[&g, &coastline_pack(&h3, &[-12_000], |_, _| 0.0)]);
    let e = World::with_inputs(WorldConfig::geometry(&id(&g)), 1, Params::default(), inputs)
        .map(|_| ())
        .unwrap_err();
    assert!(e.contains("no habitable cell"), "{e}");
}

#[test]
fn fission_never_leaves_daughters_in_flooded_cells() {
    // Every large band splits every step and its daughter goes to the least crowded
    // habitable cell, which must never be its own flooded cell.
    let params = Params { fission_probability: 1.0, exploration_probability: 0.0, ..Params::default() };
    for seed in 0..6 {
        let mut w = world_with(seed, params.clone());
        w.run_years(560, 2, 560);
        w.run_years_observed(60, 2, 60, &mut |w, _| {
            if w.tick.calendar_year() > -11_400 {
                assert_eq!(
                    people(w, flooded),
                    0,
                    "seed {seed}: people in flooded cells in {}",
                    w.tick.calendar_year()
                );
            }
            Ok(())
        })
        .unwrap();
    }
}

#[test]
fn a_flooded_island_is_emptied() {
    // Only cell 7 is land, until the sea covers it in 11,950 BCE while its few bands are
    // still growing and splitting; nobody can leave it.
    let h3 = h3_cells();
    let g = geometry_pack(&h3);
    let coast = coastline_pack(&h3, &[-12_000, -11_960, -11_950], |y, c| f64::from(c == 7 && y < -11_950));
    let inputs = inputs(&[&g, &coast]);
    let params = Params {
        initial_bands: 2,
        growth_rate: 0.05,
        fission_probability: 1.0,
        fission_threshold: 11,
        ..Params::default()
    };
    let mut w = World::with_inputs(WorldConfig::geometry(&id(&g)), 3, params, inputs).unwrap();
    assert!(w.capacity[7] > 100, "the island must have room to grow");
    w.run_years(49, 2, 49);
    assert!(w.population[7] > 0 && w.total_population() == w.population[7]);
    let before = w.total_population();
    let (mut deaths, mut births, mut fissions) = (0, 0, 0);
    w.run_years_observed(5, 2, 5, &mut |w, s| {
        (deaths, births, fissions) = (deaths + s.deaths, births + s.births, fissions + s.fissions);
        if w.tick.calendar_year() > -11_950 {
            assert_eq!(w.total_population(), 0, "people on the flooded island in {}", w.tick.calendar_year());
        }
        Ok(())
    })
    .unwrap();
    assert!(fissions > 0, "bands must be splitting when the sea arrives");
    assert_eq!(before + births, deaths, "everyone on the island is lost and counted");
}
