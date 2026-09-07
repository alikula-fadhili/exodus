//! Core domain models, shared types, outcome classifications, and contracts for Project Exodus.

pub mod esg;
pub use esg::*;

pub mod profile;
pub use profile::*;

pub mod deprecation;
pub use deprecation::*;

pub mod adapter;
pub use adapter::*;

pub mod operational_domain;
pub use operational_domain::*;

pub mod operational_item;
pub use operational_item::*;

pub mod sdlc_pipeline;
pub use sdlc_pipeline::*;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The classification of a migration outcome for a symbol, unit, or codebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MigrationOutcome {
    Verified,
    Compatible,
    Degraded,
    Blocked,
}

impl std::fmt::Display for MigrationOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verified => write!(f, "Verified"),
            Self::Compatible => write!(f, "Compatible"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Blocked => write!(f, "Blocked"),
        }
    }
}

/// Risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Precise position in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }
}

/// Byte and line-column span in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file_path: PathBuf,
    pub start: SourceLocation,
    pub end: SourceLocation,
}

impl SourceSpan {
    pub fn new(file_path: impl Into<PathBuf>, start: SourceLocation, end: SourceLocation) -> Self {
        Self {
            file_path: file_path.into(),
            start,
            end,
        }
    }

    pub fn point(file_path: impl Into<PathBuf>, line: usize, column: usize) -> Self {
        let loc = SourceLocation::new(line, column, 0);
        Self {
            file_path: file_path.into(),
            start: loc.clone(),
            end: loc,
        }
    }
}

/// Traceable evidence back to source files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEvidence {
    pub span: SourceSpan,
    pub snippet: Option<String>,
    pub ast_node_kind: String,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Diagnostic emitted during parsing, analysis, compilation, or verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub severity: Severity,
    pub span: Option<SourceSpan>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: Severity::Error,
            span,
            suggestion: None,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: Severity::Warning,
            span,
            suggestion: None,
        }
    }
}

/// Represents explicit migration debt incurred when fallback stubs or partial semantics are used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationDebt {
    pub symbol_id: String,
    pub description: String,
    pub reason: String,
    pub location: Option<SourceSpan>,
    pub fallback_strategy: String,
    pub confidence_score: f32,
    pub requires_human_review: bool,
}

/// Provenance of a behavioral-contract assertion, in the strength order the master spec requires:
/// a type signature alone never establishes complete expected behavior, so it must never be the
/// sole basis for an `expected` value (see [`OracleType::is_grounded`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleType {
    /// Existing source tests/fixtures — strongest evidence.
    SourceTest,
    /// Executable examples and approved golden files.
    GoldenFixture,
    /// The legacy unit was actually executed with controlled inputs and its output captured.
    DifferentialExecution,
    /// A declared precondition/postcondition/invariant or documented error behavior.
    DeclaredInvariant,
    /// Type signature, annotation, or docstring alone — describes shape, not complete behavior.
    TypeSignature,
    /// Synthesized with no stronger oracle available, and explicitly approved by a human reviewer.
    HumanApprovedSynthesized,
}

impl OracleType {
    /// 1 = strongest evidence, matching the master-prompt's strength ordering.
    pub fn strength_rank(&self) -> u8 {
        match self {
            Self::SourceTest => 1,
            Self::GoldenFixture => 2,
            Self::DifferentialExecution => 3,
            Self::DeclaredInvariant => 4,
            Self::TypeSignature => 5,
            Self::HumanApprovedSynthesized => 6,
        }
    }

    /// Whether this oracle alone is sufficient grounding for an `expected` value. A bare type
    /// signature never is — "a type signature alone is not sufficient evidence of expected
    /// behavior" (master prompt §11). Every other oracle kind carries some executed, declared, or
    /// human-reviewed evidence.
    pub fn is_grounded(&self) -> bool {
        !matches!(self, Self::TypeSignature)
    }
}

impl std::fmt::Display for OracleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceTest => write!(f, "source_test"),
            Self::GoldenFixture => write!(f, "golden_fixture"),
            Self::DifferentialExecution => write!(f, "differential_execution"),
            Self::DeclaredInvariant => write!(f, "declared_invariant"),
            Self::TypeSignature => write!(f, "type_signature"),
            Self::HumanApprovedSynthesized => write!(f, "human_approved_synthesized"),
        }
    }
}

/// Status of a behavioral contract's verification, mirroring `schemas/behavioral-contract.schema.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Passed,
    Failed,
    Pending,
    Degraded,
    Blocked,
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passed => write!(f, "passed"),
            Self::Failed => write!(f, "failed"),
            Self::Pending => write!(f, "pending"),
            Self::Degraded => write!(f, "degraded"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

/// Behavioral assertion for unit contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehavioralAssertion {
    pub case_id: String,
    pub input: String,
    pub expected: String,
    pub oracle: OracleType,
    /// Where the evidence for `expected` came from (a repo path/URI, a captured differential-run
    /// artifact path, or an approver identity for human-approved cases). Never empty.
    pub evidence: String,
}

/// Behavioral contract defining expected unit behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehavioralContract {
    pub schema_version: String,
    pub contract_id: String,
    pub unit_id: String,
    pub unit_name: String,
    pub unit_kind: String,
    pub source_signature: String,
    pub target_signature: String,
    pub assertions: Vec<BehavioralAssertion>,
    pub verification_status: VerificationStatus,
}

impl BehavioralContract {
    /// A contract is grounded only if every assertion carries grounded evidence — one ungrounded
    /// (type-signature-only) assertion is enough to disqualify the whole contract from being
    /// counted toward grounded-oracle-coverage metrics.
    pub fn is_grounded(&self) -> bool {
        !self.assertions.is_empty() && self.assertions.iter().all(|a| a.oracle.is_grounded())
    }
}

/// Core error types across the Exodus engine.
#[derive(thiserror::Error, Debug)]
pub enum ExodusError {
    #[error("Parse error in {file}: {message}")]
    ParseError { file: String, message: String },
    #[error("Graph error: {0}")]
    GraphError(String),
    #[error("Planning error: {0}")]
    PlanningError(String),
    #[error("Transformation error: {0}")]
    TransformationError(String),
    #[error("Unsupported target language: {0}")]
    UnsupportedTargetLanguage(String),
    #[error("Target path collision: {0}")]
    PathCollision(String),
    #[error("Verification failure: {0}")]
    VerificationFailure(String),
    #[error("Operational error: {0}")]
    OperationalError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Generic error: {0}")]
    Generic(String),
    #[error("Other error: {0}")]
    Other(String),
}

/// An actionable triage break report generated when transformation fails or hits migration debt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageBreakReport {
    pub report_id: String,
    pub timestamp: String,
    pub unit_id: String,
    pub source_language: String,
    pub target_language: String,
    pub failure_tier: MigrationOutcome,
    pub failure_category: String,
    pub compiler_diagnostics: Vec<String>,
    pub failed_assertion: Option<String>,
    pub source_snippet: String,
    pub target_snippet: Option<String>,
    pub repair_attempts: Vec<FailureTrajectory>,
    pub structural_fingerprint: Option<String>,
    pub recommended_labels: Vec<String>,
}

impl TriageBreakReport {
    pub fn to_markdown_issue(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!(
            "# [Migration Break] `{}` ({} -> {})\n\n",
            self.unit_id, self.source_language, self.target_language
        ));
        md.push_str(&format!(
            "**Outcome Tier:** `{}` | **Category:** `{}` | **Timestamp:** `{}`\n\n",
            self.failure_tier, self.failure_category, self.timestamp
        ));

        if let Some(ref fp) = self.structural_fingerprint {
            md.push_str(&format!("**Structural Fingerprint:** `{}`\n\n", fp));
        }

        md.push_str("## Source Snippet\n```");
        md.push_str(&self.source_language);
        md.push('\n');
        md.push_str(&self.source_snippet);
        md.push_str("\n```\n\n");

        if let Some(ref target) = self.target_snippet {
            md.push_str("## Target Snippet (Failed)\n```");
            md.push_str(&self.target_language);
            md.push('\n');
            md.push_str(target);
            md.push_str("\n```\n\n");
        }

        md.push_str("## Compiler & Test Diagnostics\n```text\n");
        for diag in &self.compiler_diagnostics {
            md.push_str(diag);
            md.push('\n');
        }
        if let Some(ref fa) = self.failed_assertion {
            md.push_str(&format!("Failed Assertion: {}\n", fa));
        }
        md.push_str("```\n\n");

        if !self.repair_attempts.is_empty() {
            md.push_str("## Repair Trajectory (Bounded Attempts)\n");
            for att in &self.repair_attempts {
                md.push_str(&format!(
                    "* **Attempt {}**: {}\n",
                    att.iteration, att.reason
                ));
                if let Some(ref diff) = att.patch_diff {
                    md.push_str(&format!("  ```diff\n{}\n  ```\n", diff));
                }
            }
            md.push('\n');
        }

        md.push_str("## Reproduction Payload\n");
        md.push_str("To reproduce locally in Project Exodus:\n");
        md.push_str(&format!(
            "```bash\nexodus migrate --from {} --to {} --gated\n```\n",
            self.source_language, self.target_language
        ));
        md
    }
}

/// A recorded repair iteration trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureTrajectory {
    pub iteration: u32,
    pub reason: String,
    pub patch_diff: Option<String>,
    pub outcome: MigrationOutcome,
}

/// Migration execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MigrationMode {
    /// Deterministic AST reverse-engineering and code synthesis (0 tokens, instant).
    #[default]
    Direct,
    /// Full agent reasoning, prompt orchestration, and autonomous repair.
    Ai,
    /// Direct AST synthesis first, with gated bounded AI repair on fallbacks/debts.
    Hybrid,
}

impl std::fmt::Display for MigrationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Direct => write!(f, "direct (reverse-engineering)"),
            Self::Ai => write!(f, "ai (autonomous agent)"),
            Self::Hybrid => write!(f, "hybrid (direct + gated ai repair)"),
        }
    }
}

/// The 7 scientific SDLC lifecycle stages for Project Exodus modernization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdlcStage {
    /// 1. Problem Analysis: AST parsing, symbol extraction, debt detection.
    Analysis,
    /// 2. Research & Knowledge Retrieval: Embedded catalog & promoted case lookup.
    Research,
    /// 3. Architecture Thesis Formulation: Target system hypotheses.
    ThesisFormulation,
    /// 4. System Flow Design: Topological waves, contract definitions.
    FlowDesign,
    /// 5. Implementation: Direct AST reverse-engineering or AI workload.
    Implementation,
    /// 6. Empirical Verification: Real compiler checks & behavioral oracle tests.
    EmpiricalVerification,
    /// 7. Maintenance & Feedback Loop: Case capture, debt logging, promotion.
    FeedbackCasePromotion,
}

/// SDLC and Modern System Design architectural check categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdlcCategory {
    Observability,
    Configuration12Factor,
    LifecycleAndResilience,
    SecurityAndAuth,
    DataStorage,
    ApiAndRouting,
    DeploymentAndCI,
}

impl std::fmt::Display for SdlcCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Observability => write!(f, "Observability & Tracing"),
            Self::Configuration12Factor => write!(f, "12-Factor App & Configuration"),
            Self::LifecycleAndResilience => write!(f, "Lifecycle, Graceful Shutdown & Resilience"),
            Self::SecurityAndAuth => write!(f, "Security, Headers & Auth"),
            Self::DataStorage => write!(f, "Data Storage & Connection Pooling"),
            Self::ApiAndRouting => write!(f, "API Routing & Health Probes"),
            Self::DeploymentAndCI => write!(f, "Containerization & CI/CD"),
        }
    }
}

/// Architectural modernization recommendation produced during planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModernizationRecommendation {
    pub id: String,
    pub category: SdlcCategory,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub impact_level: RiskLevel,
    pub remediation_code_sample: Option<String>,
}

/// Formulation of an empirical modernization hypothesis for target system design.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureThesis {
    pub thesis_id: String,
    pub legacy_problem_statement: String,
    pub hypothesis: String,
    pub target_architectural_pattern: String,
    pub expected_outcomes: Vec<String>,
    pub verification_assertions: Vec<String>,
}

/// Comprehensive SDLC & System Design Audit Report generated during planning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SdlcAuditReport {
    pub health_score: u8,
    pub passed_checks: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub recommendations: Vec<ModernizationRecommendation>,
    pub architecture_thesis: Option<ArchitectureThesis>,
}

/// Preset Intra-Language / Framework Modernization catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModernizationPreset {
    Python2To3,
    PythonModernTyping,
    CommonJsToEsm,
    ReactClassToFunctional,
    ExpressToFastifyOrHono,
    TokioSyncToAsync,
    Rust2018To2024,
    Custom(String),
}

impl std::fmt::Display for ModernizationPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Python2To3 => write!(f, "python2-to-3"),
            Self::PythonModernTyping => write!(f, "python-modern-typing"),
            Self::CommonJsToEsm => write!(f, "commonjs-to-esm"),
            Self::ReactClassToFunctional => write!(f, "react-class-to-functional"),
            Self::ExpressToFastifyOrHono => write!(f, "express-to-hono"),
            Self::TokioSyncToAsync => write!(f, "tokio-sync-to-async"),
            Self::Rust2018To2024 => write!(f, "rust-2018-to-2024"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

pub type Result<T> = std::result::Result<T, ExodusError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outcome_classification() {
        assert_ne!(MigrationOutcome::Verified, MigrationOutcome::Degraded);
        assert_ne!(MigrationOutcome::Compatible, MigrationOutcome::Blocked);
    }

    fn sample_contract(oracle: OracleType) -> BehavioralContract {
        BehavioralContract {
            schema_version: "1.0.0".to_string(),
            contract_id: "contract-x".to_string(),
            unit_id: "function::x".to_string(),
            unit_name: "x".to_string(),
            unit_kind: "function".to_string(),
            source_signature: "x() -> int".to_string(),
            target_signature: "x() -> i64".to_string(),
            assertions: vec![BehavioralAssertion {
                case_id: "basic".to_string(),
                input: "x()".to_string(),
                expected: "1".to_string(),
                oracle,
                evidence: "evidence".to_string(),
            }],
            verification_status: VerificationStatus::Pending,
        }
    }

    /// Required test: an ungrounded oracle (a type signature alone) must never be treated as
    /// sufficient grounding — "a type signature alone is not sufficient evidence of expected
    /// behavior" (master prompt §11).
    #[test]
    fn test_type_signature_alone_is_not_grounded() {
        assert!(!OracleType::TypeSignature.is_grounded());
        assert!(!sample_contract(OracleType::TypeSignature).is_grounded());
    }

    #[test]
    fn test_stronger_oracles_are_grounded() {
        for oracle in [
            OracleType::SourceTest,
            OracleType::GoldenFixture,
            OracleType::DifferentialExecution,
            OracleType::DeclaredInvariant,
            OracleType::HumanApprovedSynthesized,
        ] {
            assert!(oracle.is_grounded());
            assert!(sample_contract(oracle).is_grounded());
        }
    }

    #[test]
    fn test_a_single_ungrounded_assertion_disqualifies_the_whole_contract() {
        let mut contract = sample_contract(OracleType::SourceTest);
        contract.assertions.push(BehavioralAssertion {
            case_id: "second".to_string(),
            input: "x()".to_string(),
            expected: "1".to_string(),
            oracle: OracleType::TypeSignature,
            evidence: "evidence".to_string(),
        });
        assert!(
            !contract.is_grounded(),
            "one ungrounded assertion must not be hidden by other grounded ones"
        );
    }

    /// Required test: a malformed/invalid contract must be rejected rather than accepted as-is.
    /// `BehavioralContract` deserialization itself is the schema gate here — an unknown
    /// `oracle`/`verification_status` value or a missing required field fails to parse, which
    /// callers (e.g. `exodus_verifier::load_fixture_contract`) treat as "no contract available"
    /// rather than fabricating one.
    #[test]
    fn test_invalid_contract_json_is_rejected_not_silently_accepted() {
        let missing_required_field = r#"{
            "schema_version": "1.0.0",
            "contract_id": "c1",
            "unit_id": "function::x",
            "unit_name": "x",
            "unit_kind": "function",
            "assertions": [],
            "verification_status": "passed"
        }"#;
        // `source_signature`/`target_signature` are missing entirely (not merely empty), which is
        // fine (they're not marked optional but serde requires every non-Option field present) —
        // this must fail to parse rather than silently defaulting.
        let result: std::result::Result<BehavioralContract, _> =
            serde_json::from_str(missing_required_field);
        assert!(result.is_err());

        let unknown_oracle_value = r#"{
            "schema_version": "1.0.0",
            "contract_id": "c1",
            "unit_id": "function::x",
            "unit_name": "x",
            "unit_kind": "function",
            "source_signature": "x() -> int",
            "target_signature": "x() -> i64",
            "assertions": [{
                "case_id": "basic",
                "input": "x()",
                "expected": "1",
                "oracle": "vibes",
                "evidence": "evidence"
            }],
            "verification_status": "passed"
        }"#;
        let result: std::result::Result<BehavioralContract, _> =
            serde_json::from_str(unknown_oracle_value);
        assert!(
            result.is_err(),
            "an oracle value outside the known strength hierarchy must be rejected, not accepted as valid grounding"
        );
    }

    #[test]
    fn test_triage_break_report_markdown_generation() {
        let report = TriageBreakReport {
            report_id: "triage-001".to_string(),
            timestamp: "2026-08-30T09:50:00Z".to_string(),
            unit_id: "function::auth::verify_token".to_string(),
            source_language: "python".to_string(),
            target_language: "rust".to_string(),
            failure_tier: MigrationOutcome::Blocked,
            failure_category: "TypeMismatch".to_string(),
            compiler_diagnostics: vec!["error[E0308]: mismatched types".to_string()],
            failed_assertion: Some("assert verify_token('secret') == True".to_string()),
            source_snippet: "def verify_token(t): return t == 'secret'".to_string(),
            target_snippet: Some("pub fn verify_token(t: &str) -> bool { t == 1 }".to_string()),
            repair_attempts: vec![FailureTrajectory {
                iteration: 1,
                reason: "Attempted integer literal comparison".to_string(),
                patch_diff: Some("- t == 1\n+ t == \"secret\"".to_string()),
                outcome: MigrationOutcome::Blocked,
            }],
            structural_fingerprint: Some("fp-sha256-abc123".to_string()),
            recommended_labels: vec![
                "bug:migration-break".to_string(),
                "lang:python-to-rust".to_string(),
            ],
        };

        let md = report.to_markdown_issue();
        assert!(md.contains("# [Migration Break] `function::auth::verify_token`"));
        assert!(md.contains("error[E0308]: mismatched types"));
        assert!(md.contains("fp-sha256-abc123"));
    }

    #[test]
    fn test_modernization_presets_display() {
        assert_eq!(ModernizationPreset::Python2To3.to_string(), "python2-to-3");
        assert_eq!(
            ModernizationPreset::CommonJsToEsm.to_string(),
            "commonjs-to-esm"
        );
        assert_eq!(
            ModernizationPreset::ExpressToFastifyOrHono.to_string(),
            "express-to-hono"
        );
    }

    #[test]
    fn test_sdlc_and_migration_mode_models() {
        assert_eq!(MigrationMode::default(), MigrationMode::Direct);
        assert_eq!(
            MigrationMode::Direct.to_string(),
            "direct (reverse-engineering)"
        );
        assert_eq!(MigrationMode::Ai.to_string(), "ai (autonomous agent)");
        assert_eq!(
            MigrationMode::Hybrid.to_string(),
            "hybrid (direct + gated ai repair)"
        );

        let thesis = ArchitectureThesis {
            thesis_id: "thesis-1".to_string(),
            legacy_problem_statement: "Blocking thread pool".to_string(),
            hypothesis: "Async Tokio + Axum eliminates thread exhaustion".to_string(),
            target_architectural_pattern: "Event-driven asynchronous actor model".to_string(),
            expected_outcomes: vec!["Sub-10ms p99 latency".to_string()],
            verification_assertions: vec!["cargo test passes".to_string()],
        };
        assert_eq!(thesis.thesis_id, "thesis-1");

        let audit = SdlcAuditReport {
            health_score: 85,
            passed_checks: vec!["Structured Logging".to_string()],
            missing_capabilities: vec!["/healthz probe".to_string()],
            recommendations: vec![ModernizationRecommendation {
                id: "rec-1".to_string(),
                category: SdlcCategory::ApiAndRouting,
                title: "Add Healthcheck Endpoints".to_string(),
                description: "Expose /healthz and /readyz".to_string(),
                rationale: "Required for Kubernetes and cloud probes".to_string(),
                impact_level: RiskLevel::Medium,
                remediation_code_sample: Some(
                    "pub async fn healthz() -> &'static str { \"OK\" }".to_string(),
                ),
            }],
            architecture_thesis: Some(thesis),
        };
        assert_eq!(audit.health_score, 85);
        assert_eq!(audit.recommendations.len(), 1);
    }

    #[test]
    fn test_language_id_and_esg_models() {
        let py = LanguageId::new("python");
        assert_eq!(py.as_str(), "python");
        assert_eq!(py, " PYTHON ");
        assert_eq!(py, String::from("Python"));
        assert_eq!(py.as_ref(), "python");
        assert_eq!(&*py, "python");
        let borrowed: &str = std::borrow::Borrow::borrow(&py);
        assert_eq!(borrowed, "python");
        let ts = LanguageId::new("TypeScript");
        assert_eq!(ts.as_str(), "typescript");

        let node = EsgNode::new(
            "urn:sym:pkg::ClassA",
            py.clone(),
            EsgNodeKind::Class,
            "ClassA",
        );
        assert_eq!(node.grounding, GroundingTier::Deterministic);
        assert!(node.grounding.is_grounded());

        let edge = EsgEdge::new(
            "urn:sym:pkg::ClassA",
            "urn:sym:pkg::TraitB",
            EsgRelation::Implements,
        );
        assert_eq!(edge.relation, EsgRelation::Implements);
    }

    #[test]
    fn test_repository_and_deprecation_profiles() {
        let prof = RepositoryProfile::new("repo-1", LanguageId::new("python"));
        assert_eq!(prof.primary_language, LanguageId::new("python"));
        assert_eq!(
            uuid::Uuid::parse_str(&prof.snapshot_id)
                .unwrap()
                .get_version_num(),
            7
        );

        let dep = DeprecationRecord::new(
            "dep-1",
            LanguageId::new("python"),
            "os.popen",
            DeprecationStatus::Deprecated,
            DeprecationEvidenceSource::CompilerWarning,
        );
        assert_eq!(dep.status, DeprecationStatus::Deprecated);
        assert!(dep.grounding.is_grounded());
    }

    #[tokio::test]
    async fn test_adapter_registry_duplicate_rejection() {
        struct DummySourceAdapter;
        #[async_trait::async_trait]
        impl SourceLanguageAdapter for DummySourceAdapter {
            fn language_id(&self) -> LanguageId {
                LanguageId::new("python")
            }
            fn supported_extensions(&self) -> &[&'static str] {
                &["py"]
            }
            async fn parse_file(
                &self,
                _repo_root: &Path,
                _rel_path: &Path,
                _content: &str,
            ) -> Result<(
                Vec<EsgNode>,
                Vec<EsgEdge>,
                Vec<Diagnostic>,
                Vec<DeprecationRecord>,
            )> {
                Ok((Vec::new(), Vec::new(), Vec::new(), Vec::new()))
            }
            async fn profile_repository(&self, _repo_root: &Path) -> Result<RepositoryProfile> {
                Ok(RepositoryProfile::new("dummy", LanguageId::new("python")))
            }
        }

        use std::path::Path;
        let mut registry = LanguageAdapterRegistry::new();
        let adapter = std::sync::Arc::new(DummySourceAdapter);
        assert!(registry
            .register_source_adapter(adapter.clone())
            .await
            .is_ok());
        // Duplicate registration must fail
        assert!(registry.register_source_adapter(adapter).await.is_err());
    }
}
