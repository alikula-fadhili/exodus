//! Canonical Migration Intent Domain Models, Intent Graph, and Evidence Adapters.

use crate::esg::{GroundingTier, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Extracted evidence from application code, tests, scripts, configuration, containers, Terraform, CI, or docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentEvidence {
    pub id: String,
    pub origin: String,
    pub canonical_path: PathBuf,
    pub span: Option<SourceSpan>,
    pub hash: String,
    pub redacted_excerpt: String,
    pub extractor: String,
    pub grounding_tier: GroundingTier,
}

impl IntentEvidence {
    pub fn new(
        origin: impl Into<String>,
        canonical_path: impl Into<PathBuf>,
        excerpt: &str,
        extractor: impl Into<String>,
        grounding_tier: GroundingTier,
    ) -> Self {
        let origin_str = origin.into();
        let path_buf = canonical_path.into();
        let redacted = SecretScrubber::scrub(excerpt);
        let hash = format!("{:x}", md5::compute(redacted.as_bytes()));
        let id = format!("ev-{}", uuid::Uuid::now_v7());

        Self {
            id,
            origin: origin_str,
            canonical_path: path_buf,
            span: None,
            hash,
            redacted_excerpt: redacted,
            extractor: extractor.into(),
            grounding_tier,
        }
    }
}

/// Scope level in the system architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntentScope {
    #[default]
    Repository,
    Workspace,
    Package,
    Service,
    Workflow,
    ClassFunction,
    ScriptUnit,
}

/// Architectural and semantic categories for intent claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IntentCategory {
    #[default]
    Behavior,
    Workflow,
    Interface,
    Data,
    Integration,
    Configuration,
    Concurrency,
    Security,
    Runtime,
    Deployment,
    Operations,
    NonFunctional,
}

/// Review status of an intent claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReviewStatus {
    #[default]
    Pending,
    AcceptedForPlanning,
    Rejected,
    Conflicted,
    Deferred,
}

/// Candidate alternative for a claim during human review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentClaimAlternative {
    pub id: String,
    pub description: String,
    pub consequences: String,
}

/// A structured claim representing inferred or extracted semantic meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentClaim {
    pub id: String,
    pub scope: IntentScope,
    pub scope_id: String,
    pub category: IntentCategory,
    pub structured_meaning: String,
    pub conditions: Vec<String>,
    pub effects: Vec<String>,
    pub dependencies: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub confidence: f32,
    pub conflicts: Vec<String>,
    pub review_status: ClaimReviewStatus,
    pub alternatives: Vec<IntentClaimAlternative>,
    pub is_critical: bool,
    pub grounding: GroundingTier,
}

impl IntentClaim {
    pub fn new(
        scope: IntentScope,
        scope_id: impl Into<String>,
        category: IntentCategory,
        meaning: impl Into<String>,
        grounding: GroundingTier,
    ) -> Self {
        Self {
            id: format!("claim-{}", uuid::Uuid::now_v7()),
            scope,
            scope_id: scope_id.into(),
            category,
            structured_meaning: meaning.into(),
            conditions: Vec::new(),
            effects: Vec::new(),
            dependencies: Vec::new(),
            evidence_ids: Vec::new(),
            confidence: if grounding.is_grounded() { 1.0 } else { 0.75 },
            conflicts: Vec::new(),
            review_status: ClaimReviewStatus::Pending,
            alternatives: Vec::new(),
            is_critical: false,
            grounding,
        }
    }

    pub fn compute_hash(&self) -> String {
        let raw = format!(
            "{:?}:{}:{}:{}:{:?}:{:?}",
            self.scope, self.scope_id, self.category, self.structured_meaning, self.conditions, self.effects
        );
        format!("{:x}", md5::compute(raw.as_bytes()))
    }
}

/// Edge connection in the Intent Graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentEdgeKind {
    DerivesFrom,
    ConflictsWith,
    Implements,
    Constrains,
    DependsOn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentEdge {
    pub from: String,
    pub to: String,
    pub kind: IntentEdgeKind,
}

/// Canonical Migration Intent Graph linking claims, evidence, and scopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MigrationIntentGraph {
    pub claims: BTreeMap<String, IntentClaim>,
    pub evidence: BTreeMap<String, IntentEvidence>,
    pub edges: Vec<IntentEdge>,
}

impl MigrationIntentGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_evidence(&mut self, evidence: IntentEvidence) -> String {
        let id = evidence.id.clone();
        self.evidence.insert(id.clone(), evidence);
        id
    }

    pub fn add_claim(&mut self, claim: IntentClaim) -> String {
        let id = claim.id.clone();
        self.claims.insert(id.clone(), claim);
        id
    }

    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>, kind: IntentEdgeKind) {
        self.edges.push(IntentEdge {
            from: from.into(),
            to: to.into(),
            kind,
        });
    }

    pub fn get_slice_for_unit(&self, unit_id: &str) -> IntentSlice {
        let mut claims = Vec::new();
        let mut evidence = Vec::new();
        let mut seen_ev = BTreeSet::new();

        for claim in self.claims.values() {
            if claim.scope_id == unit_id
                || claim.scope == IntentScope::Repository
                || claim.scope == IntentScope::Workspace
            {
                claims.push(claim.clone());
                for ev_id in &claim.evidence_ids {
                    if seen_ev.insert(ev_id.clone()) {
                        if let Some(ev) = self.evidence.get(ev_id) {
                            evidence.push(ev.clone());
                        }
                    }
                }
            }
        }

        IntentSlice {
            unit_id: unit_id.to_string(),
            claims,
            evidence,
            invariants: Vec::new(),
        }
    }
}

/// Minimal relevant context slice supplied to a planner, emitter, verifier, or LLM agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IntentSlice {
    pub unit_id: String,
    pub claims: Vec<IntentClaim>,
    pub evidence: Vec<IntentEvidence>,
    pub invariants: Vec<String>,
}

/// Human approval decision record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentApprovalDecision {
    pub approver: String,
    pub timestamp: i64,
    pub claim_id: String,
    pub claim_hash: String,
    pub evidence_hashes: Vec<String>,
    pub decision: ClaimReviewStatus,
    pub chosen_alternative: Option<String>,
    pub rationale: Option<String>,
    pub contract_version: String,
}

/// Structured request presented to human during terminal review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentApprovalRequest {
    pub request_id: String,
    pub claim: IntentClaim,
    pub evidence: Vec<IntentEvidence>,
    pub alternatives: Vec<IntentClaimAlternative>,
    pub migration_impact: String,
    pub recommended_decision: ClaimReviewStatus,
    pub rationale: String,
    pub affected_units: Vec<String>,
}

/// The executable contract governing repository migration intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationIntentContract {
    pub contract_id: String,
    pub schema_version: String,
    pub esg_snapshot_id: String,
    pub repository_profile_id: String,
    pub goals: Vec<String>,
    pub invariants: Vec<String>,
    pub constraints: Vec<String>,
    pub accepted_changes: Vec<String>,
    pub claims: Vec<IntentClaim>,
    pub critical_gaps: Vec<String>,
    pub approvals: Vec<IntentApprovalDecision>,
    pub created_at: i64,
    pub composite_hash: String,
}

impl MigrationIntentContract {
    pub fn new(
        esg_snapshot_id: impl Into<String>,
        repository_profile_id: impl Into<String>,
        claims: Vec<IntentClaim>,
    ) -> Self {
        let contract_id = format!("intent-contract-{}", uuid::Uuid::now_v7());
        let mut contract = Self {
            contract_id,
            schema_version: "1.0.0".to_string(),
            esg_snapshot_id: esg_snapshot_id.into(),
            repository_profile_id: repository_profile_id.into(),
            goals: Vec::new(),
            invariants: Vec::new(),
            constraints: Vec::new(),
            accepted_changes: Vec::new(),
            claims,
            critical_gaps: Vec::new(),
            approvals: Vec::new(),
            created_at: chrono::Utc::now().timestamp(),
            composite_hash: String::new(),
        };
        contract.composite_hash = contract.compute_hash();
        contract
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher_str = format!(
            "{}:{}:{}:{:?}",
            self.esg_snapshot_id, self.repository_profile_id, self.claims.len(), self.critical_gaps
        );
        for claim in &self.claims {
            hasher_str.push_str(&claim.compute_hash());
        }
        format!("{:x}", md5::compute(hasher_str.as_bytes()))
    }

    pub fn has_pending_critical_claims(&self) -> bool {
        self.claims
            .iter()
            .any(|c| c.is_critical && c.review_status == ClaimReviewStatus::Pending)
    }
}

/// Traceability manifest mapping every generated target artifact to the intent claims it satisfies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TraceabilityManifest {
    pub schema_version: String,
    pub artifact_to_claims: BTreeMap<String, Vec<String>>,
    pub claim_to_artifacts: BTreeMap<String, Vec<String>>,
}

impl TraceabilityManifest {
    pub fn new() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            artifact_to_claims: BTreeMap::new(),
            claim_to_artifacts: BTreeMap::new(),
        }
    }

    pub fn record_mapping(&mut self, artifact_path: impl Into<String>, claim_id: impl Into<String>) {
        let art = artifact_path.into();
        let clm = claim_id.into();
        self.artifact_to_claims
            .entry(art.clone())
            .or_default()
            .push(clm.clone());
        self.claim_to_artifacts
            .entry(clm)
            .or_default()
            .push(art);
    }
}

/// Secret and credential scrubber preventing leakage into storage, prompts, or terminal review.
pub struct SecretScrubber;

impl SecretScrubber {
    pub fn scrub(input: &str) -> String {
        let mut result = input.to_string();
        let patterns = [
            (r#"(?i)(password|passwd|pwd|secret|api_key|apikey|token|auth_token|bearer)\s*[:=]\s*['"][^'"]+['"]"#, "$1 = \"[REDACTED_SECRET]\""),
            (r#"(?i)bearer\s+[a-zA-Z0-9_\-\.]{15,}"#, "Bearer [REDACTED_TOKEN]"),
            (r#"-----BEGIN\s+([A-Z\s]+)-----[A-Za-z0-9+/=\s]+-----END\s+\1-----"#, "[REDACTED_PRIVATE_KEY]"),
            (r#"(?i)(AWS_SECRET_ACCESS_KEY|GITHUB_TOKEN|OPENAI_API_KEY)\s*=\s*[^\s]+"#, "$1=[REDACTED_ENV_SECRET]"),
        ];

        for (pat, rep) in patterns {
            if let Ok(re) = regex::Regex::new(pat) {
                result = re.replace_all(&result, rep).to_string();
            }
        }
        result
    }
}
