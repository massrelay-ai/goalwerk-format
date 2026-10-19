//! L13: the LOCALE layer (spec 01 section 5): `i18n/<path_id>.<locale>.json`. Types, the loader and the rules, pure data.
//! `tools/packs/check.mjs` (`i18nRules`) has the same six codes; the shared fixtures prove the two agree.
//! The law: the locale layer never adds or removes a step (the count equals the template's `levels[0].steps`), and no string is empty.
use crate::loop_data::LessonTemplate;
use crate::pins::Pins;
use crate::validate::{Finding, Severity};
use crate::PathFile;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocaleFile {
    pub path_id: String,
    pub locale: String,
    #[serde(rename = "_comment", default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub nodes: BTreeMap<String, LocaleNode>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocaleNode {
    pub title: String,
    pub lesson: LocaleLesson,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocaleLesson {
    pub steps: Vec<LocaleStep>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LocaleStep {
    pub prompt: String,
    pub evidence_hint: String,
}

/// Every `i18n/*.json` of a pack root, sorted, as `("i18n/<file>", parsed)`; an absent folder is an empty list.
pub fn load_i18n(root: &Path) -> Result<Vec<(String, LocaleFile)>, String> {
    let dir = root.join("i18n");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<_> = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let s = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let l: LocaleFile = serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", f.display()))?;
        out.push((format!("i18n/{}", f.file_name().unwrap_or_default().to_string_lossy()), l));
    }
    Ok(out)
}

pub fn validate_i18n(files: &[(String, LocaleFile)], paths: &[PathFile], templates: &BTreeMap<String, LessonTemplate>, pins: &Pins) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut push = |code: &'static str, p: &str, message: String| out.push(Finding { severity: Severity::Error, code, path: p.to_string(), message });
    for (name, l) in files {
        let Some(path) = paths.iter().find(|p| p.id == l.path_id) else {
            push("i18n_path_unknown", name, format!("locale file names path \"{}\" that has no file in paths/", l.path_id));
            continue;
        };
        if !pins.locales.contains(&l.locale) {
            push("i18n_locale_unsupported", name, format!("locale \"{}\" is not in kernel-pins.json locales", l.locale));
        }
        let ids: BTreeSet<&str> = path.nodes.iter().map(|n| n.id.as_str()).collect();
        for id in l.nodes.keys() {
            if !ids.contains(id.as_str()) {
                push("i18n_node_unknown", name, format!("locale file names node \"{id}\" that path \"{}\" does not have", path.id));
            }
        }
        for n in &path.nodes {
            let Some(e) = l.nodes.get(&n.id) else {
                push("i18n_node_missing", name, format!("path node \"{}\" has no entry in the {} locale file", n.id, l.locale));
                continue;
            };
            if e.title.trim().is_empty() {
                push("i18n_empty_string", name, format!("node \"{}\" title is empty", n.id));
            }
            for (i, s) in e.lesson.steps.iter().enumerate() {
                for (k, v) in [("prompt", &s.prompt), ("evidence_hint", &s.evidence_hint)] {
                    if v.trim().is_empty() {
                        push("i18n_empty_string", name, format!("node \"{}\" step {} {k} is empty", n.id, i + 1));
                    }
                }
            }
            if let Some(want) = templates.get(&n.lesson_template).and_then(|t| t.levels.first()).map(|lv| lv.steps.len()) {
                if e.lesson.steps.len() != want {
                    push("i18n_steps_count_mismatch", name, format!("node \"{}\" has {} steps in {}, the template \"{}\" has {want}", n.id, e.lesson.steps.len(), l.locale, n.lesson_template));
                }
            }
        }
    }
    out
}
