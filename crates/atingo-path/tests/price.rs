//! L7: the in-memory edges of the price rules (the fixtures cover the three headline codes).
use atingo_path::pins::Pins;
use atingo_path::price::PriceRow;
use atingo_path::validate_price;
use std::collections::BTreeMap;
use std::path::Path;

fn pins() -> Pins {
    serde_json::from_str(&std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/packs/kernel-pins.json")).unwrap()).unwrap()
}
fn row() -> PriceRow {
    serde_json::from_str(&std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/atingo/prices/getting-ahead.json")).unwrap()).unwrap()
}
fn codes(r: PriceRow) -> Vec<&'static str> {
    validate_price(&[], &BTreeMap::from([("getting-ahead".to_string(), r)]), &pins()).into_iter().map(|f| f.code).collect()
}

#[test]
fn the_shipped_row_is_clean() { assert!(codes(row()).is_empty()); }
#[test]
fn not_a_whole_rupiah_is_refused_three_ways() {
    let mut r = row(); r.amount_mu += 1.0; assert_eq!(codes(r), ["price_amount_not_whole"]);
    let mut r = row(); r.amount_mu = 300_000_000_000.0; assert_eq!(codes(r), ["price_amount_not_whole"]);
    let mut r = row(); r.amount = 299000.5; r.amount_mu = 299_000_500_000.0; assert_eq!(codes(r), ["price_amount_not_whole"]);
}
#[test]
fn currency_rail_and_country_are_checked_against_the_pinned_rows() {
    let mut r = row(); r.currency = "USD".into(); assert_eq!(codes(r), ["price_currency_not_allowed"]);
    let mut r = row(); r.rails.push("paypal".into()); assert_eq!(codes(r), ["price_rail_not_allowed"]);
    let mut r = row(); r.country = "ZZ".into(); assert_eq!(codes(r), ["price_country_unknown"]);
}
