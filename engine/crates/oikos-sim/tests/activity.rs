//! The calibration record as a work-unit output (#46): what `oikos_cal.record.activity`
//! samples in Python, written by the engine so a calibration world runs wherever the
//! CLI runs.

use flate2::read::GzDecoder;
use oikos_sim::activity::ActivitySpec;
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
    ActivitySpec { dir: "rec".into(), every_years: 10 }
}

fn run(wu: &WorkUnit) -> Result<(WorkResult, Files), String> {
    let files = RefCell::new(Files::new());
    let result = run_work_unit(
        wu,
        |p| Err(p.to_string()),
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
    let mut uneven = unit(1, Some(ActivitySpec { dir: "rec".into(), every_years: 30 }));
    uneven.years = 300;
    assert!(run(&uneven).unwrap_err().contains("every_years"));
}
