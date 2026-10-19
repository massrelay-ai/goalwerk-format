//! L15: the loop rules (persona, lesson templates, follow-up, evidence), pure and without I/O. `tools/packs/check.mjs` has the same
//! codes (`loopRules`); the shared fixtures prove the two agree. The forbidden-instruction check is regex-free on purpose so both
//! sides tokenise identically: tokens of `[a-z0-9]+`, sentences split on `. ! ? ;` and newlines.
use crate::loop_data::LoopData;
use crate::pins::Pins;
use crate::validate::{Finding, Severity};
use crate::{GoalKinds, PathFile};
use std::collections::BTreeSet;

pub const MIN_EVAL_CASES: usize = 12;
const PROTECTED: &[&str] = &[
    "gender", "sex", "male", "female", "men", "women", "age", "old", "young", "elderly", "health", "sick", "healthy", "disabled", "disability", "religion", "faith", "faithful", "faithless", "nationality", "race", "ethnic", "orientation",
];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_string).collect()
}

fn has_markup(text: &str) -> bool {
    let b: Vec<char> = text.chars().collect();
    for i in 0..b.len() {
        if b[i] != '<' {
            continue;
        }
        let mut j = i + 1;
        if j < b.len() && b[j] == '/' {
            j += 1;
        }
        let start = j;
        while j < b.len() && b[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j > start && j < b.len() && b[j] == '>' {
            return true;
        }
    }
    false
}

/// The forbidden acts a persona's taskTemplate may not instruct: the reasons found (empty = clean).
pub fn forbidden_instruction(text: &str) -> Vec<String> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    if has_markup(text) {
        found.insert("markup".into());
    }
    for sentence in text.split(['.', '!', '?', ';', '\n']) {
        let w = words(sentence);
        let at = |pred: &dyn Fn(&str) -> bool, from: usize| w.iter().enumerate().position(|(i, t)| i >= from && pred(t));
        if let Some(m) = at(&|t| matches!(t, "mark" | "marks" | "marked" | "marking"), 0) {
            if at(&|t| matches!(t, "done" | "complete" | "completed" | "finished"), m + 1).is_some() {
                found.insert("mark a step done".into());
            }
        }
        if w.iter().any(|t| t.starts_with("promis") || t.starts_with("guarant")) {
            found.insert("promise or guarantee".into());
        }
        if w.iter().any(|t| t == "rating" || t == "ratings") {
            found.insert("rating".into());
        }
        for i in 0..w.len().saturating_sub(2) {
            if matches!(w[i].as_str(), "score" | "grade" | "rate" | "rank") && w[i + 1] == "the" && matches!(w[i + 2].as_str(), "learner" | "person" | "user" | "student") {
                found.insert(format!("{} the {}", w[i], w[i + 2]));
            }
        }
        if let Some(d) = at(&|t| ["infer", "derive", "deduce", "guess", "determine"].iter().any(|v| t.starts_with(v)), 0) {
            if at(&|t| PROTECTED.contains(&t), d + 1).is_some() {
                found.insert("derive a protected ground".into());
            }
        }
    }
    found.into_iter().collect()
}

fn all_digits(v: &str) -> bool {
    !v.is_empty() && v.chars().all(|c| c.is_ascii_digit())
}

/// The kernel's RRULE subset (`lifecycle.rs` `parse_recurrence`): FREQ required (DAILY, WEEKLY, MONTHLY, YEARLY),
/// INTERVAL 1..=366, COUNT >= 1, `;`-separated, nothing else, optional `RRULE:` prefix.
pub fn rrule_ok(rule: &str) -> bool {
    let rule = rule.trim();
    let body = rule.strip_prefix("RRULE:").unwrap_or(rule);
    let mut freq = false;
    for part in body.split(';').filter(|p| !p.is_empty()) {
        let Some((k, v)) = part.split_once('=') else { return false };
        match k.to_ascii_uppercase().as_str() {
            "FREQ" => {
                if !matches!(v.to_ascii_uppercase().as_str(), "DAILY" | "WEEKLY" | "MONTHLY" | "YEARLY") {
                    return false;
                }
                freq = true;
            }
            "INTERVAL" => {
                let t = v.trim_start_matches('0');
                if !all_digits(v) || t.is_empty() || t.len() > 3 || t.parse::<u32>().map_or(true, |n| n > 366) {
                    return false;
                }
            }
            "COUNT" => {
                if !all_digits(v) || v.trim_start_matches('0').is_empty() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    freq
}

pub fn validate_loop(path: &PathFile, kinds: &GoalKinds, pins: &Pins, data: &LoopData) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut push = |code: &'static str, p: String, message: String| out.push(Finding { severity: Severity::Error, code, path: p, message });
    let file = format!("paths/{}.json", path.id);
    match path.persona.as_deref() {
        None | Some("") => push("persona_missing", file.clone(), format!("path \"{}\" names no persona", path.id)),
        Some(name) => match data.personas.get(name) {
            None => push("persona_unknown", file.clone(), format!("path \"{}\" persona \"{name}\" has no file in personas/", path.id)),
            Some(per) => {
                for f in forbidden_instruction(&per.task_template) {
                    push("persona_forbidden_instruction", format!("personas/{}.json", per.key), format!("taskTemplate of \"{}\" instructs a forbidden act: {f}", per.key));
                }
                let n = data.eval_sets.get(&per.eval_set_ref).map_or(0, |e| e.0.len());
                if n < MIN_EVAL_CASES {
                    push("eval_set_too_small", format!("personas/evals/{}.json", per.eval_set_ref), format!("eval set \"{}\" has {n} cases (needs {MIN_EVAL_CASES})", per.eval_set_ref));
                }
            }
        },
    }
    for n in &path.nodes {
        if !data.templates.contains_key(&n.lesson_template) {
            push("lesson_template_missing", file.clone(), format!("node \"{}\" lesson_template \"{}\" has no file in lesson-templates/", n.id, n.lesson_template));
        }
    }
    let fu_file = format!("follow-up/{}.json", path.id);
    let ev_file = format!("evidence/{}.json", path.id);
    for n in &path.nodes {
        match data.follow_up.as_ref().and_then(|f| f.nodes.get(&n.id)) {
            None => push("followup_missing", fu_file.clone(), format!("node \"{}\" has no follow-up row", n.id)),
            Some(fu) => {
                if !rrule_ok(&fu.rrule) {
                    push("followup_rrule_invalid", fu_file.clone(), format!("node \"{}\" rrule \"{}\" is outside the kernel subset (FREQ=DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL, COUNT)", n.id, fu.rrule));
                }
                for u in &fu.nudges {
                    if !pins.nudge_channels.contains(&u.channel) {
                        push("followup_channel_unknown", fu_file.clone(), format!("node \"{}\" nudge channel \"{}\" is not a kernel channel", n.id, u.channel));
                    }
                }
                if let Some(people) = path.people_kinds_by_node.get(&n.id) {
                    if !people.contains(&fu.escalate.to) {
                        push("people_kind_unknown", fu_file.clone(), format!("node \"{}\" escalate.to \"{}\" is not one of its people kinds", n.id, fu.escalate.to));
                    }
                }
            }
        }
        let Some(ev) = data.evidence.as_ref().and_then(|e| e.nodes.get(&n.id)) else {
            push("evidence_binding_missing", ev_file.clone(), format!("node \"{}\" has no evidence binding", n.id));
            continue;
        };
        if !pins.verifier_kinds.contains(&ev.verifier) {
            push("evidence_verifier_unknown", ev_file.clone(), format!("node \"{}\" verifier \"{}\" is not a kernel verifier kind", n.id, ev.verifier));
        }
        if let Some(k) = kinds.kinds.iter().find(|k| k.id == n.goal_kind) {
            if !k.attestation_fallback && (ev.verifier == "attestation" || ev.fallback == "attestation") {
                push("evidence_verifier_unknown", ev_file.clone(), format!("node \"{}\" uses attestation but goal kind \"{}\" has attestation_fallback false", n.id, k.id));
            }
        }
        if let Some(c) = &ev.binding.connector_id {
            if !ev.sources_of_truth.contains(c) {
                push("evidence_binding_outside_sources", ev_file.clone(), format!("node \"{}\" binding connector \"{c}\" is not in its sources_of_truth", n.id));
            }
        }
    }
    if data.review_layer {
        let rv_file = format!("review/{}.json", path.id);
        for n in &path.nodes {
            let Some(row) = data.review.as_ref().and_then(|r| r.nodes.get(&n.id)) else {
                out.push(Finding { severity: Severity::Warning, code: "review_policy_missing", path: rv_file.clone(), message: format!("node \"{}\" has no review policy", n.id) });
                continue;
            };
            if row.engine != "fsrs" {
                out.push(Finding { severity: Severity::Error, code: "review_engine_unknown", path: rv_file.clone(), message: format!("node \"{}\" review engine \"{}\" is not \"fsrs\"", n.id, row.engine) });
            }
            if !(row.desired_retention > 0.5 && row.desired_retention < 0.99) {
                out.push(Finding { severity: Severity::Error, code: "review_retention_out_of_range", path: rv_file.clone(), message: format!("node \"{}\" desired_retention {} is outside (0.5, 0.99)", n.id, row.desired_retention) });
            }
        }
    }
    out
}
