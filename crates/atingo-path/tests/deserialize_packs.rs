use atingo_path::{GoalKinds, PathFile, Tier};
fn repo(p: &str) -> String { std::fs::read_to_string(format!("{}/../../packs/atingo/{p}", env!("CARGO_MANIFEST_DIR"))).unwrap() }
#[test] fn the_wealth_kinds_deserialize() { let k: GoalKinds = serde_json::from_str(&repo("goal-kinds/wealth.json")).unwrap(); assert_eq!(k.kinds.len(), 5); assert_eq!(k.kinds[0].id, "save_first_amount"); }
#[test] fn the_getting_ahead_path_deserializes() { let p: PathFile = serde_json::from_str(&repo("paths/getting-ahead.json")).unwrap(); assert_eq!(p.tier, Tier::Platform); assert_eq!(p.nodes.len(), 5); assert_eq!(p.edges.len(), 4); assert_eq!(p.root, "getting_ahead"); assert!(p.certificate.is_none()); }
#[test] fn an_unknown_tier_is_refused() { let bad = repo("paths/getting-ahead.json").replace("\"platform\"", "\"gold\""); assert!(serde_json::from_str::<PathFile>(&bad).is_err()); }
