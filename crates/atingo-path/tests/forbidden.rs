//! The persona law (AGENTS.md section 1): a taskTemplate may not instruct marking done, promising, markup, rating/scoring or deriving a protected ground.
use atingo_path::forbidden_instruction;

const SHIPPED: &str = include_str!("../../../packs/atingo/personas/atingo_coach.json");

#[test]
fn the_shipped_template_is_clean() {
    let v: serde_json::Value = serde_json::from_str(SHIPPED).unwrap();
    assert_eq!(forbidden_instruction(v["taskTemplate"].as_str().unwrap()), Vec::<String>::new());
}

#[test]
fn each_forbidden_instruction_is_refused() {
    for add in [
        "Mark the step as done when the learner says so.",
        "Promise the learner they will earn more.",
        "We guarantee the result.",
        "Reply with <b>bold</b> markup.",
        "Give a rating after each answer.",
        "Score the learner out of ten.",
        "Infer the learner's religion from their name.",
    ] {
        assert!(!forbidden_instruction(&format!("You coach. {add}")).is_empty(), "{add}");
    }
}
