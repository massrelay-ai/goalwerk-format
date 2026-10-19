//! L1: types. L3 adds `validate()` and the WASM target.
//!
//! These mirror `packs/atingo/SCHEMA.md`. A pack is data; nothing here decides or stores anything (AGENTS.md section 1).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod graph;
pub mod i18n;
pub mod loop_data;
pub mod pins;
pub mod price;
pub mod review;
pub mod validate;
pub mod validate_graph;
pub mod validate_loop;
#[cfg(feature = "wasm")]
pub mod wasm;
pub use validate::{validate, Finding, Severity};
pub use price::{load_prices, validate_price};
pub use validate_graph::{claim_verbs, validate_graph};
pub use validate_loop::{forbidden_instruction, rrule_ok, validate_loop};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GoalKinds {
    pub version: u32,
    pub family: String,
    pub kinds: Vec<GoalKind>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct GoalKind {
    pub id: String,
    pub title: String,
    pub evidence_kind: EvidenceKind,
    pub verifier: Verifier,
    pub window_days: u32,
    pub attestation_fallback: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    WalletBalance,
    Receipt,
    Document,
    EntityRow,
    ConnectorValue,
    Attestation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verifier {
    KeyResult,
    EvalSuite,
    Script,
    ProfessionalSignoff,
    Attestation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Platform,
    Creator,
    #[serde(rename = "self")]
    SelfTier,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PathFile {
    pub id: String,
    #[serde(rename = "_comment", default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub tier: Tier,
    pub title: String,
    pub locale_default: String,
    #[serde(default)]
    pub persona: Option<String>,
    pub root: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub people_kinds_by_node: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub price_ref: Option<String>,
    #[serde(default)]
    pub certificate: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub goal_kind: String,
    pub lesson_template: String,
    pub blocks_allowed: Vec<String>,
    pub evidence: String,
    pub receipt_kind: String,
    pub sources: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Prerequisite,
    Branch,
}
