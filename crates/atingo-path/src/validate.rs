//! The cross rules over one path file (pure, no I/O). `tools/packs/check.mjs` implements the same codes; the shared fixtures
//! under `tools/packs/fixtures/` prove the two agree.
use crate::pins::Pins;
use crate::{EdgeKind, GoalKinds, PathFile, Tier};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

pub fn validate(path: &PathFile, kinds: &GoalKinds, pins: &Pins) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut push = |severity: Severity, code: &'static str, message: String| {
        out.push(Finding { severity, code, path: path.id.clone(), message });
    };
    let kind_ids: BTreeSet<&str> = kinds.kinds.iter().map(|k| k.id.as_str()).collect();
    let ids: BTreeSet<&str> = path.nodes.iter().map(|n| n.id.as_str()).collect();

    if !ids.contains(path.root.as_str()) {
        push(Severity::Error, "root_missing", format!("root \"{}\" is not a node", path.root));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for n in &path.nodes {
        if !seen.insert(n.id.as_str()) {
            push(Severity::Error, "duplicate_node_id", format!("node id \"{}\" appears more than once", n.id));
        }
        if !kind_ids.contains(n.goal_kind.as_str()) {
            push(Severity::Error, "goal_kind_missing", format!("node \"{}\" goal_kind \"{}\" is not in goal-kinds/", n.id, n.goal_kind));
        }
        for b in &n.blocks_allowed {
            if !pins.block_kinds.contains(b) {
                push(Severity::Error, "block_kind_unknown", format!("node \"{}\" blocks_allowed \"{}\" is not a kernel block kind", n.id, b));
            }
        }
        let nsrc = n.sources.len();
        if path.tier == Tier::Platform && nsrc < 2 {
            if path.price_ref.is_some() {
                push(Severity::Error, "sources_missing_when_priced", format!("platform node \"{}\" has fewer than two sources ({nsrc}) and price_ref is set", n.id));
            } else {
                push(Severity::Warning, "sources_missing_unpriced", format!("platform node \"{}\" has fewer than two sources ({nsrc}); required before price_ref is set", n.id));
            }
        }
    }
    for e in &path.edges {
        for end in [&e.from, &e.to] {
            if !ids.contains(end.as_str()) {
                push(Severity::Error, "edge_node_missing", format!("edge \"{}\" -> \"{}\" names a missing node \"{}\"", e.from, e.to, end));
            }
        }
    }
    for (nid, ks) in &path.people_kinds_by_node {
        if !ids.contains(nid.as_str()) {
            push(Severity::Error, "people_node_missing", format!("people_kinds_by_node names a missing node \"{nid}\""));
        }
        for k in ks {
            if !pins.people_kinds.contains(k) {
                push(Severity::Error, "people_kind_unknown", format!("node \"{nid}\" people kind \"{k}\" is not in the kernel's goal-kinds.json"));
            }
        }
    }
    if path.certificate.is_some() && !matches!(path.tier, Tier::Platform) && !path.certificate.as_ref().is_some_and(|c| c.is_null()) {
        let tier = match path.tier {
            Tier::Platform => "platform",
            Tier::Creator => "creator",
            Tier::SelfTier => "self",
        };
        push(Severity::Error, "certificate_on_non_platform", format!("certificate on tier {tier} (a certificate belongs to platform only)"));
    }

    // prerequisite cycle: DFS in node order over prerequisite edges only (a branch loop is allowed); the first cycle found
    let mut next: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in path.edges.iter().filter(|e| e.kind == EdgeKind::Prerequisite) {
        next.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }
    let mut state: BTreeMap<&str, u8> = BTreeMap::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut cycle: Option<Vec<&str>> = None;
    for n in &path.nodes {
        dfs(n.id.as_str(), &next, &mut state, &mut stack, &mut cycle);
    }
    if let Some(c) = cycle {
        push(Severity::Error, "prerequisite_cycle", format!("prerequisite cycle: {}", c.join(" -> ")));
    }

    // reachability from the root over all edges; skipped when the root itself is missing (already one error)
    if ids.contains(path.root.as_str()) {
        let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for e in &path.edges {
            adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
        }
        let mut reach: BTreeSet<&str> = BTreeSet::from([path.root.as_str()]);
        let mut q: VecDeque<&str> = VecDeque::from([path.root.as_str()]);
        while let Some(n) = q.pop_front() {
            for m in adj.get(n).into_iter().flatten() {
                if reach.insert(m) {
                    q.push_back(m);
                }
            }
        }
        for id in &ids {
            if !reach.contains(id) {
                push(Severity::Error, "node_unreachable_from_root", format!("node \"{id}\" is not reachable from root \"{}\"", path.root));
            }
        }
    }
    out
}

fn dfs<'a>(n: &'a str, next: &BTreeMap<&'a str, Vec<&'a str>>, state: &mut BTreeMap<&'a str, u8>, stack: &mut Vec<&'a str>, cycle: &mut Option<Vec<&'a str>>) {
    if cycle.is_some() {
        return;
    }
    match state.get(n) {
        Some(1) => {
            let at = stack.iter().position(|s| *s == n).unwrap_or(0);
            let mut c = stack[at..].to_vec();
            c.push(n);
            *cycle = Some(c);
            return;
        }
        Some(2) => return,
        _ => {}
    }
    state.insert(n, 1);
    stack.push(n);
    for m in next.get(n).into_iter().flatten() {
        dfs(m, next, state, stack, cycle);
    }
    stack.pop();
    state.insert(n, 2);
}
