//! L7: the price rows (`packs/atingo/prices/<id>.json`, the shape the kernel's price book will hold, handover 1.13) and their rules.
//! `tools/packs/check.mjs` (`priceRules`) has the same codes; the shared fixtures prove the two agree. Pure data: nothing here prices, charges or decides.
use crate::pins::Pins;
use crate::validate::{Finding, Severity};
use crate::PathFile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const MU: f64 = 1_000_000.0;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tax {
    pub kind: String,
    pub rate_bp: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PriceRow {
    pub id: String,
    #[serde(rename = "_comment", default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub country: String,
    pub currency: String,
    pub amount: f64,
    pub amount_mu: f64,
    pub tax: Tax,
    pub rails: Vec<String>,
    pub period: String,
}

/// The rows of `<dir>/prices/*.json` keyed by file stem (a missing folder is an empty map: the layer is opt-in by folder).
pub fn load_prices(dir: &Path) -> Result<BTreeMap<String, PriceRow>, String> {
    let mut out = BTreeMap::new();
    let pdir = dir.join("prices");
    if !pdir.exists() {
        return Ok(out);
    }
    let mut files: Vec<_> = fs::read_dir(&pdir).map_err(|e| format!("{}: {e}", pdir.display()))?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    for f in files {
        let s = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let row: PriceRow = serde_json::from_str(s.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: {e}", f.display()))?;
        out.insert(f.file_stem().unwrap_or_default().to_string_lossy().to_string(), row);
    }
    Ok(out)
}

pub fn validate_price(paths: &[PathFile], prices: &BTreeMap<String, PriceRow>, pins: &Pins) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut push = |code: &'static str, path: String, message: String| out.push(Finding { severity: Severity::Error, code, path, message });
    for p in paths {
        if let Some(r) = &p.price_ref {
            if !prices.contains_key(r) {
                push("price_ref_unresolved", format!("paths/{}.json", p.id), format!("price_ref \"{r}\" has no row in prices/"));
            }
        }
    }
    for (id, r) in prices {
        let path = format!("prices/{id}.json");
        let Some(c) = pins.countries.get(&r.country) else {
            push("price_country_unknown", path, format!("price country \"{}\" has no country row in kernel-pins.json", r.country));
            continue;
        };
        if !c.currencies.contains(&r.currency) {
            push("price_currency_not_allowed", path.clone(), format!("price currency \"{}\" is not one the {} country rows allow", r.currency, r.country));
        }
        if r.amount_mu.fract() != 0.0 || r.amount_mu % MU != 0.0 || r.amount_mu != r.amount * MU {
            push("price_amount_not_whole", path.clone(), format!("amount_mu {} is not a whole {} unit in micro-units (amount {} x 1000000)", r.amount_mu, r.currency, r.amount));
        }
        for rail in &r.rails {
            if !c.rails.contains(rail) {
                push("price_rail_not_allowed", path.clone(), format!("price rail \"{rail}\" is not one the {} country rows allow", r.country));
            }
        }
    }
    out
}
