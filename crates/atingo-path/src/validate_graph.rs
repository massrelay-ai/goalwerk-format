//! L16: the graph rules (spec 03 §4.1), pure and without I/O. `tools/packs/check.mjs` (`graphRules`) has the same codes; the shared
//! fixtures (`graph_*`) prove the two agree. Claim verbs are matched as token runs of `[a-z0-9]+` so both sides tokenise identically.
use crate::graph::Graph;
use crate::validate::{Finding, Severity};
use std::collections::{BTreeMap, BTreeSet};

const CLAIMS: &[&[&str]] = &[&["guaranteed"], &["will", "earn"], &["cures"], &["risk", "free"]];

fn words(s: &str) -> Vec<String> {
    s.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_string).collect()
}

/// The claim phrases found in `text` (empty = clean).
pub fn claim_verbs(text: &str) -> Vec<String> {
    let w = words(text);
    CLAIMS.iter().filter(|c| (0..w.len()).any(|i| c.iter().enumerate().all(|(k, t)| w.get(i + k).is_some_and(|x| x == t)))).map(|c| c.join(" ")).collect()
}

/// `today` is an ISO date (`YYYY-MM-DD`), compared as a string against `valid_until`.
pub fn validate_graph(g: &Graph, today: &str) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut push = |severity: Severity, code: &'static str, path: &str, message: String| out.push(Finding { severity, code, path: path.to_string(), message });
    let sectors: BTreeMap<&str, bool> = g.sectors.iter().map(|s| (s.id.as_str(), s.regulated)).collect();
    let comps: BTreeMap<&str, &crate::graph::Competency> = g.competencies.iter().map(|(_, c)| (c.id.as_str(), c)).collect();
    let occ_ids: BTreeSet<&str> = g.occupations.iter().map(|(_, o)| o.id.as_str()).collect();
    let item_ids: BTreeSet<&str> = g.items.iter().flat_map(|(_, f)| f.items.iter().map(|i| i.id.as_str())).collect();
    let sources: BTreeMap<&str, &str> = g.sources.iter().map(|r| (r.url.as_str(), r.sha256.as_str())).collect();
    let mut used: BTreeSet<&str> = BTreeSet::new();
    for (name, o) in &g.occupations {
        if !sectors.contains_key(o.part_of.as_str()) {
            push(Severity::Error, "graph_parent_missing", name, format!("occupation \"{}\" part_of \"{}\" is not a sector", o.id, o.part_of));
        }
        for t in &o.top {
            if !comps.contains_key(t.comp.as_str()) {
                push(Severity::Error, "graph_parent_missing", name, format!("occupation \"{}\" names competency \"{}\" that does not exist", o.id, t.comp));
            }
            used.insert(t.comp.as_str());
        }
    }
    for (name, c) in &g.competencies {
        if !sectors.contains_key(c.part_of.as_str()) {
            push(Severity::Error, "graph_parent_missing", name, format!("competency \"{}\" part_of \"{}\" is not a sector", c.id, c.part_of));
        }
        for r in &c.requires {
            if !comps.contains_key(r.as_str()) {
                push(Severity::Error, "graph_parent_missing", name, format!("competency \"{}\" requires \"{r}\" that does not exist", c.id));
            }
        }
        for oc in &c.occupations {
            if occ_ids.contains(oc.as_str()) {
                used.insert(c.id.as_str());
            } else {
                push(Severity::Error, "graph_parent_missing", name, format!("competency \"{}\" names occupation \"{oc}\" that does not exist", c.id));
            }
        }
    }
    for (_, c) in &g.competencies {
        if !used.contains(c.id.as_str()) {
            push(Severity::Error, "graph_competency_orphan", "competencies", format!("competency \"{}\" is in no occupation (and no path teaches it)", c.id));
        }
    }
    // requires cycle across every competency file: DFS in sorted id order, the first cycle found
    let mut state: BTreeMap<&str, u8> = BTreeMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut cycle: Option<Vec<&str>> = None;
    fn dfs<'a>(n: &'a str, comps: &BTreeMap<&'a str, &'a crate::graph::Competency>, state: &mut BTreeMap<&'a str, u8>, stack: &mut Vec<&'a str>, cycle: &mut Option<Vec<&'a str>>) {
        if cycle.is_some() || !comps.contains_key(n) {
            return;
        }
        match state.get(n) {
            Some(1) => {
                let at = stack.iter().position(|x| *x == n).unwrap_or(0);
                let mut c: Vec<&str> = stack[at..].to_vec();
                c.push(n);
                *cycle = Some(c);
                return;
            }
            Some(2) => return,
            _ => {}
        }
        state.insert(n, 1);
        stack.push(n);
        for m in &comps[n].requires {
            dfs(m.as_str(), comps, state, stack, cycle);
        }
        stack.pop();
        state.insert(n, 2);
    }
    for id in comps.keys() {
        dfs(id, &comps, &mut state, &mut stack, &mut cycle);
    }
    if let Some(c) = cycle {
        push(Severity::Error, "graph_requires_cycle", "competencies", format!("requires cycle: {}", c.join(" -> ")));
    }
    for (name, f) in &g.items {
        let comp = comps.get(f.competency.as_str());
        let sector = comp.and_then(|c| sectors.get_key_value(c.part_of.as_str()));
        let regulated = sector.is_some_and(|(_, r)| *r);
        if comp.is_none() {
            push(Severity::Error, "graph_parent_missing", name, format!("items file names competency \"{}\" that does not exist", f.competency));
        }
        for it in &f.items {
            let urls: BTreeSet<&str> = it.sources.iter().map(|s| s.url.as_str()).collect();
            if urls.len() < 2 {
                push(Severity::Error, "graph_item_sources_lt_2", name, format!("item \"{}\" has {} distinct source(s) (needs 2)", it.id, urls.len()));
            }
            for s in &it.sources {
                if sources.get(s.url.as_str()) != Some(&s.sha256.as_str()) {
                    push(Severity::Error, "graph_item_source_unfetched", name, format!("item \"{}\" source {} has no matching record (url and sha256) in sources/", it.id, s.url));
                }
            }
            if it.kind == "figure" && it.valid_until.is_none() {
                push(Severity::Error, "graph_figure_without_valid_until", name, format!("figure item \"{}\" has no valid_until", it.id));
            }
            if let Some(v) = &it.valid_until {
                if v.as_str() < today {
                    push(Severity::Warning, "graph_valid_until_past", name, format!("item \"{}\" valid_until {v} is past ({today})", it.id));
                }
            }
            if regulated {
                if it.badge.as_deref() != Some("information_not_advice") {
                    push(Severity::Error, "graph_regulated_unbadged", name, format!("item \"{}\" is under a regulated sector without badge information_not_advice", it.id));
                }
                for v in claim_verbs(&it.text) {
                    push(Severity::Error, "graph_regulated_claim_verb", name, format!("item \"{}\" says \"{v}\" in a regulated sector", it.id));
                }
            }
        }
    }
    for (name, f) in &g.lessons {
        for l in &f.lessons {
            for st in &l.steps {
                if !item_ids.contains(st.item.as_str()) {
                    push(Severity::Error, "graph_parent_missing", name, format!("lesson \"{}\" step names item \"{}\" that does not exist", l.id, st.item));
                }
            }
        }
    }
    out
}
