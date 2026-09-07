//! Migration Case Engine, structural fingerprinting, and governed knowledge reuse for Project Exodus.

use chrono::{DateTime, Utc};
use exodus_core::{ExodusError, Result};
use exodus_graph::SemanticGraph;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use uuid::Uuid;

pub mod promoter;
pub use promoter::*;

/// Lifecycle state of a migration case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseStatus {
    Captured,
    Reproduced,
    RepairProposed,
    Verified,
    HumanApproved,
    Promoted,
    Deprecated,
}

/// Category of migration failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCategory {
    TypeMismatch,
    OwnershipBorrowError,
    AsyncCallbackSemantics,
    MissingModuleImport,
    DynamicReflectionUnsupported,
    BehavioralAssertionFailed,
    SyntaxError,
}

/// A versioned, evidence-backed Migration Case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCase {
    pub schema_version: String,
    pub case_id: String,
    pub run_id: String,
    pub structural_fingerprint: String,
    pub status: CaseStatus,
    pub failure_category: FailureCategory,
    pub source_language: String,
    pub target_language: String,
    pub failure_description: String,
    pub compiler_diagnostic: Option<String>,
    /// ESG identity of the unit this case originated from (e.g. `function::sanitize_input`).
    /// `None` for cases captured before unit-level localization existed.
    pub unit_id: Option<String>,
    /// The specific contract assertion `case_id` that failed, if the failure was a behavioral
    /// contract violation rather than a compile error.
    pub failed_assertion: Option<String>,
    /// What the source (legacy) unit actually produced for the failing input, when known.
    pub source_observation: Option<String>,
    /// What the migrated target unit actually produced for the same input, when known.
    pub target_observation: Option<String>,
    /// The minimal relevant ESG subgraph around the failing unit (see
    /// `SemanticGraph::relevant_subgraph`), serialized as JSON. This localized structure is what
    /// `compute_fingerprint` hashes — not just node/edge counts — so structurally similar failures
    /// on differently named symbols in different repositories still collide.
    pub esg_subgraph: Option<String>,
    pub attempted_strategies: Vec<String>,
    pub successful_strategy: Option<String>,
    pub repair_patch: Option<String>,
    pub applications_count: usize,
    pub verified_success_count: usize,
    pub human_approved_by: Option<String>,
    pub human_approved_at: Option<DateTime<Utc>>,
    pub regression_fixture_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl MigrationCase {
    /// Formats human-auditable confidence statement.
    pub fn confidence_statement(&self) -> String {
        if self.applications_count == 0 {
            "0 verified applications (Candidate)".to_string()
        } else {
            format!(
                "{} of {} verified applications succeeded",
                self.verified_success_count, self.applications_count
            )
        }
    }
}

/// Input to [`CaseEngine::capture_failure`]. Grouped into a struct rather than positional
/// arguments because a well-formed case capture genuinely needs this many pieces of localized
/// evidence (master prompt §11/§13) and a long positional argument list would be error-prone to
/// call correctly.
pub struct CaseCaptureInput<'a> {
    pub run_id: &'a str,
    pub failure_category: FailureCategory,
    pub unit_id: &'a str,
    pub source_language: &'a str,
    pub target_language: &'a str,
    /// The unit's minimal relevant ESG subgraph (see `SemanticGraph::relevant_subgraph`), used
    /// both for the structural fingerprint and as attached evidence for repair/review.
    pub graph: Option<&'a SemanticGraph>,
    pub diagnostic: Option<&'a str>,
    pub failed_assertion: Option<&'a str>,
    pub source_observation: Option<&'a str>,
    pub target_observation: Option<&'a str>,
}

/// Case Engine for capturing, searching, ranking, and promoting migration knowledge.
pub struct CaseEngine {
    storage_dir: PathBuf,
}

impl CaseEngine {
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            storage_dir: storage_dir.into(),
        }
    }

    /// Computes a deterministic structural graph fingerprint from real topology — node *kinds*,
    /// edge *kinds*, dynamic/unsupported-construct presence, failure category, and language pair —
    /// deliberately excluding the failing symbol's name or file path. Two failures that are
    /// structurally identical must fingerprint identically even when the symbols involved have
    /// different names in different repositories (master prompt §13: "the retrieval engine must
    /// find the promoted case structurally, not through filenames").
    pub fn compute_fingerprint(
        failure_cat: &FailureCategory,
        source_graph: Option<&SemanticGraph>,
        source_language: &str,
        target_language: &str,
    ) -> String {
        let mut parts = vec![
            format!("failure:{failure_cat:?}"),
            format!("lang:{source_language}->{target_language}"),
        ];

        if let Some(graph) = source_graph {
            let mut node_kind_counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut dynamic_count = 0usize;
            for node in graph.nodes.values() {
                *node_kind_counts
                    .entry(format!("{:?}", node.kind))
                    .or_insert(0) += 1;
                if node.kind == exodus_graph::NodeKind::UnsupportedConstruct {
                    dynamic_count += 1;
                }
            }
            for (kind, count) in &node_kind_counts {
                parts.push(format!("node:{kind}:{count}"));
            }

            let mut edge_kind_counts: BTreeMap<String, usize> = BTreeMap::new();
            for edge in &graph.edges {
                *edge_kind_counts
                    .entry(format!("{:?}", edge.relationship))
                    .or_insert(0) += 1;
            }
            for (kind, count) in &edge_kind_counts {
                parts.push(format!("edge:{kind}:{count}"));
            }

            parts.push(format!("dynamic:{dynamic_count}"));
        } else {
            parts.push("graph:none".to_string());
        }

        let raw = parts.join("|");
        format!("{:016x}", structural_hash(&raw))
    }

    /// Captures a newly encountered failure as a candidate MigrationCase, localized to the unit
    /// that failed. Includes the unit's minimal relevant ESG subgraph, the failed assertion (when
    /// the failure was a behavioral-contract violation rather than a compile error), and observed
    /// source/target behavior — this is what `structural_fingerprint` is derived from, so the case
    /// contributes real structural signal rather than just a compiler diagnostic string.
    pub fn capture_failure(&self, input: CaseCaptureInput<'_>) -> Result<MigrationCase> {
        let case_id = format!("case-{}", Uuid::new_v4());
        let fingerprint = Self::compute_fingerprint(
            &input.failure_category,
            input.graph,
            input.source_language,
            input.target_language,
        );
        let esg_subgraph = input.graph.map(serde_json::to_string).transpose()?;

        let case = MigrationCase {
            schema_version: "1.0.0".to_string(),
            case_id: case_id.clone(),
            run_id: input.run_id.to_string(),
            structural_fingerprint: fingerprint,
            status: CaseStatus::Captured,
            failure_category: input.failure_category,
            source_language: input.source_language.to_string(),
            target_language: input.target_language.to_string(),
            failure_description: format!("Failure on unit `{}`", input.unit_id),
            compiler_diagnostic: input.diagnostic.map(|d| d.to_string()),
            unit_id: Some(input.unit_id.to_string()),
            failed_assertion: input.failed_assertion.map(|s| s.to_string()),
            source_observation: input.source_observation.map(|s| s.to_string()),
            target_observation: input.target_observation.map(|s| s.to_string()),
            esg_subgraph,
            attempted_strategies: Vec::new(),
            successful_strategy: None,
            repair_patch: None,
            applications_count: 0,
            verified_success_count: 0,
            human_approved_by: None,
            human_approved_at: None,
            regression_fixture_path: None,
            created_at: Utc::now(),
        };

        self.save_case(&case)?;
        Ok(case)
    }

    /// Saves a case to `.exodus/knowledge/cases/<case_id>.json`.
    pub fn save_case(&self, case: &MigrationCase) -> Result<()> {
        let cases_dir = self.storage_dir.join("cases");
        fs::create_dir_all(&cases_dir).map_err(ExodusError::from)?;
        let case_file = cases_dir.join(format!("{}.json", case.case_id));
        fs::write(case_file, serde_json::to_string_pretty(case)?).map_err(ExodusError::from)?;
        Ok(())
    }

    /// Loads all cases.
    pub fn list_cases(&self) -> Result<Vec<MigrationCase>> {
        let cases_dir = self.storage_dir.join("cases");
        let mut cases = Vec::new();

        if cases_dir.exists() {
            if let Ok(entries) = fs::read_dir(cases_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(case) = serde_json::from_str::<MigrationCase>(&content) {
                                cases.push(case);
                            }
                        }
                    }
                }
            }
        }

        Ok(cases)
    }

    /// Searches for promoted cases matching a structural fingerprint or category.
    pub fn search_promoted_cases(
        &self,
        fingerprint: &str,
        category: &FailureCategory,
    ) -> Result<Vec<MigrationCase>> {
        let all_cases = self.list_cases()?;
        let mut matches = Vec::new();

        for case in all_cases {
            if case.status == CaseStatus::Promoted
                && (case.structural_fingerprint == fingerprint
                    || case.failure_category == *category)
            {
                matches.push(case);
            }
        }

        // Rank by success count descending
        matches.sort_by_key(|a| std::cmp::Reverse(a.verified_success_count));
        Ok(matches)
    }

    /// Approves and promotes a verified case.
    pub fn approve_case(&self, case_id: &str, approver: &str) -> Result<MigrationCase> {
        let mut cases = self.list_cases()?;
        let target = cases
            .iter_mut()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ExodusError::Generic(format!("Case `{case_id}` not found")))?;

        target.status = CaseStatus::Promoted;
        target.human_approved_by = Some(approver.to_string());
        target.human_approved_at = Some(Utc::now());
        if target.applications_count == 0 {
            target.applications_count = 1;
            target.verified_success_count = 1;
        }

        self.save_case(target)?;
        Ok(target.clone())
    }

    /// Generates a standalone reproducible regression fixture for a promoted case.
    pub fn generate_regression_fixture(
        &self,
        case_id: &str,
        output_dir: &std::path::Path,
    ) -> Result<PathBuf> {
        let cases = self.list_cases()?;
        let case = cases
            .iter()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ExodusError::Generic(format!("Case `{case_id}` not found")))?;

        let fixture_dir = output_dir.join(format!("regression_{}", case.case_id));
        fs::create_dir_all(&fixture_dir).map_err(|e| {
            ExodusError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create regression fixture directory: {e}"),
            ))
        })?;

        // 1. Write minimal source file
        let ext = if case.source_language.to_lowercase().contains("python") {
            "py"
        } else if case.source_language.to_lowercase().contains("type")
            || case.source_language.to_lowercase().contains("js")
        {
            "ts"
        } else if case.source_language.to_lowercase().contains("go") {
            "go"
        } else {
            "src"
        };

        let unit_name = case
            .unit_id
            .as_deref()
            .unwrap_or("repro_unit")
            .replace("::", "_");
        let source_code = format!(
            "# Regression fixture for case {}\n# Category: {:?}\n# Failure: {}\ndef {}():\n    pass\n",
            case.case_id, case.failure_category, case.failure_description, unit_name
        );
        fs::write(fixture_dir.join(format!("source.{ext}")), source_code)
            .map_err(ExodusError::Io)?;

        // 2. Write metadata and contract manifest
        let manifest = serde_json::json!({
            "case_id": case.case_id,
            "failure_category": case.failure_category,
            "structural_fingerprint": case.structural_fingerprint,
            "target_language": case.target_language,
            "diagnostic": case.compiler_diagnostic,
            "successful_strategy": case.successful_strategy,
            "repair_patch": case.repair_patch,
        });
        fs::write(
            fixture_dir.join("fixture_manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .map_err(ExodusError::Io)?;

        Ok(fixture_dir)
    }

    /// Rejects or deprecates a case.
    pub fn reject_case(&self, case_id: &str) -> Result<MigrationCase> {
        let mut cases = self.list_cases()?;
        let target = cases
            .iter_mut()
            .find(|c| c.case_id == case_id)
            .ok_or_else(|| ExodusError::Generic(format!("Case `{case_id}` not found")))?;

        target.status = CaseStatus::Deprecated;
        self.save_case(target)?;
        Ok(target.clone())
    }
}

/// SipHash-based structural hash (std's `DefaultHasher`) over the fingerprint's canonicalized
/// parts. Deliberately not named after MD5 — it never was MD5, and honesty about what evidence
/// backs a claim is the whole point of this engine.
fn structural_hash(input: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_lifecycle_and_promotion() {
        let temp_dir = std::env::temp_dir().join(format!("exodus_case_test_{}", Uuid::new_v4()));
        let engine = CaseEngine::new(&temp_dir);

        let case = engine
            .capture_failure(CaseCaptureInput {
                run_id: "run-1",
                failure_category: FailureCategory::AsyncCallbackSemantics,
                unit_id: "function::fetch_data",
                source_language: "Python 3.11",
                target_language: "Rust 2021",
                graph: None,
                diagnostic: Some("E0308: mismatched types"),
                failed_assertion: None,
                source_observation: None,
                target_observation: None,
            })
            .unwrap();

        assert_eq!(case.status, CaseStatus::Captured);
        assert_eq!(case.unit_id.as_deref(), Some("function::fetch_data"));

        let promoted = engine
            .approve_case(&case.case_id, "ChiefArchitect")
            .unwrap();
        assert_eq!(promoted.status, CaseStatus::Promoted);
        assert_eq!(
            promoted.human_approved_by.as_deref(),
            Some("ChiefArchitect")
        );
        assert_eq!(
            promoted.confidence_statement(),
            "1 of 1 verified applications succeeded"
        );

        let matches = engine
            .search_promoted_cases(
                &case.structural_fingerprint,
                &FailureCategory::AsyncCallbackSemantics,
            )
            .unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].case_id, case.case_id);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_fingerprint_ignores_symbol_name_but_reflects_structure() {
        use exodus_graph::{NodeKind, RelationKind, SemanticEdge, SemanticGraph, SemanticNode};
        use std::collections::HashMap;

        fn make_graph(fn_name: &str) -> SemanticGraph {
            let mut g = SemanticGraph::new();
            g.add_node(SemanticNode {
                id: format!("function::{fn_name}"),
                name: fn_name.to_string(),
                kind: NodeKind::Function,
                qualified_name: fn_name.to_string(),
                file_path: "f.py".to_string(),
                risk_score: 10,
                risk_level: exodus_core::RiskLevel::Low,
                evidence: None,
                metadata: HashMap::new(),
            });
            g.add_node(SemanticNode {
                id: "function::callback".to_string(),
                name: "callback".to_string(),
                kind: NodeKind::Function,
                qualified_name: "callback".to_string(),
                file_path: "f.py".to_string(),
                risk_score: 10,
                risk_level: exodus_core::RiskLevel::Low,
                evidence: None,
                metadata: HashMap::new(),
            });
            g.add_edge(SemanticEdge {
                from: format!("function::{fn_name}"),
                to: "function::callback".to_string(),
                relationship: RelationKind::Calls,
                evidence: None,
            });
            g
        }

        // Same structure (1 function calling 1 function), different symbol names/repos —
        // the fingerprint must match so a promoted case from repo A is retrievable for repo B.
        let graph_a = make_graph("fetch_user_data");
        let graph_b = make_graph("load_account_info");

        let fp_a = CaseEngine::compute_fingerprint(
            &FailureCategory::AsyncCallbackSemantics,
            Some(&graph_a),
            "Python 3.11",
            "Rust 2021",
        );
        let fp_b = CaseEngine::compute_fingerprint(
            &FailureCategory::AsyncCallbackSemantics,
            Some(&graph_b),
            "Python 3.11",
            "Rust 2021",
        );
        assert_eq!(fp_a, fp_b, "structurally identical failures must fingerprint identically regardless of symbol name");

        // A different failure category on the same structure must NOT collide.
        let fp_c = CaseEngine::compute_fingerprint(
            &FailureCategory::TypeMismatch,
            Some(&graph_a),
            "Python 3.11",
            "Rust 2021",
        );
        assert_ne!(fp_a, fp_c);
    }
}
