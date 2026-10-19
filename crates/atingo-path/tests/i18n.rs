//! L13: the locale-layer rules in the Rust twin, one test per code beyond the three shared fixtures (which fixtures.rs runs), plus the shipped files.
use atingo_path::i18n::{load_i18n, validate_i18n, LocaleFile};
use atingo_path::loop_data::load_dir;
use atingo_path::pins::Pins;
use atingo_path::PathFile;
use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn pack() -> PathBuf {
    repo().join("packs/atingo")
}
fn pins() -> Pins {
    serde_json::from_str(&fs::read_to_string(repo().join("tools/packs/kernel-pins.json")).unwrap()).unwrap()
}
fn path() -> PathFile {
    serde_json::from_str(&fs::read_to_string(pack().join("paths/getting-ahead.json")).unwrap()).unwrap()
}
fn id_file() -> LocaleFile {
    load_i18n(&pack()).unwrap().into_iter().find(|(n, _)| n.ends_with(".id.json")).unwrap().1
}
fn codes(l: LocaleFile) -> Vec<&'static str> {
    let data = load_dir(&pack(), "getting-ahead").unwrap();
    let mut v: Vec<&'static str> = validate_i18n(&[("i18n/x.json".to_string(), l)], &[path()], &data.templates, &pins()).into_iter().map(|f| f.code).collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn the_shipped_id_and_en_files_are_green() {
    let files = load_i18n(&pack()).unwrap();
    assert_eq!(files.len(), 2);
    let data = load_dir(&pack(), "getting-ahead").unwrap();
    assert!(validate_i18n(&files, &[path()], &data.templates, &pins()).is_empty());
}
#[test]
fn a_path_that_does_not_exist_is_path_unknown() {
    let mut l = id_file();
    l.path_id = "no-such-path".into();
    assert_eq!(codes(l), vec!["i18n_path_unknown"]);
}
#[test]
fn a_locale_outside_the_pins_is_unsupported() {
    let mut l = id_file();
    l.locale = "xx".into();
    assert_eq!(codes(l), vec!["i18n_locale_unsupported"]);
}
#[test]
fn an_empty_title_prompt_or_hint_is_empty_string() {
    let mut a = id_file();
    a.nodes.get_mut("getting_ahead").unwrap().title = "  ".into();
    assert_eq!(codes(a), vec!["i18n_empty_string"]);
    let mut b = id_file();
    b.nodes.get_mut("industry").unwrap().lesson.steps[1].prompt = String::new();
    assert_eq!(codes(b), vec!["i18n_empty_string"]);
    let mut c = id_file();
    c.nodes.get_mut("marketing").unwrap().lesson.steps[2].evidence_hint = String::new();
    assert_eq!(codes(c), vec!["i18n_empty_string"]);
}
#[test]
fn an_extra_step_is_a_count_mismatch() {
    let mut l = id_file();
    let s = l.nodes.get("industry").unwrap().lesson.steps[0].clone();
    l.nodes.get_mut("industry").unwrap().lesson.steps.push(s);
    assert_eq!(codes(l), vec!["i18n_steps_count_mismatch"]);
}
#[test]
fn node_unknown_and_node_missing() {
    let mut a = id_file();
    let n = a.nodes.get("industry").unwrap().clone();
    a.nodes.insert("ghost".into(), n);
    assert_eq!(codes(a), vec!["i18n_node_unknown"]);
    let mut b = id_file();
    b.nodes.remove("marketing");
    assert_eq!(codes(b), vec!["i18n_node_missing"]);
}
