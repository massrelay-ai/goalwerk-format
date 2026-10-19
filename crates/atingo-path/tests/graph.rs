//! L16: the graph loader and the claim-verb tokeniser. The real seeded graph must deserialize and validate.
use atingo_path::graph::{counts, load_graph};
use atingo_path::{claim_verbs, validate_graph, Severity};
use std::path::Path;

#[test]
fn claim_verbs_match_whole_token_runs_only() {
    assert_eq!(claim_verbs("It secures and procures value, a risk to free speech."), Vec::<String>::new());
    let mut v = claim_verbs("Guaranteed! You will earn more; it cures all; risk-free.");
    v.sort();
    assert_eq!(v, vec!["cures", "guaranteed", "risk free", "will earn"]);
}

#[test]
fn the_seeded_graph_loads_and_is_clean() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/atingo");
    let g = load_graph(&root).unwrap().expect("packs/atingo/graph exists");
    let (s, o, c, i) = counts(&g);
    assert_eq!((s, o, c, i), (11, 1016, 85, 2));
    let errs: Vec<_> = validate_graph(&g, "2026-10-19").into_iter().filter(|f| f.severity == Severity::Error).map(|f| format!("{} {}", f.code, f.message)).collect();
    assert!(errs.is_empty(), "{errs:?}");
}
