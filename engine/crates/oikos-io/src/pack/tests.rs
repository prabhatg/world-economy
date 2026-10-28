use super::*;

const H3: [u64; 3] = [0x8528_0003_ffff_ffff, 0x8528_0007_ffff_ffff, 0x8528_000b_ffff_ffff];

fn region() -> Region {
    Region { name: "test".into(), resolution: 5, cells: 3, cells_hash: cells_hash(&H3) }
}

fn field(name: &str, dims: Dims) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        unit: "m".into(),
        description: format!("test field {name}"),
        dims,
        scale: 1.0,
        offset: 0.0,
        nodata: None,
    }
}

fn sample() -> Vec<u8> {
    let mut b = PackBuilder::new("terrain", region());
    b.time(vec![-12000, -11900, -11800], TimeRule::Linear);
    b.source(Source {
        dataset: "test-dem".into(),
        version: "1".into(),
        license: "CC-BY-4.0".into(),
        citation: "Test (2026)".into(),
        manifest_sha256: "00".repeat(32),
    });
    b.field(
        FieldSpec { scale: 0.5, offset: -10.0, nodata: Some(65535), ..field("elev", Dims::Cell) },
        Data::U16(vec![20, 65535, 40]),
    )
    .unwrap();
    b.field(field("temp", Dims::TimeCell), Data::I16(vec![1, 2, 3, 11, 12, 13, -21, -22, -23])).unwrap();
    b.field(field("frac", Dims::Cell), Data::F32(vec![0.25, 0.5, 1.0])).unwrap();
    b.build()
}

#[test]
fn round_trip_applies_scale_offset_and_nodata() {
    let p = Pack::parse(sample()).unwrap();
    assert_eq!(p.header().kind, "terrain");
    let elev = p.field("elev").unwrap();
    assert_eq!(elev.get(0, 0), Some(0.0));
    assert_eq!(elev.get(0, 1), None);
    assert_eq!(elev.get(0, 2), Some(10.0));
    let temp = p.field("temp").unwrap();
    assert_eq!((temp.get(0, 2), temp.get(1, 0), temp.get(2, 1)), (Some(3.0), Some(11.0), Some(-22.0)));
    assert_eq!(p.field("frac").unwrap().get(0, 1), Some(0.5));
    assert!(p.field("ghost").unwrap_err().contains("no field ghost"));
}

#[test]
fn id_is_the_blake3_of_the_file_and_is_checked() {
    let bytes = sample();
    let id = blake3::hash(&bytes).to_hex().to_string();
    assert_eq!(Pack::parse(bytes.clone()).unwrap().id(), id);
    assert_eq!(Pack::open(bytes.clone(), &id).unwrap().id(), id);
    let e = Pack::open(bytes, &"ab".repeat(32)).unwrap_err();
    assert!(e.contains("pack id mismatch") && e.contains(&id), "{e}");
}

#[test]
fn building_is_deterministic_and_the_header_canonical() {
    let (a, b) = (sample(), sample());
    assert_eq!(a, b);
    let len = u32::from_le_bytes(a[8..12].try_into().unwrap()) as usize;
    let header = std::str::from_utf8(&a[12..12 + len]).unwrap();
    let v: serde_json::Value = serde_json::from_str(header).unwrap();
    assert_eq!(header, serde_json::to_string(&v).unwrap(), "sorted keys, no whitespace");
    for f in v["fields"].as_array().unwrap() {
        assert_eq!(f["offset"].as_u64().unwrap() % 8, 0);
    }
}

#[test]
fn time_rules_step_and_linear_clamp_outside_the_covered_years() {
    let p = Pack::parse(sample()).unwrap();
    let temp = p.field("temp").unwrap();
    assert_eq!(temp.at(p.slice_at(-12000), 0), Some(1.0));
    assert_eq!(temp.at(p.slice_at(-11950), 0), Some(6.0));
    assert_eq!(temp.at(p.slice_at(-11850), 2), Some((13.0 - 23.0) / 2.0));
    assert_eq!(temp.at(p.slice_at(-13000), 1), Some(2.0));
    assert_eq!(temp.at(p.slice_at(-9000), 1), Some(-22.0));
    assert_eq!(p.field("elev").unwrap().at(p.slice_at(-11950), 2), Some(10.0), "static fields ignore time");

    let mut b = PackBuilder::new("climate", region());
    b.time(vec![0, 100], TimeRule::Step);
    b.field(field("t", Dims::TimeCell), Data::U8(vec![1, 2, 3, 7, 8, 9])).unwrap();
    let p = Pack::parse(b.build()).unwrap();
    let t = p.field("t").unwrap();
    assert_eq!(
        (t.at(p.slice_at(99), 0), t.at(p.slice_at(100), 0), t.at(p.slice_at(-5), 0)),
        (Some(1.0), Some(7.0), Some(1.0))
    );
}

#[test]
fn nodata_in_either_slice_gives_nodata_when_interpolating() {
    let mut b = PackBuilder::new("climate", region());
    b.time(vec![0, 100], TimeRule::Linear);
    b.field(FieldSpec { nodata: Some(0), ..field("t", Dims::TimeCell) }, Data::U8(vec![0, 2, 3, 7, 8, 9]))
        .unwrap();
    let p = Pack::parse(b.build()).unwrap();
    assert_eq!(p.field("t").unwrap().at(p.slice_at(50), 0), None);
    assert_eq!(p.field("t").unwrap().at(p.slice_at(50), 1), Some(5.0));
}

#[test]
fn geometry_packs_must_match_their_cell_hash() {
    let mut b = PackBuilder::new("geometry", region());
    b.field(FieldSpec { unit: "h3 index".into(), ..field("h3", Dims::Cell) }, Data::U64(H3.to_vec()))
        .unwrap();
    assert!(Pack::parse(b.build()).is_ok());

    let mut wrong = H3;
    wrong[2] += 1 << 40;
    let mut b = PackBuilder::new("geometry", region());
    b.field(FieldSpec { unit: "h3 index".into(), ..field("h3", Dims::Cell) }, Data::U64(wrong.to_vec()))
        .unwrap();
    assert!(Pack::parse(b.build()).unwrap_err().contains("cells_hash"));
}

#[test]
fn builder_rejects_inconsistent_fields() {
    let mut b = PackBuilder::new("terrain", region());
    assert!(b.field(field("x", Dims::Cell), Data::U8(vec![1, 2])).unwrap_err().contains("3 values"));
    assert!(b.field(field("y", Dims::TimeCell), Data::U8(vec![1, 2, 3])).unwrap_err().contains("time"));
    b.field(field("z", Dims::Cell), Data::U8(vec![1, 2, 3])).unwrap();
    assert!(b.field(field("z", Dims::Cell), Data::U8(vec![1, 2, 3])).unwrap_err().contains("duplicate"));
}

fn header_of(bytes: &[u8]) -> serde_json::Value {
    let len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[12..12 + len]).unwrap()
}

/// Rewrites the header (padding the file so arrays keep their offsets).
fn with_header(bytes: &[u8], edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let start = (12 + len).next_multiple_of(8);
    let mut h = header_of(bytes);
    edit(&mut h);
    let text = serde_json::to_string(&h).unwrap();
    assert!(12 + text.len() <= start, "edited header must fit in the original space");
    let mut out = bytes[..8].to_vec();
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out.resize(start, 0);
    out.extend_from_slice(&bytes[start..]);
    out
}

#[test]
fn malformed_packs_are_rejected_with_a_reason() {
    let good = sample();
    let err = |b: Vec<u8>| Pack::parse(b).unwrap_err();
    assert!(err(b"NOTAPACK....".to_vec()).contains("not an Oikos forcing pack"));
    assert!(err(good[..10].to_vec()).contains("truncated"));
    assert!(err(good[..good.len() - 1].to_vec()).contains("outside the file"));
    let mut big = good.clone();
    big[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(err(big).contains("truncated"));
    assert!(err(with_header(&good, |h| h["format"] = 9.into())).contains("format 9"));
    assert!(err(with_header(&good, |h| h["fields"][0]["dtype"] = "u12".into())).contains("u12"));
    assert!(err(with_header(&good, |h| h["fields"][0]["offset"] = 3.into())).contains("aligned"));
    assert!(err(with_header(&good, |h| h["fields"][0]["len"] = 4.into())).contains("elev"));
    assert!(
        err(with_header(&good, |h| h["time"]["calendar_year"] = serde_json::json!([0, 0, 1])))
            .contains("ascending")
    );
    assert!(err(with_header(&good, |h| h["time"] = serde_json::Value::Null)).contains("time"));
}

#[test]
fn summary_lists_fields_with_ranges_and_nodata_counts() {
    let p = Pack::parse(sample()).unwrap();
    let s = p.summary();
    assert_eq!(s["id"], p.id());
    assert_eq!(s["kind"], "terrain");
    assert_eq!(s["region"]["cells"], 3);
    assert_eq!(s["time"]["calendar_year"], serde_json::json!([-12000, -11900, -11800]));
    let elev = &s["fields"][0];
    assert_eq!(
        (elev["name"].as_str(), elev["min"].as_f64(), elev["max"].as_f64()),
        (Some("elev"), Some(0.0), Some(10.0))
    );
    assert_eq!(elev["nodata"], 1);
    assert_eq!(s["fields"][1]["min"], -23.0);
    assert_eq!(s["sources"][0]["dataset"], "test-dem");
}
