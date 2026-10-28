//! Forcing packs as run inputs: regions defined by a geometry pack, packs named and
//! verified by id in work units and checkpoints, and fields visible to mechanisms
//! (docs/03-engine.md §3.8a).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, PackBuilder, Region, cells_hash};
use oikos_sim::inputs::{Inputs, PackRef};
use oikos_sim::mechanisms::{Mechanism, PartitionView, Sense};
use oikos_sim::schedule::{Component, MechanismSpec};
use oikos_sim::{Params, WorkUnit, World, WorldConfig, checkpoint, run_work_unit};
use std::collections::BTreeMap;
use std::ops::Range;

const BASE: WorldConfig = WorldConfig::base_cell(22, 3);

fn h3_cells(base: u8, res: u8) -> Vec<u64> {
    let g = Grid::from_base_cell(base, res).unwrap();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

fn region(h3: &[u64]) -> Region {
    Region { name: "test".into(), resolution: 3, cells: h3.len() as u32, cells_hash: cells_hash(h3) }
}

fn spec(name: &str, unit: &str) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        unit: unit.into(),
        description: name.into(),
        dims: Dims::Cell,
        scale: 1.0,
        offset: 0.0,
        nodata: None,
    }
}

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", "h3 index"), Data::U64(h3.to_vec())).unwrap();
    b.build()
}

/// Elevation 2000 m in every third cell, 100 m elsewhere.
fn terrain_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("terrain", region(h3));
    let elev = (0..h3.len()).map(|c| if c % 3 == 0 { 2000 } else { 100 }).collect();
    b.field(spec("elev", "m"), Data::U16(elev)).unwrap();
    b.build()
}

fn id(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

struct Files(BTreeMap<String, Vec<u8>>);

impl Files {
    fn new(packs: &[&[u8]]) -> Files {
        Files(packs.iter().map(|p| (format!("packs/{}.oikospack", id(p)), p.to_vec())).collect())
    }
    fn refs(&self) -> Vec<PackRef> {
        self.0.keys().map(|path| PackRef { id: path[6..70].to_string(), path: path.clone() }).collect()
    }
    fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        self.0.get(path).cloned().ok_or_else(|| format!("{path}: not found"))
    }
}

fn geo_world(seed: u64) -> (World, Files) {
    let h3 = h3_cells(22, 3);
    let (g, t) = (geometry_pack(&h3), terrain_pack(&h3));
    let files = Files::new(&[&g, &t]);
    let inputs = Inputs::load(&files.refs(), |p| files.read(p)).unwrap();
    let config = WorldConfig::geometry(&id(&g));
    (World::with_inputs(config, seed, Params::default(), inputs).unwrap(), files)
}

#[test]
fn a_geometry_pack_region_runs_like_the_same_base_cell_region() {
    let mut by_cell = World::new(BASE, 5, Params::default()).unwrap();
    let (mut by_pack, _) = geo_world(5);
    assert_eq!(by_pack.grid.len(), by_cell.grid.len());
    assert_eq!(by_pack.run_years(300, 3, 50), by_cell.run_years(300, 3, 50));
    assert_eq!(by_pack.population, by_cell.population);
}

#[test]
fn geometry_regions_are_partition_independent() {
    let run = |parts| {
        let (mut w, _) = geo_world(9);
        w.run_years(200, parts, 100);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    assert_eq!(run(1), run(7));
}

#[test]
fn inputs_are_verified() {
    let h3 = h3_cells(22, 3);
    let (g, t) = (geometry_pack(&h3), terrain_pack(&h3));
    let files = Files::new(&[&g, &t]);
    let load = |refs: &[PackRef]| Inputs::load(refs, |p| files.read(p)).map(|_| ()).unwrap_err();

    let mut refs = files.refs();
    refs[0].id = "ab".repeat(32);
    assert!(load(&refs).contains("pack id mismatch"));

    let other = h3_cells(31, 3);
    let foreign = terrain_pack(&other);
    let files2 = Files::new(&[&g, &foreign]);
    let e = Inputs::load(&files2.refs(), |p| files2.read(p)).map(|_| ()).unwrap_err();
    assert!(e.contains("cells_hash") || e.contains("region"), "{e}");

    let files3 = Files::new(&[&g, &geometry_pack(&h3[..h3.len() - 1])]);
    let e = Inputs::load(&files3.refs(), |p| files3.read(p)).map(|_| ()).unwrap_err();
    assert!(e.contains("geometry"), "{e}");

    let inputs = Inputs::load(&files.refs(), |p| files.read(p)).unwrap();
    let e = World::with_inputs(WorldConfig::geometry(&id(&t)), 1, Params::default(), inputs)
        .map(|_| ())
        .unwrap_err();
    assert!(e.contains("geometry pack"), "{e}");
    let inputs = Inputs::load(&files.refs(), |p| files.read(p)).unwrap();
    let e = World::with_inputs(BASE, 1, Params::default(), inputs).map(|_| ()).unwrap_err();
    assert!(e.contains("geometry"), "{e}");
    let e = World::new(WorldConfig::geometry(&id(&g)), 1, Params::default()).map(|_| ()).unwrap_err();
    assert!(e.contains(&id(&g)), "{e}");
}

#[test]
fn checkpoints_record_their_packs() {
    let (mut w, files) = geo_world(3);
    w.run_years(100, 2, 100);
    let bytes = checkpoint::encode(&w);
    let e = checkpoint::decode(&bytes).map(|_| ()).unwrap_err();
    assert!(e.contains("forcing packs"), "{e}");

    let inputs = Inputs::load(&files.refs(), |p| files.read(p)).unwrap();
    let mut restored = checkpoint::decode_with(&bytes, inputs).unwrap();
    assert_eq!(checkpoint::state_hash(&restored), checkpoint::state_hash(&w));
    w.run_years(100, 2, 100);
    restored.run_years(100, 3, 100);
    assert_eq!(checkpoint::state_hash(&restored), checkpoint::state_hash(&w));

    let h3 = h3_cells(22, 3);
    let only_geometry = Files::new(&[&geometry_pack(&h3)]);
    let inputs = Inputs::load(&only_geometry.refs(), |p| only_geometry.read(p)).unwrap();
    let e = checkpoint::decode_with(&bytes, inputs).map(|_| ()).unwrap_err();
    assert!(e.contains("forcing packs"), "{e}");
}

#[test]
fn base_cell_checkpoints_keep_format_version_1() {
    let w = World::new(BASE, 3, Params::default()).unwrap();
    let bytes = checkpoint::encode(&w);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 1);
}

#[test]
fn work_units_name_their_packs() {
    let (mut direct, files) = geo_world(21);
    direct.run_years(200, 2, 100);
    let g_id = Inputs::load(&files.refs(), |p| files.read(p)).unwrap().geometry().unwrap().id().to_string();
    let wu = |start: serde_json::Value, years: u64, out: Option<&str>| -> WorkUnit {
        serde_json::from_value(serde_json::json!({
            "format": 1, "world": {"geometry": g_id}, "seed": 21, "start": start, "years": years,
            "partitions": 2, "summary_every_years": 100, "checkpoint_out": out,
            "inputs": files.refs().iter().map(|r| serde_json::json!({"id": r.id, "path": r.path})).collect::<Vec<_>>(),
        }))
        .unwrap()
    };
    let written = std::cell::RefCell::new(BTreeMap::<String, Vec<u8>>::new());
    let read = |p: &str| written.borrow().get(p).cloned().map_or_else(|| files.read(p), Ok);
    let write = |p: &str, b: &[u8]| {
        written.borrow_mut().insert(p.to_string(), b.to_vec());
        Ok(())
    };
    let whole = run_work_unit(&wu(serde_json::json!({"kind": "initial"}), 200, None), read, write).unwrap();
    assert_eq!(whole.state_hash, checkpoint::state_hash(&direct));

    run_work_unit(&wu(serde_json::json!({"kind": "initial"}), 100, Some("mid.ckpt")), read, write).unwrap();
    let resumed = run_work_unit(
        &wu(serde_json::json!({"kind": "checkpoint", "path": "mid.ckpt"}), 100, None),
        read,
        write,
    )
    .unwrap();
    assert_eq!(resumed.state_hash, whole.state_hash);
}

/// Kills everyone in cells above 1000 m: a mechanism reading a pack field.
struct Highlands;

impl Mechanism for Highlands {
    fn spec(&self) -> MechanismSpec {
        MechanismSpec {
            name: "highlands",
            period_ticks: 365,
            reads: vec![Component::Forcing],
            writes: vec![Component::AgentWeight],
            after: vec![],
        }
    }

    fn decide(&self, sense: &Sense<'_>, part: &mut PartitionView<'_>, tile: Range<usize>) {
        let elev = sense.inputs.field("terrain", "elev").unwrap();
        for i in tile {
            if elev.get(0, part.agents.cell[i] as usize).unwrap() > 1000.0 {
                part.stats.deaths += part.agents.weight[i];
                part.agents.weight[i] = 0;
            }
        }
    }
}

#[test]
fn mechanisms_read_pack_fields_through_sense() {
    let (mut w, _) = geo_world(4);
    w.run_years(400, 2, 400);
    let high = |w: &World| -> u64 { (0..w.grid.len()).filter(|c| c % 3 == 0).map(|c| w.population[c]).sum() };
    let before = (w.total_population(), high(&w));
    assert!(before.1 > 0, "population must have reached high cells");
    w.set_mechanisms(vec![Box::new(Highlands)]).unwrap();
    let stats = w.step(3);
    assert_eq!(high(&w), 0);
    assert_eq!(stats.deaths, before.1);
    assert_eq!(w.total_population(), before.0 - before.1);
}
