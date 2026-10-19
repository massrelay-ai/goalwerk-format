//! L20 (D17): review intervals for lesson items, scheduled by FSRS-6 from the published `fsrs` crate 6.6.2
//! (https://github.com/open-spaced-repetition/fsrs-rs, BSD-3-Clause, Copyright (c) 2023, Open Spaced Repetition).
//! Taken whole, not rewritten: the memory state is `FSRS::memory_state` and the interval is `FSRS::next_interval`
//! (the crate's `next_states` path, src/inference.rs:353-370, and `Model::step`, src/model.rs:160-168), with the crate's
//! default FSRS-6 parameters. What is ours is the wrapper only: the policy (retention bounds, first and maximum interval),
//! whole-day rounding, and the JSON shape. A review is practice scheduling, never proof (AGENTS.md section 1; R4).
use fsrs::{current_retrievability, FSRSItem, FSRSReview, DEFAULT_PARAMETERS, FSRS, FSRS6_DEFAULT_DECAY};
use serde::{Deserialize, Serialize};

/// One past review of an item: the learner's answer (1 again, 2 hard, 3 good, 4 easy) and the whole days since the previous
/// review (0 for the first).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReviewLog {
    pub rating: u8,
    pub elapsed_days: u32,
}

/// The pack's per-node policy (`packs/atingo/review/<path_id>.json`), reduced to what the schedule needs.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct ReviewPolicy {
    pub desired_retention: f64,
    pub max_interval_days: u32,
    #[serde(default = "default_first")]
    pub first_interval_days: u32,
}

fn default_first() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ReviewPlan {
    pub due_in_days: u32,
    pub stability: f64,
    pub difficulty: f64,
    pub retrievability: f64,
}

/// Retention outside (0.5, 0.99), a maximum below the first interval, or a rating outside 1..=4 is refused.
pub fn next_review(history: &[ReviewLog], policy: &ReviewPolicy) -> Result<ReviewPlan, String> {
    let r = policy.desired_retention;
    if !(r > 0.5 && r < 0.99) {
        return Err(format!("desired_retention {r} is outside (0.5, 0.99)"));
    }
    if policy.first_interval_days == 0 || policy.max_interval_days < policy.first_interval_days {
        return Err("max_interval_days must be at least first_interval_days, and first_interval_days at least 1".into());
    }
    if let Some(bad) = history.iter().find(|l| !(1..=4).contains(&l.rating)) {
        return Err(format!("rating {} is outside 1..=4", bad.rating));
    }
    let (first, max) = (policy.first_interval_days, policy.max_interval_days);
    if history.is_empty() {
        return Ok(ReviewPlan { due_in_days: first, stability: 0.0, difficulty: 0.0, retrievability: 1.0 });
    }
    let fsrs = FSRS::new(&DEFAULT_PARAMETERS).map_err(|e| e.to_string())?;
    let item = FSRSItem { reviews: history.iter().map(|l| FSRSReview { rating: l.rating as u32, delta_t: l.elapsed_days }).collect() };
    let state = fsrs.memory_state(item, None).map_err(|e| e.to_string())?;
    let interval = fsrs.next_interval(Some(state.stability), r as f32, 0);
    if !interval.is_finite() {
        return Err("the interval is not finite".into());
    }
    let due = interval.round().clamp(first as f32, max as f32);
    let retrievability = current_retrievability(state, due, FSRS6_DEFAULT_DECAY);
    Ok(ReviewPlan { due_in_days: due as u32, stability: state.stability as f64, difficulty: state.difficulty as f64, retrievability: retrievability as f64 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VECTORS: &str = include_str!("../../../tools/packs/fixtures/review-vectors.json");

    fn vectors() -> Value {
        serde_json::from_str(VECTORS).unwrap()
    }

    fn history(v: &Value) -> Vec<ReviewLog> {
        serde_json::from_value(v.clone()).unwrap()
    }

    #[test]
    fn every_vector_case_matches_the_crate_output() {
        let v = vectors();
        let cases = v["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 7);
        for c in cases {
            let policy: ReviewPolicy = serde_json::from_value(c["policy"].clone()).unwrap();
            let plan = next_review(&history(&c["history"]), &policy).unwrap();
            let e = &c["expect"];
            let name = c["name"].as_str().unwrap();
            assert_eq!(plan.due_in_days as u64, e["due_in_days"].as_u64().unwrap(), "{name} due");
            for (k, got) in [("stability", plan.stability), ("difficulty", plan.difficulty), ("retrievability", plan.retrievability)] {
                let want = e[k].as_f64().unwrap();
                assert!((got - want).abs() <= 1e-6 * want.abs().max(1.0), "{name} {k}: {got} vs {want}");
            }
        }
    }

    #[test]
    fn the_crates_own_tests_reused() {
        let v = vectors();
        let ms = &v["crate_tests"]["memory_state"];
        let policy = ReviewPolicy { desired_retention: 0.9, max_interval_days: 36500, first_interval_days: 1 };
        let plan = next_review(&history(&ms["history"]), &policy).unwrap();
        assert!((plan.stability - ms["stability"].as_f64().unwrap()).abs() < 1e-4, "{}", plan.stability);
        assert!((plan.difficulty - ms["difficulty"].as_f64().unwrap()).abs() < 1e-4, "{}", plan.difficulty);
        // test_next_interval: one first review whose stability is 1.0 is not reachable by rating, so the interval vector
        // checks the crate call directly (retention i/10 over stability 1.0).
        let ni = &v["crate_tests"]["next_interval"];
        let f = FSRS::new(&DEFAULT_PARAMETERS).unwrap();
        let got: Vec<i64> = ni["retentions"].as_array().unwrap().iter().map(|r| f.next_interval(Some(1.0), r.as_f64().unwrap() as f32, 1).round().max(1.0) as i64).collect();
        let want: Vec<i64> = ni["intervals"].as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect();
        assert_eq!(got, want);
    }

    #[test]
    fn empty_history_is_the_first_interval_never_zero_or_nan() {
        let p = ReviewPolicy { desired_retention: 0.9, max_interval_days: 180, first_interval_days: 1 };
        let plan = next_review(&[], &p).unwrap();
        assert_eq!(plan.due_in_days, 1);
        assert!(plan.stability.is_finite() && plan.difficulty.is_finite() && plan.retrievability.is_finite());
        let p3 = ReviewPolicy { first_interval_days: 3, ..p };
        assert_eq!(next_review(&[], &p3).unwrap().due_in_days, 3);
    }

    #[test]
    fn growth_cap_and_lapse() {
        let p = ReviewPolicy { desired_retention: 0.9, max_interval_days: 180, first_interval_days: 1 };
        let mut h = vec![];
        let mut last = 0;
        for (i, gap) in [0u32, 1, 3, 8, 20].iter().enumerate() {
            h.push(ReviewLog { rating: 3, elapsed_days: *gap });
            let d = next_review(&h, &p).unwrap().due_in_days;
            if i > 0 {
                assert!(d > last, "grows: {d} > {last}");
            }
            last = d;
        }
        let capped = ReviewPolicy { max_interval_days: 30, ..p };
        assert_eq!(next_review(&h, &capped).unwrap().due_in_days, 30);
        h.push(ReviewLog { rating: 1, elapsed_days: last });
        assert!(next_review(&h, &p).unwrap().due_in_days < last);
    }

    #[test]
    fn policies_and_ratings_out_of_range_are_refused() {
        let base = ReviewPolicy { desired_retention: 0.9, max_interval_days: 180, first_interval_days: 1 };
        for r in [0.5, 0.99, 0.2, 1.0, f64::NAN] {
            let e = next_review(&[], &ReviewPolicy { desired_retention: r, ..base }).unwrap_err();
            assert!(e.contains("desired_retention"), "{r}: {e}");
        }
        assert!(next_review(&[], &ReviewPolicy { max_interval_days: 0, ..base }).is_err());
        assert!(next_review(&[ReviewLog { rating: 0, elapsed_days: 0 }], &base).is_err());
        assert!(next_review(&[ReviewLog { rating: 5, elapsed_days: 0 }], &base).is_err());
        assert!(next_review(&[ReviewLog { rating: 3, elapsed_days: 0 }], &ReviewPolicy { first_interval_days: 200, ..base }).is_err());
    }
}
