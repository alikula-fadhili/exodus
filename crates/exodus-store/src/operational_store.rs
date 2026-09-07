//! Operational Storage layer for the Enterprise Agentic Execution Fabric.
//!
//! Provides relational and graph storage for:
//! - `OperationalItem`: multi-domain event records and 6-stage lifecycle audit logs.
//! - CRM Accounts & Policies: customer ARR, contract tiers, and discount compliance governance.
//! - Product Taxonomy & Feedback: canonical product feature trees and survey sentiment classification.
//! - SDLC Pipeline & Remote Repo: CI/CD triggers, remote repo configs, and webhook event conversion.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use exodus_core::{
    CiFailureEventPayload, CrmRequestPayload, DomainPayload, OperationalDomainTag, OperationalItem,
    OperationalLifecycleState, ProdBugPayload, Result, SdlcIntegrationSettings,
    SurveyMappingPayload, SurveyResponseItem,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Record representing a customer account in the CRM policy graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrmAccountRecord {
    pub account_id: String,
    pub name: String,
    pub arr: f64,
    pub tier: String,
    pub max_allowed_discount_pct: f64,
    pub sla_level: String,
    pub renewal_date: Option<DateTime<Utc>>,
}

impl CrmAccountRecord {
    pub fn new(
        account_id: impl Into<String>,
        name: impl Into<String>,
        arr: f64,
        tier: impl Into<String>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            name: name.into(),
            arr,
            tier: tier.into(),
            max_allowed_discount_pct: 15.0,
            sla_level: "Standard".to_string(),
            renewal_date: None,
        }
    }
}

/// Declarative business rule for commercial governance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrmPolicyRule {
    pub id: String,
    pub name: String,
    pub max_discount_pct: f64,
    pub required_approval_role: String,
    pub min_arr_threshold: f64,
    pub description: String,
}

/// Evaluation result for a CRM contract mutation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrmEvaluationReport {
    pub account_id: String,
    pub compliant: bool,
    pub requested_discount_pct: f64,
    pub authorized_max_discount_pct: f64,
    pub required_role: String,
    pub requester_role: String,
    pub reason: String,
}

/// Node in the canonical product taxonomy tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyNodeRecord {
    pub id: String,
    pub name: String,
    pub category: String,
    pub parent_id: Option<String>,
    pub keywords: Vec<String>,
    pub description: String,
}

impl TaxonomyNodeRecord {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
        keywords: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: category.into(),
            parent_id: None,
            keywords,
            description: String::new(),
        }
    }
}

/// Customer feedback mapping record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurveyFeedbackRecord {
    pub id: String,
    pub feedback_text: String,
    pub mapped_taxonomy_id: Option<String>,
    pub match_confidence: f64,
    pub verified: bool,
}

/// Storage trait for cross-functional operational items and domain knowledge graphs.
#[async_trait]
pub trait OperationalStore: Send + Sync {
    // --- Operational Items Lifecycle ---
    async fn save_operational_item(&mut self, item: &OperationalItem) -> Result<()>;
    async fn get_operational_item(&self, id: &str) -> Result<Option<OperationalItem>>;
    async fn list_operational_items(&self) -> Result<Vec<OperationalItem>>;
    async fn list_operational_items_by_domain(
        &self,
        domain: OperationalDomainTag,
    ) -> Result<Vec<OperationalItem>>;
    async fn list_operational_items_by_state(
        &self,
        state: OperationalLifecycleState,
    ) -> Result<Vec<OperationalItem>>;

    // --- Commercial Operations (CRM) ---
    async fn save_crm_account(&mut self, account: &CrmAccountRecord) -> Result<()>;
    async fn get_crm_account(&self, account_id: &str) -> Result<Option<CrmAccountRecord>>;
    async fn list_crm_accounts(&self) -> Result<Vec<CrmAccountRecord>>;
    async fn save_crm_policy(&mut self, policy: &CrmPolicyRule) -> Result<()>;
    async fn list_crm_policies(&self) -> Result<Vec<CrmPolicyRule>>;
    async fn evaluate_crm_discount(
        &self,
        account_id: &str,
        requested_discount_pct: f64,
        requester_role: &str,
    ) -> Result<CrmEvaluationReport>;

    // --- Product Operations (Taxonomy & Survey) ---
    async fn save_taxonomy_node(&mut self, node: &TaxonomyNodeRecord) -> Result<()>;
    async fn get_taxonomy_node(&self, id: &str) -> Result<Option<TaxonomyNodeRecord>>;
    async fn list_taxonomy_nodes(&self) -> Result<Vec<TaxonomyNodeRecord>>;
    async fn save_survey_feedback(&mut self, feedback: &SurveyFeedbackRecord) -> Result<()>;
    async fn list_survey_feedbacks(&self) -> Result<Vec<SurveyFeedbackRecord>>;
    async fn match_feedback_to_taxonomy(
        &self,
        feedback_text: &str,
    ) -> Result<Option<(TaxonomyNodeRecord, f64)>>;

    // --- SDLC Pipeline & Remote Repo Integration ---
    async fn get_sdlc_settings(&self) -> Result<SdlcIntegrationSettings>;
    async fn save_sdlc_settings(&mut self, settings: &SdlcIntegrationSettings) -> Result<()>;
    async fn generate_pipeline_plugin_scaffold(&self) -> Result<HashMap<String, String>>;
    async fn ingest_ci_failure_event(
        &mut self,
        event: CiFailureEventPayload,
    ) -> Result<OperationalItem>;
}

/// Serializable snapshot representing the entire persistent state of the operational store.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoreSnapshot {
    items: HashMap<String, OperationalItem>,
    accounts: HashMap<String, CrmAccountRecord>,
    policies: HashMap<String, CrmPolicyRule>,
    taxonomy: HashMap<String, TaxonomyNodeRecord>,
    feedbacks: HashMap<String, SurveyFeedbackRecord>,
    sdlc_settings: SdlcIntegrationSettings,
}

/// Thread-safe in-memory and embedded Surreal-backed implementation of `OperationalStore`.
#[derive(Clone)]
pub struct EmbeddedOperationalStore {
    items: Arc<RwLock<HashMap<String, OperationalItem>>>,
    accounts: Arc<RwLock<HashMap<String, CrmAccountRecord>>>,
    policies: Arc<RwLock<HashMap<String, CrmPolicyRule>>>,
    taxonomy: Arc<RwLock<HashMap<String, TaxonomyNodeRecord>>>,
    feedbacks: Arc<RwLock<HashMap<String, SurveyFeedbackRecord>>>,
    sdlc_settings: Arc<RwLock<SdlcIntegrationSettings>>,
}

impl Default for EmbeddedOperationalStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedOperationalStore {
    pub fn new() -> Self {
        let mut policies = HashMap::new();
        policies.insert(
            "policy-tier1-exec".to_string(),
            CrmPolicyRule {
                id: "policy-tier1-exec".to_string(),
                name: "Standard Sales Executive Threshold".to_string(),
                max_discount_pct: 15.0,
                required_approval_role: "AccountExec".to_string(),
                min_arr_threshold: 0.0,
                description: "Sales Executives can authorize up to 15% discount".to_string(),
            },
        );
        policies.insert(
            "policy-tier2-lead".to_string(),
            CrmPolicyRule {
                id: "policy-tier2-lead".to_string(),
                name: "Sales Lead Approval Threshold".to_string(),
                max_discount_pct: 25.0,
                required_approval_role: "SalesLead".to_string(),
                min_arr_threshold: 50_000.0,
                description: "Sales Leads can authorize up to 25% discount for ARR >= $50k"
                    .to_string(),
            },
        );
        policies.insert(
            "policy-tier3-vp".to_string(),
            CrmPolicyRule {
                id: "policy-tier3-vp".to_string(),
                name: "VP Commercial Operations Threshold".to_string(),
                max_discount_pct: 40.0,
                required_approval_role: "VP".to_string(),
                min_arr_threshold: 100_000.0,
                description:
                    "VP approval required for discounts up to 40% and enterprise contracts"
                        .to_string(),
            },
        );

        let mut taxonomy = HashMap::new();
        taxonomy.insert(
            "tax-perf-latency".to_string(),
            TaxonomyNodeRecord::new(
                "tax-perf-latency",
                "Performance & Latency",
                "Infrastructure",
                vec![
                    "slow".to_string(),
                    "latency".to_string(),
                    "timeout".to_string(),
                    "lag".to_string(),
                    "delay".to_string(),
                    "hang".to_string(),
                ],
            ),
        );
        taxonomy.insert(
            "tax-auth-login".to_string(),
            TaxonomyNodeRecord::new(
                "tax-auth-login",
                "Authentication & Single Sign-On",
                "Security",
                vec![
                    "login".to_string(),
                    "sso".to_string(),
                    "oauth".to_string(),
                    "password".to_string(),
                    "mfa".to_string(),
                    "saml".to_string(),
                    "auth".to_string(),
                ],
            ),
        );
        taxonomy.insert(
            "tax-bill-pricing".to_string(),
            TaxonomyNodeRecord::new(
                "tax-bill-pricing",
                "Billing & Invoice Clarity",
                "Commercial",
                vec![
                    "invoice".to_string(),
                    "billing".to_string(),
                    "charge".to_string(),
                    "expensive".to_string(),
                    "receipt".to_string(),
                    "price".to_string(),
                ],
            ),
        );

        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(policies)),
            taxonomy: Arc::new(RwLock::new(taxonomy)),
            feedbacks: Arc::new(RwLock::new(HashMap::new())),
            sdlc_settings: Arc::new(RwLock::new(SdlcIntegrationSettings::default())),
        }
    }

    /// Persists the current operational store snapshot to disk.
    pub async fn persist_to_disk(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let snapshot = StoreSnapshot {
            items: self.items.read().await.clone(),
            accounts: self.accounts.read().await.clone(),
            policies: self.policies.read().await.clone(),
            taxonomy: self.taxonomy.read().await.clone(),
            feedbacks: self.feedbacks.read().await.clone(),
            sdlc_settings: self.sdlc_settings.read().await.clone(),
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(path, json).map_err(exodus_core::ExodusError::Io)?;
        Ok(())
    }

    /// Loads the store snapshot from disk, or seeds demo fabric data and initializes if missing.
    pub async fn load_or_init(path: &std::path::Path) -> Result<Self> {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(snapshot) = serde_json::from_str::<StoreSnapshot>(&content) {
                    return Ok(Self {
                        items: Arc::new(RwLock::new(snapshot.items)),
                        accounts: Arc::new(RwLock::new(snapshot.accounts)),
                        policies: Arc::new(RwLock::new(snapshot.policies)),
                        taxonomy: Arc::new(RwLock::new(snapshot.taxonomy)),
                        feedbacks: Arc::new(RwLock::new(snapshot.feedbacks)),
                        sdlc_settings: Arc::new(RwLock::new(snapshot.sdlc_settings)),
                    });
                }
            }
        }
        let mut store = Self::new();
        store.seed_demo_fabric().await?;
        let _ = store.persist_to_disk(path).await;
        Ok(store)
    }

    /// Seeds realistic multi-domain enterprise fabric demonstration data if empty.
    pub async fn seed_demo_fabric(&mut self) -> Result<()> {
        let mut accounts_map = self.accounts.write().await;
        accounts_map.insert(
            "acc-enterprise-042".to_string(),
            CrmAccountRecord {
                account_id: "acc-enterprise-042".to_string(),
                name: "Globex Financial Cloud".to_string(),
                arr: 240_000.0,
                tier: "Enterprise".to_string(),
                max_allowed_discount_pct: 25.0,
                sla_level: "Enterprise-Mission-Critical".to_string(),
                renewal_date: None,
            },
        );
        drop(accounts_map);

        let mut items_map = self.items.write().await;
        if items_map.is_empty() {
            // 1. Engineering bug
            let bug_payload = ProdBugPayload {
                commit_id: "8fa24b91e0a4".to_string(),
                error_message: "SessionTimeout: unhandled token refresh failure in AuthHandler::verify_token".to_string(),
                stack_trace: Some("thread 'tokio-runtime-worker' panicked at 'called `Option::unwrap()` on a `None` value': crates/auth-service/src/token.rs:88:14".to_string()),
                target_file: Some(std::path::PathBuf::from("crates/auth-service/src/token.rs")),
                target_symbol: Some("AuthHandler::verify_token".to_string()),
                reproduction_command: Some("cargo test --package auth-service -- test_token_refresh".to_string()),
            };
            let mut bug_item = OperationalItem::new(
                "SessionTimeout in AuthHandler::verify_token",
                "Production crash report: unhandled token refresh failure during token validation in AuthHandler.",
                "ci-pipeline",
                DomainPayload::ProdBug(bug_payload),
            );
            bug_item.id = "op-prod-bug-001".to_string();
            let _ = bug_item.mark_sandboxed("exodus-engine", ".exodus/worktrees/op-prod-bug-001");
            items_map.insert(bug_item.id.clone(), bug_item);

            // 2. Commercial request
            let mut crm_payload = CrmRequestPayload::new(
                "acc-enterprise-042",
                "Globex Financial Cloud",
                240_000.0,
                22.5,
                "Enterprise",
                "SalesLead",
            );
            crm_payload.contract_term_months = 24;
            crm_payload.justification = "Multi-year renewal commitment with 200 additional user seats. Discount authorized under Sales Lead governance band.".to_string();
            let mut crm_item = OperationalItem::new(
                "Globex Financial Cloud — 22.5% Enterprise Multi-Year Discount",
                "Commercial discount authorization request for multi-year enterprise renewal",
                "sarah.chen@sales.internal",
                DomainPayload::CrmRequest(crm_payload),
            );
            crm_item.id = "op-crm-req-002".to_string();
            let _ = crm_item.mark_sandboxed("crm-sync-worker", "shadow-tx-globex-2026");
            items_map.insert(crm_item.id.clone(), crm_item);

            // 3. Product survey mapping
            let responses = vec![
                SurveyResponseItem {
                    id: "resp-q3-001".to_string(),
                    feedback_text: "The login page with SSO times out intermittently on Chrome"
                        .to_string(),
                    customer_segment: Some("Enterprise".to_string()),
                    candidate_taxonomy_node: Some("tax-auth-login".to_string()),
                },
                SurveyResponseItem {
                    id: "resp-q3-002".to_string(),
                    feedback_text:
                        "Slow dashboard query latency during peak Monday morning reports"
                            .to_string(),
                    customer_segment: Some("Enterprise".to_string()),
                    candidate_taxonomy_node: Some("tax-perf-latency".to_string()),
                },
                SurveyResponseItem {
                    id: "resp-q3-003".to_string(),
                    feedback_text:
                        "Invoice itemization is hard to reconcile with our accounting department"
                            .to_string(),
                    customer_segment: Some("Mid-Market".to_string()),
                    candidate_taxonomy_node: Some("tax-bill-pricing".to_string()),
                },
            ];
            let mut survey_payload = SurveyMappingPayload::new(
                "batch-q3-nps-enterprise",
                "Qualtrics-Enterprise-NPS",
                "root-product-taxonomy",
            );
            survey_payload.responses = responses;
            let mut survey_item = OperationalItem::new(
                "Q3 Enterprise Customer CSAT/NPS Feedback Ontology Alignment",
                "Customer sentiment batch from Qualtrics awaiting taxonomy classification and sentiment attribution",
                "product-ops-bot",
                DomainPayload::SurveyMapping(survey_payload),
            );
            survey_item.id = "op-survey-003".to_string();
            let _ = survey_item.mark_sandboxed("survey-pipeline", "staging-partition-q3-nps");
            items_map.insert(survey_item.id.clone(), survey_item);
        }

        Ok(())
    }
}

#[async_trait]
impl OperationalStore for EmbeddedOperationalStore {
    async fn save_operational_item(&mut self, item: &OperationalItem) -> Result<()> {
        let mut map = self.items.write().await;
        map.insert(item.id.clone(), item.clone());
        Ok(())
    }

    async fn get_operational_item(&self, id: &str) -> Result<Option<OperationalItem>> {
        let map = self.items.read().await;
        Ok(map.get(id).cloned())
    }

    async fn list_operational_items(&self) -> Result<Vec<OperationalItem>> {
        let map = self.items.read().await;
        let mut list: Vec<OperationalItem> = map.values().cloned().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(list)
    }

    async fn list_operational_items_by_domain(
        &self,
        domain: OperationalDomainTag,
    ) -> Result<Vec<OperationalItem>> {
        let map = self.items.read().await;
        let mut list: Vec<OperationalItem> = map
            .values()
            .filter(|i| i.domain_tag == domain)
            .cloned()
            .collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(list)
    }

    async fn list_operational_items_by_state(
        &self,
        state: OperationalLifecycleState,
    ) -> Result<Vec<OperationalItem>> {
        let map = self.items.read().await;
        let mut list: Vec<OperationalItem> =
            map.values().filter(|i| i.state == state).cloned().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(list)
    }

    async fn save_crm_account(&mut self, account: &CrmAccountRecord) -> Result<()> {
        let mut map = self.accounts.write().await;
        map.insert(account.account_id.clone(), account.clone());
        Ok(())
    }

    async fn get_crm_account(&self, account_id: &str) -> Result<Option<CrmAccountRecord>> {
        let map = self.accounts.read().await;
        Ok(map.get(account_id).cloned())
    }

    async fn list_crm_accounts(&self) -> Result<Vec<CrmAccountRecord>> {
        let map = self.accounts.read().await;
        Ok(map.values().cloned().collect())
    }

    async fn save_crm_policy(&mut self, policy: &CrmPolicyRule) -> Result<()> {
        let mut map = self.policies.write().await;
        map.insert(policy.id.clone(), policy.clone());
        Ok(())
    }

    async fn list_crm_policies(&self) -> Result<Vec<CrmPolicyRule>> {
        let map = self.policies.read().await;
        Ok(map.values().cloned().collect())
    }

    async fn evaluate_crm_discount(
        &self,
        account_id: &str,
        requested_discount_pct: f64,
        requester_role: &str,
    ) -> Result<CrmEvaluationReport> {
        let accounts = self.accounts.read().await;
        let _policies = self.policies.read().await;

        let account = accounts.get(account_id);
        let _arr = account.map(|a| a.arr).unwrap_or(0.0);

        // Find policy threshold for this requester's role
        let role_tier_rank = match requester_role {
            "CEO" | "Executive" => 4,
            "VP" | "VicePresident" => 3,
            "SalesLead" | "Lead" | "Manager" => 2,
            _ => 1, // Sales Rep / Account Exec
        };

        // Determine maximum discount permitted for the role
        let (authorized_max, required_role) = if requested_discount_pct <= 15.0 {
            (15.0, "AccountExec")
        } else if requested_discount_pct <= 25.0 {
            (25.0, "SalesLead")
        } else if requested_discount_pct <= 40.0 {
            (40.0, "VP")
        } else {
            (100.0, "CEO")
        };

        let required_rank = match required_role {
            "CEO" => 4,
            "VP" => 3,
            "SalesLead" => 2,
            _ => 1,
        };

        let role_sufficient = role_tier_rank >= required_rank;
        let compliant = role_sufficient;

        let reason = if compliant {
            format!(
                "Discount {:.1}% is authorized for role '{}' (threshold requires >= '{}')",
                requested_discount_pct, requester_role, required_role
            )
        } else {
            format!(
                "Discount {:.1}% exceeds authorized limit for role '{}'. Requires '{}' authorization.",
                requested_discount_pct, requester_role, required_role
            )
        };

        Ok(CrmEvaluationReport {
            account_id: account_id.to_string(),
            compliant,
            requested_discount_pct,
            authorized_max_discount_pct: authorized_max,
            required_role: required_role.to_string(),
            requester_role: requester_role.to_string(),
            reason,
        })
    }

    async fn save_taxonomy_node(&mut self, node: &TaxonomyNodeRecord) -> Result<()> {
        let mut map = self.taxonomy.write().await;
        map.insert(node.id.clone(), node.clone());
        Ok(())
    }

    async fn get_taxonomy_node(&self, id: &str) -> Result<Option<TaxonomyNodeRecord>> {
        let map = self.taxonomy.read().await;
        Ok(map.get(id).cloned())
    }

    async fn list_taxonomy_nodes(&self) -> Result<Vec<TaxonomyNodeRecord>> {
        let map = self.taxonomy.read().await;
        Ok(map.values().cloned().collect())
    }

    async fn save_survey_feedback(&mut self, feedback: &SurveyFeedbackRecord) -> Result<()> {
        let mut map = self.feedbacks.write().await;
        map.insert(feedback.id.clone(), feedback.clone());
        Ok(())
    }

    async fn list_survey_feedbacks(&self) -> Result<Vec<SurveyFeedbackRecord>> {
        let map = self.feedbacks.read().await;
        Ok(map.values().cloned().collect())
    }

    async fn match_feedback_to_taxonomy(
        &self,
        feedback_text: &str,
    ) -> Result<Option<(TaxonomyNodeRecord, f64)>> {
        let map = self.taxonomy.read().await;
        let lower = feedback_text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        let mut best_match: Option<(TaxonomyNodeRecord, f64)> = None;
        let mut highest_score = 0.0;

        for node in map.values() {
            let mut matches = 0;
            for kw in &node.keywords {
                if lower.contains(&kw.to_lowercase()) {
                    matches += 2; // direct phrase/keyword match
                }
            }
            for word in &words {
                if node.name.to_lowercase().contains(word) {
                    matches += 1;
                }
            }

            if matches > 0 {
                let score = (matches as f64) / ((node.keywords.len() + 2) as f64);
                let normalized_score = (score * 1.5).min(1.0);
                if normalized_score > highest_score {
                    highest_score = normalized_score;
                    best_match = Some((node.clone(), normalized_score));
                }
            }
        }

        Ok(best_match)
    }

    async fn get_sdlc_settings(&self) -> Result<SdlcIntegrationSettings> {
        let settings = self.sdlc_settings.read().await;
        Ok(settings.clone())
    }

    async fn save_sdlc_settings(&mut self, settings: &SdlcIntegrationSettings) -> Result<()> {
        let mut map = self.sdlc_settings.write().await;
        *map = settings.clone();
        Ok(())
    }

    async fn generate_pipeline_plugin_scaffold(&self) -> Result<HashMap<String, String>> {
        let settings = self.sdlc_settings.read().await;
        Ok(settings.generate_ci_scaffold())
    }

    async fn ingest_ci_failure_event(
        &mut self,
        event: CiFailureEventPayload,
    ) -> Result<OperationalItem> {
        let item = event.into_operational_item("sdlc-ci-webhook");
        self.save_operational_item(&item).await?;
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_core::{DomainPayload, ProdBugPayload, RemoteRepoProvider};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_operational_store_crud_and_query() {
        let mut store = EmbeddedOperationalStore::new();

        let payload = DomainPayload::ProdBug(ProdBugPayload::new(
            "commit-123",
            "IndexOutOfBoundsException",
            Some(PathBuf::from("src/index.rs")),
        ));
        let item = OperationalItem::new("Bug in Index", "CI failure", "ci", payload);
        let id = item.id.clone();

        assert!(store.save_operational_item(&item).await.is_ok());

        let retrieved = store.get_operational_item(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Bug in Index");

        let items_by_domain = store
            .list_operational_items_by_domain(OperationalDomainTag::ProdBug)
            .await
            .unwrap();
        assert_eq!(items_by_domain.len(), 1);
    }

    #[tokio::test]
    async fn test_crm_policy_evaluation() {
        let store = EmbeddedOperationalStore::new();

        // 12% discount by AccountExec is permitted
        let report1 = store
            .evaluate_crm_discount("acc-1", 12.0, "AccountExec")
            .await
            .unwrap();
        assert!(report1.compliant);

        // 35% discount by AccountExec is rejected (requires VP)
        let report2 = store
            .evaluate_crm_discount("acc-1", 35.0, "AccountExec")
            .await
            .unwrap();
        assert!(!report2.compliant);
        assert_eq!(report2.required_role, "VP");

        // 35% discount by VP is permitted
        let report3 = store
            .evaluate_crm_discount("acc-1", 35.0, "VP")
            .await
            .unwrap();
        assert!(report3.compliant);
    }

    #[tokio::test]
    async fn test_taxonomy_keyword_matching() {
        let store = EmbeddedOperationalStore::new();

        let matched = store
            .match_feedback_to_taxonomy("The login page SSO is timing out repeatedly")
            .await
            .unwrap();
        assert!(matched.is_some());
        let (node, score) = matched.unwrap();
        assert_eq!(node.id, "tax-auth-login");
        assert!(score > 0.3);
    }

    #[tokio::test]
    async fn test_sdlc_pipeline_scaffolding_and_ci_event_ingestion() {
        let mut store = EmbeddedOperationalStore::new();

        // Check CI scaffold generation for GitHub Actions
        let scaffold = store.generate_pipeline_plugin_scaffold().await.unwrap();
        assert!(scaffold.contains_key(".github/workflows/exodus-verify.yml"));
        let github_yml = &scaffold[".github/workflows/exodus-verify.yml"];
        assert!(github_yml.contains("Exodus Unit Contract Verification"));

        // Ingest a remote CI build crash event
        let event = CiFailureEventPayload {
            provider: RemoteRepoProvider::GitHub,
            repository: "org/service-auth".to_string(),
            commit_sha: "abc8899".to_string(),
            branch: "feature/fast-login".to_string(),
            workflow_name: "Rust CI".to_string(),
            job_name: "test-auth".to_string(),
            run_id: "run-987654".to_string(),
            failing_step: "cargo test --package auth".to_string(),
            error_logs: "assertion failed: auth::validate_token() == Ok".to_string(),
        };

        let item = store.ingest_ci_failure_event(event).await.unwrap();
        assert_eq!(item.domain_tag, OperationalDomainTag::ProdBug);
        assert_eq!(item.state, OperationalLifecycleState::Captured);
        assert!(item.title.contains("CI Failure in org/service-auth"));

        // Item is retrieved from store
        let retrieved = store.get_operational_item(&item.id).await.unwrap();
        assert!(retrieved.is_some());
    }
}
