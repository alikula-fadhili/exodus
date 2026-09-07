//! Intent review synthesizer: bounded inference of provisional claims and construction of interactive human approval requests.

use exodus_core::{
    ClaimReviewStatus, IntentApprovalRequest, IntentClaim, IntentClaimAlternative,
    MigrationIntentGraph,
};

pub struct IntentReviewSynthesizer {
    pub max_repair_iterations: usize,
}

impl Default for IntentReviewSynthesizer {
    fn default() -> Self {
        Self {
            max_repair_iterations: 3,
        }
    }
}

impl IntentReviewSynthesizer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Synthesizes structured terminal approval requests for all pending critical or conflicted claims.
    pub fn generate_approval_requests(
        &self,
        graph: &MigrationIntentGraph,
    ) -> Vec<IntentApprovalRequest> {
        let mut requests = Vec::new();

        for claim in graph.claims.values() {
            if claim.is_critical || claim.review_status == ClaimReviewStatus::Conflicted {
                let evidence: Vec<_> = claim
                    .evidence_ids
                    .iter()
                    .filter_map(|eid| graph.evidence.get(eid).cloned())
                    .collect();

                let mut alternatives = claim.alternatives.clone();
                if alternatives.is_empty() {
                    alternatives.push(IntentClaimAlternative {
                        id: "alt-default-pass".to_string(),
                        description: "Accept conservative default mapping without external exposure".to_string(),
                        consequences: "Limits public access; protects against ungrounded assumptions".to_string(),
                    });
                }

                let recommendation = if claim.grounding.is_grounded() {
                    ClaimReviewStatus::AcceptedForPlanning
                } else {
                    ClaimReviewStatus::Deferred
                };

                let rationale = if claim.grounding.is_grounded() {
                    "Claim is directly supported by deterministic evidence.".to_string()
                } else {
                    "Claim is provisional/inferred. Human verification recommended before planning.".to_string()
                };

                let impact = format!(
                    "Scope: {:?} ({}), Category: {:?}. Affects target generation and behavioral verification constraints.",
                    claim.scope, claim.scope_id, claim.category
                );

                requests.push(IntentApprovalRequest {
                    request_id: format!("req-{}", claim.id),
                    claim: claim.clone(),
                    evidence,
                    alternatives,
                    migration_impact: impact,
                    recommended_decision: recommendation,
                    rationale,
                    affected_units: vec![claim.scope_id.clone()],
                });
            }
        }

        requests
    }
}
