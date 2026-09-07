//! Repository-wide Sense Phase: multi-format evidence extraction, text-first segmentation, and intent graph reconciliation.

use exodus_core::{
    ClaimReviewStatus, GroundingTier, IntentCategory, IntentClaim, IntentClaimAlternative,
    IntentEdgeKind, IntentEvidence, IntentScope, MigrationIntentContract, MigrationIntentGraph,
    SecretScrubber,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Segment classification produced by text-first decomposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentKind {
    Prose,
    CodeFence { language: String },
    Comment,
    Command,
    EnvDeclaration,
    Manifest,
    DamagedSnippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    pub kind: SegmentKind,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Text-first segmenter capable of decomposing damaged fragments, markdown, scripts, and configs.
pub struct TextFirstSegmenter;

impl TextFirstSegmenter {
    pub fn segment(content: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.starts_with("```") {
                let lang = trimmed.trim_start_matches("```").trim().to_string();
                let start = i + 1;
                let mut fence_lines = Vec::new();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    fence_lines.push(lines[i]);
                    i += 1;
                }
                let end = i + 1;
                if i < lines.len() {
                    i += 1;
                }
                segments.push(TextSegment {
                    kind: SegmentKind::CodeFence { language: lang },
                    content: fence_lines.join("\n"),
                    start_line: start,
                    end_line: end,
                });
            } else if trimmed.starts_with('#') || trimmed.starts_with("//") {
                let start = i + 1;
                let mut comment_lines = vec![line];
                i += 1;
                while i < lines.len()
                    && (lines[i].trim().starts_with('#') || lines[i].trim().starts_with("//"))
                {
                    comment_lines.push(lines[i]);
                    i += 1;
                }
                segments.push(TextSegment {
                    kind: SegmentKind::Comment,
                    content: comment_lines.join("\n"),
                    start_line: start,
                    end_line: i,
                });
            } else if trimmed.contains("export ") || (trimmed.contains('=') && !trimmed.contains("==") && !trimmed.starts_with("def ")) {
                segments.push(TextSegment {
                    kind: SegmentKind::EnvDeclaration,
                    content: line.to_string(),
                    start_line: i + 1,
                    end_line: i + 1,
                });
                i += 1;
            } else if trimmed.starts_with("curl ")
                || trimmed.starts_with("docker ")
                || trimmed.starts_with("cargo ")
                || trimmed.starts_with("go ")
                || trimmed.starts_with("npm ")
                || trimmed.starts_with("python ")
            {
                segments.push(TextSegment {
                    kind: SegmentKind::Command,
                    content: line.to_string(),
                    start_line: i + 1,
                    end_line: i + 1,
                });
                i += 1;
            } else {
                segments.push(TextSegment {
                    kind: SegmentKind::Prose,
                    content: line.to_string(),
                    start_line: i + 1,
                    end_line: i + 1,
                });
                i += 1;
            }
        }
        segments
    }
}

/// Registry-driven trait for deterministic intent evidence extraction.
pub trait IntentEvidenceAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn can_handle(&self, path: &Path, content: &str) -> bool;
    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)>;
}

/// Deterministic adapter for Bash scripts and automation.
pub struct BashEvidenceAdapter;

impl IntentEvidenceAdapter for BashEvidenceAdapter {
    fn name(&self) -> &'static str {
        "bash_automation_adapter"
    }

    fn can_handle(&self, path: &Path, content: &str) -> bool {
        path.extension().map_or(false, |ext| ext == "sh" || ext == "bash")
            || content.starts_with("#!/bin/bash")
            || content.starts_with("#!/bin/sh")
    }

    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)> {
        let mut results = Vec::new();
        let ev = IntentEvidence::new(
            "script_file",
            path,
            content,
            self.name(),
            GroundingTier::Deterministic,
        );

        let mut claims = Vec::new();
        if content.contains("set -e") || content.contains("set -o pipefail") {
            let mut claim = IntentClaim::new(
                IntentScope::ScriptUnit,
                path.to_string_lossy(),
                IntentCategory::Operations,
                "Script enforces fail-fast execution policy (set -e/pipefail)",
                GroundingTier::Deterministic,
            );
            claim.evidence_ids.push(ev.id.clone());
            claims.push(claim);
        }

        for line in content.lines() {
            let t = line.trim();
            if t.starts_with("export ") || (t.contains('=') && !t.contains("==") && !t.starts_with('#')) {
                if let Some((k, _)) = t.trim_start_matches("export ").split_once('=') {
                    let var_name = k.trim();
                    if !var_name.is_empty() && var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                        let mut claim = IntentClaim::new(
                            IntentScope::Workflow,
                            path.to_string_lossy(),
                            IntentCategory::Configuration,
                            format!("Requires environment configuration `{var_name}`"),
                            GroundingTier::Deterministic,
                        );
                        claim.evidence_ids.push(ev.id.clone());
                        claims.push(claim);
                    }
                }
            }
        }

        results.push((ev, claims));
        results
    }
}

/// Deterministic adapter for Dockerfile and Docker Compose manifests.
pub struct ContainerEvidenceAdapter;

impl IntentEvidenceAdapter for ContainerEvidenceAdapter {
    fn name(&self) -> &'static str {
        "container_evidence_adapter"
    }

    fn can_handle(&self, path: &Path, _content: &str) -> bool {
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        filename == "Dockerfile"
            || filename.starts_with("Dockerfile.")
            || filename == "docker-compose.yml"
            || filename == "docker-compose.yaml"
            || filename == "compose.yaml"
    }

    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)> {
        let mut results = Vec::new();
        let ev = IntentEvidence::new(
            "container_manifest",
            path,
            content,
            self.name(),
            GroundingTier::Deterministic,
        );

        let mut claims = Vec::new();
        for line in content.lines() {
            let t = line.trim();
            if t.starts_with("EXPOSE ") {
                let port = t.trim_start_matches("EXPOSE ").trim();
                let mut claim = IntentClaim::new(
                    IntentScope::Service,
                    path.to_string_lossy(),
                    IntentCategory::Deployment,
                    format!("Service binds and exposes port {port}"),
                    GroundingTier::Deterministic,
                );
                claim.is_critical = true;
                claim.evidence_ids.push(ev.id.clone());
                claims.push(claim);
            } else if t.starts_with("ENTRYPOINT ") || t.starts_with("CMD ") {
                let mut claim = IntentClaim::new(
                    IntentScope::Service,
                    path.to_string_lossy(),
                    IntentCategory::Runtime,
                    format!("Container entrypoint execution: {t}"),
                    GroundingTier::Deterministic,
                );
                claim.evidence_ids.push(ev.id.clone());
                claims.push(claim);
            }
        }

        results.push((ev, claims));
        results
    }
}

/// Deterministic adapter for CI/CD workflow manifests (GitHub Actions, GitLab CI).
pub struct WorkflowEvidenceAdapter;

impl IntentEvidenceAdapter for WorkflowEvidenceAdapter {
    fn name(&self) -> &'static str {
        "workflow_ci_adapter"
    }

    fn can_handle(&self, path: &Path, _content: &str) -> bool {
        let p = path.to_string_lossy();
        p.contains(".github/workflows") || p.contains(".gitlab-ci.yml")
    }

    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)> {
        let mut results = Vec::new();
        let ev = IntentEvidence::new(
            "ci_workflow",
            path,
            content,
            self.name(),
            GroundingTier::Deterministic,
        );

        let mut claims = Vec::new();
        if content.contains("runs-on:") {
            let mut claim = IntentClaim::new(
                IntentScope::Workflow,
                path.to_string_lossy(),
                IntentCategory::Operations,
                "Workflow defines automated CI runner execution",
                GroundingTier::Deterministic,
            );
            claim.evidence_ids.push(ev.id.clone());
            claims.push(claim);
        }

        results.push((ev, claims));
        results
    }
}

/// Deterministic adapter for Terraform infrastructure files.
pub struct TerraformEvidenceAdapter;

impl IntentEvidenceAdapter for TerraformEvidenceAdapter {
    fn name(&self) -> &'static str {
        "terraform_adapter"
    }

    fn can_handle(&self, path: &Path, _content: &str) -> bool {
        path.extension().map_or(false, |ext| ext == "tf" || ext == "hcl")
    }

    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)> {
        let mut results = Vec::new();
        let ev = IntentEvidence::new(
            "terraform_manifest",
            path,
            content,
            self.name(),
            GroundingTier::Deterministic,
        );

        let mut claims = Vec::new();
        for line in content.lines() {
            let t = line.trim();
            if t.starts_with("variable ") {
                if let Some(var_name) = t.strip_prefix("variable ").and_then(|s| s.split_whitespace().next()) {
                    let clean_name = var_name.trim_matches('"');
                    let mut claim = IntentClaim::new(
                        IntentScope::Workspace,
                        path.to_string_lossy(),
                        IntentCategory::Configuration,
                        format!("Terraform declares infrastructure input variable `{clean_name}`"),
                        GroundingTier::Deterministic,
                    );
                    claim.evidence_ids.push(ev.id.clone());
                    claims.push(claim);
                }
            } else if t.starts_with("resource ") {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if parts.len() >= 3 {
                    let rtype = parts[1].trim_matches('"');
                    let rname = parts[2].trim_matches('"');
                    let mut claim = IntentClaim::new(
                        IntentScope::Service,
                        path.to_string_lossy(),
                        IntentCategory::Deployment,
                        format!("Infrastructure provisions resource `{rtype}` named `{rname}`"),
                        GroundingTier::Deterministic,
                    );
                    claim.is_critical = true;
                    claim.evidence_ids.push(ev.id.clone());
                    claims.push(claim);
                }
            }
        }

        results.push((ev, claims));
        results
    }
}

/// Deterministic adapter for Documentation and README fragments.
pub struct DocumentationEvidenceAdapter;

impl IntentEvidenceAdapter for DocumentationEvidenceAdapter {
    fn name(&self) -> &'static str {
        "documentation_adapter"
    }

    fn can_handle(&self, path: &Path, _content: &str) -> bool {
        path.extension().map_or(false, |ext| ext == "md" || ext == "txt" || ext == "rst")
    }

    fn extract(&self, path: &Path, content: &str) -> Vec<(IntentEvidence, Vec<IntentClaim>)> {
        let mut results = Vec::new();
        let ev = IntentEvidence::new(
            "documentation",
            path,
            content,
            self.name(),
            GroundingTier::Deterministic,
        );

        let mut claims = Vec::new();
        let segments = TextFirstSegmenter::segment(content);
        for seg in segments {
            if let SegmentKind::CodeFence { language } = &seg.kind {
                if language == "bash" || language == "sh" {
                    let mut claim = IntentClaim::new(
                        IntentScope::Workflow,
                        path.to_string_lossy(),
                        IntentCategory::Operations,
                        format!("Documentation specifies execution recipe:\n{}", seg.content),
                        GroundingTier::Deterministic,
                    );
                    claim.evidence_ids.push(ev.id.clone());
                    claims.push(claim);
                }
            }
        }

        results.push((ev, claims));
        results
    }
}

/// Orchestrator for the Sense Phase.
pub struct SenseOrchestrator {
    adapters: Vec<Box<dyn IntentEvidenceAdapter>>,
}

impl Default for SenseOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SenseOrchestrator {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(BashEvidenceAdapter),
                Box::new(ContainerEvidenceAdapter),
                Box::new(WorkflowEvidenceAdapter),
                Box::new(TerraformEvidenceAdapter),
                Box::new(DocumentationEvidenceAdapter),
            ],
        }
    }

    pub fn analyze_file(
        &self,
        path: &Path,
        content: &str,
        graph: &mut MigrationIntentGraph,
    ) {
        for adapter in &self.adapters {
            if adapter.can_handle(path, content) {
                let extracted = adapter.extract(path, content);
                for (evidence, claims) in extracted {
                    let ev_id = graph.add_evidence(evidence);
                    for mut claim in claims {
                        if !claim.evidence_ids.contains(&ev_id) {
                            claim.evidence_ids.push(ev_id.clone());
                        }
                        let cl_id = graph.add_claim(claim);
                        graph.add_edge(ev_id.clone(), cl_id, IntentEdgeKind::DerivesFrom);
                    }
                }
            }
        }
    }

    /// Reconciles graph and detects conflicting claims without dropping contradictions.
    pub fn reconcile_and_detect_conflicts(&self, graph: &mut MigrationIntentGraph) -> Vec<String> {
        let mut conflict_reports = Vec::new();
        let mut port_bindings: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for claim in graph.claims.values() {
            if claim.structured_meaning.contains("exposes port ") {
                if let Some(port) = claim.structured_meaning.split("exposes port ").nth(1) {
                    port_bindings.entry(port.trim().to_string()).or_default().push(claim.id.clone());
                }
            }
        }

        for (port, claim_ids) in port_bindings {
            if claim_ids.len() > 1 {
                conflict_reports.push(format!("Conflicting service port binding for port {port} across claims: {:?}", claim_ids));
                for cid in &claim_ids {
                    if let Some(c) = graph.claims.get_mut(cid) {
                        c.review_status = ClaimReviewStatus::Conflicted;
                        c.conflicts.extend(claim_ids.clone());
                    }
                }
            }
        }

        conflict_reports
    }
}
