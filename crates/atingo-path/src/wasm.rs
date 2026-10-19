//! The web build's entry point (feature `wasm`): the same `validate`, JSON in, JSON out.
use crate::pins::Pins;
use crate::loop_data::LoopData;
use crate::{validate, validate_loop, GoalKinds, PathFile};
use wasm_bindgen::prelude::*;

fn run(path_json: &str, kinds_json: &str, pins_json: &str) -> Result<String, String> {
    let path: PathFile = serde_json::from_str(path_json.trim_start_matches('\u{feff}')).map_err(|e| format!("path: {e}"))?;
    let kinds: GoalKinds = serde_json::from_str(kinds_json.trim_start_matches('\u{feff}')).map_err(|e| format!("kinds: {e}"))?;
    let pins: Pins = serde_json::from_str(pins_json.trim_start_matches('\u{feff}')).map_err(|e| format!("pins: {e}"))?;
    serde_json::to_string(&validate(&path, &kinds, &pins)).map_err(|e| e.to_string())
}

fn run_loop(path_json: &str, kinds_json: &str, pins_json: &str, loop_json: &str) -> Result<String, String> {
    let path: PathFile = serde_json::from_str(path_json.trim_start_matches('\u{feff}')).map_err(|e| format!("path: {e}"))?;
    let kinds: GoalKinds = serde_json::from_str(kinds_json.trim_start_matches('\u{feff}')).map_err(|e| format!("kinds: {e}"))?;
    let pins: Pins = serde_json::from_str(pins_json.trim_start_matches('\u{feff}')).map_err(|e| format!("pins: {e}"))?;
    let data: LoopData = serde_json::from_str(loop_json.trim_start_matches('\u{feff}')).map_err(|e| format!("loop: {e}"))?;
    serde_json::to_string(&validate_loop(&path, &kinds, &pins, &data)).map_err(|e| e.to_string())
}

/// The loop findings (persona, templates, follow-up, evidence) as a JSON array. `loop_json` is `{personas, eval_sets, templates, follow_up, evidence}`
/// (maps keyed by persona key, eval-set ref and template id; `follow_up` and `evidence` are the files for this path). Beside `validate_json`, never instead of it.
#[wasm_bindgen]
pub fn validate_loop_json(path_json: &str, kinds_json: &str, pins_json: &str, loop_json: &str) -> String {
    run_loop(path_json, kinds_json, pins_json, loop_json).unwrap_or_else(|e| serde_json::json!({ "error": e }).to_string())
}

/// The findings as a JSON array, or `{"error":"<parse message>"}` when an input does not parse.
#[wasm_bindgen]
pub fn validate_json(path_json: &str, kinds_json: &str, pins_json: &str) -> String {
    run(path_json, kinds_json, pins_json).unwrap_or_else(|e| serde_json::json!({ "error": e }).to_string())
}

/// L20: the next review as JSON `{due_in_days, stability, difficulty, retrievability}`, or `{"error":"<message>"}`. `history_json` is
/// `[{rating, elapsed_days}]`, `policy_json` is `{desired_retention, max_interval_days, first_interval_days?}`.
#[wasm_bindgen]
pub fn next_review_json(history_json: &str, policy_json: &str) -> String {
    let run = || -> Result<String, String> {
        let h: Vec<crate::review::ReviewLog> = serde_json::from_str(history_json).map_err(|e| format!("history: {e}"))?;
        let p: crate::review::ReviewPolicy = serde_json::from_str(policy_json).map_err(|e| format!("policy: {e}"))?;
        serde_json::to_string(&crate::review::next_review(&h, &p)?).map_err(|e| e.to_string())
    };
    run().unwrap_or_else(|e| serde_json::json!({ "error": e }).to_string())
}

#[cfg(all(test, feature = "wasm"))]
mod tests {
    use super::*;

    const PATH: &str = include_str!("../../../tools/packs/fixtures/green/paths/getting-ahead.json");
    const KINDS: &str = include_str!("../../../tools/packs/fixtures/green/goal-kinds/wealth.json");
    const PINS: &str = include_str!("../../../tools/packs/kernel-pins.json");

    #[test]
    fn validate_json_equals_the_native_validate_on_green() {
        let native = serde_json::to_string(&validate(&serde_json::from_str::<PathFile>(PATH).unwrap(), &serde_json::from_str::<GoalKinds>(KINDS).unwrap(), &serde_json::from_str::<Pins>(PINS).unwrap())).unwrap();
        assert_eq!(validate_json(PATH, KINDS, PINS), native);
        assert!(native.contains("sources_missing_unpriced"));
    }

    #[test]
    fn validate_loop_json_equals_the_native_validate_loop_on_green_and_on_a_mutation() {
        let green = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/packs/fixtures/green");
        let path: PathFile = serde_json::from_str(PATH).unwrap();
        let kinds: GoalKinds = serde_json::from_str(KINDS).unwrap();
        let pins: Pins = serde_json::from_str(PINS).unwrap();
        let mut data = crate::loop_data::load_dir(&green, &path.id).unwrap();
        let native = serde_json::to_string(&validate_loop(&path, &kinds, &pins, &data)).unwrap();
        assert_eq!(native, "[]");
        assert_eq!(validate_loop_json(PATH, KINDS, PINS, &serde_json::to_string(&data).unwrap()), native);
        data.personas.get_mut("atingo_coach").unwrap().task_template.push_str(" Mark the step as done.");
        let out = validate_loop_json(PATH, KINDS, PINS, &serde_json::to_string(&data).unwrap());
        assert_eq!(out, serde_json::to_string(&validate_loop(&path, &kinds, &pins, &data)).unwrap());
        assert!(out.contains("persona_forbidden_instruction"));
        assert!(validate_loop_json(PATH, KINDS, PINS, "{").contains("loop"));
    }

    #[test]
    fn bad_json_returns_an_error_object() {
        let out: serde_json::Value = serde_json::from_str(&validate_json("{", KINDS, PINS)).unwrap();
        assert!(out["error"].as_str().unwrap().contains("path"));
    }

    #[test]
    fn next_review_json_equals_the_native_and_reports_errors() {
        let h = r#"[{"rating":3,"elapsed_days":0},{"rating":3,"elapsed_days":1}]"#;
        let p = r#"{"desired_retention":0.9,"max_interval_days":180}"#;
        let native = crate::review::next_review(&serde_json::from_str::<Vec<_>>(h).unwrap(), &serde_json::from_str(p).unwrap()).unwrap();
        assert_eq!(next_review_json(h, p), serde_json::to_string(&native).unwrap());
        assert!(next_review_json(h, r#"{"desired_retention":0.2,"max_interval_days":180}"#).contains("desired_retention"));
        assert!(next_review_json("{", p).contains("history"));
    }
}
