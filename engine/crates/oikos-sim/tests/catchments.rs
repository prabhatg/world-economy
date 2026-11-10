//! Catchments (#50): people work the land and live among the people within a distance of
//! home, not within a grid cell. So the same land cut into finer cells feeds as many
//! people and holds villages as large, and a village draws farmers from the cells around
//! it as one cell's land could never feed them all.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::ALONE;
use oikos_sim::environment::{forager_density, npp_miami};
use oikos_sim::inputs::Inputs;
use oikos_sim::{Params, World, WorldConfig, checkpoint};
use std::collections::BTreeMap;

/// Base cell 22's cells at `res`: 343 at resolution 3, 2,401 at 4.
fn h3_cells(res: u8) -> Vec<u64> {
    let g = Grid::from_base_cell(22, res).unwrap();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

/// Every cell's area at `res`: the same land, 4,000 km2 a cell at resolution 3.
fn area_km2(res: u8) -> f64 {
    4_000.0 / 7f64.powi(res as i32 - 3)
}

fn region(h3: &[u64], res: u8) -> Region {
    Region { name: "test".into(), resolution: res, cells: h3.len() as u32, cells_hash: cells_hash(h3) }
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

fn geometry_pack(h3: &[u64], res: u8) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3, res));
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    let area = vec![(area_km2(res) * 1e6) as u32; h3.len()];
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(area)).unwrap();
    b.build()
}

/// Wild-cereal country everywhere: 15 degC, 400 mm.
const CLIMATE: (f64, f64) = (15.0, 400.0);

fn climate_pack(h3: &[u64], res: u8) -> Vec<u8> {
    climate_pack_at(h3, res, CLIMATE.1)
}

/// 15 degC and `precip` mm everywhere.
fn climate_pack_at(h3: &[u64], res: u8, precip: f64) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3, res));
    b.time(vec![-12_000], TimeRule::Linear);
    let t = (CLIMATE.0 * 100.0) as i16;
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![t; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![precip as u16; h3.len()])).unwrap();
    b.build()
}

fn hydrology_pack(h3: &[u64], res: u8, river: impl Fn(usize) -> f64) -> Vec<u8> {
    let mut b = PackBuilder::new("hydrology", region(h3, res));
    let q = (0..h3.len()).map(|c| (river(c) * 1000.0) as u32).collect();
    b.field(spec("river_discharge", Dims::Cell, 0.001), Data::U32(q)).unwrap();
    b.field(spec("lake_fraction", Dims::Cell, 1e-4), Data::U16(vec![0; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

fn world(packs: &[&[u8]], seed: u64, p: Params) -> World {
    let id = blake3::hash(packs[0]).to_hex().to_string();
    World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(packs)).unwrap()
}

/// The radius of a circle of `area` km2.
fn radius(area: f64) -> f64 {
    (area / std::f64::consts::PI).sqrt()
}

/// Whole millimetres of rain at which the climate feeds about `density` foragers a km2.
fn precip_for(density: f64) -> f64 {
    let p = Params::default();
    let at = |mm: f64| forager_density(npp_miami(CLIMATE.0, mm, &p), &p);
    (1..2_000)
        .map(|mm| mm as f64)
        .min_by(|a, b| (at(*a) - density).abs().total_cmp(&(at(*b) - density).abs()))
        .unwrap()
}

/// People after `years` of foragers alone, one band a cell at the start, on land feeding
/// about `density` a km2 cut at `res`, with catchments of `radius_km`.
fn foragers(res: u8, density: f64, radius_km: f64, years: u64) -> u64 {
    let h3 = h3_cells(res);
    let p = Params {
        initial_occupancy: 1.0,
        cultivation_abundance_weight: 0.0,
        cultivation_pressure_weight: 0.0,
        cultivation_stress_weight: 0.0,
        cultivation_variability_weight: 0.0,
        catchment_radius_km: radius_km,
        ..Params::default()
    };
    let climate = climate_pack_at(&h3, res, precip_for(density));
    let mut w = world(&[&geometry_pack(&h3, res), &climate], 3, p);
    w.run_years(years, 2, years);
    w.population.iter().sum()
}

#[test]
fn land_cut_finer_feeds_as_many_people() {
    // A desert, under one person a cell at resolution 3, and steppe, ten: a cell feeds at
    // least one person and whole people only, so cut finer the desert fed seven times as
    // many and the steppe a third fewer. Counted over catchments, the land feeds the same.
    let catchment = radius(100_000.0);
    for (density, years) in [(0.0002, 300), (0.0025, 300)] {
        let (coarse, fine) = (foragers(3, density, catchment, years), foragers(4, density, catchment, years));
        let ratio = fine as f64 / coarse as f64;
        assert!((0.8..1.25).contains(&ratio), "density {density}: {fine} against {coarse} people");
        let (coarse, fine) = (foragers(3, density, 0.0, years), foragers(4, density, 0.0, years));
        let ratio = fine as f64 / coarse as f64;
        assert!(!(0.8..1.25).contains(&ratio), "density {density}, cell by cell: {fine} against {coarse}");
    }
}

#[test]
fn water_at_hand_reaches_the_land_within_a_catchment() {
    // A river through one cell feeds fishers on the land of its catchment: with catchments
    // of seven cells (resolution-2 cells), on the land of the cells beside it in one.
    let res = 3;
    let h3 = h3_cells(res);
    let g = Grid::from_base_cell(22, res).unwrap();
    let parent = |c: usize| g.parent(c as u32, 2);
    let river =
        (0..g.len()).find(|&c| (0..g.len()).filter(|&m| parent(m) == parent(c)).count() == 7).unwrap();
    let hydrology = hydrology_pack(&h3, res, |c| if c == river { 50.0 } else { 0.0 });
    let packs: [&[u8]; 3] = [&geometry_pack(&h3, res), &climate_pack(&h3, res), &hydrology];
    let near = (0..g.len()).find(|&c| c != river && parent(c) == parent(river)).unwrap();
    let far = (0..g.len()).find(|&c| parent(c) != parent(river)).unwrap();
    let seven = Params { catchment_radius_km: radius(7.0 * area_km2(res)), ..Params::default() };
    let w = world(&packs, 1, seven);
    let own = world(&packs, 1, Params::default());
    assert_eq!(w.forage_capacity[river], own.forage_capacity[river]);
    assert_eq!(w.forage_capacity[near], own.forage_capacity[river], "the river is at hand");
    assert!(own.forage_capacity[near] < own.forage_capacity[river], "the river is another cell's");
    assert_eq!(w.forage_capacity[far], own.forage_capacity[far], "another catchment's river is not");
}

/// Farmers spreading from a few founding bands over land feeding `farmers` a km2 cut at
/// `res`, with catchments of `radius_km`; settlements of three groups' worth.
fn farming_world(res: u8, radius_km: f64, seed: u64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells(res);
    let (g, c) = (geometry_pack(&h3, res), climate_pack(&h3, res));
    let p = Params {
        initial_occupancy: 0.0,
        fission_probability: 0.2,
        cultivation_pressure_weight: 0.0,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.8,
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        growth_rate: 0.03,
        sedentary_growth_rate: 0.03,
        dispersal_probability: 0.1,
        forager_density_ref: 0.01,
        farm_density: 0.2,
        selection_coefficient: 0.005,
        emmer_selection_ratio: 1.0,
        einkorn_selection_ratio: 1.0,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        fission_threshold: 60,
        village_fission_size: 127,
        aggregation_rate: 0.1,
        disease_mortality: 0.0,
        neighbor_learning: 0.0,
        success_bias: 0.0,
        settlement_integration: 3.0,
        settlement_hierarchy: 0.0,
        catchment_radius_km: radius_km,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// People of each farming settlement (by cell and label, or a lone group's id).
fn settlements(w: &World) -> Vec<u64> {
    let mut out: BTreeMap<(u32, u32, u64), (u64, u64)> = BTreeMap::new();
    for a in w.agent_slices() {
        for i in 0..a.len() {
            let alone = if a.settlement[i] == ALONE { a.id[i].0 } else { 0 };
            let s = out.entry((a.cell[i], a.settlement[i], alone)).or_default();
            s.0 += a.weight[i];
            s.1 += a.cultivation[i] as u64;
        }
    }
    out.into_values().filter(|&(n, effort)| n > 0 && effort > 0).map(|(n, _)| n).collect()
}

fn mean_log(sizes: &[u64]) -> f64 {
    sizes.iter().map(|&n| (n as f64).ln()).sum::<f64>() / sizes.len() as f64
}

#[test]
fn villages_are_as_large_however_finely_the_land_is_cut() {
    // A catchment of a resolution-3 cell's area: there, each cell is its own, and farmers'
    // villages grow to what its land feeds. Cut seven times finer, a village draws its
    // farmers and its food from the cells around it, and is as large; cell by cell it
    // could hold no more than one cell's land feeds.
    let catchment = radius(area_km2(3)) - 0.01;
    let years = 800;
    let size = |res, radius_km| {
        let (mut w, ..) = farming_world(res, radius_km, 6);
        w.run_years(years, 2, years);
        mean_log(&settlements(&w))
    };
    let coarse = size(3, catchment);
    assert_eq!(coarse, size(3, 0.0), "each resolution-3 cell is its own catchment");
    let (fine, cell_by_cell) = (size(4, catchment), size(4, 0.0));
    assert!((fine - coarse).abs() < 0.3, "mean log size {fine} against {coarse}");
    assert!(cell_by_cell < coarse - 0.7, "cell by cell {cell_by_cell} against {coarse}");
}

#[test]
fn catchments_are_partition_independent_and_restore() {
    let catchment = radius(area_km2(3));
    let run = |parts| {
        let (mut w, ..) = farming_world(4, catchment, 9);
        w.run_years(500, parts, 500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(4, catchment, 9);
    w.run_years(317, 3, 317);
    assert!(w.agent_slices().any(|a| !a.alone(0..a.len())), "the checkpoint holds villages");
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(183, 2, 183);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn obsidian_passes_between_catchments_as_between_cells() {
    // Resolution-3 cells in catchments of seven (resolution-2 cells): obsidian passes from
    // catchment to catchment as it would from cell to cell of a resolution-2 grid.
    use oikos_sim::catchment::Catchments;
    use oikos_sim::exchange::{pass_between_catchments, pass_down_the_line};
    let (fine, coarse) = (Grid::from_base_cell(22, 3).unwrap(), Grid::from_base_cell(22, 2).unwrap());
    let c = Catchments::new(&fine, |_| 100.0, radius(700.0));
    let coarse_of = |cell: usize| {
        let p = fine.parent(cell as u32, 2);
        (0..coarse.len()).find(|&k| coarse.h3(k as u32) == p).unwrap()
    };
    let source = 0;
    let everyone = vec![1u64; fine.len()];
    let supplied: Vec<bool> = (0..fine.len()).map(|x| coarse_of(x) == source).collect();
    let (mut x, mut next) = (vec![0u32; fine.len()], vec![0u32; fine.len()]);
    let (mut y, mut y_next) = (vec![0u32; coarse.len()], vec![0u32; coarse.len()]);
    let coarse_supplied: Vec<bool> = (0..coarse.len()).map(|k| k == source).collect();
    for _ in 0..30 {
        pass_between_catchments(&fine, &c, &everyone, &supplied, 0.02, &x, &mut next);
        std::mem::swap(&mut x, &mut next);
        pass_down_the_line(&coarse, &vec![1; coarse.len()], &coarse_supplied, 0.02, &y, &mut y_next);
        std::mem::swap(&mut y, &mut y_next);
    }
    assert!(x.iter().filter(|&&v| v > 0).count() > fine.len() / 2, "it spreads");
    for cell in 0..fine.len() {
        assert_eq!(x[cell], y[coarse_of(cell)], "cell {cell}");
    }
}

#[test]
fn a_checkpoint_restores_unchanged_at_a_change_of_the_land() {
    // Checkpointed as a capacity period ends, before the next one's land is computed: the
    // restored world holds what the checkpoint did.
    let res = 4;
    let h3 = h3_cells(res);
    let mut b = PackBuilder::new("climate", region(&h3, res));
    b.time(vec![-12_000, -11_000], TimeRule::Linear);
    let t = vec![(CLIMATE.0 * 100.0) as i16; 2 * h3.len()];
    let p: Vec<u16> = [100u16, 3000].iter().flat_map(|&mm| vec![mm; h3.len()]).collect();
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(t)).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(p)).unwrap();
    let (g, c) = (geometry_pack(&h3, res), b.build());
    let params = Params { catchment_radius_km: radius(area_km2(3)), ..Params::default() };
    let mut w = world(&[&g, &c], 4, params);
    w.run_years(20, 2, 20);
    let bytes = checkpoint::encode(&w);
    let restored = checkpoint::decode_with(&bytes, inputs(&[&g, &c])).unwrap();
    assert!(checkpoint::encode(&restored) == bytes, "the restored world encodes as the checkpoint");
}
