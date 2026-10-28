use super::*;

const VALID: &str = r#"
name = "test"
version = 1

[params.growth_rate]
group = "demography"
unit = "1/period"
description = "Intrinsic growth rate."
kind = { type = "real", min = 0.0, max = 0.1 }
default = 0.01
prior = { dist = "uniform", lo = 0.002, hi = 0.03 }
status = "literature"
sources = [{ citation = "Bocquet-Appel 2011, Science 333:560" }]

[params.band_size]
group = "social"
unit = "people"
description = "Initial band size."
kind = { type = "integer", min = 1, max = 500 }
default = 30
prior = { dist = "fixed" }
status = "placeholder"
note = "Engine test value."

[params.split]
group = "social"
unit = "probability"
description = "Split probability."
kind = { type = "probability" }
default = 0.2
prior = { dist = "log_uniform", lo = 0.01, hi = 0.5 }
status = "placeholder"
note = "Engine test value."
"#;

fn with(replace: &str, by: &str) -> String {
    assert!(VALID.contains(replace), "fixture lacks {replace}");
    VALID.replacen(replace, by, 1)
}

fn err(src: &str) -> String {
    Registry::from_toml(src).unwrap_err()
}

#[test]
fn parses_a_valid_registry() {
    let r = Registry::from_toml(VALID).unwrap();
    assert_eq!(r.name, "test");
    assert_eq!(r.keys().collect::<Vec<_>>(), ["band_size", "growth_rate", "split"]);
    assert_eq!(r.get("band_size").unwrap().default, Value::Int(30));
    assert_eq!(r.get("growth_rate").unwrap().default, Value::Real(0.01));
}

#[test]
fn defaults_resolve_and_overrides_are_checked() {
    let r = Registry::from_toml(VALID).unwrap();
    let mut o = BTreeMap::new();
    o.insert("band_size".to_string(), Value::Int(40));
    let v = r.resolve(&o).unwrap();
    assert_eq!(v["band_size"], Value::Int(40));
    assert_eq!(v["split"], Value::Real(0.2));

    o.insert("band_size".to_string(), Value::Int(900));
    let e = r.resolve(&o).unwrap_err();
    assert!(e.contains("band_size") && e.contains("500") && e.contains("people"), "{e}");

    let mut o = BTreeMap::new();
    o.insert("ghost".to_string(), Value::Int(1));
    assert!(r.resolve(&o).unwrap_err().contains("unknown parameter ghost"));

    let mut o = BTreeMap::new();
    o.insert("band_size".to_string(), Value::Real(30.5));
    assert!(r.resolve(&o).unwrap_err().contains("integer"));

    let mut o = BTreeMap::new();
    o.insert("split".to_string(), Value::Real(1.5));
    assert!(r.resolve(&o).unwrap_err().contains("split"));
}

#[test]
fn integers_are_accepted_for_reals() {
    let r = Registry::from_toml(VALID).unwrap();
    let mut o = BTreeMap::new();
    o.insert("split".to_string(), Value::Int(1));
    assert_eq!(r.resolve(&o).unwrap()["split"], Value::Real(1.0));
}

#[test]
fn rejects_bad_definitions() {
    assert!(err(&with("default = 30", "default = 0")).contains("band_size"));
    assert!(err(&with("min = 0.0, max = 0.1", "min = 0.2, max = 0.1")).contains("min"));
    assert!(err(&with("lo = 0.002, hi = 0.03", "lo = 0.002, hi = 0.3")).contains("prior"));
    assert!(err(&with("lo = 0.01, hi = 0.5", "lo = 0.0, hi = 0.5")).contains("log_uniform"));
    assert!(err(&with("unit = \"people\"", "unit = \"\"")).contains("unit"));
    assert!(err(&with("description = \"Initial band size.\"", "description = \"\"")).contains("description"));
    let no_source = with("sources = [{ citation = \"Bocquet-Appel 2011, Science 333:560\" }]", "");
    assert!(err(&no_source).contains("source"));
    let no_note = with("status = \"placeholder\"\nnote = \"Engine test value.\"", "status = \"placeholder\"");
    assert!(err(&no_note).contains("note"));
    let e = err(&with("kind = { type = \"integer\"", "kind = { type = \"integr\""));
    assert!(e.contains("integr") && e.contains("line"), "{e}");
    assert!(err("name = \"x\"\nversion = 1\n").contains("no parameters"));
}

#[test]
fn json_export_is_stable_and_complete() {
    let r = Registry::from_toml(VALID).unwrap();
    let a = r.to_json();
    assert_eq!(a, Registry::from_toml(VALID).unwrap().to_json());
    let v: serde_json::Value = serde_json::from_str(&a).unwrap();
    assert_eq!(v["params"]["growth_rate"]["prior"]["dist"], "uniform");
    assert_eq!(v["params"]["band_size"]["unit"], "people");
}

#[test]
fn values_convert_to_json_numbers() {
    let r = Registry::from_toml(VALID).unwrap();
    let j = r.values_json(&r.resolve(&BTreeMap::new()).unwrap());
    assert!(j["band_size"].is_u64());
    assert!(j["growth_rate"].is_f64());
}

fn u(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|&(k, v)| (k.to_string(), v)).collect()
}

#[test]
fn sample_maps_the_unit_interval_onto_uniform_priors() {
    let r = Registry::from_toml(VALID).unwrap();
    let real = |x: f64| match r.sample(&u(&[("growth_rate", x)])).unwrap()["growth_rate"] {
        Value::Real(v) => v,
        other => panic!("expected a real, got {other:?}"),
    };
    assert_eq!(real(0.0), 0.002);
    assert_eq!(real(1.0), 0.03);
    assert_eq!(real(0.5), 0.002 + 0.5 * (0.03 - 0.002));
}

#[test]
fn sample_maps_log_uniform_priors_with_reproducible_math() {
    let r = Registry::from_toml(VALID).unwrap();
    let s = |x: f64| r.sample(&u(&[("split", x)])).unwrap()["split"];
    assert_eq!(s(0.0), Value::Real(0.01));
    assert_eq!(s(1.0), Value::Real(0.5));
    let mid = 0.01 * oikos_math::exp(0.5 * oikos_math::ln(0.5 / 0.01));
    assert_eq!(s(0.5), Value::Real(mid));
    assert!((mid - (0.01f64 * 0.5).sqrt()).abs() < 1e-12, "geometric mean, got {mid}");
}

#[test]
fn sample_rounds_integer_parameters_to_the_nearest_integer() {
    let src = with("prior = { dist = \"fixed\" }", "prior = { dist = \"uniform\", lo = 10.0, hi = 50.0 }");
    let r = Registry::from_toml(&src).unwrap();
    let s = |x: f64| r.sample(&u(&[("band_size", x)])).unwrap()["band_size"];
    assert_eq!(s(0.0), Value::Int(10));
    assert_eq!(s(1.0), Value::Int(50));
    assert_eq!(s(0.5), Value::Int(30));
    assert_eq!(s(0.51), Value::Int(30));
    assert_eq!(s(0.52), Value::Int(31));
}

#[test]
fn sample_rejects_what_it_cannot_map() {
    let r = Registry::from_toml(VALID).unwrap();
    let e = r.sample(&u(&[("band_size", 0.5)])).unwrap_err();
    assert!(e.contains("band_size") && e.contains("fixed"), "{e}");
    assert!(r.sample(&u(&[("ghost", 0.5)])).unwrap_err().contains("unknown parameter ghost"));
    for bad in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        let e = r.sample(&u(&[("growth_rate", bad)])).unwrap_err();
        assert!(e.contains("growth_rate") && e.contains("[0, 1]"), "{e}");
    }
    let src = with(
        "prior = { dist = \"uniform\", lo = 0.002, hi = 0.03 }",
        "prior = { dist = \"normal\", mean = 0.01, sd = 0.005 }",
    );
    let e = Registry::from_toml(&src).unwrap().sample(&u(&[("growth_rate", 0.5)])).unwrap_err();
    assert!(e.contains("growth_rate") && e.contains("normal"), "{e}");
}

#[test]
fn sample_returns_only_the_requested_keys() {
    let r = Registry::from_toml(VALID).unwrap();
    let v = r.sample(&u(&[("split", 0.3), ("growth_rate", 0.7)])).unwrap();
    assert_eq!(v.keys().collect::<Vec<_>>(), ["growth_rate", "split"]);
    assert!(r.sample(&BTreeMap::new()).unwrap().is_empty());
}

proptest::proptest! {
    #[test]
    fn samples_stay_inside_the_prior_and_grow_with_u(a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
        let src = with("prior = { dist = \"fixed\" }", "prior = { dist = \"uniform\", lo = 10.0, hi = 50.0 }");
        let r = Registry::from_toml(&src).unwrap();
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let keys = ["band_size", "growth_rate", "split"];
        let at = |x: f64| r.sample(&keys.iter().map(|k| (k.to_string(), x)).collect()).unwrap();
        let (vl, vh) = (at(lo), at(hi));
        for k in keys {
            r.check(k, vl[k]).unwrap();
            let num = |v: Value| match v { Value::Int(i) => i as f64, Value::Real(x) => x };
            proptest::prop_assert!(num(vl[k]) <= num(vh[k]), "{k}: {:?} > {:?}", vl[k], vh[k]);
        }
    }
}
