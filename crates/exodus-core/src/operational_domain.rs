//! Operational Domain definitions for the Enterprise Agentic Execution Fabric.
//!
//! Provides typed tags and domain payloads for the three cross-functional operations:
//! - `#prod-bug`: Engineering compiler/CI/runtime failures.
//! - `#crm-request`: Commercial discount, SLA exception, and tier upgrade governance.
//! - `#survey-mapping`: Product customer sentiment and taxonomy ontology alignment.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// Tag identifying the operational domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationalDomainTag {
    /// Engineering runtime, CI, or compiler bug (#prod-bug)
    #[serde(rename = "#prod-bug")]
    ProdBug,
    /// Commercial discount, tier, or SLA mutation (#crm-request)
    #[serde(rename = "#crm-request")]
    CrmRequest,
    /// Product customer feedback and taxonomy mapping (#survey-mapping)
    #[serde(rename = "#survey-mapping")]
    SurveyMapping,
}

impl OperationalDomainTag {
    /// Returns the canonical hashtag representation.
    pub fn as_tag(&self) -> &'static str {
        match self {
            Self::ProdBug => "#prod-bug",
            Self::CrmRequest => "#crm-request",
            Self::SurveyMapping => "#survey-mapping",
        }
    }

    /// Human-friendly domain name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ProdBug => "Engineering Operations",
            Self::CrmRequest => "Commercial Operations",
            Self::SurveyMapping => "Product & Market Operations",
        }
    }
}

impl fmt::Display for OperationalDomainTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_tag())
    }
}

impl FromStr for OperationalDomainTag {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();
        match normalized.as_str() {
            "#prod-bug" | "prod-bug" | "prodbug" | "bug" | "engineering" => Ok(Self::ProdBug),
            "#crm-request" | "crm-request" | "crmrequest" | "crm" | "commercial" => {
                Ok(Self::CrmRequest)
            }
            "#survey-mapping" | "survey-mapping" | "survey" | "taxonomy" | "product" => {
                Ok(Self::SurveyMapping)
            }
            _ => Err(format!(
                "Unknown operational domain tag: '{}'. Valid tags: #prod-bug, #crm-request, #survey-mapping",
                s
            )),
        }
    }
}

/// Payload for engineering runtime/CI bug operations (#prod-bug).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProdBugPayload {
    /// Target Git commit ID or branch where error occurred.
    pub commit_id: String,
    /// Failing error message or compiler diagnostic string.
    pub error_message: String,
    /// Stack trace or detailed compiler output log.
    pub stack_trace: Option<String>,
    /// Path to failing source file.
    pub target_file: Option<PathBuf>,
    /// Symbol ID or function name identified as root boundary.
    pub target_symbol: Option<String>,
    /// Command used to trigger and reproduce the failure (e.g. `cargo test`).
    pub reproduction_command: Option<String>,
}

impl ProdBugPayload {
    pub fn new(
        commit_id: impl Into<String>,
        error_message: impl Into<String>,
        target_file: Option<PathBuf>,
    ) -> Self {
        Self {
            commit_id: commit_id.into(),
            error_message: error_message.into(),
            stack_trace: None,
            target_file,
            target_symbol: None,
            reproduction_command: None,
        }
    }
}

/// Payload for commercial operations and CRM contract mutations (#crm-request).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrmRequestPayload {
    /// Target account ID in the CRM system.
    pub account_id: String,
    /// Account / Company name.
    pub account_name: String,
    /// Current annual recurring revenue (ARR) in USD.
    pub current_arr: f64,
    /// Proposed discount percentage (e.g. 25.0 for 25%).
    pub requested_discount_pct: f64,
    /// Target plan tier (e.g. "Enterprise", "Custom", "Scale").
    pub requested_tier: String,
    /// Organizational role of the requester (e.g. "AccountExec", "SalesLead", "VP").
    pub requester_role: String,
    /// Contract duration in months (e.g. 12, 24, 36).
    pub contract_term_months: u32,
    /// Justification or business memo for exception approval.
    pub justification: String,
}

impl CrmRequestPayload {
    pub fn new(
        account_id: impl Into<String>,
        account_name: impl Into<String>,
        current_arr: f64,
        requested_discount_pct: f64,
        requested_tier: impl Into<String>,
        requester_role: impl Into<String>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            account_name: account_name.into(),
            current_arr,
            requested_discount_pct,
            requested_tier: requested_tier.into(),
            requester_role: requester_role.into(),
            contract_term_months: 12,
            justification: String::new(),
        }
    }
}

/// Single raw customer feedback item in a survey batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurveyResponseItem {
    pub id: String,
    pub feedback_text: String,
    pub customer_segment: Option<String>,
    pub candidate_taxonomy_node: Option<String>,
}

/// Payload for product operations and taxonomy mapping (#survey-mapping).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurveyMappingPayload {
    /// Batch identifier from the survey platform.
    pub batch_id: String,
    /// Source platform (e.g. "Typeform", "Qualtrics", "CSAT-Widget").
    pub source_platform: String,
    /// Unstructured customer survey response items.
    pub responses: Vec<SurveyResponseItem>,
    /// Target canonical product taxonomy root identifier.
    pub target_taxonomy_id: String,
}

impl SurveyMappingPayload {
    pub fn new(
        batch_id: impl Into<String>,
        source_platform: impl Into<String>,
        target_taxonomy_id: impl Into<String>,
    ) -> Self {
        Self {
            batch_id: batch_id.into(),
            source_platform: source_platform.into(),
            responses: Vec::new(),
            target_taxonomy_id: target_taxonomy_id.into(),
        }
    }

    pub fn add_response(&mut self, id: impl Into<String>, text: impl Into<String>) {
        self.responses.push(SurveyResponseItem {
            id: id.into(),
            feedback_text: text.into(),
            customer_segment: None,
            candidate_taxonomy_node: None,
        });
    }
}

/// Polymorphic operational domain payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain_type", content = "data")]
pub enum DomainPayload {
    #[serde(rename = "prod_bug")]
    ProdBug(ProdBugPayload),
    #[serde(rename = "crm_request")]
    CrmRequest(CrmRequestPayload),
    #[serde(rename = "survey_mapping")]
    SurveyMapping(SurveyMappingPayload),
}

impl DomainPayload {
    pub fn domain_tag(&self) -> OperationalDomainTag {
        match self {
            Self::ProdBug(_) => OperationalDomainTag::ProdBug,
            Self::CrmRequest(_) => OperationalDomainTag::CrmRequest,
            Self::SurveyMapping(_) => OperationalDomainTag::SurveyMapping,
        }
    }
}
