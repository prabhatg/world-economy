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
