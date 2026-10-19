//! L20: the review-layer rules in the Rust twin. `review_policy_missing` and `review_retention_out_of_range` are shared fixtures
//! (fixtures.rs runs them); `review_engine_unknown`, the opt-in-by-folder rule and the shipped file are here.
use atingo_path::loop_data::{load_dir, LoopData};
use atingo_path::pins::Pins;
use atingo_path::{validate_loop, GoalKinds, PathFile};
use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn pack() -> PathBuf {
    repo().join("packs/atingo")
}
fn read(p: PathBuf) -> String {
    fs::read_to_string(p).unwrap()
}
fn codes(d: &LoopData) -> Vec<(&'static str, String)> {
    let path: PathFile = serde_json::from_str(&read(pack().join("paths/getting-ahead.json"))).unwrap();
    let kinds: GoalKinds = serde_json::from_str(&read(pack().join("goal-kinds/wealth.json"))).unwrap();
    let pins: Pins = serde_json::from_str(&read(repo().join("tools/packs/kernel-pins.json"))).unwrap();
    validate_loop(&path, &kinds, &pins, d).into_iter().filter(|f| f.code.starts_with("review_")).map(|f| (f.code, format!("{:?}", f.severity))).collect()
}

#[test]
fn the_shipped_review_file_is_green() {
    let d = load_dir(&pack(), "getting-ahead").unwrap();
    assert!(d.review_layer);
    assert_eq!(d.review.as_ref().unwrap().nodes.len(), 5);
    assert!(codes(&d).is_empty());
}

#[test]
fn an_engine_other_than_fsrs_is_review_engine_unknown() {
    let mut d = load_dir(&pack(), "getting-ahead").unwrap();
    d.review.as_mut().unwrap().nodes.get_mut("industry").unwrap().engine = "sm2".into();
    assert_eq!(codes(&d), vec![("review_engine_unknown", "Error".to_string())]);
}

#[test]
fn retention_at_or_outside_the_open_interval_is_refused() {
    for v in [0.5, 0.99, 0.2, 1.0] {
        let mut d = load_dir(&pack(), "getting-ahead").unwrap();
        d.review.as_mut().unwrap().nodes.get_mut("industry").unwrap().desired_retention = v;
        assert_eq!(codes(&d), vec![("review_retention_out_of_range", "Error".to_string())], "{v}");
    }
    let mut d = load_dir(&pack(), "getting-ahead").unwrap();
    d.review.as_mut().unwrap().nodes.get_mut("industry").unwrap().desired_retention = 0.6;
    assert!(codes(&d).is_empty());
}

#[test]
fn no_review_folder_is_silent_and_a_missing_node_is_a_warning() {
    let mut d = load_dir(&pack(), "getting-ahead").unwrap();
    d.review_layer = false;
    d.review = None;
    assert!(codes(&d).is_empty());
    d.review_layer = true;
    assert_eq!(codes(&d).len(), 5);
    assert!(codes(&d).iter().all(|c| *c == ("review_policy_missing", "Warning".to_string())));
}
