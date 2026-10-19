//! Reads the same fixture directory as tools/packs/check.test.mjs; the two validators must agree on every case.
use atingo_path::pins::Pins;
use atingo_path::loop_data::load_dir;
use atingo_path::graph::load_graph;
use atingo_path::i18n::{load_i18n, validate_i18n};
use atingo_path::{load_prices, validate_price, validate, validate_graph, validate_loop, GoalKinds, PathFile, Severity};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())).trim_start_matches('\u{feff}').to_string()
}

fn codes(v: &[String]) -> BTreeSet<String> {
    v.iter().cloned().collect()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/packs")
}

#[test]
fn every_fixture_agrees_with_expected_json() {
    let pins: Pins = serde_json::from_str(&read(&root().join("kernel-pins.json"))).unwrap();
    let mut dirs: Vec<PathBuf> = fs::read_dir(root().join("fixtures")).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    assert_eq!(dirs.len(), 38, "the fixture directory has 38 cases (21 path cases, 10 graph cases, 3 i18n cases, 4 price cases)");
    let mut failures: Vec<String> = Vec::new();
    for d in &dirs {
        let name = d.file_name().unwrap().to_string_lossy().to_string();
        let mut kinds: Vec<GoalKinds> = Vec::new();
        if d.join("goal-kinds").exists() {
            for f in fs::read_dir(d.join("goal-kinds")).unwrap() {
                kinds.push(serde_json::from_str(&read(&f.unwrap().path())).unwrap());
            }
        }
        let merged = GoalKinds { version: 1, family: kinds.first().map(|k| k.family.clone()).unwrap_or_default(), kinds: kinds.into_iter().flat_map(|k| k.kinds).collect() };
        let mut errs: Vec<String> = Vec::new();
        let mut warns: Vec<String> = Vec::new();
        if let Some(g) = load_graph(d).unwrap_or_else(|e| panic!("{name}: {e}")) {
            for x in validate_graph(&g, "2026-10-19") {
                match x.severity {
                    Severity::Error => errs.push(x.code.to_string()),
                    Severity::Warning => warns.push(x.code.to_string()),
                }
            }
        }
        let paths: Vec<_> = if d.join("paths").exists() { fs::read_dir(d.join("paths")).unwrap().map(|e| e.unwrap()).collect() } else { Vec::new() };
        let mut parsed: Vec<PathFile> = Vec::new();
        let mut templates = std::collections::BTreeMap::new();
        for f in paths {
            let p: PathFile = serde_json::from_str(&read(&f.path())).unwrap();
            let data = load_dir(d, &p.id).unwrap_or_else(|e| panic!("{name}: {e}"));
            templates.extend(data.templates.clone());
            let all = validate(&p, &merged, &pins).into_iter().chain(validate_loop(&p, &merged, &pins, &data));
            for x in all {
                match x.severity {
                    Severity::Error => errs.push(x.code.to_string()),
                    Severity::Warning => warns.push(x.code.to_string()),
                }
            }
            parsed.push(p);
        }
        for x in validate_price(&parsed, &load_prices(d).unwrap_or_else(|e| panic!("{name}: {e}")), &pins) {
            match x.severity {
                Severity::Error => errs.push(x.code.to_string()),
                Severity::Warning => warns.push(x.code.to_string()),
            }
        }
        let locale_files = load_i18n(d).unwrap_or_else(|e| panic!("{name}: {e}"));
        for x in validate_i18n(&locale_files, &parsed, &templates, &pins) {
            match x.severity {
                Severity::Error => errs.push(x.code.to_string()),
                Severity::Warning => warns.push(x.code.to_string()),
            }
        }
        let want: serde_json::Value = serde_json::from_str(&read(&d.join("expected.json"))).unwrap();
        let w = |k: &str| -> Vec<String> { want[k].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect() };
        if codes(&errs) != codes(&w("errors")) || codes(&warns) != codes(&w("warnings")) {
            failures.push(format!("{name}: got errors {:?} warnings {:?}, want {:?} / {:?}", codes(&errs), codes(&warns), w("errors"), w("warnings")));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
