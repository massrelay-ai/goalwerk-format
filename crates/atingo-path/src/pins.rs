//! The kernel's pinned vocabularies (`tools/packs/kernel-pins.json`): block kinds and people kinds the packs may name.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Pins {
    pub block_kinds: BTreeSet<String>,
    pub people_kinds: BTreeSet<String>,
    #[serde(default)]
    pub nudge_channels: BTreeSet<String>,
    #[serde(default)]
    pub verifier_kinds: BTreeSet<String>,
    /// L13: the locales the locale layer may name.
    #[serde(default)]
    pub locales: BTreeSet<String>,
    /// L7: the country layer as pinned from the kernel's `countries/<ISO>.json` (currencies are stated here, rails and receipt fields are the kernel's).
    #[serde(default)]
    pub countries: BTreeMap<String, CountryPin>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct CountryPin {
    pub currencies: BTreeSet<String>,
    pub rails: BTreeSet<String>,
    #[serde(default)]
    pub receipt_fields: Vec<String>,
    #[serde(default)]
    pub confirmed: bool,
}
