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

/// A world of herders with Egypt's hypotheses off (#51): calibrated on since ADR-0116,
/// each test turns on the one it tests. Stock exchange alone carries domestic animals
/// across these tests' deserts, along the one-person bands a cell's capacity keeps there.
fn herders() -> Params {
    Params {
        herd_exchange: 0.0,
        herd_reserve: 0.0,
        herder_reach_km: 0.0,
        floodplain_grazing: 0.0,
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
        herd_reserve: 0.0,
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
fn herds_move_toward_the_domestic_share_of_the_herds_around_them() {
    // Animals pass along social ties, as gifts, bridewealth and loans that pool the risk
    // of losing a herd (Aktipis, Cronk and de Aguiar 2011), so a band's stock takes on
    // the domestic share of the herds of the people it is tied to (#51).
    let c = Cultivation::from_params(&Params { herd_exchange: 0.2, ..Params::default() });
    assert_eq!(c.exchange_stock(0.1, 0.9), 0.1 + 0.2 * (0.9 - 0.1));
    assert_eq!(c.exchange_stock(0.9, 0.1), 0.9 - 0.2 * 0.8, "wild animals come too");
    assert_eq!(c.exchange_stock(0.0, 1.0), 0.2, "a band without a herd gets animals to start one");
    let off = Cultivation::from_params(&Params { herd_exchange: 0.0, ..Params::default() });
    assert_eq!(off.exchange_stock(0.1, 0.9), 0.1, "off at zero");
    let all = Cultivation::from_params(&Params { herd_exchange: 1.0, ..Params::default() });
    assert_eq!(all.exchange_stock(0.1, 0.9), 0.9, "never past what it sees");
    assert_eq!(Params::default().herd_exchange, 0.2, "calibrated on M1 (ADR-0116)");
}

#[test]
fn domestic_animals_pass_to_neighbors_where_no_wild_ones_lived() {
    // Wild herds range over a core of cells; around it lies steppe whose grass would feed
    // domestic herds but where no wild ones lived. Nobody moves (no fission or
    // dispersal), so only what passes between people can carry herding beyond the core:
    // learning alone carries the practice but no animals to practise it on; exchange
    // carries the animals (#51).
    let h3 = h3_cells();
    let g = Grid::from_base_cell(22, 3).unwrap();
    let core = |c: usize| g.neighbors(0).contains(&(c as u32)) || c == 0;
    let (geo, c, k) = (geometry_pack(&h3), climate_pack(&h3), caprines_pack(&h3, core));
    let id = blake3::hash(&geo).to_hex().to_string();
    let beyond = |cell: u32| !core(cell as usize);
    let run = |herd_exchange: f64| {
        let p = Params {
            initial_occupancy: 1.0,
            fission_probability: 0.0,
            dispersal_probability: 0.0,
            neighbor_learning: 1.0,
            herd_selection: 0.1,
            herd_exchange,
            ..herders()
        };
        let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, p, inputs(&[&geo, &c, &k])).unwrap();
        w.run_years(2_000, 2, 2_000);
        (herding_in(&w, |c| core(c as usize)), herding_in(&w, beyond))
    };
    let ((bred, _), (alone, _)) = run(0.0);
    assert!(bred > 100, "{bred} herd in the core");
    assert_eq!(alone, 0, "without exchange nobody beyond the core herds");
    let (_, (exchanged, domestic)) = run(0.2);
    assert!(exchanged > 1_000, "{exchanged} herd beyond the core with exchange");
    assert!(domestic > 0.5, "their herds are domestic: {domestic}");
}

#[test]
fn bands_keep_a_few_domestic_animals_as_a_reserve_wherever_they_can_graze_any() {
    // Animals are food that keeps: hunters kept a few goats as a reserve against lean
    // times, a handful of bones among their game, where the land fed few herds (Linseele
    // et al. 2014; #51). The reserve is the least a band with domestic stock keeps, in
    // proportion to its share; where the land repays more herding, the land sets it.
    let c = Cultivation::from_params(&Params { herding_weight: 0.5, herd_reserve: 0.1, ..Params::default() });
    assert!((c.herding_target(0.0, 0.02, 1.0, 0.0) - 0.1).abs() < 1e-12, "the reserve, not 0.5 x 0.02");
    assert!((c.herding_target(0.0, 0.02, 0.5, 0.0) - 0.05).abs() < 1e-12, "half domestic: half the reserve");
    assert_eq!(c.herding_target(0.0, 1.0, 1.0, 0.0), 0.5, "rich pasture: the land's target");
    assert_eq!(c.herding_target(0.0, 0.0, 1.0, 0.0), 0.0, "nothing to graze: no herd");
    assert_eq!(c.herding_target(0.0, 1.0, 0.0, 0.0), 0.0, "no domestic animals: no reserve");
    let wild = c.herding_target(0.1, 0.1, 0.0, 0.0);
    assert!((wild - 0.05).abs() < 1e-12, "wild herds are no reserve: {wild}");
    assert_eq!(Params::default().herd_reserve, 0.2, "calibrated on M1 (ADR-0116)");
    // Animals a band holds are tended at once, not learned from others first: the
    // reserve is the least effort a band with domestic stock keeps on it.
    assert!((c.kept(0.0, 0.02, 0.6) - 0.06).abs() < 1e-12);
    assert_eq!(c.kept(0.0, 0.0, 0.6), 0.0, "nothing to graze: nothing kept");
    assert_eq!(c.kept(0.3, 0.3, 0.0), 0.0, "wild herds: nothing kept");
}

fn keeping(w: &World, which: impl Fn(u32) -> bool, least: f64) -> u64 {
    let least = (least * SHARE_ONE as f64) as u32;
    let mut people = 0;
    for a in w.agent_slices() {
        for i in 0..a.len() {
            if which(a.cell[i]) && a.herding[i] >= least {
                people += a.weight[i];
            }
        }
    }
    people
}

#[test]
fn hunters_beside_herders_keep_a_reserve_of_animals_on_poor_grazing() {
    // Wild herds range over a core of cells; around it lies dry land (160 mm) whose
    // grass feeds few herds. Nobody moves, and animals pass between neighbors (#51):
    // with a reserve, bands around the core keep a few domestic animals; without one,
    // they herd only what the poor grazing repays.
    let h3 = h3_cells();
    let g = Grid::from_base_cell(22, 3).unwrap();
    let core = |c: usize| g.neighbors(0).contains(&(c as u32)) || c == 0;
    let mut b = PackBuilder::new("climate", region(&h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    let precip = (0..h3.len()).map(|c| if core(c) { 250 } else { 160 }).collect();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(precip)).unwrap();
    let (geo, c, k) = (geometry_pack(&h3), b.build(), caprines_pack(&h3, core));
    let id = blake3::hash(&geo).to_hex().to_string();
    let beyond = |cell: u32| !core(cell as usize);
    let run = |herd_reserve: f64| {
        let p = Params {
            initial_occupancy: 1.0,
            fission_probability: 0.0,
            dispersal_probability: 0.0,
            neighbor_learning: 1.0,
            herd_selection: 0.1,
            herd_exchange: 0.2,
            wadi_share: 0.0,
            herding_weight: 0.12,
            herd_reserve,
            ..herders()
        };
        let mut w = World::with_inputs(WorldConfig::geometry(&id), 2, p, inputs(&[&geo, &c, &k])).unwrap();
        w.run_years(2_000, 2, 2_000);
        keeping(&w, beyond, 0.02)
    };
    let (without, with) = (run(0.0), run(0.05));
    assert!(with > 1_000, "{with} keep a reserve beyond the core");
    assert!(with > 10 * without.max(10), "{with} with a reserve, {without} without");
}

/// Herders beyond a desert after 1,000 years, and their herds' domestic share: steppe
/// where wild herds range, around base cell 22's first resolution-3 cell; then a belt of
/// barren desert three such cells (some 200 km) wide, whose wadis are dry and whose
/// foraging feeds about one person a cell; then steppe beyond, with no wild herds. The
/// land is cut at `res` (4,000 km2 a cell at resolution 3), with catchments of
/// resolution-3 cells' size.
fn beyond_a_desert(res: u8, herder_reach_km: f64) -> (u64, f64) {
    let g3 = Grid::from_base_cell(22, 3).unwrap();
    let mut ring = vec![u32::MAX; g3.len()];
    let mut frontier = vec![0u32];
    ring[0] = 0;
    while let Some(c) = frontier.pop() {
        for &n in g3.neighbors(c) {
            if ring[n as usize] > ring[c as usize] + 1 {
                ring[n as usize] = ring[c as usize] + 1;
                frontier.push(n);
            }
        }
    }
    let index: std::collections::BTreeMap<u64, usize> = (0..g3.len()).map(|c| (g3.h3(c as u32), c)).collect();
    let g = Grid::from_base_cell(22, res).unwrap();
    let h3: Vec<u64> = (0..g.len() as u32).map(|c| g.h3(c)).collect();
    let zone = |c: usize| match ring[index[&g.parent(c as u32, 3)]] {
        0..=3 => 0,
        4..=6 => 1,
        _ => 2,
    };
    let region =
        Region { name: "test".into(), resolution: res, cells: h3.len() as u32, cells_hash: cells_hash(&h3) };
    let mut b = PackBuilder::new("geometry", region.clone());
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.clone())).unwrap();
    let area = (4_000.0 / 7f64.powi(res as i32 - 3) * 1e6) as u32;
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(vec![area; h3.len()])).unwrap();
    let geo = b.build();
    let mut b = PackBuilder::new("climate", region.clone());
    b.time(vec![-12_000], TimeRule::Linear);
    let temp = (0..h3.len()).map(|c| if zone(c) == 1 { -4000 } else { 1500 }).collect();
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(temp)).unwrap();
    let precip = (0..h3.len()).map(|c| if zone(c) == 1 { 60 } else { 250 }).collect();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(precip)).unwrap();
    let climate = b.build();
    let mut b = PackBuilder::new("caprines", region);
    let upland = (0..h3.len()).map(|c| if zone(c) == 0 { 10_000 } else { 0 }).collect();
    b.field(spec("upland", Dims::Cell, 1e-4), Data::U16(upland)).unwrap();
    let caprines = b.build();
    let id = blake3::hash(&geo).to_hex().to_string();
    let catchment_radius_km = (4_000.0 / std::f64::consts::PI).sqrt();
    let p =
        Params { wadi_share: 0.0, herd_selection: 0.1, herder_reach_km, catchment_radius_km, ..herders() };
    let mut w =
        World::with_inputs(WorldConfig::geometry(&id), 3, p, inputs(&[&geo, &climate, &caprines])).unwrap();
    w.run_years(1_000, 2, 1_000);
    herding_in(&w, |c| zone(c as usize) == 2)
}

#[test]
fn herders_whose_neighbors_have_no_room_travel_as_far_as_their_reach_to_find_some() {
    // A group leaving a crowded place goes next door if it can; herders who find no room
    // there travel on, as far as their reach, to the nearest land that can take them and
    // their flocks (#51). Without a reach nobody herds beyond the desert.
    // Cells of 4,000 km2 lie about 68 km apart: a reach of 300 km is four steps.
    let ((stayed, _), (travelled, domestic)) = (beyond_a_desert(3, 0.0), beyond_a_desert(3, 300.0));
    assert_eq!(stayed, 0, "without a reach nobody herds beyond the desert");
    assert!(travelled > 1_000, "{travelled} herd beyond the desert with a reach");
    assert!(domestic > 0.5, "with the domestic flocks they brought: {domestic}");
    let (short, _) = beyond_a_desert(3, 150.0);
    assert_eq!(short, 0, "a reach of two steps does not span three cells of desert");
    assert_eq!(Params::default().herder_reach_km, 250.0, "calibrated on M1 (ADR-0116)");
}

#[test]
fn a_reach_spans_as_many_km_however_finely_the_land_is_cut() {
    // The same land cut seven times finer, with catchments the size of the coarser cells:
    // a step goes from catchment to neighboring catchment, so the reach is in km.
    let (travelled, _) = beyond_a_desert(4, 300.0);
    assert!(travelled > 1_000, "{travelled} herd beyond the desert with a reach");
    assert_eq!(beyond_a_desert(4, 150.0).0, 0, "two catchments' steps do not span the desert");
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
