//! Floodplain farming (ADR-0085): where a river floods its plain each year, fields need no
//! rain, so domestic crops grow in deserts (the Nile, the Mesopotamian alluvium). No wild
//! cereals grow there, so only bands that bring domestic seed sow them, and floodplain
//! fields yield only from domestic crops (Butzer 1976).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::environment::{Farmland, farmed_capacity};
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

/// 15 degC everywhere; `rain(c)` mm in cell c, unchanging.
fn climate_pack(h3: &[u64], rain: impl Fn(usize) -> u16) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16((0..h3.len()).map(rain).collect())).unwrap();
    b.build()
}

/// No lakes; `floodplain` of every cell floods.
fn hydrology_pack(h3: &[u64], floodplain: f64) -> Vec<u8> {
    let mut b = PackBuilder::new("hydrology", region(h3));
    b.field(spec("lake_fraction", Dims::Cell, 1e-4), Data::U16(vec![0; h3.len()])).unwrap();
    let f = (floodplain * 1e4) as u16;
    b.field(spec("floodplain_fraction", Dims::Cell, 1e-4), Data::U16(vec![f; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

#[test]
fn a_rainless_floodplain_feeds_farmers_of_domestic_crops_only() {
    let h3 = h3_cells();
    let (g, c, h) = (geometry_pack(&h3), climate_pack(&h3, |_| 0), hydrology_pack(&h3, 0.5));
    let p = Params::default();
    let land = Farmland::new(&inputs(&[&g, &c, &h]), -6_000, &p, h3.len());
    assert_eq!(land.suitability[0], 0.0, "no wild cereals without rain");
    assert_eq!(land.farm[0], 0.0, "no rain-fed fields");
    assert_eq!(land.floodplain[0], 0.5, "half the land floods, at full yield");
    let want = 0.5 * 4000.0 * p.farm_density * p.floodplain_yield;
    assert!((land.flood[0] - want).abs() < 1e-9 * want, "{} against {want}", land.flood[0]);
    let forage = 100;
    assert_eq!(
        farmed_capacity(forage, 0.0, land.flood[0], 0.5, 0.0, &p),
        50,
        "wild seed grows nothing there"
    );
    let domestic = farmed_capacity(forage, 0.0, land.flood[0], 0.5, 0.5, &p);
    assert_eq!(domestic, (50.0 + 0.5 * land.flood[0]) as u64, "domestic seed yields in full");
    let dry = Farmland::new(&inputs(&[&g, &c]), -6_000, &p, h3.len());
    assert!(dry.flood.iter().all(|&f| f == 0.0) && dry.floodplain.iter().all(|&f| f == 0.0), "no rivers");
}

#[test]
fn floodplains_add_only_what_rain_does_not_already_grow() {
    let h3 = h3_cells();
    let (g, c, h) = (geometry_pack(&h3), climate_pack(&h3, |_| 400), hydrology_pack(&h3, 0.5));
    let at = |y: f64| {
        Farmland::new(
            &inputs(&[&g, &c, &h]),
            -6_000,
            &Params { floodplain_yield: y, ..Params::default() },
            h3.len(),
        )
    };
    let rainfed = at(1.0);
    assert_eq!(rainfed.suitability[0], 1.0);
    assert_eq!(rainfed.floodplain[0], 0.0, "rain already grows full crops on the plain");
    let rich = at(1.5);
    assert!((rich.floodplain[0] - 0.25).abs() < 1e-12, "half the land, half again: {}", rich.floodplain[0]);
    assert_eq!(at(0.0).floodplain[0], 0.0);
}

#[test]
fn farmers_bring_domestic_crops_to_the_desert_floodplain_and_farm_it() {
    // Wild cereals grow in the first cells, where cultivation begins and crops become
    // domestic; the rest is rainless but floods. Farming reaches the floodplain only if it
    // can be sown, and the bands there that farm (a tenth of their effort or more; a band
    // that has just arrived still carries some from home) have domestic seed.
    let h3 = h3_cells();
    let wet = 60;
    let (g, c, h) =
        (geometry_pack(&h3), climate_pack(&h3, |i| if i < wet { 500 } else { 0 }), hydrology_pack(&h3, 0.5));
    let run = |floodplain_yield: f64| {
        let p = Params {
            initial_occupancy: 0.3,
            cultivation_pressure_weight: 0.5,
            cultivation_abundance_weight: 0.8,
            adoption_rate: 0.05,
            innovation_share: 1.0,
            selection_coefficient: 0.01,
            dispersal_probability: 0.3,
            fission_probability: 0.3,
            floodplain_yield,
            ..Params::default()
        };
        let id = blake3::hash(&g).to_hex().to_string();
        let mut w = World::with_inputs(WorldConfig::geometry(&id), 4, p, inputs(&[&g, &c, &h])).unwrap();
        w.run_years(3_000, 2, 3_000);
        let (mut farmers, mut seed) = (0u64, 0u64);
        for a in w.agent_slices() {
            for i in 0..a.len() {
                if a.cell[i] as usize >= wet && a.cultivation[i] >= oikos_sim::agents::SHARE_ONE / 10 {
                    farmers += a.weight[i];
                    seed = seed.max(a.domestication[i] as u64);
                }
            }
        }
        (farmers, seed)
    };
    let (farmers, seed) = run(1.0);
    assert!(farmers > 1_000, "{farmers} people farm the desert floodplain");
    assert!(seed > oikos_sim::agents::SHARE_ONE as u64 / 2, "with domestic crops");
    let without = run(0.0).0;
    assert!(without < 100, "without floodplain fields only bands just arrived: {without}");
}
