//! The African Humid Period (ADR-0084): rain the forcing's climate model misses, added
//! from reconstructions along a time course the parameters set. Foragers spread into a
//! desert while it is green and leave it when it dries again (Kuper and Kröpelin 2006).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::environment::{capacity_into, humid_share};
use oikos_sim::inputs::Inputs;
use oikos_sim::{Params, World, WorldConfig};

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

/// Desert where the forcing's model puts it: 25 degC, `rain` mm in the first cells, none
/// elsewhere, unchanging.
fn climate_pack(h3: &[u64], rain: u16, wet_cells: usize) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![2500; h3.len()])).unwrap();
    let precip = (0..h3.len()).map(|c| if c < wet_cells { rain } else { 0 }).collect();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(precip)).unwrap();
    b.build()
}

/// The Humid Period adds `mm` everywhere at its height.
fn humid_pack(h3: &[u64], mm: f64) -> Vec<u8> {
    let mut b = PackBuilder::new("humid_period", region(h3));
    b.field(spec("precip_anomaly", Dims::Cell, 0.1), Data::U16(vec![(mm * 10.0) as u16; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

/// Calendar year of `bp` years before present (1950).
fn year(bp: i64) -> i64 {
    1950 - bp
}

#[test]
fn the_humid_period_rises_and_falls_over_its_transitions() {
    let p = Params {
        humid_period_onset_bp: 11_000,
        humid_period_end_bp: 6_500,
        humid_period_transition_years: 500,
        ..Params::default()
    };
    assert_eq!(humid_share(year(12_000), &p), 0.0);
    assert_eq!(humid_share(year(11_250), &p), 0.0, "the onset starts 250 years before its midpoint");
    assert_eq!(humid_share(year(11_000), &p), 0.5);
    assert_eq!(humid_share(year(10_750), &p), 1.0);
    assert_eq!(humid_share(year(8_000), &p), 1.0);
    assert_eq!(humid_share(year(6_625), &p), 0.75);
    assert_eq!(humid_share(year(6_250), &p), 0.0);
    assert_eq!(humid_share(year(3_000), &p), 0.0);
}

#[test]
fn a_humid_period_that_ends_before_it_begins_is_refused() {
    let p = Params { humid_period_onset_bp: 5_000, humid_period_end_bp: 11_000, ..Params::default() };
    let err = p.validate().expect_err("refused");
    assert!(err.contains("humid_period_onset_bp"), "{err}");
    let touching = Params { humid_period_onset_bp: 7_000, humid_period_end_bp: 6_800, ..Params::default() };
    assert!(touching.validate().is_err(), "the transitions would overlap: never fully humid");
}

#[test]
fn the_anomaly_adds_to_the_forcings_rain_while_the_humid_period_lasts() {
    let h3 = h3_cells();
    let (g, c, h) = (geometry_pack(&h3), climate_pack(&h3, 0, 0), humid_pack(&h3, 200.0));
    let capacity = |packs: &[&[u8]], bp: i64, scale: f64| {
        let p = Params { humid_period_scale: scale, ..Params::default() };
        let mut out = vec![0; h3.len()];
        assert!(capacity_into(&inputs(packs), year(bp), &p, &mut out));
        out[0]
    };
    let dry = capacity(&[&g, &c], 8_000, 1.0);
    assert_eq!(dry, 1, "a desert without the pack (at least one person)");
    let green = capacity(&[&g, &c, &h], 8_000, 1.0);
    assert!(green > 30, "200 mm feed foragers: {green}");
    assert!(capacity(&[&g, &c, &h], 8_000, 2.0) > green, "a larger amplitude, more rain");
    assert_eq!(capacity(&[&g, &c, &h], 8_000, 0.0), dry, "scale 0: the forcing alone");
    assert_eq!(capacity(&[&g, &c, &h], 12_000, 1.0), dry, "before the onset");
    assert_eq!(capacity(&[&g, &c, &h], 5_000, 1.0), dry, "after the end");
}

#[test]
fn foragers_spread_into_the_green_desert_and_leave_it_when_it_dries() {
    // Foragers start in the few cells the forcing waters (one band in each cell that feeds
    // one); the rest is desert but for the Humid Period, which ends at 6,500 BP (4,550 BCE).
    let h3 = h3_cells();
    let (g, c, h) = (geometry_pack(&h3), climate_pack(&h3, 300, 7), humid_pack(&h3, 300.0));
    let p = Params {
        initial_occupancy: 0.5,
        humid_period_onset_bp: 11_000,
        humid_period_end_bp: 6_500,
        humid_period_transition_years: 500,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    let mut w = World::with_inputs(WorldConfig::geometry(&id), 3, p, inputs(&[&g, &c, &h])).unwrap();
    let occupied = |w: &World| {
        let mut cells: Vec<u32> = w.agent_slices().flat_map(|a| a.cell.clone()).collect();
        cells.sort_unstable();
        cells.dedup();
        cells.len()
    };
    w.run_years(1_000, 2, 1_000); // 11,000 BCE: before the onset (9,050 BCE)
    let before = occupied(&w);
    w.run_years(4_000, 2, 4_000); // 7,000 BCE: green
    let green = occupied(&w);
    w.run_years(4_000, 2, 4_000); // 3,000 BCE: dry again
    let after = occupied(&w);
    assert!(before <= 10, "{before} cells before the onset");
    assert!(green > 4 * before, "{green} cells while green, {before} before");
    assert!(after < green / 2, "{after} cells once dry, {green} while green");
}
