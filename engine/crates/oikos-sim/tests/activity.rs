//! The calibration record as a work-unit output (#46): what `oikos_cal.record.activity`
//! samples in Python, written by the engine so a calibration world runs wherever the
//! CLI runs.

use flate2::read::GzDecoder;
use oikos_sim::activity::ActivitySpec;
use oikos_sim::inputs::PackRef;
use oikos_sim::workunit::Start;
use oikos_sim::{WorkResult, WorkUnit, World, WorldConfig, run_work_unit};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;

type Files = BTreeMap<String, Vec<u8>>;

fn unit(partitions: usize, activity: Option<ActivitySpec>) -> WorkUnit {
    WorkUnit {
        format: 1,
        id: None,
        world: WorldConfig::SOUTHWEST_ASIA,
        seed: 42,
        params: None,
        start: Start::Initial,
        years: 300,
        partitions,
        summary_every_years: 100,
        checkpoint_out: None,
        outputs: None,
        activity,
        inputs: vec![],
    }
}

fn spec() -> ActivitySpec {
    ActivitySpec { dir: "rec".into(), every_years: 10, zones: None }
}

fn run(wu: &WorkUnit) -> Result<(WorkResult, Files), String> {
    run_with(wu, &Files::new())
}

fn run_with(wu: &WorkUnit, inputs: &Files) -> Result<(WorkResult, Files), String> {
    let files = RefCell::new(Files::new());
    let result = run_work_unit(
        wu,
        |p| inputs.get(p).cloned().ok_or_else(|| p.to_string()),
        |p, b| {
            files.borrow_mut().insert(p.to_string(), b.to_vec());
            Ok(())
        },
    )?;
    Ok((result, files.into_inner()))
}

/// A float64 [century, cell] array from its single chunk column(s).
fn array(files: &Files, name: &str, cells: usize) -> Vec<Vec<f64>> {
    let meta: serde_json::Value =
        serde_json::from_slice(&files[&format!("rec/activity.zarr/{name}/zarr.json")]).unwrap();
    assert_eq!(meta["data_type"], "float64");
    let shape: Vec<usize> =
        meta["shape"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
    let chunks: Vec<usize> = meta["chunk_grid"]["configuration"]["chunk_shape"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    assert_eq!(shape[1], cells);
    let mut out = vec![vec![0.0; cells]; shape[0]];
    for (ti, band) in out.chunks_mut(chunks[0]).enumerate() {
        for ci in 0..cells.div_ceil(chunks[1]) {
            let mut raw = Vec::new();
            GzDecoder::new(&files[&format!("rec/activity.zarr/{name}/c/{ti}/{ci}")][..])
                .read_to_end(&mut raw)
                .unwrap();
            let v: Vec<f64> =
                raw.chunks_exact(8).map(|c| f64::from_le_bytes(c.try_into().unwrap())).collect();
            for (r, row) in band.iter_mut().enumerate() {
                for j in 0..chunks[1].min(cells - ci * chunks[1]) {
                    row[ci * chunks[1] + j] = v[r * chunks[1] + j];
                }
            }
        }
    }
    out
}

#[test]
fn people_years_are_the_samples_of_each_century() {
    let (_, files) = run(&unit(2, Some(spec()))).unwrap();
    // The same world, stepped by hand: a sample every 10 years, weighted by 10.
    let mut w = World::new(WorldConfig::SOUTHWEST_ASIA, 42, Default::default()).unwrap();
    let cells = w.grid.len();
    let mut want = vec![vec![0.0f64; cells]; 3];
    for s in 0..30 {
        for (c, p) in w.population.iter().enumerate() {
            want[s / 10][c] += 10.0 * *p as f64;
        }
        w.run_years(10, 1, 100);
    }
    let got = array(&files, "people", cells);
    assert_eq!(got, want);
    assert!(got[2].iter().sum::<f64>() > 0.0);
    let group: serde_json::Value = serde_json::from_slice(&files["rec/activity.zarr/zarr.json"]).unwrap();
    assert_eq!(group["attributes"]["first_year"], -12_000);
    assert_eq!(group["attributes"]["end_year"], -11_700);
    assert_eq!(group["attributes"]["every_years"], 10);
    for name in [
        "cultivators",
        "domestic",
        "settlements",
        "inequality",
        "village_size",
        "infected",
        "herders",
        "herd_domestic",
        "hunters",
    ] {
        assert_eq!(array(&files, name, cells).len(), 3, "{name}");
    }
}

#[test]
fn recording_activity_does_not_change_the_simulation() {
    let (plain, _) = run(&unit(2, None)).unwrap();
    let (recorded, files) = run(&unit(2, Some(spec()))).unwrap();
    assert_eq!(plain.state_hash, recorded.state_hash);
    assert_eq!(plain.summaries, recorded.summaries);
    assert!(recorded.files.iter().any(|f| f == "rec/activity.zarr/people/zarr.json"));
    assert!(recorded.files.iter().all(|f| files.contains_key(f)));
}

#[test]
fn the_record_is_identical_across_partition_counts() {
    let (_, one) = run(&unit(1, Some(spec()))).unwrap();
    let (_, four) = run(&unit(4, Some(spec()))).unwrap();
    assert_eq!(one, four);
}

#[test]
fn records_span_whole_centuries_in_samples_that_divide_them() {
    let mut odd = unit(1, Some(spec()));
    odd.years = 250;
    assert!(run(&odd).unwrap_err().contains("centuries"));
    let mut uneven = unit(1, Some(ActivitySpec { dir: "rec".into(), every_years: 30, zones: None }));
    uneven.years = 300;
    assert!(run(&uneven).unwrap_err().contains("every_years"));
}

/// A zone file as `oikos_cal.zones.Zones.encode` writes it.
fn zone_file(
    world: &World,
    zones: serde_json::Value,
    weighted: &[usize],
    weights: serde_json::Value,
) -> Vec<u8> {
    let h3: Vec<u64> = (0..world.grid.len() as u32).map(|c| world.grid.h3(c)).collect();
    serde_json::to_vec(&serde_json::json!({
        "format": "oikos-zones/1",
        "cells_hash": oikos_io::pack::cells_hash(&h3),
        "cells": h3.len(),
        "first_year": -12_000,
        "centuries": 3,
        "zones": zones,
        "weighted": weighted,
        "weights": weights,
    }))
    .unwrap()
}

fn zoned(bytes: &[u8]) -> (WorkUnit, Files) {
    let id = blake3::hash(bytes).to_hex().to_string();
    let spec = ActivitySpec { zones: Some(PackRef { id, path: "zones.json".into() }), ..spec() };
    (unit(2, Some(spec)), Files::from([("zones.json".to_string(), bytes.to_vec())]))
}

#[test]
fn zones_record_the_sums_of_their_cells_in_each_century() {
    let world = World::new(WorldConfig::SOUTHWEST_ASIA, 42, Default::default()).unwrap();
    let cells = world.grid.len();
    let (_, full) = run(&unit(2, Some(spec()))).unwrap();
    let w5 = [0.5f64, 0.25, 1.0];
    let weights = serde_json::json!({
        "cells": [5], "row": [0], "rows": [w5.iter().map(|w| w.to_bits()).collect::<Vec<_>>()],
    });
    let all: Vec<usize> = (0..cells).collect();
    let bytes = zone_file(&world, serde_json::json!([[0, 1, 2], all, [3, 5]]), &[2], weights);
    let (wu, inputs) = zoned(&bytes);
    let (result, files) = run_with(&wu, &inputs).unwrap();
    let (plain, _) = run(&unit(2, None)).unwrap();
    assert_eq!(result.state_hash, plain.state_hash, "zones only read the world");
    for name in ["people", "domestic", "hunters"] {
        let f = array(&full, name, cells);
        let got = array(&files, name, 3);
        for t in 0..3 {
            let want = [
                0.0 + f[t][0] + f[t][1] + f[t][2],
                f[t].iter().fold(0.0, |a, v| a + v),
                0.0 + f[t][3] + f[t][5] * w5[t],
            ];
            assert_eq!(got[t], want, "{name} in century {t}");
        }
    }
    let group: serde_json::Value = serde_json::from_slice(&files["rec/activity.zarr/zarr.json"]).unwrap();
    assert_eq!(
        group["attributes"]["zones"],
        wu.activity.as_ref().unwrap().zones.as_ref().unwrap().id.as_str()
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&files["rec/activity.zarr/people/zarr.json"]).unwrap();
    assert_eq!(meta["dimension_names"], serde_json::json!(["century", "zone"]));
    assert!(!files.contains_key("rec/activity.zarr/cell_h3/zarr.json"));
    let (_, four) = run_with(&WorkUnit { partitions: 4, ..wu }, &inputs).unwrap();
    assert_eq!(four, files, "the same bytes for any partition count");
}

#[test]
fn zone_files_must_fit_the_world_and_the_record() {
    let world = World::new(WorldConfig::SOUTHWEST_ASIA, 42, Default::default()).unwrap();
    let none = serde_json::json!({ "cells": [], "row": [], "rows": [] });
    let ok = zone_file(&world, serde_json::json!([[0, 1]]), &[], none.clone());
    let (mut wu, inputs) = zoned(&ok);
    wu.activity.as_mut().unwrap().zones.as_mut().unwrap().id = "0".repeat(64);
    assert!(run_with(&wu, &inputs).unwrap_err().contains("hash"));
    let outside = zone_file(&world, serde_json::json!([[0, world.grid.len()]]), &[], none.clone());
    assert!(run_with(&zoned(&outside).0, &zoned(&outside).1).unwrap_err().contains("outside"));
    let short = zone_file(
        &world,
        serde_json::json!([[0]]),
        &[0],
        serde_json::json!({ "cells": [0], "row": [0], "rows": [[1.0f64.to_bits()]] }),
    );
    assert!(run_with(&zoned(&short).0, &zoned(&short).1).unwrap_err().contains("centuries"));
    let mut other: serde_json::Value = serde_json::from_slice(&ok).unwrap();
    other["cells_hash"] = "x".into();
    let other = serde_json::to_vec(&other).unwrap();
    assert!(run_with(&zoned(&other).0, &zoned(&other).1).unwrap_err().contains("cells"));
    for (zones, weighted, why) in [
        (serde_json::json!([[1, 0]]), vec![], "ascending"),
        (serde_json::json!([[0], [0]]), vec![], "once"),
        (serde_json::json!([[0], [1]]), vec![0], "last"),
    ] {
        let bytes = zone_file(&world, zones, &weighted, none.clone());
        let err = run_with(&zoned(&bytes).0, &zoned(&bytes).1).unwrap_err();
        assert!(err.contains(why), "{why}: {err}");
    }
}
