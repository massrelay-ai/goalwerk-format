//! Runs the built `atingo-path` binary on two shared fixtures.
use std::path::Path;
use std::process::Command;

fn run(case: &str) -> (Option<i32>, serde_json::Value) {
    let tools = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/packs");
    let out = Command::new(env!("CARGO_BIN_EXE_atingo-path"))
        .arg("check")
        .arg(tools.join("fixtures").join(case))
        .arg("--pins")
        .arg(tools.join("kernel-pins.json"))
        .output()
        .unwrap();
    let line = String::from_utf8(out.stdout).unwrap();
    (out.status.code(), serde_json::from_str(line.trim()).unwrap_or_else(|e| panic!("stdout is not one JSON line: {e}: {line}")))
}

fn has(v: &serde_json::Value, code: &str) -> bool {
    v["findings"].as_array().unwrap().iter().any(|f| f["code"] == code)
}

#[test]
fn green_exits_zero_with_only_warnings() {
    let (code, v) = run("green");
    assert_eq!(code, Some(0));
    assert!(has(&v, "sources_missing_unpriced"));
    assert_eq!(v["counts"]["nodes"], 5);
}

#[test]
fn a_prerequisite_cycle_exits_one_and_names_the_code() {
    let (code, v) = run("prerequisite_cycle");
    assert_eq!(code, Some(1));
    assert!(has(&v, "prerequisite_cycle"));
}

#[test]
fn a_price_fixture_names_its_code_and_counts_the_row() {
    let (code, v) = run("price_currency_not_allowed");
    assert_eq!(code, Some(1));
    assert!(has(&v, "price_currency_not_allowed"));
    assert_eq!(v["counts"]["prices"], 1);
    let (code, v) = run("price_green");
    assert_eq!(code, Some(0));
    assert_eq!(v["counts"]["prices"], 1);
}
