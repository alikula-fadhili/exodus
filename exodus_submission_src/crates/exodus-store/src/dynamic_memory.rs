//! Dynamic Living Memory layer for Project Exodus.
//!
//! Stores and queries data-driven architectural patterns, framework definitions,
//! dynamic agent role blueprints with tunable parameters, ABAC scoping policies,
//! and cloud-native SDLC checklists in embedded SurrealDB.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use exodus_core::{Result, TargetLanguageSpecRecord};
use exodus_toolchain::DomainArchetype;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Permitted or restricted actions under the ABAC Policy Guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyAction {
    ReadFile,
    WriteFile,
    ExecuteCompiler,
    RunTest,
    ModifySchema,
    ScaffoldConfig,
}

/// Decision returned by the ABAC policy evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Declarative Attribute-Based Access Control (ABAC) policy for an agent role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolePolicyRecord {
    pub id: String,
    pub role_id: String,
    pub allowed_path_globs: Vec<String>,
    pub denied_path_globs: Vec<String>,
    pub allowed_actions: Vec<PolicyAction>,
    pub denied_actions: Vec<PolicyAction>,
    pub allow_cross_file_mutation: bool,
}

impl Default for RolePolicyRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            role_id: String::new(),
            allowed_path_globs: vec!["**/*".to_string()],
            denied_path_globs: Vec::new(),
            allowed_actions: vec![
                PolicyAction::ReadFile,
                PolicyAction::WriteFile,
                PolicyAction::RunTest,
            ],
            denied_actions: Vec::new(),
            allow_cross_file_mutation: false,
        }
    }
}

impl RolePolicyRecord {
    /// Evaluates whether an action on a given resource path is permitted under this policy.
    pub fn evaluate(&self, resource_path: &Path, action: PolicyAction) -> PolicyDecision {
        if self.denied_actions.contains(&action) {
            return PolicyDecision::Deny {
                reason: format!(
                    "Action {:?} is explicitly denied for role `{}`",
                    action, self.role_id
                ),
            };
        }

        if !self.allowed_actions.is_empty() && !self.allowed_actions.contains(&action) {
            return PolicyDecision::Deny {
                reason: format!(
                    "Action {:?} is not in the allowed actions for role `{}`",
                    action, self.role_id
                ),
            };
        }

        let path_str = resource_path.to_string_lossy();

        // Check explicit deny globs first
        for pattern in &self.denied_path_globs {
            if glob_match(pattern, &path_str) {
                return PolicyDecision::Deny {
                    reason: format!(
                        "Resource path `{}` matches denied pattern `{}` for role `{}`",
                        path_str, pattern, self.role_id
                    ),
                };
            }
        }

        // Check allowed globs
        let mut path_allowed = false;
        for pattern in &self.allowed_path_globs {
            if glob_match(pattern, &path_str) {
                path_allowed = true;
                break;
            }
        }

        if path_allowed {
            PolicyDecision::Allow
        } else {
            PolicyDecision::Deny {
                reason: format!(
                    "Resource path `{}` is not covered by any allowed pattern for role `{}`",
                    path_str, self.role_id
                ),
            }
        }
    }
}

/// Simple glob matching helper for path policies (* and **).
fn glob_match(pattern: &str, path: &str) -> bool {
    if pattern == "**/*" || pattern == "*" {
        return true;
    }
    let p_clean = pattern.trim_start_matches("./");
    let target = path.trim_start_matches("./");

    if let Some(prefix) = p_clean.strip_suffix("/**") {
        return target.starts_with(prefix);
    }
    if let Some(suffix) = p_clean.strip_prefix("**/") {
        return target.ends_with(suffix) || target.contains(&format!("/{suffix}"));
    }
    if let Some(suffix) = p_clean.strip_prefix("*.") {
        return target.ends_with(&format!(".{suffix}"));
    }
    target.contains(p_clean)
}

/// Tunable runtime execution parameters for a dynamic agent role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleParameters {
    pub model_tier: String, // "Standard" | "Reasoning" | "Frontier"
    pub temperature: f32,
    pub max_tokens: usize,
    pub timeout_seconds: u64,
    pub concurrency_limit: usize,
    pub custom_flags: HashMap<String, String>,
}

impl Default for RoleParameters {
    fn default() -> Self {
        Self {
            model_tier: "Standard".to_string(),
            temperature: 0.1,
            max_tokens: 16_000,
            timeout_seconds: 45,
            concurrency_limit: 4,
            custom_flags: HashMap::new(),
        }
    }
}

/// Dynamic, data-driven Agent Role definition persisted in the Embedded DB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicRoleDefinitionRecord {
    pub id: String,
    pub name: String,
    pub target_archetype: DomainArchetype,
    pub parameters: RoleParameters,
    pub domain_focus: String,
    pub system_prompt_template: String,
    pub abac_policy: RolePolicyRecord,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic architectural pattern persisted in the embedded database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchitecturalPatternRecord {
    pub id: String,
    pub archetype: DomainArchetype,
    pub source_lang: String,
    pub target_lang: String,
    pub thesis_title: String,
    pub thesis_hypothesis: String,
    pub target_runtime: String,
    pub target_patterns: Vec<String>,
    pub missing_capabilities: Vec<String>,
    pub recommendations: Vec<String>,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic target framework mapping definition stored in the embedded database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameworkDefinitionRecord {
    pub id: String,
    pub source_framework: String,
    pub target_lang: String,
    pub target_framework: String,
    pub package_dependencies: Vec<String>,
    pub idiomatic_replacements: HashMap<String, String>,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// Category of repository setup skill or blueprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCategory {
    Architecture,
    Toolchain,
    DevOps,
    Observability,
    Testing,
    Documentation,
}

/// A repository setup skill or blueprint providing actionable guidance and file scaffolding templates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoSetupSkillRecord {
    pub id: String,
    pub name: String,
    pub category: SkillCategory,
    pub target_language: String, // "rust" | "go" | "typescript" | "python" | "any"
    pub target_archetype: DomainArchetype,
    pub description: String,
    pub setup_guidelines: String,
    pub recommended_scaffold_files: Vec<String>,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic SDLC Checklist check stored in the embedded database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdlcChecklistRecord {
    pub id: String,
    pub category: String,
    pub name: String,
    pub description: String,
    pub default_weight: u32,
    pub remediation_guidance: String,
    pub version: String,
}

/// Category classification for host toolchains and development environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HostToolCategory {
    VCS,
    Compiler,
    PackageManager,
    Runtime,
    ContainerEngine,
    AgentCLI,
    Linter,
    Custom,
}

/// Dynamic host toolchain or agent provider registered in the embedded database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostToolDefinitionRecord {
    pub id: String,
    pub name: String,
    pub category: HostToolCategory,
    pub binary_names: Vec<String>,
    pub version_flag: String,
    pub install_guidance: String,
    pub supported_lanes: Vec<String>,
    pub is_agent_provider: bool,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// Dynamic runtime execution and probing state for a host tool recorded in memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostToolStateRecord {
    pub tool_id: String,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub status: String,
    pub execution_mode: String,
    pub probed_at: DateTime<Utc>,
}

/// Interface for querying and updating dynamic living memory in embedded storage.
#[async_trait]
pub trait DynamicLivingMemory: Send + Sync {
    async fn get_role(&self, role_id: &str) -> Result<Option<DynamicRoleDefinitionRecord>>;
    async fn list_roles_for_archetype(
        &self,
        archetype: DomainArchetype,
    ) -> Result<Vec<DynamicRoleDefinitionRecord>>;
    async fn list_all_roles(&self) -> Result<Vec<DynamicRoleDefinitionRecord>>;
    async fn upsert_role(&self, role: &DynamicRoleDefinitionRecord) -> Result<()>;

    async fn get_pattern(
        &self,
        source_lang: &str,
        target_lang: &str,
        archetype: DomainArchetype,
    ) -> Result<Option<ArchitecturalPatternRecord>>;
    async fn upsert_pattern(&self, pattern: &ArchitecturalPatternRecord) -> Result<()>;
    async fn list_patterns(&self) -> Result<Vec<ArchitecturalPatternRecord>>;

    async fn get_framework_definition(
        &self,
        source_framework: &str,
        target_lang: &str,
    ) -> Result<Option<FrameworkDefinitionRecord>>;
    async fn upsert_framework_definition(&self, def: &FrameworkDefinitionRecord) -> Result<()>;

    async fn list_sdlc_checklist(&self) -> Result<Vec<SdlcChecklistRecord>>;
    async fn upsert_sdlc_checklist_item(&self, item: &SdlcChecklistRecord) -> Result<()>;

    async fn get_skill(&self, skill_id: &str) -> Result<Option<RepoSetupSkillRecord>>;
    async fn get_skills_for_target(
        &self,
        target_lang: &str,
        archetype: &DomainArchetype,
    ) -> Result<Vec<RepoSetupSkillRecord>>;
    async fn list_skills(&self) -> Result<Vec<RepoSetupSkillRecord>>;
    async fn upsert_skill(&self, skill: &RepoSetupSkillRecord) -> Result<()>;

    async fn get_tool_definition(&self, tool_id: &str) -> Result<Option<HostToolDefinitionRecord>>;
    async fn list_tool_definitions(&self) -> Result<Vec<HostToolDefinitionRecord>>;
    async fn list_tools_for_lane(&self, lane: &str) -> Result<Vec<HostToolDefinitionRecord>>;
    async fn list_agent_tool_definitions(&self) -> Result<Vec<HostToolDefinitionRecord>>;
    async fn upsert_tool_definition(&self, tool: &HostToolDefinitionRecord) -> Result<()>;

    async fn record_tool_state(&self, state: &HostToolStateRecord) -> Result<()>;
    async fn get_tool_state(&self, tool_id: &str) -> Result<Option<HostToolStateRecord>>;
    async fn list_tool_states(&self) -> Result<Vec<HostToolStateRecord>>;
    async fn probe_and_sync_tool_states(&self) -> Result<Vec<HostToolStateRecord>>;

    async fn get_language_spec(&self, query: &str) -> Result<Option<TargetLanguageSpecRecord>>;
    async fn list_language_specs(&self) -> Result<Vec<TargetLanguageSpecRecord>>;
    async fn upsert_language_spec(&self, spec: &TargetLanguageSpecRecord) -> Result<()>;
    async fn reset_language_specs(&self) -> Result<()>;

    async fn ensure_seeded(&self) -> Result<()>;
}

/// In-memory & Surreal-backed implementation of DynamicLivingMemory.
#[derive(Clone)]
pub struct InMemoryLivingMemory {
    roles: Arc<RwLock<HashMap<String, DynamicRoleDefinitionRecord>>>,
    patterns: Arc<RwLock<HashMap<String, ArchitecturalPatternRecord>>>,
    frameworks: Arc<RwLock<HashMap<String, FrameworkDefinitionRecord>>>,
    checklists: Arc<RwLock<HashMap<String, SdlcChecklistRecord>>>,
    skills: Arc<RwLock<HashMap<String, RepoSetupSkillRecord>>>,
    tools: Arc<RwLock<HashMap<String, HostToolDefinitionRecord>>>,
    tool_states: Arc<RwLock<HashMap<String, HostToolStateRecord>>>,
    languages: Arc<RwLock<HashMap<String, TargetLanguageSpecRecord>>>,
}

impl InMemoryLivingMemory {
    pub fn new() -> Self {
        let roles = Self::seed_default_roles();
        let patterns = Self::seed_default_patterns();
        let skills = Self::seed_default_skills();
        let tools = Self::seed_default_tools();
        let languages = Self::seed_default_languages();
        Self {
            roles: Arc::new(RwLock::new(roles)),
            patterns: Arc::new(RwLock::new(patterns)),
            frameworks: Arc::new(RwLock::new(HashMap::new())),
            checklists: Arc::new(RwLock::new(HashMap::new())),
            skills: Arc::new(RwLock::new(skills)),
            tools: Arc::new(RwLock::new(tools)),
            tool_states: Arc::new(RwLock::new(HashMap::new())),
            languages: Arc::new(RwLock::new(languages)),
        }
    }

    pub fn seed_default_languages() -> HashMap<String, TargetLanguageSpecRecord> {
        TargetLanguageSpecRecord::default_specs()
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect()
    }

    pub async fn ensure_seeded(&self) -> Result<()> {
        let mut r = self.roles.write().await;
        if r.is_empty() {
            *r = Self::seed_default_roles();
        }
        let mut p = self.patterns.write().await;
        if p.is_empty() {
            *p = Self::seed_default_patterns();
        }
        let mut s = self.skills.write().await;
        if s.is_empty() {
            *s = Self::seed_default_skills();
        }
        Ok(())
    }

    fn seed_default_roles() -> HashMap<String, DynamicRoleDefinitionRecord> {
        let now = Utc::now();

        // 1. Seed Dynamic Roles with ABAC Policies & Parameters
        let mut roles_map = HashMap::new();

        // Lead Architect
        let lead_architect = DynamicRoleDefinitionRecord {
            id: "lead_architect".to_string(),
            name: "Lead System Architect".to_string(),
            target_archetype: DomainArchetype::Unknown,
            parameters: RoleParameters {
                model_tier: "Frontier".to_string(),
                temperature: 0.2,
                max_tokens: 32_000,
                timeout_seconds: 60,
                concurrency_limit: 1,
                custom_flags: HashMap::new(),
            },
            domain_focus: "Overall system design, interface contracts, wave planning, and thesis formulation".to_string(),
            system_prompt_template: "You are the Lead System Architect for Project Exodus. Analyze the codebase deeply, formulate empirical architecture hypotheses, and coordinate specialized engineering subagents.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_lead_architect".to_string(),
                role_id: "lead_architect".to_string(),
                allowed_path_globs: vec!["**/*".to_string()],
                denied_path_globs: Vec::new(),
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::RunTest, PolicyAction::ExecuteCompiler],
                denied_actions: Vec::new(),
                allow_cross_file_mutation: true,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(lead_architect.id.clone(), lead_architect);

        // Backend Engineer
        let backend_engineer = DynamicRoleDefinitionRecord {
            id: "backend_engineer".to_string(),
            name: "Backend & API Engineer".to_string(),
            target_archetype: DomainArchetype::BackendService,
            parameters: RoleParameters {
                model_tier: "Reasoning".to_string(),
                temperature: 0.05,
                max_tokens: 16_000,
                timeout_seconds: 45,
                concurrency_limit: 4,
                custom_flags: HashMap::new(),
            },
            domain_focus: "REST/gRPC routing, controller handlers, domain business logic, and request lifecycles".to_string(),
            system_prompt_template: "You are the Backend & API Engineer. Migrate route handlers and business logic from {source_lang} to idiomatic {target_lang}.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_backend_engineer".to_string(),
                role_id: "backend_engineer".to_string(),
                allowed_path_globs: vec!["src/**/*.rs".to_string(), "src/**/*.go".to_string(), "src/**/*.ts".to_string(), "**/*.py".to_string()],
                denied_path_globs: vec!["Dockerfile".to_string(), ".github/**".to_string(), ".env*".to_string(), "migrations/**".to_string()],
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::ExecuteCompiler, PolicyAction::RunTest],
                denied_actions: vec![PolicyAction::ModifySchema, PolicyAction::ScaffoldConfig],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(backend_engineer.id.clone(), backend_engineer);

        // Database / Persistence Engineer
        let database_engineer = DynamicRoleDefinitionRecord {
            id: "database_engineer".to_string(),
            name: "Database & Persistence Engineer".to_string(),
            target_archetype: DomainArchetype::BackendService,
            parameters: RoleParameters {
                model_tier: "Reasoning".to_string(),
                temperature: 0.0,
                max_tokens: 16_000,
                timeout_seconds: 45,
                concurrency_limit: 2,
                custom_flags: HashMap::new(),
            },
            domain_focus: "Connection pooling, SQL query migration, ORM/query builder layer, and transactional boundaries".to_string(),
            system_prompt_template: "You are the Database & Persistence Engineer. Migrate data access layers, connection pools, and database queries from {source_lang} to idiomatic {target_lang}.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_database_engineer".to_string(),
                role_id: "database_engineer".to_string(),
                allowed_path_globs: vec!["**/db/**".to_string(), "**/models/**".to_string(), "**/repositories/**".to_string(), "**/schema/**".to_string(), "**/migrations/**".to_string()],
                denied_path_globs: vec!["Dockerfile".to_string(), ".github/**".to_string(), "**/frontend/**".to_string()],
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::ModifySchema, PolicyAction::RunTest],
                denied_actions: vec![PolicyAction::ScaffoldConfig],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(database_engineer.id.clone(), database_engineer);

        // DevOps & SRE Engineer
        let devops_engineer = DynamicRoleDefinitionRecord {
            id: "devops_sre_engineer".to_string(),
            name: "DevOps & Cloud-Native SRE Engineer".to_string(),
            target_archetype: DomainArchetype::BackendService,
            parameters: RoleParameters {
                model_tier: "Standard".to_string(),
                temperature: 0.0,
                max_tokens: 8_000,
                timeout_seconds: 30,
                concurrency_limit: 2,
                custom_flags: HashMap::new(),
            },
            domain_focus: "Containerization (distroless Dockerfile), 12-factor configuration (.env.example), health probes (/healthz), and CI/CD pipelines".to_string(),
            system_prompt_template: "You are the DevOps & SRE Engineer. Generate cloud-native containerfiles, GitHub Actions workflows, and 12-factor environment templates for {target_lang}.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_devops_sre".to_string(),
                role_id: "devops_sre_engineer".to_string(),
                allowed_path_globs: vec!["Dockerfile".to_string(), "Containerfile".to_string(), ".github/**".to_string(), ".env*".to_string(), ".tool-versions".to_string(), ".mise.toml".to_string()],
                denied_path_globs: vec!["src/api/**".to_string(), "src/models/**".to_string(), "src/routes/**".to_string()],
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::ScaffoldConfig],
                denied_actions: vec![PolicyAction::ModifySchema],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(devops_engineer.id.clone(), devops_engineer);

        // Core Algorithms Engineer (for Shared Libraries)
        let core_algorithms_engineer = DynamicRoleDefinitionRecord {
            id: "core_algorithms_engineer".to_string(),
            name: "Core Algorithms & Memory Safety Engineer".to_string(),
            target_archetype: DomainArchetype::SharedLibrary,
            parameters: RoleParameters {
                model_tier: "Reasoning".to_string(),
                temperature: 0.05,
                max_tokens: 16_000,
                timeout_seconds: 45,
                concurrency_limit: 4,
                custom_flags: HashMap::new(),
            },
            domain_focus: "Pure functions, memory-safe data structures, deterministic state mutations, and zero-allocation transforms".to_string(),
            system_prompt_template: "You are the Core Algorithms Engineer. Translate algorithms and data structures to zero-overhead, memory-safe {target_lang}.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_core_algorithms".to_string(),
                role_id: "core_algorithms_engineer".to_string(),
                allowed_path_globs: vec!["src/**/*.rs".to_string(), "src/**/*.go".to_string(), "src/**/*.zig".to_string()],
                denied_path_globs: vec!["Dockerfile".to_string(), ".github/**".to_string()],
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::ExecuteCompiler, PolicyAction::RunTest],
                denied_actions: vec![PolicyAction::ModifySchema, PolicyAction::ScaffoldConfig],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(
            core_algorithms_engineer.id.clone(),
            core_algorithms_engineer,
        );

        // CLI UX Engineer
        let cli_ux_engineer = DynamicRoleDefinitionRecord {
            id: "cli_ux_engineer".to_string(),
            name: "CLI UX & POSIX Harness Engineer".to_string(),
            target_archetype: DomainArchetype::CliTool,
            parameters: RoleParameters {
                model_tier: "Standard".to_string(),
                temperature: 0.1,
                max_tokens: 12_000,
                timeout_seconds: 30,
                concurrency_limit: 2,
                custom_flags: HashMap::new(),
            },
            domain_focus: "Command-line argument parsing, terminal output formatting, POSIX exit codes, and interactive repl loops".to_string(),
            system_prompt_template: "You are the CLI UX Engineer. Translate CLI commands, flag parsers, and terminal harnesses from {source_lang} to {target_lang}.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_cli_ux".to_string(),
                role_id: "cli_ux_engineer".to_string(),
                allowed_path_globs: vec!["src/cli/**".to_string(), "src/main.*".to_string(), "src/bin/**".to_string(), "cmd/**".to_string()],
                denied_path_globs: vec!["migrations/**".to_string()],
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile, PolicyAction::ExecuteCompiler, PolicyAction::RunTest],
                denied_actions: vec![PolicyAction::ModifySchema],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        roles_map.insert(cli_ux_engineer.id.clone(), cli_ux_engineer);
        roles_map
    }

    fn seed_default_patterns() -> HashMap<String, ArchitecturalPatternRecord> {
        let now = Utc::now();
        let mut patterns_map = HashMap::new();
        let backend_pattern = ArchitecturalPatternRecord {
            id: "pattern_backend_async".to_string(),
            archetype: DomainArchetype::BackendService,
            source_lang: "python".to_string(),
            target_lang: "rust".to_string(),
            thesis_title: "Asynchronous Non-Blocking HTTP Router with Connection Pooling & Signal Trapping".to_string(),
            thesis_hypothesis: "Migrating synchronous python service to idiomatic rust async runtime eliminates thread pool starvation, reduces p99 latency to <15ms, and ensures zero-downtime rolling deploys.".to_string(),
            target_runtime: "tokio / axum".to_string(),
            target_patterns: vec![
                "Structured JSON logging via tracing subscriber".to_string(),
                "Deadpool / SQLx connection pooling".to_string(),
                "Axum Router with nested state extractor".to_string(),
            ],
            missing_capabilities: vec![
                "Liveness & Readiness Probes (/healthz, /readyz)".to_string(),
                "Graceful Shutdown Signal Trapping (SIGTERM/SIGINT)".to_string(),
                "OpenTelemetry Distributed Tracing Spans".to_string(),
                "12-Factor Externalized Configuration (.env.example)".to_string(),
                "Multi-Stage Distroless Containerfile".to_string(),
            ],
            recommendations: vec![
                "Structured JSON Logging & Distributed Tracing".to_string(),
                "12-Factor App Externalized Configuration".to_string(),
                "Graceful Shutdown & Signal Trapping".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        patterns_map.insert(
            format!(
                "{}_{}_{:?}",
                backend_pattern.source_lang, backend_pattern.target_lang, backend_pattern.archetype
            ),
            backend_pattern,
        );
        patterns_map
    }

    fn seed_default_skills() -> HashMap<String, RepoSetupSkillRecord> {
        let now = Utc::now();
        let mut skills_map = HashMap::new();

        // 1. Rust Multi-Package Workspace Setup
        let rust_ws = RepoSetupSkillRecord {
            id: "skill_rust_workspace".to_string(),
            name: "Rust Multi-Package Cargo Workspace Blueprint".to_string(),
            category: SkillCategory::Toolchain,
            target_language: "rust".to_string(),
            target_archetype: DomainArchetype::Unknown,
            description: "Standard idiomatic Rust multi-package workspace structure with shared dependencies, strict lints, and format profiles.".to_string(),
            setup_guidelines: r#"## Rust Workspace Setup Guidelines
1. Create a root `Cargo.toml` with `[workspace]` and `resolver = "2"`.
2. Organize individual crates in `crates/<crate_name>/`.
3. Hoist common dependencies into `[workspace.dependencies]` (e.g. `tokio`, `serde`, `thiserror`, `anyhow`, `tracing`).
4. Enforce workspace-wide linting:
   ```toml
   [workspace.lints.rust]
   unsafe_code = "forbid"
   missing_docs = "warn"

   [workspace.lints.clippy]
   all = "warn"
   pedantic = "warn"
   ```
5. Place standard `rustfmt.toml` (`edition = "2021"`, `max_width = 100`) at root.
"#.to_string(),
            recommended_scaffold_files: vec![
                "Cargo.toml".to_string(),
                "rustfmt.toml".to_string(),
                "clippy.toml".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(rust_ws.id.clone(), rust_ws);

        // 2. Distroless Containerization Blueprint
        let distroless = RepoSetupSkillRecord {
            id: "skill_distroless_container".to_string(),
            name: "Production Multi-Stage Distroless Container Blueprint".to_string(),
            category: SkillCategory::DevOps,
            target_language: "any".to_string(),
            target_archetype: DomainArchetype::BackendService,
            description: "Minimalist, attack-surface-minimized multi-stage container build using Google Distroless and unprivileged user UID 65532.".to_string(),
            setup_guidelines: r#"## Distroless Container Setup Guidelines
1. Stage 1 (Builder): Compile release static binary inside official compiler image.
2. Stage 2 (Runtime): Use `gcr.io/distroless/static-debian12:nonroot` or `scratch`.
3. Set `USER nonroot:nonroot` (UID 65532:65532).
4. Do not include shell (`/bin/sh`), package managers, or debug utilities in final image.
5. Provide `.dockerignore` ignoring `.git`, `target`, `.env`, and local caches.
"#.to_string(),
            recommended_scaffold_files: vec![
                "Dockerfile".to_string(),
                ".dockerignore".to_string(),
                "compose.yaml".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(distroless.id.clone(), distroless);

        // 3. OpenTelemetry Distributed Tracing & Health Probes
        let otel = RepoSetupSkillRecord {
            id: "skill_opentelemetry_observability".to_string(),
            name: "OpenTelemetry Distributed Tracing & Health Probes Blueprint".to_string(),
            category: SkillCategory::Observability,
            target_language: "any".to_string(),
            target_archetype: DomainArchetype::BackendService,
            description: "Standardized liveness/readiness probes (/healthz, /readyz), JSON structured tracing subscriber, and SIGTERM graceful signal draining.".to_string(),
            setup_guidelines: r#"## Observability & Health Probes Setup
1. Scaffold `/healthz` (200 OK shallow liveness check) and `/readyz` (deep downstream DB/cache ping check).
2. Configure `tracing-subscriber` with `tracing_subscriber::fmt::layer().json()`.
3. Propagate W3C TraceContext headers (`traceparent`) across RPC / HTTP boundaries.
4. Implement SIGTERM and SIGINT traps with `tokio::signal::unix` allowing a 15-30s drain period for in-flight requests.
"#.to_string(),
            recommended_scaffold_files: vec![
                "src/telemetry.rs".to_string(),
                "src/health.rs".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(otel.id.clone(), otel);

        // 4. Automated Multi-Stage CI Pipeline
        let ci_skill = RepoSetupSkillRecord {
            id: "skill_github_actions_ci".to_string(),
            name: "GitHub Actions High-Performance CI Blueprint".to_string(),
            category: SkillCategory::DevOps,
            target_language: "any".to_string(),
            target_archetype: DomainArchetype::Unknown,
            description: "Declarative CI workflow with caching, matrix verification, security auditing, and formatting checks.".to_string(),
            setup_guidelines: r#"## CI/CD Pipeline Setup Guidelines
1. Trigger on `push` and `pull_request` to `main`.
2. Run matrix checks: `check`, `fmt -- --check`, `clippy -- -D warnings`, `test`.
3. Incorporate `cargo-audit` for supply chain vulnerability scanning.
4. Leverage caching actions (`Swatinem/rust-cache` or `actions/cache`) to minimize build latency.
"#.to_string(),
            recommended_scaffold_files: vec![
                ".github/workflows/ci.yml".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(ci_skill.id.clone(), ci_skill);

        // 5. OpenWiki Documentation Setup
        let openwiki_skill = RepoSetupSkillRecord {
            id: "skill_openwiki_docs".to_string(),
            name: "OpenWiki Architecture & Grounded Knowledge Base Blueprint".to_string(),
            category: SkillCategory::Documentation,
            target_language: "any".to_string(),
            target_archetype: DomainArchetype::Unknown,
            description: "Grounded documentation repository layout with claims validation, architecture maps, and quickstart task routing.".to_string(),
            setup_guidelines: r#"## OpenWiki Documentation Setup Guidelines
1. Initialize `.agents/skills/openwiki/SKILL.md` skill definition.
2. Structure documentation hierarchically under `openwiki/` (e.g., `openwiki/architecture/`, `openwiki/operations/`, `openwiki/quickstart.md`).
3. Ground every claim with repository source evidence citations.
4. Ensure `openwiki/quickstart.md` provides an operational task-routing map.
"#.to_string(),
            recommended_scaffold_files: vec![
                ".agents/skills/openwiki/SKILL.md".to_string(),
                "openwiki/quickstart.md".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(openwiki_skill.id.clone(), openwiki_skill);

        // 6. Go Standard Project Layout
        let go_layout = RepoSetupSkillRecord {
            id: "skill_go_standard_layout".to_string(),
            name: "Idiomatic Go Standard Project Layout Blueprint".to_string(),
            category: SkillCategory::Toolchain,
            target_language: "go".to_string(),
            target_archetype: DomainArchetype::Unknown,
            description: "Standard Go project directory layout with cmd/, internal/, pkg/, go.work, and golangci-lint.".to_string(),
            setup_guidelines: r#"## Go Project Layout Setup Guidelines
1. Place executable entrypoints in `cmd/<app_name>/main.go`.
2. Keep private domain packages in `internal/` to forbid external imports.
3. Expose reusable public library code in `pkg/`.
4. Add `.golangci.yml` configuring `errcheck`, `govet`, and `staticcheck`.
5. Use `go.mod` and `go.work` for multi-module repositories.
"#.to_string(),
            recommended_scaffold_files: vec![
                "go.mod".to_string(),
                ".golangci.yml".to_string(),
            ],
            version: "1.0.0".to_string(),
            updated_at: now,
        };
        skills_map.insert(go_layout.id.clone(), go_layout);

        skills_map
    }

    fn seed_default_tools() -> HashMap<String, HostToolDefinitionRecord> {
        let now = Utc::now();
        let mut tools_map = HashMap::new();

        let tools = vec![
            HostToolDefinitionRecord {
                id: "tool_git".to_string(),
                name: "Git VCS".to_string(),
                category: HostToolCategory::VCS,
                binary_names: vec!["git".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install git from https://git-scm.com".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_cargo".to_string(),
                name: "Cargo (Rust)".to_string(),
                category: HostToolCategory::PackageManager,
                binary_names: vec!["cargo".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Rust via rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh".to_string(),
                supported_lanes: vec!["target:rust".to_string(), "source:rust".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_rustc".to_string(),
                name: "Rustc Compiler".to_string(),
                category: HostToolCategory::Compiler,
                binary_names: vec!["rustc".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Rust via rustup".to_string(),
                supported_lanes: vec!["target:rust".to_string(), "source:rust".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_python".to_string(),
                name: "Python 3 Runtime".to_string(),
                category: HostToolCategory::Runtime,
                binary_names: vec!["python3".to_string(), "python".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Python 3 from https://python.org or package manager".to_string(),
                supported_lanes: vec!["source:python".to_string(), "target:python".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_node".to_string(),
                name: "Node.js Runtime".to_string(),
                category: HostToolCategory::Runtime,
                binary_names: vec!["node".to_string(), "nodejs".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Node.js from https://nodejs.org".to_string(),
                supported_lanes: vec!["source:typescript".to_string(), "target:typescript".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_go".to_string(),
                name: "Go Toolchain".to_string(),
                category: HostToolCategory::Compiler,
                binary_names: vec!["go".to_string()],
                version_flag: "version".to_string(),
                install_guidance: "Install Go from https://golang.org/dl".to_string(),
                supported_lanes: vec!["target:go".to_string(), "source:go".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_zig".to_string(),
                name: "Zig Compiler".to_string(),
                category: HostToolCategory::Compiler,
                binary_names: vec!["zig".to_string()],
                version_flag: "version".to_string(),
                install_guidance: "Install Zig from https://ziglang.org/download".to_string(),
                supported_lanes: vec!["target:zig".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_bun".to_string(),
                name: "Bun JavaScript/TypeScript Runtime".to_string(),
                category: HostToolCategory::Runtime,
                binary_names: vec!["bun".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Bun: curl -fsSL https://bun.sh/install | bash".to_string(),
                supported_lanes: vec!["target:typescript".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_deno".to_string(),
                name: "Deno Runtime".to_string(),
                category: HostToolCategory::Runtime,
                binary_names: vec!["deno".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Deno from https://deno.land".to_string(),
                supported_lanes: vec!["target:typescript".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_java".to_string(),
                name: "Java Development Kit (JDK)".to_string(),
                category: HostToolCategory::Compiler,
                binary_names: vec!["javac".to_string(), "java".to_string()],
                version_flag: "-version".to_string(),
                install_guidance: "Install OpenJDK 17+ or Eclipse Temurin".to_string(),
                supported_lanes: vec!["source:java".to_string(), "target:java".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_kotlinc".to_string(),
                name: "Kotlin Compiler".to_string(),
                category: HostToolCategory::Compiler,
                binary_names: vec!["kotlinc".to_string()],
                version_flag: "-version".to_string(),
                install_guidance: "Install Kotlin via SDKMAN: sdk install kotlin".to_string(),
                supported_lanes: vec!["source:kotlin".to_string(), "target:kotlin".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_docker".to_string(),
                name: "Docker Container Engine".to_string(),
                category: HostToolCategory::ContainerEngine,
                binary_names: vec!["docker".to_string(), "podman".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Docker from https://docker.com or Podman".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: false,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_agy".to_string(),
                name: "Antigravity Agent CLI".to_string(),
                category: HostToolCategory::AgentCLI,
                binary_names: vec!["agy".to_string(), "antigravity".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Antigravity CLI ecosystem harness".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: true,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_claude".to_string(),
                name: "Claude Code CLI".to_string(),
                category: HostToolCategory::AgentCLI,
                binary_names: vec!["claude".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Claude Code CLI via npm: npm i -g @anthropic-ai/claude-code".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: true,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_codex".to_string(),
                name: "OpenAI Codex CLI".to_string(),
                category: HostToolCategory::AgentCLI,
                binary_names: vec!["codex".to_string(), "openai".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install OpenAI CLI via pip: pip install openai".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: true,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
            HostToolDefinitionRecord {
                id: "tool_ollama".to_string(),
                name: "Ollama Local LLM Runner".to_string(),
                category: HostToolCategory::AgentCLI,
                binary_names: vec!["ollama".to_string()],
                version_flag: "--version".to_string(),
                install_guidance: "Install Ollama from https://ollama.com".to_string(),
                supported_lanes: vec!["all".to_string()],
                is_agent_provider: true,
                version: "1.0.0".to_string(),
                updated_at: now,
            },
        ];

        for t in tools {
            tools_map.insert(t.id.clone(), t);
        }

        tools_map
    }
}

impl Default for InMemoryLivingMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DynamicLivingMemory for InMemoryLivingMemory {
    async fn get_role(&self, role_id: &str) -> Result<Option<DynamicRoleDefinitionRecord>> {
        let guard = self.roles.read().await;
        Ok(guard.get(role_id).cloned())
    }

    async fn list_roles_for_archetype(
        &self,
        archetype: DomainArchetype,
    ) -> Result<Vec<DynamicRoleDefinitionRecord>> {
        let guard = self.roles.read().await;
        let mut matching: Vec<DynamicRoleDefinitionRecord> = guard
            .values()
            .filter(|r| {
                r.target_archetype == archetype
                    || r.target_archetype == DomainArchetype::Unknown
                    || r.id == "lead_architect"
            })
            .cloned()
            .collect();
        matching.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matching)
    }

    async fn list_all_roles(&self) -> Result<Vec<DynamicRoleDefinitionRecord>> {
        let guard = self.roles.read().await;
        let mut all: Vec<DynamicRoleDefinitionRecord> = guard.values().cloned().collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(all)
    }

    async fn upsert_role(&self, role: &DynamicRoleDefinitionRecord) -> Result<()> {
        let mut guard = self.roles.write().await;
        guard.insert(role.id.clone(), role.clone());
        Ok(())
    }

    async fn get_pattern(
        &self,
        source_lang: &str,
        target_lang: &str,
        archetype: DomainArchetype,
    ) -> Result<Option<ArchitecturalPatternRecord>> {
        let guard = self.patterns.read().await;
        let key = format!("{source_lang}_{target_lang}_{archetype:?}");
        if let Some(pat) = guard.get(&key) {
            return Ok(Some(pat.clone()));
        }
        // Fallback matching by archetype
        for pat in guard.values() {
            if pat.archetype == archetype {
                return Ok(Some(pat.clone()));
            }
        }
        Ok(None)
    }

    async fn upsert_pattern(&self, pattern: &ArchitecturalPatternRecord) -> Result<()> {
        let mut guard = self.patterns.write().await;
        let key = format!(
            "{}_{}_{:?}",
            pattern.source_lang, pattern.target_lang, pattern.archetype
        );
        guard.insert(key, pattern.clone());
        Ok(())
    }

    async fn list_patterns(&self) -> Result<Vec<ArchitecturalPatternRecord>> {
        let guard = self.patterns.read().await;
        Ok(guard.values().cloned().collect())
    }

    async fn get_framework_definition(
        &self,
        source_framework: &str,
        target_lang: &str,
    ) -> Result<Option<FrameworkDefinitionRecord>> {
        let guard = self.frameworks.read().await;
        let key = format!("{source_framework}_{target_lang}");
        Ok(guard.get(&key).cloned())
    }

    async fn upsert_framework_definition(&self, def: &FrameworkDefinitionRecord) -> Result<()> {
        let mut guard = self.frameworks.write().await;
        let key = format!("{}_{}", def.source_framework, def.target_lang);
        guard.insert(key, def.clone());
        Ok(())
    }

    async fn list_sdlc_checklist(&self) -> Result<Vec<SdlcChecklistRecord>> {
        let guard = self.checklists.read().await;
        Ok(guard.values().cloned().collect())
    }

    async fn upsert_sdlc_checklist_item(&self, item: &SdlcChecklistRecord) -> Result<()> {
        let mut guard = self.checklists.write().await;
        guard.insert(item.id.clone(), item.clone());
        Ok(())
    }

    async fn get_skill(&self, skill_id: &str) -> Result<Option<RepoSetupSkillRecord>> {
        let guard = self.skills.read().await;
        Ok(guard.get(skill_id).cloned())
    }

    async fn get_skills_for_target(
        &self,
        target_lang: &str,
        archetype: &DomainArchetype,
    ) -> Result<Vec<RepoSetupSkillRecord>> {
        let guard = self.skills.read().await;
        let mut matching: Vec<RepoSetupSkillRecord> = guard
            .values()
            .filter(|s| {
                let lang_match = s.target_language == "any"
                    || s.target_language.eq_ignore_ascii_case(target_lang);
                let arch_match = s.target_archetype == DomainArchetype::Unknown
                    || s.target_archetype == *archetype;
                lang_match && arch_match
            })
            .cloned()
            .collect();
        matching.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matching)
    }

    async fn list_skills(&self) -> Result<Vec<RepoSetupSkillRecord>> {
        let guard = self.skills.read().await;
        let mut all: Vec<RepoSetupSkillRecord> = guard.values().cloned().collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(all)
    }

    async fn upsert_skill(&self, skill: &RepoSetupSkillRecord) -> Result<()> {
        let mut guard = self.skills.write().await;
        guard.insert(skill.id.clone(), skill.clone());
        Ok(())
    }

    async fn get_tool_definition(&self, tool_id: &str) -> Result<Option<HostToolDefinitionRecord>> {
        let guard = self.tools.read().await;
        Ok(guard.get(tool_id).cloned())
    }

    async fn list_tool_definitions(&self) -> Result<Vec<HostToolDefinitionRecord>> {
        let guard = self.tools.read().await;
        let mut all: Vec<HostToolDefinitionRecord> = guard.values().cloned().collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(all)
    }

    async fn list_tools_for_lane(&self, lane: &str) -> Result<Vec<HostToolDefinitionRecord>> {
        let guard = self.tools.read().await;
        let mut matching: Vec<HostToolDefinitionRecord> = guard
            .values()
            .filter(|t| t.supported_lanes.iter().any(|l| l == "all" || l == lane))
            .cloned()
            .collect();
        matching.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matching)
    }

    async fn list_agent_tool_definitions(&self) -> Result<Vec<HostToolDefinitionRecord>> {
        let guard = self.tools.read().await;
        let mut matching: Vec<HostToolDefinitionRecord> = guard
            .values()
            .filter(|t| t.is_agent_provider)
            .cloned()
            .collect();
        matching.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(matching)
    }

    async fn upsert_tool_definition(&self, tool: &HostToolDefinitionRecord) -> Result<()> {
        let mut guard = self.tools.write().await;
        guard.insert(tool.id.clone(), tool.clone());
        Ok(())
    }

    async fn record_tool_state(&self, state: &HostToolStateRecord) -> Result<()> {
        let mut guard = self.tool_states.write().await;
        guard.insert(state.tool_id.clone(), state.clone());
        Ok(())
    }

    async fn get_tool_state(&self, tool_id: &str) -> Result<Option<HostToolStateRecord>> {
        let guard = self.tool_states.read().await;
        Ok(guard.get(tool_id).cloned())
    }

    async fn list_tool_states(&self) -> Result<Vec<HostToolStateRecord>> {
        let guard = self.tool_states.read().await;
        let mut all: Vec<HostToolStateRecord> = guard.values().cloned().collect();
        all.sort_by(|a, b| a.tool_id.cmp(&b.tool_id));
        Ok(all)
    }

    async fn probe_and_sync_tool_states(&self) -> Result<Vec<HostToolStateRecord>> {
        let defs = self.list_tool_definitions().await?;
        let mut recorded_states = Vec::new();
        let now = Utc::now();

        for def in defs {
            let (probed_status, probed_path, probed_ver, exec_mode) =
                exodus_toolchain::ToolchainInspector::probe_binary_dynamic(
                    &def.binary_names,
                    &def.version_flag,
                );

            let status_str = match probed_status {
                exodus_toolchain::ToolStatus::Available => "Available".to_string(),
                exodus_toolchain::ToolStatus::Missing => "Missing".to_string(),
                exodus_toolchain::ToolStatus::Incompatible(reason) => {
                    format!("Incompatible: {reason}")
                }
            };

            let mode_str = match exec_mode {
                exodus_toolchain::ExecutionMode::Native => "Native".to_string(),
                exodus_toolchain::ExecutionMode::ContainerFallback => {
                    "ContainerFallback".to_string()
                }
                exodus_toolchain::ExecutionMode::Unavailable => "Unavailable".to_string(),
            };

            let state_record = HostToolStateRecord {
                tool_id: def.id.clone(),
                executable_path: probed_path,
                version: probed_ver,
                status: status_str,
                execution_mode: mode_str,
                probed_at: now,
            };

            self.record_tool_state(&state_record).await?;
            recorded_states.push(state_record);
        }

        Ok(recorded_states)
    }

    async fn get_language_spec(&self, query: &str) -> Result<Option<TargetLanguageSpecRecord>> {
        let guard = self.languages.read().await;
        for spec in guard.values() {
            if spec.matches_query(query) {
                return Ok(Some(spec.clone()));
            }
        }
        Ok(None)
    }

    async fn list_language_specs(&self) -> Result<Vec<TargetLanguageSpecRecord>> {
        let guard = self.languages.read().await;
        let mut list: Vec<_> = guard.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    async fn upsert_language_spec(&self, spec: &TargetLanguageSpecRecord) -> Result<()> {
        let mut guard = self.languages.write().await;
        guard.insert(spec.id.clone(), spec.clone());
        Ok(())
    }

    async fn reset_language_specs(&self) -> Result<()> {
        let mut guard = self.languages.write().await;
        *guard = Self::seed_default_languages();
        Ok(())
    }

    async fn ensure_seeded(&self) -> Result<()> {
        let mut r = self.roles.write().await;
        if r.is_empty() {
            *r = Self::seed_default_roles();
        }
        let mut p = self.patterns.write().await;
        if p.is_empty() {
            *p = Self::seed_default_patterns();
        }
        let mut s = self.skills.write().await;
        if s.is_empty() {
            *s = Self::seed_default_skills();
        }
        let mut t = self.tools.write().await;
        if t.is_empty() {
            *t = Self::seed_default_tools();
        }
        let mut l = self.languages.write().await;
        if l.is_empty() {
            *l = Self::seed_default_languages();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_core::ToolchainProfile;

    #[tokio::test]
    async fn test_dynamic_living_memory_seeding_and_queries() {
        let mem = InMemoryLivingMemory::new();
        mem.ensure_seeded().await.unwrap();

        // 1. Query Lead Architect
        let lead = mem
            .get_role("lead_architect")
            .await
            .unwrap()
            .expect("Lead architect present");
        assert_eq!(lead.parameters.model_tier, "Frontier");

        // 2. Query Roles for BackendService
        let backend_roles = mem
            .list_roles_for_archetype(DomainArchetype::BackendService)
            .await
            .unwrap();
        let role_ids: Vec<_> = backend_roles.iter().map(|r| r.id.as_str()).collect();
        assert!(role_ids.contains(&"lead_architect"));
        assert!(role_ids.contains(&"backend_engineer"));
        assert!(role_ids.contains(&"database_engineer"));
        assert!(role_ids.contains(&"devops_sre_engineer"));

        // 3. ABAC Policy evaluation: DevOps writing to Dockerfile vs src/api.rs
        let devops = mem
            .get_role("devops_sre_engineer")
            .await
            .unwrap()
            .expect("DevOps present");
        let docker_decision = devops
            .abac_policy
            .evaluate(Path::new("Dockerfile"), PolicyAction::WriteFile);
        assert_eq!(docker_decision, PolicyDecision::Allow);

        let code_decision = devops
            .abac_policy
            .evaluate(Path::new("src/api/routes.rs"), PolicyAction::WriteFile);
        assert!(matches!(code_decision, PolicyDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_dynamic_role_upsert_and_custom_parameters() {
        let mem = InMemoryLivingMemory::new();
        let custom_role = DynamicRoleDefinitionRecord {
            id: "security_auditor".to_string(),
            name: "Security & Compliance Auditor".to_string(),
            target_archetype: DomainArchetype::Unknown,
            parameters: RoleParameters {
                model_tier: "Reasoning".to_string(),
                temperature: 0.0,
                max_tokens: 20_000,
                timeout_seconds: 60,
                concurrency_limit: 1,
                custom_flags: HashMap::from([("audit_owasp".to_string(), "true".to_string())]),
            },
            domain_focus: "OWASP vulnerability detection and secret elimination".to_string(),
            system_prompt_template: "Audit migrated code for vulnerabilities.".to_string(),
            abac_policy: RolePolicyRecord {
                id: "policy_security".to_string(),
                role_id: "security_auditor".to_string(),
                allowed_path_globs: vec!["**/*".to_string()],
                denied_path_globs: Vec::new(),
                allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::RunTest],
                denied_actions: vec![PolicyAction::WriteFile],
                allow_cross_file_mutation: false,
            },
            version: "1.0.0".to_string(),
            updated_at: Utc::now(),
        };

        mem.upsert_role(&custom_role).await.unwrap();

        let fetched = mem
            .get_role("security_auditor")
            .await
            .unwrap()
            .expect("Auditor found");
        assert_eq!(
            fetched.parameters.custom_flags.get("audit_owasp").unwrap(),
            "true"
        );

        // Deny write for read-only auditor
        let write_decision = fetched
            .abac_policy
            .evaluate(Path::new("src/lib.rs"), PolicyAction::WriteFile);
        assert!(matches!(write_decision, PolicyDecision::Deny { .. }));
    }

    #[tokio::test]
    async fn test_dynamic_repo_setup_skills() {
        let mem = InMemoryLivingMemory::new();
        mem.ensure_seeded().await.unwrap();

        // 1. Fetch Rust skills for BackendService
        let rust_skills = mem
            .get_skills_for_target("rust", &DomainArchetype::BackendService)
            .await
            .unwrap();
        let skill_ids: Vec<_> = rust_skills.iter().map(|s| s.id.as_str()).collect();
        assert!(skill_ids.contains(&"skill_rust_workspace"));
        assert!(skill_ids.contains(&"skill_distroless_container"));
        assert!(skill_ids.contains(&"skill_opentelemetry_observability"));
        assert!(skill_ids.contains(&"skill_github_actions_ci"));
        assert!(skill_ids.contains(&"skill_openwiki_docs"));

        // 2. Fetch Go skills for SharedLibrary
        let go_skills = mem
            .get_skills_for_target("go", &DomainArchetype::SharedLibrary)
            .await
            .unwrap();
        let go_ids: Vec<_> = go_skills.iter().map(|s| s.id.as_str()).collect();
        assert!(go_ids.contains(&"skill_go_standard_layout"));
        assert!(go_ids.contains(&"skill_openwiki_docs"));
        assert!(!go_ids.contains(&"skill_rust_workspace"));

        // 3. Upsert a custom skill
        let custom_skill = RepoSetupSkillRecord {
            id: "skill_biome_formatting".to_string(),
            name: "Biome High-Performance Linter/Formatter".to_string(),
            category: SkillCategory::Toolchain,
            target_language: "typescript".to_string(),
            target_archetype: DomainArchetype::FrontendApp,
            description: "Biome unified formatting and linting setup".to_string(),
            setup_guidelines: "Install @biomejs/biome and initialize biome.json".to_string(),
            recommended_scaffold_files: vec!["biome.json".to_string()],
            version: "1.0.0".to_string(),
            updated_at: Utc::now(),
        };
        mem.upsert_skill(&custom_skill).await.unwrap();

        let fetched = mem
            .get_skill("skill_biome_formatting")
            .await
            .unwrap()
            .expect("Skill present");
        assert_eq!(fetched.name, "Biome High-Performance Linter/Formatter");
    }

    #[tokio::test]
    async fn test_dynamic_language_spec_seeding_and_mutation() {
        let mem = InMemoryLivingMemory::new();
        mem.ensure_seeded().await.unwrap();

        let specs = mem.list_language_specs().await.unwrap();
        assert!(specs.len() >= 9);

        let zig = mem
            .get_language_spec("zig")
            .await
            .unwrap()
            .expect("Zig found");
        assert_eq!(zig.manifest_name, "build.zig");
        assert_eq!(zig.file_extension, "zig");

        let kotlin = mem
            .get_language_spec("kt")
            .await
            .unwrap()
            .expect("Kotlin alias found");
        assert_eq!(kotlin.id, "kotlin");
        assert_eq!(kotlin.manifest_name, "build.gradle.kts");

        let swift_spec = TargetLanguageSpecRecord {
            id: "swift".to_string(),
            name: "Swift".to_string(),
            aliases: vec![],
            file_extension: "swift".to_string(),
            source_dir: "Sources".to_string(),
            manifest_name: "Package.swift".to_string(),
            entrypoint_filename: "main.swift".to_string(),
            ecosystem_container_term: "Package".to_string(),
            entrypoint_template: "// Autonomous Project Exodus Migrated Target (Swift)\nprint(\"Hello Swift\")\n".to_string(),
            module_stub_template: "// Migrated Swift module `{{module_name}}`\n".to_string(),
            module_export_template: None,
            package_manifest_template: "// swift-tools-version: 5.9\nimport PackageDescription\n\nlet package = Package(name: \"{{pkg_name}}\")\n".to_string(),
            workspace_manifest_filename: None,
            workspace_manifest_template: None,
            workspace_member_template: None,
            internal_dependency_template: None,
            version_files: vec![],
            quickstart_command: "swift test".to_string(),
            test_harness_template: "import XCTest\n\nfinal class Tests: XCTestCase {\n{{test_cases}}\n}\n".to_string(),
            test_case_template: "func test_{{case_id}}() {}".to_string(),
            toolchain_profile: ToolchainProfile::new("swift", ["swift", "build"]),
            signature_rule_sets: Vec::new(),
        };
        mem.upsert_language_spec(&swift_spec).await.unwrap();

        let fetched_swift = mem
            .get_language_spec("swift")
            .await
            .unwrap()
            .expect("Swift found");
        assert_eq!(fetched_swift.manifest_name, "Package.swift");
    }
}
