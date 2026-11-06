//! Herding (#37): bands manage sheep and goats where wild herds range, learn it from the
//! people around them as they learn cultivation, and breed the herds domestic; domestic
//! herds can be grazed on pasture beyond the wild range, wild ones not, and nobody tames
//! animals where none lived (Africa).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
use oikos_sim::environment::{Land, Practice, capacity_of, pasture_share};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::Cultivation;
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

/// Dry steppe everywhere (15 degC, 250 mm): too dry for wild cereals, grazing for herds.
fn climate_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![250; h3.len()])).unwrap();
    b.build()
}

/// Upland, where wild sheep and goats live, in cells `upland(c)`.
fn caprines_pack(h3: &[u64], upland: impl Fn(usize) -> bool) -> Vec<u8> {
    let mut b = PackBuilder::new("caprines", region(h3));
    let v = (0..h3.len()).map(|c| if upland(c) { 10_000 } else { 0 }).collect();
    b.field(spec("upland", Dims::Cell, 1e-4), Data::U16(v)).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

fn herders() -> Params {
    Params {
        initial_occupancy: 0.5,
        herding_weight: 0.6,
        adoption_rate: 0.05,
        innovation_share: 1.0,
        herd_selection: 0.02,
        dispersal_probability: 0.3,
        fission_probability: 0.3,
        ..Params::default()
    }
}

#[test]
fn bands_aim_to_herd_where_wild_herds_range_and_domestic_ones_can_graze() {
    let c = Cultivation::from_params(&Params { herding_weight: 0.5, ..Params::default() });
    assert_eq!(c.herding_target(0.8, 1.0, 0.0, 0.0), 0.4, "wild herds on their range only");
    assert_eq!(c.herding_target(0.0, 1.0, 0.0, 0.0), 0.0, "no wild animals to start with");
    assert_eq!(c.herding_target(0.0, 1.0, 1.0, 0.0), 0.5, "a domestic herd grazes any pasture");
    assert_eq!(c.herding_target(0.2, 0.6, 0.5, 0.0), 0.2, "half domestic: the range and half the rest");
}

#[test]
fn crowded_bands_aim_to_herd_more_wherever_their_flocks_can_graze() {
    // Need drives herding as it drives cultivation (#44): people beyond what foraging
    // feeds turn to their domestic flocks, on any land they can graze, however little.
    let p = Params {
        herding_weight: 0.1,
        herd_pressure_weight: 0.6,
        cultivation_crowding: 1.0,
        ..Params::default()
    };
    let c = Cultivation::from_params(&p);
    assert_eq!(c.pressure(100, 100), 0.0, "foragers at their land's capacity feel none");
    assert_eq!(c.pressure(150, 100), 0.5);
    assert_eq!(c.pressure(300, 100), 1.0, "full at one capacity more");
    assert!((c.herding_target(0.0, 1.0, 1.0, 1.0) - 0.7).abs() < 1e-12);
    assert!((c.herding_target(0.0, 1.0, 1.0, 0.5) - 0.4).abs() < 1e-12);
    let desert = c.herding_target(0.0, 0.02, 1.0, 1.0);
    assert!(
        (desert - (0.1 * 0.02 + 0.6)).abs() < 1e-12,
        "a little grazing is enough to keep a flock: {desert}"
    );
    assert_eq!(c.herding_target(0.0, 1.0, 0.0, 1.0), 0.0, "no wild animals and no herd: nothing to herd");
    // Wild herds cannot be worked harder to feed more people: need acts through domestic
    // stock, in proportion to its share.
    assert!((c.herding_target(1.0, 1.0, 0.0, 1.0) - 0.1).abs() < 1e-12, "wild herds: the land alone");
    assert!(
        (c.herding_target(1.0, 1.0, 0.5, 1.0) - (0.1 + 0.3)).abs() < 1e-12,
        "half domestic: half the need"
    );
    assert_eq!(c.herding_target(0.0, 0.0, 1.0, 1.0), 0.0, "nothing to graze: no herding");
    let none = Cultivation::from_params(&Params { herd_pressure_weight: 0.0, ..p });
    assert_eq!(none.herding_target(0.0, 1.0, 1.0, 1.0), 0.1, "weight 0: the land alone, as before");
}

#[test]
fn wadis_gather_runoff_into_grazing_where_rain_alone_grows_none() {
    // Below pasture_precip_min the open land grows no grazing, but wadi beds, a share
    // wadi_share of the land, collect runoff_coefficient of the rain on the rest (#44).
    let p = Params {
        pasture_precip_min: 150.0,
        pasture_precip_full: 300.0,
        wadi_share: 0.1,
        runoff_coefficient: 0.2,
        ..Params::default()
    };
    let wadi = |rain: f64| rain * (1.0 + 0.2 * 0.9 / 0.1);
    let at100 = pasture_share(100.0, &p);
    assert!((at100 - 0.1 * (wadi(100.0) - 150.0) / 150.0).abs() < 1e-12, "{at100}");
    assert_eq!(pasture_share(0.0, &p), 0.0, "no rain, no runoff");
    assert_eq!(pasture_share(300.0, &p), 1.0, "full grazing everywhere");
    let at200 = pasture_share(200.0, &p);
    assert!((at200 - (0.9 * 50.0 / 150.0 + 0.1)).abs() < 1e-12, "open land and full wadis: {at200}");
    let mut last = 0.0;
    for rain in 0..400 {
        let s = pasture_share(rain as f64, &p);
        assert!((0.0..=1.0).contains(&s) && s >= last, "{rain} mm: {s}");
        last = s;
    }
    let dry = Params { wadi_share: 0.0, ..p };
    assert_eq!(pasture_share(100.0, &dry), 0.0, "without wadis, as before");
    assert!((pasture_share(225.0, &dry) - 0.5).abs() < 1e-12);
}

#[test]
fn cultivation_and_herding_share_one_effort_budget() {
    assert_eq!(Cultivation::share_effort(0.3, 0.4), (0.3, 0.4));
    let (e, h) = Cultivation::share_effort(0.9, 0.6);
    assert!((e + h - 1.0).abs() < 1e-12 && (e / h - 1.5).abs() < 1e-12, "{e} {h}");
}

#[test]
fn herds_become_domestic_from_wild_animals_only_where_they_range() {
    let c = Cultivation::from_params(&Params {
        herd_selection: 0.1,
        herd_wild_frequency: 0.01,
        ..Params::default()
    });
    let d = c.domesticate_herd(0.0, 1.0, 1.0);
    assert!((d - 0.001).abs() < 1e-12, "from the wild animals' domestic-type share: {d}");
    assert_eq!(c.domesticate_herd(0.0, 1.0, 0.0), 0.0, "beyond the range there is nothing to tame");
    assert!(c.domesticate_herd(0.5, 1.0, 0.0) > 0.5, "a herd brought there keeps being bred");
    assert_eq!(c.domesticate_herd(0.5, 0.0, 0.0), 0.5, "and keeps its traits, with no wild animals near");
    assert!(c.domesticate_herd(0.5, 0.0, 1.0) < 0.5, "where they range, gene flow pulls it back");
}

#[test]
fn wild_herds_feed_people_only_on_their_range_and_domestic_ones_on_all_pasture() {
    let p = Params { wild_herd_ratio: 0.5, ..Params::default() };
    let land = Land { forage: 100, herd_range: 200.0, herd_pasture: 600.0, ..Land::default() };
    let at = |herding: f64, herd_crop: f64| {
        capacity_of(&land, &Practice { herding, herd_crop, ..Practice::default() }, &p)
    };
    assert_eq!(at(0.0, 0.0), 100);
    assert_eq!(at(0.5, 0.0), 50 + 50, "wild herds: half of the range's 200 at half yield");
    assert_eq!(at(0.5, 0.5), 50 + 100 + 200, "domestic herds: half of the range and of the rest");
}

/// Herding bands' people and their herds' mean domestic share in cells `which`.
fn herding_in(w: &World, which: impl Fn(u32) -> bool) -> (u64, f64) {
    let (mut people, mut domestic) = (0u64, 0.0);
    for a in w.agent_slices() {
        for i in 0..a.len() {
            if which(a.cell[i]) && a.herding[i] >= SHARE_ONE / 10 {
                people += a.weight[i];
                domestic += a.weight[i] as f64 * a.herd_domestication[i] as f64 / SHARE_ONE as f64;
            }
        }
    }
    (people, if people > 0 { domestic / people as f64 } else { 0.0 })
}

#[test]
fn bands_herd_and_breed_herds_domestic_where_wild_sheep_and_goats_range() {
    let h3 = h3_cells();
    let (g, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, |_| true));
    let id = blake3::hash(&g).to_hex().to_string();
    let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, herders(), inputs(&[&g, &c, &k])).unwrap();
    w.run_years(2_000, 2, 2_000);
    let (people, domestic) = herding_in(&w, |_| true);
    assert!(people > 1_000, "{people} people herd");
    assert!(domestic > 0.5, "their herds are mostly domestic: {domestic}");
    for a in w.agent_slices() {
        for i in 0..a.len() {
            assert!(a.cultivation[i] as u64 + a.herding[i] as u64 <= SHARE_ONE as u64);
        }
    }

    let none = Params { herding_weight: 0.0, ..herders() };
    let mut w0 = World::with_inputs(WorldConfig::geometry(&id), 2, none, inputs(&[&g, &c, &k])).unwrap();
    w0.run_years(2_000, 2, 2_000);
    assert_eq!(herding_in(&w0, |_| true).0, 0, "herding_weight 0: nobody herds");
}

#[test]
fn nobody_tames_animals_where_no_wild_ones_lived() {
    let h3 = h3_cells();
    let (g, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, |_| false));
    let id = blake3::hash(&g).to_hex().to_string();
    let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, herders(), inputs(&[&g, &c, &k])).unwrap();
    w.run_years(2_000, 2, 2_000);
    assert_eq!(herding_in(&w, |_| true).0, 0, "pasture alone, no wild herds: no herding");
    let mut bare = World::with_inputs(WorldConfig::geometry(&id), 2, herders(), inputs(&[&g, &c])).unwrap();
    bare.run_years(500, 2, 500);
    assert_eq!(herding_in(&bare, |_| true).0, 0, "without a caprines pack, none either");
}

#[test]
fn domestic_herds_are_grazed_beyond_the_wild_range() {
    // Wild herds range over half the cells; herders breed them domestic and then graze
    // the pasture of the other half too, which wild herds alone never let them.
    let h3 = h3_cells();
    let half = h3.len() / 2;
    let (g, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, |i| i < half));
    let id = blake3::hash(&g).to_hex().to_string();
    let beyond = |cell: u32| cell as usize >= half;
    let run = |herd_selection: f64| {
        let p = Params { herd_selection, ..herders() };
        let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, p, inputs(&[&g, &c, &k])).unwrap();
        w.run_years(3_000, 2, 3_000);
        herding_in(&w, beyond).0
    };
    let (bred, wild) = (run(0.02), run(0.0));
    assert!(bred > 10 * wild.max(100), "{bred} herd beyond the range with domestic herds, {wild} without");
}

#[test]
fn herders_move_onto_pasture_where_foragers_find_nothing_and_nobody_onto_barren_land() {
    // A third of the land is warm steppe where wild herds range; a third is frozen steppe
    // (-40 degC, 250 mm), whose grass feeds herds but whose foraging feeds about one
    // person a cell; the last third is frozen desert, which feeds nobody. Herders judge
    // where to go by what their herds would graze there (#45), so they fill the frozen
    // steppe, and nobody lives in the desert beyond the one person a cell's capacity
    // never falls below.
    let h3 = h3_cells();
    let third = h3.len() / 3;
    let zone = |c: usize| c / third.max(1); // 0 warm, 1 frozen steppe, 2 frozen desert
    let mut b = PackBuilder::new("climate", region(&h3));
    b.time(vec![-12_000], TimeRule::Linear);
    let temp = (0..h3.len()).map(|c| if zone(c) == 0 { 1500 } else { -4000 }).collect();
    let precip = (0..h3.len()).map(|c| if zone(c) == 2 { 0 } else { 250 }).collect();
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(temp)).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(precip)).unwrap();
    let (g, c, k) = (geometry_pack(&h3), b.build(), caprines_pack(&h3, |i| zone(i) == 0));
    let id = blake3::hash(&g).to_hex().to_string();
    let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, herders(), inputs(&[&g, &c, &k])).unwrap();
    w.run_years(3_000, 2, 3_000);
    let steppe = herding_in(&w, |c| zone(c as usize) == 1).0;
    let cells = h3.len() - 2 * third;
    assert!(steppe > 100 * third as u64, "{steppe} herders on the frozen steppe's {third} cells");
    let desert: u64 = (0..h3.len()).filter(|&c| zone(c) == 2).map(|c| w.population[c]).sum();
    assert!(desert <= cells as u64, "{desert} people in the desert's {cells} cells");
}

#[test]
fn herders_cross_a_desert_whose_wadis_feed_their_flocks() {
    // Warm steppe where wild herds range, around the grid's first cell; then a belt of
    // frozen desert (-40 degC, 60 mm) three cells wide, whose foraging feeds about one
    // person a cell and whose open land grows no grass, only its wadis; then steppe
    // beyond, with no wild herds. Herders carry their domestic
    // flocks across only if need keeps them herding where their land feeds little else
    // and the wadis feed their flocks (#44); without wadis nobody herds beyond.
    let h3 = h3_cells();
    let g = Grid::from_base_cell(22, 3).unwrap();
    let mut ring = vec![u32::MAX; g.len()];
    let mut frontier = vec![0u32];
    ring[0] = 0;
    while let Some(c) = frontier.pop() {
        for &n in g.neighbors(c) {
            if ring[n as usize] > ring[c as usize] + 1 {
                ring[n as usize] = ring[c as usize] + 1;
                frontier.push(n);
            }
        }
    }
    let zone = |c: usize| match ring[c] {
        0..=3 => 0, // steppe with wild herds
        4..=6 => 1, // desert
        _ => 2,     // steppe beyond
    };
    assert!((0..h3.len()).filter(|&c| zone(c) == 2).count() > 50, "a steppe beyond the desert");
    let mut b = PackBuilder::new("climate", region(&h3));
    b.time(vec![-12_000], TimeRule::Linear);
    let temp = (0..h3.len()).map(|c| if zone(c) == 1 { -4000 } else { 1500 }).collect();
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(temp)).unwrap();
    let precip = (0..h3.len()).map(|c| if zone(c) == 1 { 60 } else { 250 }).collect();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(precip)).unwrap();
    let (geo, c, k) = (geometry_pack(&h3), b.build(), caprines_pack(&h3, |i| zone(i) == 0));
    let id = blake3::hash(&geo).to_hex().to_string();
    let beyond = |cell: u32| zone(cell as usize) == 2;
    // Herders beyond the desert after 500 years, and the desert's people after 1,500.
    let run = |p: Params| {
        let mut w = World::with_inputs(WorldConfig::geometry(&id), 3, p, inputs(&[&geo, &c, &k])).unwrap();
        w.run_years(500, 2, 500);
        let crossed = herding_in(&w, beyond).0;
        w.run_years(1_000, 2, 1_000);
        (crossed, (0..h3.len()).filter(|&c| zone(c) == 1).map(|c| w.population[c]).sum::<u64>())
    };
    let crossing = Params {
        herd_pressure_weight: 0.8,
        wadi_share: 0.05,
        runoff_coefficient: 0.2,
        herd_selection: 0.1, // herds domestic within a few centuries, so need acts
        ..herders()
    };
    let (crossed, desert) = run(crossing.clone());
    let (dry, empty) = run(Params { wadi_share: 0.0, ..crossing.clone() });
    let (unneeded, sparse) = run(Params { herd_pressure_weight: 0.0, ..crossing });
    assert!(crossed > 1_000, "{crossed} herd beyond the desert");
    assert!(crossed > 10 * dry.max(10), "{crossed} cross with wadis, {dry} without");
    assert!(crossed > 10 * unneeded.max(10), "{crossed} cross with need, {unneeded} without");
    // A cell's capacity never falls below one person, so a few cross even without wadis,
    // but only herders kept herding by need live in the desert itself.
    assert!(desert > 10 * empty.max(10), "{desert} live in the desert with wadis, {empty} without");
    assert!(desert > 3 * sparse, "{desert} with need, {sparse} without");
}

#[test]
fn herding_is_partition_independent_and_restores() {
    let h3 = h3_cells();
    let (g, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, |i| i % 3 != 0));
    let id = blake3::hash(&g).to_hex().to_string();
    let world =
        || World::with_inputs(WorldConfig::geometry(&id), 7, herders(), inputs(&[&g, &c, &k])).unwrap();
    let run = |parts| {
        let mut w = world();
        w.run_years(1_200, parts, 1_200);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let mut w = world();
    w.run_years(700, 3, 700);
    assert!(w.agent_slices().any(|a| a.herding.iter().any(|&h| h > 0)), "the checkpoint holds herders");
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c, &k])).unwrap();
    restored.run_years(500, 2, 500);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn surveys_of_herding_count_each_cells_herders() {
    let h3 = h3_cells();
    let (g, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, |_| true));
    let id = blake3::hash(&g).to_hex().to_string();
    let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, herders(), inputs(&[&g, &c, &k])).unwrap();
    w.run_years(1_500, 2, 1_500);
    let n = w.grid.len();
    let (mut people, mut herd, mut domestic) = (vec![0u128; n], vec![0u128; n], vec![0u128; n]);
    for a in w.agent_slices() {
        for i in 0..a.len() {
            let (cell, wt, h) = (a.cell[i] as usize, a.weight[i] as u128, a.herding[i] as u128);
            people[cell] += wt;
            herd[cell] += wt * h;
            domestic[cell] += wt * (h * a.herd_domestication[i] as u128 / SHARE_ONE as u128);
        }
    }
    let (herding, herd_domestic) = (w.herding(), w.herd_domestication());
    for c in 0..n {
        let want = if people[c] > 0 { herd[c] as f64 / (people[c] as f64 * SHARE_ONE as f64) } else { 0.0 };
        assert!((herding[c] - want).abs() < 1e-9, "cell {c}: {} against {want}", herding[c]);
        let want = if herd[c] > 0 { domestic[c] as f64 / herd[c] as f64 } else { 0.0 };
        assert!((herd_domestic[c] - want).abs() < 1e-9, "cell {c}");
    }
    assert!(herding.iter().any(|&h| h > 0.1));
}
