//! L15: the loop as data. Types for the four pack folders `personas/`, `lesson-templates/`, `follow-up/`, `evidence/`
//! (mirroring `tools/packs/schemas/*.schema.json`), and the loader both the CLI and the fixture suite use. Pure data, no state.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Persona {
    pub key: String,
    pub persona: String,
    #[serde(rename = "taskTemplate")]
    pub task_template: String,
    #[serde(rename = "toolConfigRef")]
    pub tool_config_ref: Option<String>,
    #[serde(rename = "evalSetRef")]
    pub eval_set_ref: String,
    #[serde(rename = "_comment", default)]
    pub comment: Option<String>,
    pub atingo: PersonaExtras,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PersonaExtras {
    pub path_families: Vec<String>,
    pub voice: String,
    pub never: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvalCase {
    pub id: String,
    pub prompt: String,
    #[serde(rename = "mustContain")]
    pub must_contain: Vec<String>,
    #[serde(rename = "mustNotContain")]
    pub must_not_contain: Vec<String>,
    pub intent: String,
    pub why: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct EvalSet(pub Vec<EvalCase>);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LessonTemplate {
    pub id: String,
    pub title: String,
    #[serde(rename = "_comment", default)]
    pub comment: Option<String>,
    pub items_kind: String,
    pub template: TemplateHead,
    pub levels: Vec<Level>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TemplateHead {
    pub title: String,
    pub step_prompt: String,
    pub evidence_hint: String,
    pub steps_per_lesson: u32,
    pub review_max: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Level {
    pub id: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub prompt: String,
    pub evidence_hint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FollowUp {
    pub path_id: String,
    #[serde(rename = "_comment", default)]
    pub comment: Option<String>,
    pub nodes: BTreeMap<String, FollowUpRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FollowUpRow {
    pub rrule: String,
    pub grace_days: u32,
    pub nudges: Vec<Nudge>,
    pub escalate: Escalate,
    pub pause_rules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Nudge {
    pub after_hours: u32,
    pub channel: String,
    pub template_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Escalate {
    pub after_days: u32,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBindings {
    pub path_id: String,
    #[serde(rename = "_comment", default)]
    pub comment: Option<String>,
    pub nodes: BTreeMap<String, EvidenceRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRow {
    pub verifier: String,
    pub binding: Binding,
    pub fallback: String,
    pub sources_of_truth: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    #[serde(default)]
    pub connector_id: Option<String>,
    #[serde(default)]
    pub method_id: Option<String>,
    #[serde(default)]
    pub eval_set: Option<String>,
    #[serde(default)]
    pub script_ref: Option<String>,
    #[serde(default)]
    pub expect: Option<Expect>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub field: String,
    pub op: String,
    pub value: String,
}

/// L20: `review/<path_id>.json`, the per-node review policy (mirrors `tools/packs/schemas/review.schema.json`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviewFile {
    pub path_id: String,
    #[serde(rename = "_comment", default)]
    pub comment: Option<String>,
    pub nodes: BTreeMap<String, ReviewRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviewRow {
    pub engine: String,
    pub desired_retention: f64,
    pub max_interval_days: u32,
    #[serde(default)]
    pub first_interval_days: Option<u32>,
    pub items_from: ItemsFrom,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemsFrom {
    LessonSteps,
    GraphItems,
}

/// Everything `validate_loop` reads besides the path, the goal kinds and the pins.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct LoopData {
    #[serde(default)]
    pub personas: BTreeMap<String, Persona>,
    #[serde(default)]
    pub eval_sets: BTreeMap<String, EvalSet>,
    #[serde(default)]
    pub templates: BTreeMap<String, LessonTemplate>,
    #[serde(default)]
    pub follow_up: Option<FollowUp>,
    #[serde(default)]
    pub evidence: Option<EvidenceBindings>,
    /// L20: the review file for this path, and whether the pack root has a `review/` folder at all (the layer is opt-in by folder).
    #[serde(default)]
    pub review: Option<ReviewFile>,
    #[serde(default)]
    pub review_layer: bool,
}

fn json_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut v: Vec<_> = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json")).collect();
    v.sort();
    Ok(v)
}

fn read<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T, String> {
    let s = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", p.display()))
}

/// Reads the four folders of a pack root; `follow_up` and `evidence` are the files whose `path_id` is `path_id`.
pub fn load_dir(root: &Path, path_id: &str) -> Result<LoopData, String> {
    let mut d = LoopData::default();
    for f in json_files(&root.join("personas"))? {
        let p: Persona = read(&f)?;
        d.personas.insert(p.key.clone(), p);
    }
    for f in json_files(&root.join("personas").join("evals"))? {
        let stem = f.file_stem().unwrap_or_default().to_string_lossy().to_string();
        d.eval_sets.insert(stem, read(&f)?);
    }
    for f in json_files(&root.join("lesson-templates"))? {
        let t: LessonTemplate = read(&f)?;
        d.templates.insert(t.id.clone(), t);
    }
    for f in json_files(&root.join("follow-up"))? {
        let x: FollowUp = read(&f)?;
        if x.path_id == path_id {
            d.follow_up = Some(x);
        }
    }
    for f in json_files(&root.join("evidence"))? {
        let x: EvidenceBindings = read(&f)?;
        if x.path_id == path_id {
            d.evidence = Some(x);
        }
    }
    d.review_layer = root.join("review").is_dir();
    for f in json_files(&root.join("review"))? {
        let x: ReviewFile = read(&f)?;
        if x.path_id == path_id {
            d.review = Some(x);
        }
    }
    Ok(d)
}
