use super::*;
use flate2::read::GzDecoder;
use std::collections::BTreeMap;
use std::io::Read;

type Files = BTreeMap<String, Vec<u8>>;

fn sink(files: &mut Files) -> impl FnMut(&str, &[u8]) -> Result<(), String> + '_ {
    |p, b| {
        files.insert(p.to_string(), b.to_vec());
        Ok(())
    }
}

fn decode_chunk(bytes: &[u8]) -> Vec<u64> {
    let mut raw = Vec::new();
    GzDecoder::new(bytes).read_to_end(&mut raw).unwrap();
    raw.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
}

#[test]
fn array_metadata_follows_zarr_v3() {
    let meta =
        zarr::array_metadata(&[5, 7], &[2, 3], &["time", "cell"], serde_json::json!({"unit": "people"}));
    let v: serde_json::Value = serde_json::from_str(&meta).unwrap();
    assert_eq!(v["zarr_format"], 3);
    assert_eq!(v["node_type"], "array");
    assert_eq!(v["shape"], serde_json::json!([5, 7]));
    assert_eq!(v["data_type"], "uint64");
    assert_eq!(v["chunk_grid"]["name"], "regular");
    assert_eq!(v["chunk_grid"]["configuration"]["chunk_shape"], serde_json::json!([2, 3]));
    assert_eq!(v["chunk_key_encoding"]["configuration"]["separator"], "/");
    assert_eq!(v["fill_value"], 0);
    assert_eq!(v["codecs"][0]["name"], "bytes");
    assert_eq!(v["codecs"][0]["configuration"]["endian"], "little");
    assert_eq!(v["codecs"][1]["name"], "gzip");
    assert_eq!(v["dimension_names"], serde_json::json!(["time", "cell"]));
    assert_eq!(v["attributes"]["unit"], "people");
}

#[test]
fn time_series_round_trips_through_padded_chunks() {
    let (t_len, n) = (5u64, 7u64);
    let value = |t: u64, c: u64| 100 * t + c + 1;
    let mut files = Files::new();
    let mut w = zarr::TimeSeries::new("pop", n, 2, 3, &["time", "cell"], serde_json::json!({}));
    for t in 0..t_len {
        let row: Vec<u64> = (0..n).map(|c| value(t, c)).collect();
        w.push(&row, &mut sink(&mut files)).unwrap();
    }
    w.finish(&mut sink(&mut files)).unwrap();

    let meta: serde_json::Value = serde_json::from_slice(&files["pop/zarr.json"]).unwrap();
    assert_eq!(meta["shape"], serde_json::json!([5, 7]));
    let chunks: Vec<&String> = files.keys().filter(|k| k.starts_with("pop/c/")).collect();
    assert_eq!(chunks.len(), 9, "{chunks:?}");
    for ti in 0..3u64 {
        for ci in 0..3u64 {
            let data = decode_chunk(&files[&format!("pop/c/{ti}/{ci}")]);
            assert_eq!(data.len(), 6, "full chunk shape, edge chunks padded");
            for (k, &got) in data.iter().enumerate() {
                let (t, c) = (ti * 2 + k as u64 / 3, ci * 3 + k as u64 % 3);
                let want = if t < t_len && c < n { value(t, c) } else { 0 };
                assert_eq!(got, want, "t {t} c {c}");
            }
        }
    }
}

#[test]
fn time_series_rejects_wrong_row_length() {
    let mut files = Files::new();
    let mut w = zarr::TimeSeries::new("pop", 4, 2, 2, &["time", "cell"], serde_json::json!({}));
    assert!(w.push(&[1, 2, 3], &mut sink(&mut files)).unwrap_err().contains("4"));
}

#[test]
fn static_array_and_group() {
    let mut files = Files::new();
    zarr::write_1d("h3", &[9, 8, 7], 2, "cell", serde_json::json!({}), &mut sink(&mut files)).unwrap();
    zarr::write_group("", serde_json::json!({"engine": "oikos"}), &mut sink(&mut files)).unwrap();
    assert_eq!(decode_chunk(&files["h3/c/0"]), [9, 8]);
    assert_eq!(decode_chunk(&files["h3/c/1"]), [7, 0]);
    let g: serde_json::Value = serde_json::from_slice(&files["zarr.json"]).unwrap();
    assert_eq!(g["node_type"], "group");
    assert_eq!(g["attributes"]["engine"], "oikos");
}

#[test]
fn output_bytes_are_deterministic() {
    let run = || {
        let mut files = Files::new();
        let mut w = zarr::TimeSeries::new("p", 3, 2, 2, &["time", "cell"], serde_json::json!({}));
        w.push(&[1, 2, 3], &mut sink(&mut files)).unwrap();
        w.finish(&mut sink(&mut files)).unwrap();
        files
    };
    assert_eq!(run(), run());
}

#[test]
fn parquet_table_round_trips() {
    use parquet::file::reader::{FileReader, SerializedFileReader};
    let bytes = table::write(&[
        table::Column::U64("tick", vec![0, 365, 730]),
        table::Column::U64("people", vec![u64::MAX, 2, 3]),
        table::Column::I64("year", vec![-12000, -11999, -11998]),
    ])
    .unwrap();
    let reader = SerializedFileReader::new(bytes::Bytes::from(bytes)).unwrap();
    assert_eq!(reader.metadata().file_metadata().num_rows(), 3);
    let rows: Vec<String> = reader.get_row_iter(None).unwrap().map(|r| r.unwrap().to_string()).collect();
    assert_eq!(rows[0], format!("{{tick: 0, people: {}, year: -12000}}", u64::MAX));
    assert_eq!(rows[2], "{tick: 730, people: 3, year: -11998}");
}

#[test]
fn parquet_rejects_ragged_columns() {
    let e =
        table::write(&[table::Column::U64("a", vec![1, 2]), table::Column::U64("b", vec![1])]).unwrap_err();
    assert!(e.contains("b"), "{e}");
}
