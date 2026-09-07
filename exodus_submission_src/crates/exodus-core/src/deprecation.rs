//! First-class deprecation lifecycle management and evidence tracking.

use crate::esg::{GroundingTier, LanguageId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Deprecation lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeprecationStatus {
    Active,
    Deprecated,
    Removed,
    Insecure,
}

/// Source of evidence backing a deprecation finding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeprecationEvidenceSource {
    CompilerWarning,
    LinterDiagnostic,
    ManifestOrAdvisory,
    VerifiedMigrationCase,
    LlmCandidate,
    UserRule,
}

/// First-class deprecation record across languages and package ecosystems.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeprecationRecord {
    pub id: String,
    pub language: LanguageId,
    pub target_symbol: String,
    pub affected_version_range: Option<String>,
    pub status: DeprecationStatus,
    pub replacement_recommendation: Option<String>,
    pub evidence_source: DeprecationEvidenceSource,
    pub detected_at: DateTime<Utc>,
    pub confidence: f32,
    pub grounding: GroundingTier,
    pub affected_esg_node_ids: Vec<String>,
}

impl DeprecationRecord {
    pub fn new(
        id: impl Into<String>,
        language: LanguageId,
        target_symbol: impl Into<String>,
        status: DeprecationStatus,
        evidence_source: DeprecationEvidenceSource,
    ) -> Self {
        let grounding = match evidence_source {
            DeprecationEvidenceSource::LlmCandidate => GroundingTier::Provisional,
            _ => GroundingTier::Deterministic,
        };

        Self {
            id: id.into(),
            language,
            target_symbol: target_symbol.into(),
            affected_version_range: None,
            status,
            replacement_recommendation: None,
            evidence_source,
            detected_at: Utc::now(),
            confidence: if grounding.is_grounded() { 1.0 } else { 0.6 },
            grounding,
            affected_esg_node_ids: Vec::new(),
        }
    }
}
