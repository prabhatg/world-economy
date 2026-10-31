//! Run outputs: Zarr fields and Parquet tables (docs/03-engine.md §3.8).

use flate2::read::GzDecoder;
use oikos_sim::output::{OutputSpec, panel_sampled};
use oikos_sim::workunit::Start;
use oikos_sim::{WorkResult, WorkUnit, WorldConfig, run_work_unit};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;

type Files = BTreeMap<String, Vec<u8>>;

fn unit(partitions: usize, outputs: Option<OutputSpec>) -> WorkUnit {
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
        outputs,
        inputs: vec![],
    }
}

fn spec() -> OutputSpec {
    OutputSpec { dir: "out".into(), every_years: 25, panel_one_in: 4, time_chunk: 5, cell_chunk: 1000 }
}

fn run(wu: &WorkUnit) -> (WorkResult, Files) {
    let files = RefCell::new(Files::new());
    let result = run_work_unit(
        wu,
        |p| Err(p.to_string()),
        |p, b| {
            files.borrow_mut().insert(p.to_string(), b.to_vec());
            Ok(())
        },
    )
    .unwrap();
    (result, files.into_inner())
}

fn table_rows(bytes: &[u8]) -> Vec<parquet::record::Row> {
    let reader = SerializedFileReader::new(bytes::Bytes::from(bytes.to_vec())).unwrap();
    reader.get_row_iter(None).unwrap().map(|r| r.unwrap()).collect()
}

fn decode(bytes: &[u8]) -> Vec<u64> {
    let mut raw = Vec::new();
    GzDecoder::new(bytes).read_to_end(&mut raw).unwrap();
    raw.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

#[test]
fn recording_outputs_does_not_change_the_simulation() {
    let (plain, files) = run(&unit(2, None));
    let (recorded, _) = run(&unit(2, Some(spec())));
    assert!(files.is_empty());
    assert!(plain.files.is_empty());
    assert_eq!(plain.state_hash, recorded.state_hash);
    assert_eq!(plain.summaries, recorded.summaries);
}

#[test]
fn writes_zarr_fields_and_parquet_tables() {
    let (result, files) = run(&unit(2, Some(spec())));
    for f in [
        "out/fields.zarr/zarr.json",
        "out/fields.zarr/population/zarr.json",
        "out/fields.zarr/tick/zarr.json",
        "out/fields.zarr/cell_h3/zarr.json",
        "out/fields.zarr/capacity/zarr.json",
        "out/fields.zarr/cultivation/zarr.json",
        "out/fields.zarr/domestication/zarr.json",
        "out/fields.zarr/soil/zarr.json",
        "out/summary.parquet",
        "out/panel.parquet",
    ] {
        assert!(files.contains_key(f), "missing {f}");
        assert!(result.files.contains(&f.to_string()), "result does not list {f}");
    }
    // 300 years every 25 years, plus the initial state.
    let meta: serde_json::Value =
        serde_json::from_slice(&files["out/fields.zarr/population/zarr.json"]).unwrap();
    assert_eq!(meta["shape"], serde_json::json!([13, 2401]));
    assert_eq!(meta["dimension_names"], serde_json::json!(["time", "cell"]));

    // Summary table rows agree with the run's own summaries at shared ticks.
    let rows = table_rows(&files["out/summary.parquet"]);
    assert_eq!(rows.len(), 13);
    let last = rows.last().unwrap();
    let final_summary = result.summaries.last().unwrap();
    assert_eq!(last.get_ulong(0).unwrap(), final_summary.tick);
    assert_eq!(last.get_ulong(2).unwrap(), final_summary.population);

    // The population field's last row sums to the same total (time chunk 5 → band 2, row 2).
    let row_total: u64 = (0..3)
        .map(|ci| {
            let chunk = decode(&files[&format!("out/fields.zarr/population/c/2/{ci}")]);
            chunk[2 * 1000..3 * 1000].iter().sum::<u64>()
        })
        .sum();
    assert_eq!(row_total, final_summary.population);
}

#[test]
fn shares_are_recorded_as_parts_per_10000_over_time() {
    let (_, files) = run(&unit(2, Some(spec())));
    for name in ["cultivation", "domestication", "soil"] {
        let meta: serde_json::Value =
            serde_json::from_slice(&files[&format!("out/fields.zarr/{name}/zarr.json")]).unwrap();
        assert_eq!(meta["shape"], serde_json::json!([13, 2401]), "{name}");
        assert_eq!(meta["attributes"]["scale_factor"], serde_json::json!(1e-4), "{name}");
        assert_eq!(meta["attributes"]["unit"], "1", "{name}");
    }
    assert!(
        !files.keys().any(|f| f.contains("/land/") || f.contains("obsidian")),
        "the toy has no such packs"
    );
    let soil = decode(&files["out/fields.zarr/soil/c/0/0"]);
    assert!(soil[..1000].iter().all(|&s| s == 10_000), "soils start whole");
    let farming = decode(&files["out/fields.zarr/cultivation/c/2/0"]);
    assert!(farming.iter().all(|&e| e == 0), "toy worlds never cultivate");
}

#[test]
fn panel_contains_only_sampled_agents() {
    let (_, files) = run(&unit(1, Some(spec())));
    let rows = table_rows(&files["out/panel.parquet"]);
    assert!(!rows.is_empty());
    for r in &rows {
        let id = oikos_core::EntityId(r.get_ulong(1).unwrap());
        assert!(panel_sampled(id, 4), "unsampled agent {id:?} in panel");
    }
}

#[test]
fn outputs_are_identical_across_partition_counts() {
    let (_, one) = run(&unit(1, Some(spec())));
    let (_, three) = run(&unit(3, Some(spec())));
    assert_eq!(one, three);
}
