//! Migration plan generation, wave scheduling, risk analysis, and human approval checkpoints.

use chrono::{DateTime, Utc};
use exodus_core::{ExodusError, Result, RiskLevel};
use exodus_graph::{NodeKind, SemanticGraph};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Migration execution wave (phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MigrationWave {
    Wave0Foundations,
    Wave1DomainEntities,
    Wave2Services,
    Wave3Entrypoints,
}

impl std::fmt::Display for MigrationWave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wave0Foundations => write!(f, "Wave 0: Foundations & Leaves"),
            Self::Wave1DomainEntities => write!(f, "Wave 1: Domain Entities & Types"),
            Self::Wave2Services => write!(f, "Wave 2: Business Services & Logic"),
            Self::Wave3Entrypoints => write!(f, "Wave 3: Entry Points & Workflows"),
        }
    }
}

/// A discrete step in the migration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStep {
    pub step_id: String,
    pub symbol_id: String,
    pub symbol_name: String,
    pub node_kind: NodeKind,
    pub file_path: String,
    pub wave: MigrationWave,
    pub risk_score: u8,
    pub risk_level: RiskLevel,
    pub dependencies: Vec<String>,
    pub requires_fallback: bool,
    pub requires_human_approval: bool,
    pub acceptance_criteria: Vec<String>,
}

/// Human approval state for a migration plan.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApprovalCheckpoint {
    pub approved: bool,
    pub approved_by: Option<String>,
    pub approval_timestamp: Option<DateTime<Utc>>,
    pub required_reasons: Vec<String>,
}

/// A complete, wave-ordered migration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub plan_id: String,
    pub created_at: DateTime<Utc>,
    pub steps: Vec<MigrationStep>,
    pub circular_dependencies: Vec<Vec<String>>,
    pub high_risk_symbols: Vec<String>,
    pub unsupported_constructs: Vec<String>,
    pub approval: ApprovalCheckpoint,
    pub sdlc_audit: Option<exodus_core::SdlcAuditReport>,
    pub research_report: Option<exodus_parser::DeepResearchReport>,
    #[serde(default)]
    pub recommended_setup_skills: Vec<exodus_store::RepoSetupSkillRecord>,
}

impl MigrationPlan {
    pub fn is_approved(&self) -> bool {
        self.approval.approved
    }

    pub fn approve(&mut self, approver: &str) {
        self.approval.approved = true;
        self.approval.approved_by = Some(approver.to_string());
        self.approval.approval_timestamp = Some(Utc::now());
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(ExodusError::from)
    }

    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(ExodusError::from)
    }

    pub fn with_research_report(mut self, report: exodus_parser::DeepResearchReport) -> Self {
        self.research_report = Some(report);
        self
    }

    pub fn with_setup_skills(mut self, skills: Vec<exodus_store::RepoSetupSkillRecord>) -> Self {
        self.recommended_setup_skills = skills;
        self
    }

    pub fn format_skills_summary(&self) -> String {
        let mut out = String::new();
        if !self.recommended_setup_skills.is_empty() {
            out.push_str("🛠️ Recommended Repository Setup Skills & Blueprints:\n");
            for skill in &self.recommended_setup_skills {
                out.push_str(&format!(
                    "   • [{:?}] {} (v{})\n     └─ {}\n",
                    skill.category, skill.name, skill.version, skill.description
                ));
                if !skill.recommended_scaffold_files.is_empty() {
                    out.push_str(&format!(
                        "        Scaffold targets: {}\n",
                        skill.recommended_scaffold_files.join(", ")
                    ));
                }
            }
        }
        out
    }

    pub fn format_research_summary(&self) -> String {
        if let Some(r) = &self.research_report {
            r.format_summary()
        } else {
            String::new()
        }
    }

    pub fn format_sdlc_summary(&self) -> String {
        let mut out = String::new();
        if let Some(audit) = &self.sdlc_audit {
            out.push_str(&format!(
                "📊 SDLC Modernization Readiness Score: {}/100\n",
                audit.health_score
            ));
            if let Some(thesis) = &audit.architecture_thesis {
                out.push_str(&format!(
                    "🔬 Architecture Thesis: {}\n   • Hypothesis: {}\n",
                    thesis.target_architectural_pattern, thesis.hypothesis
                ));
            }
            if !audit.missing_capabilities.is_empty() {
                out.push_str("⚠️ Missing Cloud-Native Capabilities:\n");
                for cap in &audit.missing_capabilities {
                    out.push_str(&format!("   • {cap}\n"));
                }
            }
        }
        out
    }
}

/// A planned package migration unit inside a multi-package workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePlan {
    pub name: String,
    pub relative_path: String,
    pub source_language: String,
    pub domain_archetype: exodus_toolchain::DomainArchetype,
    pub recommended_framework: String,
    pub wave_index: usize,
    pub dependencies: Vec<String>,
    pub source_file_count: usize,
    pub lines_of_code: usize,
    pub risk_score: u8,
}

/// A comprehensive, multi-wave workspace migration plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMigrationPlan {
    pub plan_id: String,
    pub created_at: DateTime<Utc>,
    pub toolchain: String,
    pub target_language: String,
    pub package_waves: Vec<Vec<PackagePlan>>,
    pub total_packages: usize,
    pub total_lines_of_code: usize,
    pub max_trials_per_symbol: usize,
    pub approval: ApprovalCheckpoint,
    pub sdlc_audit: Option<exodus_core::SdlcAuditReport>,
    #[serde(default)]
    pub recommended_setup_skills: Vec<exodus_store::RepoSetupSkillRecord>,
}

impl WorkspaceMigrationPlan {
    pub fn is_approved(&self) -> bool {
        self.approval.approved
    }

    pub fn approve(&mut self, approver: &str) {
        self.approval.approved = true;
        self.approval.approved_by = Some(approver.to_string());
        self.approval.approval_timestamp = Some(Utc::now());
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(ExodusError::from)
    }

    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(ExodusError::from)
    }

    pub fn with_setup_skills(mut self, skills: Vec<exodus_store::RepoSetupSkillRecord>) -> Self {
        self.recommended_setup_skills = skills;
        self
    }

    pub fn format_skills_summary(&self) -> String {
        let mut out = String::new();
        if !self.recommended_setup_skills.is_empty() {
            out.push_str("🛠️ Recommended Workspace Setup Skills & Blueprints:\n");
            for skill in &self.recommended_setup_skills {
                out.push_str(&format!(
                    "   • [{:?}] {} (v{})\n     └─ {}\n",
                    skill.category, skill.name, skill.version, skill.description
                ));
            }
        }
        out
    }

    pub fn format_sdlc_summary(&self) -> String {
        let mut out = String::new();
        if let Some(audit) = &self.sdlc_audit {
            out.push_str(&format!(
                "📊 Workspace SDLC Readiness Score: {}/100\n",
                audit.health_score
            ));
            if let Some(thesis) = &audit.architecture_thesis {
                out.push_str(&format!(
                    "🔬 Architecture Thesis: {}\n   • Hypothesis: {}\n",
                    thesis.target_architectural_pattern, thesis.hypothesis
                ));
            }
            if !audit.missing_capabilities.is_empty() {
                out.push_str("⚠️ Missing Cloud-Native Capabilities:\n");
                for cap in &audit.missing_capabilities {
                    out.push_str(&format!("   • {cap}\n"));
                }
            }
        }
        out
    }
}

/// Planner responsible for wave generation, topological sequencing, and risk evaluation.
pub struct MigrationPlanner;

impl MigrationPlanner {
    pub fn new() -> Self {
        Self
    }

    /// Generates a structured migration plan from an Exodus Semantic Graph.
    pub fn generate_plan(&self, graph: &SemanticGraph) -> Result<MigrationPlan> {
        let plan_id = format!("plan-{}", Uuid::new_v4());
        let created_at = Utc::now();

        let topological_order = graph.topological_sort()?;
        let circular_dependencies = graph.detect_cycles();
        let cycle_nodes: HashSet<String> =
            circular_dependencies.iter().flatten().cloned().collect();

        let mut steps = Vec::new();
        let mut high_risk_symbols = Vec::new();
        let mut unsupported_constructs = Vec::new();
        let mut approval_reasons = Vec::new();

        if !circular_dependencies.is_empty() {
            approval_reasons.push(format!(
                "Graph contains {} circular dependency cycle(s) requiring manual architecture resolution.",
                circular_dependencies.len()
            ));
        }

        for (idx, node_id) in topological_order.iter().enumerate() {
            if let Some(node) = graph.get_node(node_id) {
                if node.kind == NodeKind::Repository {
                    continue; // Skip the root repository node in migration steps
                }

                let in_cycle = cycle_nodes.contains(node_id);
                let dependencies = graph.dependencies_of(node_id);
                let is_unsupported = node.kind == NodeKind::UnsupportedConstruct;

                if node.risk_level >= RiskLevel::High {
                    high_risk_symbols.push(node_id.clone());
                }

                if is_unsupported {
                    unsupported_constructs.push(node_id.clone());
                }

                let wave = match node.kind {
                    NodeKind::Parameter | NodeKind::State => MigrationWave::Wave0Foundations,
                    NodeKind::Type => MigrationWave::Wave1DomainEntities,
                    NodeKind::Function | NodeKind::Method => {
                        if dependencies.is_empty() {
                            MigrationWave::Wave0Foundations
                        } else {
                            MigrationWave::Wave2Services
                        }
                    }
                    NodeKind::Module => MigrationWave::Wave3Entrypoints,
                    NodeKind::UnsupportedConstruct | NodeKind::ExternalDependency => {
                        MigrationWave::Wave1DomainEntities
                    }
                    NodeKind::Repository => MigrationWave::Wave3Entrypoints,
                };

                let requires_approval =
                    in_cycle || is_unsupported || node.risk_level == RiskLevel::Critical;
                if requires_approval && in_cycle {
                    approval_reasons.push(format!(
                        "Symbol `{}` participates in a dependency cycle.",
                        node.qualified_name
                    ));
                }

                let mut acceptance_criteria = vec![
                    format!("Translate construct `{}` to valid Rust syntax", node.name),
                    "Pass rustc compilation check".to_string(),
                ];

                if !is_unsupported {
                    acceptance_criteria.push("Pass behavioral unit test contracts".to_string());
                } else {
                    acceptance_criteria
                        .push("Record explicit MigrationDebt fallback stub".to_string());
                }

                steps.push(MigrationStep {
                    step_id: format!("step-{:04}", idx + 1),
                    symbol_id: node.id.clone(),
                    symbol_name: node.name.clone(),
                    node_kind: node.kind,
                    file_path: node.file_path.clone(),
                    wave,
                    risk_score: node.risk_score,
                    risk_level: node.risk_level,
                    dependencies,
                    requires_fallback: is_unsupported,
                    requires_human_approval: requires_approval,
                    acceptance_criteria,
                });
            }
        }

        // Sort steps by wave first, then risk score ascending
        steps.sort_by(|a, b| {
            a.wave
                .cmp(&b.wave)
                .then_with(|| a.risk_score.cmp(&b.risk_score))
        });

        let approval = ApprovalCheckpoint {
            approved: approval_reasons.is_empty(), // auto-approve only if 0 risky blockers
            approved_by: None,
            approval_timestamp: None,
            required_reasons: approval_reasons,
        };

        // Determine archetype for SDLC audit from graph characteristics
        let has_functions = steps.iter().any(|s| s.node_kind == NodeKind::Function);
        let archetype = if has_functions && steps.len() > 2 {
            exodus_toolchain::DomainArchetype::BackendService
        } else {
            exodus_toolchain::DomainArchetype::SharedLibrary
        };

        let sdlc_audit = Some(
            exodus_store::ArchitectureKnowledgeCatalog::audit_system_design(
                &archetype,
                "python",
                "rust",
                !circular_dependencies.is_empty(),
            ),
        );

        Ok(MigrationPlan {
            plan_id,
            created_at,
            steps,
            circular_dependencies,
            high_risk_symbols,
            unsupported_constructs,
            approval,
            sdlc_audit,
            research_report: None,
            recommended_setup_skills: Vec::new(),
        })
    }

    /// Generates a structured multi-wave migration plan for a monorepo workspace.
    pub fn generate_workspace_plan(
        &self,
        workspace: &exodus_toolchain::WorkspaceDescriptor,
        target_language: &str,
    ) -> Result<WorkspaceMigrationPlan> {
        let plan_id = format!("ws-plan-{}", Uuid::new_v4());
        let created_at = Utc::now();
        let waves = workspace.topological_waves();

        let mut package_waves = Vec::new();
        let approval_reasons = Vec::new();

        for (wave_idx, wave_pkgs) in waves.into_iter().enumerate() {
            let mut current_wave = Vec::new();
            for pkg in wave_pkgs {
                let rec_fw = pkg.domain_archetype.recommended_framework(target_language);
                let risk_score = match pkg.domain_archetype {
                    exodus_toolchain::DomainArchetype::BackendService => 40,
                    exodus_toolchain::DomainArchetype::WorkerQueue => 35,
                    exodus_toolchain::DomainArchetype::FrontendApp => 50,
                    exodus_toolchain::DomainArchetype::CliTool => 25,
                    exodus_toolchain::DomainArchetype::SharedLibrary => 15,
                    exodus_toolchain::DomainArchetype::Unknown => 30,
                };

                current_wave.push(PackagePlan {
                    name: pkg.name.clone(),
                    relative_path: pkg.relative_path.to_string_lossy().to_string(),
                    source_language: pkg.source_language.clone(),
                    domain_archetype: pkg.domain_archetype,
                    recommended_framework: rec_fw.to_string(),
                    wave_index: wave_idx,
                    dependencies: pkg.dependencies.clone(),
                    source_file_count: pkg.source_files.len(),
                    lines_of_code: pkg.lines_of_code,
                    risk_score,
                });
            }
            package_waves.push(current_wave);
        }

        let approval = ApprovalCheckpoint {
            approved: approval_reasons.is_empty(),
            approved_by: None,
            approval_timestamp: None,
            required_reasons: approval_reasons,
        };

        let primary_archetype = workspace
            .packages
            .first()
            .map(|p| p.domain_archetype)
            .unwrap_or(exodus_toolchain::DomainArchetype::BackendService);

        let sdlc_audit = Some(
            exodus_store::ArchitectureKnowledgeCatalog::audit_system_design(
                &primary_archetype,
                "polyglot_source",
                target_language,
                false,
            ),
        );

        Ok(WorkspaceMigrationPlan {
            plan_id,
            created_at,
            toolchain: format!("{:?}", workspace.toolchain),
            target_language: target_language.to_string(),
            package_waves,
            total_packages: workspace.packages.len(),
            total_lines_of_code: workspace.total_lines_of_code,
            max_trials_per_symbol: 3,
            approval,
            sdlc_audit,
            recommended_setup_skills: Vec::new(),
        })
    }
}

impl Default for MigrationPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_graph::{NodeKind, RelationKind, SemanticEdge, SemanticNode};
    use std::collections::HashMap;

    #[test]
    fn test_planner_wave_generation() {
        let mut graph = SemanticGraph::new();

        graph.add_node(SemanticNode {
            id: "type::User".to_string(),
            name: "User".to_string(),
            kind: NodeKind::Type,
            qualified_name: "models::User".to_string(),
            file_path: "models.py".to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_node(SemanticNode {
            id: "function::get_user".to_string(),
            name: "get_user".to_string(),
            kind: NodeKind::Function,
            qualified_name: "service::get_user".to_string(),
            file_path: "service.py".to_string(),
            risk_score: 15,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_edge(SemanticEdge {
            from: "function::get_user".to_string(),
            to: "type::User".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let planner = MigrationPlanner::new();
        let plan = planner.generate_plan(&graph).unwrap();

        assert!(!plan.steps.is_empty());
        assert!(plan.is_approved()); // No cycles or unsupported constructs
    }

    #[test]
    fn test_planner_requires_approval_on_cycle() {
        let mut graph = SemanticGraph::new();

        graph.add_node(SemanticNode {
            id: "A".to_string(),
            name: "A".to_string(),
            kind: NodeKind::Function,
            qualified_name: "A".to_string(),
            file_path: "a.py".to_string(),
            risk_score: 40,
            risk_level: RiskLevel::Medium,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_node(SemanticNode {
            id: "B".to_string(),
            name: "B".to_string(),
            kind: NodeKind::Function,
            qualified_name: "B".to_string(),
            file_path: "b.py".to_string(),
            risk_score: 40,
            risk_level: RiskLevel::Medium,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_edge(SemanticEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "B".to_string(),
            to: "A".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let planner = MigrationPlanner::new();
        let mut plan = planner.generate_plan(&graph).unwrap();

        assert!(!plan.is_approved());
        assert!(!plan.approval.required_reasons.is_empty());

        plan.approve("lead_architect");
        assert!(plan.is_approved());
        assert_eq!(plan.approval.approved_by.as_deref(), Some("lead_architect"));
    }

    #[test]
    fn test_generate_workspace_plan() {
        use exodus_toolchain::{
            DomainArchetype, PackageDescriptor, WorkspaceDescriptor, WorkspaceToolchain,
        };
        use std::path::PathBuf;

        let pkg_models = PackageDescriptor {
            name: "core-models".to_string(),
            root_path: PathBuf::from("packages/core-models"),
            relative_path: PathBuf::from("packages/core-models"),
            manifest_file: None,
            source_language: "python".to_string(),
            domain_archetype: DomainArchetype::SharedLibrary,
            dependencies: vec![],
            source_files: vec![PathBuf::from("packages/core-models/user.py")],
            lines_of_code: 80,
        };

        let pkg_service = PackageDescriptor {
            name: "payment-service".to_string(),
            root_path: PathBuf::from("apps/payment-service"),
            relative_path: PathBuf::from("apps/payment-service"),
            manifest_file: None,
            source_language: "python".to_string(),
            domain_archetype: DomainArchetype::BackendService,
            dependencies: vec!["core-models".to_string()],
            source_files: vec![PathBuf::from("apps/payment-service/api.py")],
            lines_of_code: 200,
        };

        let ws = WorkspaceDescriptor {
            root_dir: PathBuf::from("."),
            toolchain: WorkspaceToolchain::Turborepo,
            packages: vec![pkg_service, pkg_models],
            dependency_graph: std::collections::HashMap::new(),
            total_lines_of_code: 280,
        };

        let planner = MigrationPlanner::new();
        let plan = planner.generate_workspace_plan(&ws, "rust").unwrap();

        assert_eq!(plan.total_packages, 2);
        assert_eq!(plan.package_waves.len(), 2);
        assert_eq!(plan.package_waves[0][0].name, "core-models");
        assert_eq!(plan.package_waves[1][0].name, "payment-service");
        assert!(plan.package_waves[1][0]
            .recommended_framework
            .contains("Axum"));
        assert!(plan.sdlc_audit.is_some());
        let summary = plan.format_sdlc_summary();
        assert!(summary.contains("SDLC Readiness Score"));
    }
}
