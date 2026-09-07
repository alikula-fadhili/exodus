//! Monorepo, multi-package workspace discovery, and domain archetype classification.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Supported monorepo and workspace build toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceToolchain {
    Turborepo,
    Moonrepo,
    Nx,
    PnpmWorkspaces,
    YarnWorkspaces,
    NpmWorkspaces,
    CargoWorkspace,
    GoWorkspace,
    GradleMultiProject,
    MavenMultiModule,
    PythonMonorepo,
    Standalone,
}

impl WorkspaceToolchain {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Turborepo => "Turborepo (turbo.json)",
            Self::Moonrepo => "Moonrepo (.moon/workspace.yml)",
            Self::Nx => "Nx Monorepo (nx.json)",
            Self::PnpmWorkspaces => "pnpm Workspaces (pnpm-workspace.yaml)",
            Self::YarnWorkspaces => "Yarn Workspaces",
            Self::NpmWorkspaces => "npm Workspaces",
            Self::CargoWorkspace => "Cargo Workspace (Cargo.toml [workspace])",
            Self::GoWorkspace => "Go Workspace (go.work)",
            Self::GradleMultiProject => "Gradle Multi-Project (settings.gradle)",
            Self::MavenMultiModule => "Maven Multi-Module (pom.xml)",
            Self::PythonMonorepo => "Python Monorepo (uv.workspace / pyproject.toml)",
            Self::Standalone => "Standalone Single-Package Repository",
        }
    }
}

/// Domain archetype identifying the architectural role of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainArchetype {
    /// REST / GraphQL / gRPC Backend API microservice (e.g. Express, FastAPI, Axum, Gin).
    BackendService,
    /// Command-line tool, CLI utility, or harness (e.g. Commander, Click, Clap, Cobra).
    CliTool,
    /// Background asynchronous worker, cron runner, or queue consumer (e.g. Celery, BullMQ).
    WorkerQueue,
    /// Shared domain entities, DTOs, utilities, and validation logic.
    SharedLibrary,
    /// Web application, SPA, or frontend bundle (e.g. Next.js, React, Vue, Svelte).
    FrontendApp,
    /// Unspecified or generic domain.
    Unknown,
}

impl DomainArchetype {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BackendService => "Backend API Service",
            Self::CliTool => "CLI Tool / Harness",
            Self::WorkerQueue => "Background Worker / Queue Consumer",
            Self::SharedLibrary => "Shared Core / Domain Library",
            Self::FrontendApp => "Frontend / Web Application",
            Self::Unknown => "Generic Package",
        }
    }

    /// Recommends idiomatic frameworks for a domain archetype in a target language,
    /// querying the embedded knowledge registry (with support for live session user overrides)
    /// and falling back to embedded defaults.
    pub fn recommended_framework(&self, target_lang: &str) -> String {
        if let Ok(reg) = global_framework_registry().read() {
            reg.get_framework(*self, target_lang)
        } else {
            Self::embedded_default_framework(*self, target_lang).to_string()
        }
    }

    /// Hardcoded embedded fallback mappings.
    pub fn embedded_default_framework(archetype: Self, target_lang: &str) -> &'static str {
        let lang = target_lang.to_lowercase();
        match (archetype, lang.as_str()) {
            (Self::BackendService, "rust" | "rs") => "Axum + Tokio + Tower",
            (Self::BackendService, "go" | "golang") => "Gin / Fiber HTTP Router",
            (Self::BackendService, "typescript" | "ts" | "javascript" | "js") => "Fastify + Zod",
            (Self::BackendService, "python" | "py") => "FastAPI + Pydantic v2",
            (Self::BackendService, "java") => "Spring Boot Web Starter",
            (Self::BackendService, "kotlin" | "kt") => "Ktor / Spring Boot",
            (Self::BackendService, "csharp" | "cs") => "ASP.NET Core Minimal APIs",

            (Self::CliTool, "rust" | "rs") => "Clap (derive) + Indicatif",
            (Self::CliTool, "go" | "golang") => "Cobra + Pflag",
            (Self::CliTool, "typescript" | "ts" | "javascript" | "js") => "Commander + Inquirer",
            (Self::CliTool, "python" | "py") => "Typer + Rich",

            (Self::WorkerQueue, "rust" | "rs") => "Tokio Tasks + Lapin (AMQP) / RDKafka",
            (Self::WorkerQueue, "go" | "golang") => "Goroutines Channels + Asynq",
            (Self::WorkerQueue, "typescript" | "ts" | "javascript" | "js") => "BullMQ + Redis",
            (Self::WorkerQueue, "python" | "py") => "Celery + Redis / Arq",

            (Self::SharedLibrary, "rust" | "rs") => "Serde + Thiserror + Anyhow",
            (Self::SharedLibrary, "go" | "golang") => "Idiomatic Go Structs with JSON tags",
            (Self::SharedLibrary, "typescript" | "ts" | "javascript" | "js") => "Zod + ts-pattern",
            (Self::SharedLibrary, "python" | "py") => "Pydantic + Dataclasses",

            (Self::FrontendApp, "rust" | "rs") => "Leptos / Dioxus Web (WASM)",
            (Self::FrontendApp, "typescript" | "ts" | "javascript" | "js") => {
                "Next.js / Vite React"
            }
            (Self::FrontendApp, "go" | "golang") => "Templ + HTMX",
            (Self::FrontendApp, "python" | "py") => "Reflex / NiceGUI / Streamlit",

            _ => "Standard Idiomatic Library Structure",
        }
    }
}

/// Persistent, learned rule mapping a domain archetype to an idiomatic framework and dependencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameworkRule {
    pub archetype: DomainArchetype,
    pub target_language: String,
    pub recommended_framework: String,
    pub default_dependencies: Vec<String>,
    pub is_user_override: bool,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Embedded Knowledge Registry for domain archetypes and framework recommendations.
/// Seeded with hardcoded defaults for offline reliability, but supports live session updates
/// and persistent overrides for novel, user-specific, or unthought-of scenarios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkRegistry {
    pub rules: Vec<FrameworkRule>,
}

static GLOBAL_FRAMEWORK_REGISTRY: std::sync::OnceLock<std::sync::RwLock<FrameworkRegistry>> =
    std::sync::OnceLock::new();

pub fn global_framework_registry() -> &'static std::sync::RwLock<FrameworkRegistry> {
    GLOBAL_FRAMEWORK_REGISTRY
        .get_or_init(|| std::sync::RwLock::new(FrameworkRegistry::embedded_defaults()))
}

impl Default for FrameworkRegistry {
    fn default() -> Self {
        Self::embedded_defaults()
    }
}

impl FrameworkRegistry {
    /// Seed embedded default framework rules across all supported target languages.
    pub fn embedded_defaults() -> Self {
        let mut rules = Vec::new();

        let add_rule = |rules: &mut Vec<FrameworkRule>,
                        arch: DomainArchetype,
                        lang: &str,
                        fw: &str,
                        deps: &[&str]| {
            rules.push(FrameworkRule {
                archetype: arch,
                target_language: lang.to_string(),
                recommended_framework: fw.to_string(),
                default_dependencies: deps.iter().map(|s| s.to_string()).collect(),
                is_user_override: false,
                notes: None,
            });
        };

        // BackendService
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "rust",
            "Axum + Tokio + Tower",
            &[
                "axum",
                "tokio",
                "tower",
                "tower-http",
                "serde",
                "serde_json",
            ],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "go",
            "Gin / Fiber HTTP Router",
            &["github.com/gin-gonic/gin"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "typescript",
            "Fastify + Zod",
            &["fastify", "zod"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "python",
            "FastAPI + Pydantic v2",
            &["fastapi", "pydantic", "uvicorn"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "java",
            "Spring Boot Web Starter",
            &["org.springframework.boot:spring-boot-starter-web"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "kotlin",
            "Ktor / Spring Boot",
            &["io.ktor:ktor-server-core"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::BackendService,
            "csharp",
            "ASP.NET Core Minimal APIs",
            &["Microsoft.AspNetCore.App"],
        );

        // CliTool
        add_rule(
            &mut rules,
            DomainArchetype::CliTool,
            "rust",
            "Clap (derive) + Indicatif",
            &["clap", "indicatif", "anyhow"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::CliTool,
            "go",
            "Cobra + Pflag",
            &["github.com/spf13/cobra"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::CliTool,
            "typescript",
            "Commander + Inquirer",
            &["commander", "inquirer"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::CliTool,
            "python",
            "Typer + Rich",
            &["typer", "rich"],
        );

        // WorkerQueue
        add_rule(
            &mut rules,
            DomainArchetype::WorkerQueue,
            "rust",
            "Tokio Tasks + Lapin (AMQP) / RDKafka",
            &["tokio", "lapin", "rdkafka"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::WorkerQueue,
            "go",
            "Goroutines Channels + Asynq",
            &["github.com/hibiken/asynq"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::WorkerQueue,
            "typescript",
            "BullMQ + Redis",
            &["bullmq", "ioredis"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::WorkerQueue,
            "python",
            "Celery + Redis / Arq",
            &["celery", "redis"],
        );

        // SharedLibrary
        add_rule(
            &mut rules,
            DomainArchetype::SharedLibrary,
            "rust",
            "Serde + Thiserror + Anyhow",
            &["serde", "thiserror", "anyhow"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::SharedLibrary,
            "go",
            "Idiomatic Go Structs with JSON tags",
            &[],
        );
        add_rule(
            &mut rules,
            DomainArchetype::SharedLibrary,
            "typescript",
            "Zod + ts-pattern",
            &["zod", "ts-pattern"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::SharedLibrary,
            "python",
            "Pydantic + Dataclasses",
            &["pydantic"],
        );

        // FrontendApp
        add_rule(
            &mut rules,
            DomainArchetype::FrontendApp,
            "rust",
            "Leptos / Dioxus Web (WASM)",
            &["leptos", "wasm-bindgen"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::FrontendApp,
            "typescript",
            "Next.js / Vite React",
            &["react", "react-dom", "next"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::FrontendApp,
            "go",
            "Templ + HTMX",
            &["github.com/a-h/templ"],
        );
        add_rule(
            &mut rules,
            DomainArchetype::FrontendApp,
            "python",
            "Reflex / NiceGUI / Streamlit",
            &["streamlit"],
        );

        Self { rules }
    }

    /// Normalize language aliases (e.g. "rs" -> "rust", "golang" -> "go", "ts" -> "typescript").
    pub fn normalize_language(lang: &str) -> String {
        match lang.to_lowercase().trim() {
            "rust" | "rs" => "rust".to_string(),
            "go" | "golang" => "go".to_string(),
            "typescript" | "ts" | "javascript" | "js" => "typescript".to_string(),
            "python" | "py" => "python".to_string(),
            "java" => "java".to_string(),
            "kotlin" | "kt" => "kotlin".to_string(),
            "csharp" | "cs" | "c#" => "csharp".to_string(),
            other => other.to_string(),
        }
    }

    /// Retrieve recommended framework, prioritizing active session user overrides before default rules.
    pub fn get_framework(&self, archetype: DomainArchetype, target_lang: &str) -> String {
        let norm_lang = Self::normalize_language(target_lang);

        // 1. Check user overrides first
        if let Some(rule) = self.rules.iter().find(|r| {
            r.is_user_override
                && r.archetype == archetype
                && Self::normalize_language(&r.target_language) == norm_lang
        }) {
            return rule.recommended_framework.clone();
        }

        // 2. Check standard rules
        if let Some(rule) = self.rules.iter().find(|r| {
            r.archetype == archetype && Self::normalize_language(&r.target_language) == norm_lang
        }) {
            return rule.recommended_framework.clone();
        }

        // 3. Fallback
        DomainArchetype::embedded_default_framework(archetype, target_lang).to_string()
    }

    /// Dynamically update or register a framework recommendation rule during a user session.
    pub fn upsert_rule(&mut self, rule: FrameworkRule) {
        let norm_lang = Self::normalize_language(&rule.target_language);
        if let Some(pos) = self.rules.iter().position(|r| {
            r.archetype == rule.archetype
                && Self::normalize_language(&r.target_language) == norm_lang
        }) {
            self.rules[pos] = rule;
        } else {
            self.rules.push(rule);
        }
    }

    /// Set an explicit user override for a specific archetype and target language.
    pub fn set_user_override(
        &mut self,
        archetype: DomainArchetype,
        target_lang: &str,
        framework: &str,
        dependencies: Vec<String>,
        notes: Option<String>,
    ) {
        self.upsert_rule(FrameworkRule {
            archetype,
            target_language: target_lang.to_string(),
            recommended_framework: framework.to_string(),
            default_dependencies: dependencies,
            is_user_override: true,
            notes,
        });
    }

    /// Load knowledge registry from disk if present, merging with defaults; otherwise writes initialized defaults.
    pub fn load_or_init(knowledge_dir: &Path) -> Self {
        let file_path = knowledge_dir.join("framework_rules.json");
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(mut loaded) = serde_json::from_str::<FrameworkRegistry>(&content) {
                    // Merge any missing defaults
                    let defaults = Self::embedded_defaults();
                    for def_rule in defaults.rules {
                        let norm = Self::normalize_language(&def_rule.target_language);
                        if !loaded.rules.iter().any(|r| {
                            r.archetype == def_rule.archetype
                                && Self::normalize_language(&r.target_language) == norm
                        }) {
                            loaded.rules.push(def_rule);
                        }
                    }
                    return loaded;
                }
            }
        }

        let default_reg = Self::embedded_defaults();
        let _ = default_reg.save_to_dir(knowledge_dir);
        default_reg
    }

    /// Save the framework registry to `.exodus/knowledge/framework_rules.json`.
    pub fn save_to_dir(&self, knowledge_dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(knowledge_dir)?;
        let file_path = knowledge_dir.join("framework_rules.json");
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&file_path, json)?;
        Ok(file_path)
    }
}

/// Discovered member package inside a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub name: String,
    pub root_path: PathBuf,
    pub relative_path: PathBuf,
    pub manifest_file: Option<PathBuf>,
    pub source_language: String,
    pub domain_archetype: DomainArchetype,
    pub dependencies: Vec<String>,
    pub source_files: Vec<PathBuf>,
    pub lines_of_code: usize,
}

/// Comprehensive workspace descriptor capturing topology, packages, and dependency DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDescriptor {
    pub root_dir: PathBuf,
    pub toolchain: WorkspaceToolchain,
    pub packages: Vec<PackageDescriptor>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub total_lines_of_code: usize,
}

impl WorkspaceDescriptor {
    /// Returns packages topologically sorted (foundations/leaves first, dependents last).
    pub fn topological_waves(&self) -> Vec<Vec<&PackageDescriptor>> {
        let pkg_map: HashMap<&str, &PackageDescriptor> =
            self.packages.iter().map(|p| (p.name.as_str(), p)).collect();

        let mut in_degrees: HashMap<&str, usize> = HashMap::new();
        let mut dependents_map: HashMap<&str, Vec<&str>> = HashMap::new();

        for pkg in &self.packages {
            in_degrees.entry(&pkg.name).or_insert(0);
            for dep in &pkg.dependencies {
                if pkg_map.contains_key(dep.as_str()) {
                    *in_degrees.entry(&pkg.name).or_insert(0) += 1;
                    dependents_map
                        .entry(dep.as_str())
                        .or_default()
                        .push(&pkg.name);
                }
            }
        }

        let mut waves: Vec<Vec<&PackageDescriptor>> = Vec::new();
        let mut remaining_in_degrees = in_degrees;
        let mut visited: HashSet<&str> = HashSet::new();

        loop {
            let current_wave_names: Vec<&str> = remaining_in_degrees
                .iter()
                .filter(|(&name, &deg)| deg == 0 && !visited.contains(name))
                .map(|(&name, _)| name)
                .collect();

            if current_wave_names.is_empty() {
                // Break any cycles or add remaining
                let remaining: Vec<&str> = remaining_in_degrees
                    .keys()
                    .copied()
                    .filter(|&k| !visited.contains(k))
                    .collect();
                if !remaining.is_empty() {
                    let wave_pkgs: Vec<&PackageDescriptor> = remaining
                        .into_iter()
                        .filter_map(|k| pkg_map.get(k).copied())
                        .collect();
                    waves.push(wave_pkgs);
                }
                break;
            }

            for &name in &current_wave_names {
                visited.insert(name);
                if let Some(dependents) = dependents_map.get(name) {
                    for &dep in dependents {
                        if let Some(deg) = remaining_in_degrees.get_mut(dep) {
                            if *deg > 0 {
                                *deg -= 1;
                            }
                        }
                    }
                }
            }

            let wave_pkgs: Vec<&PackageDescriptor> = current_wave_names
                .into_iter()
                .filter_map(|name| pkg_map.get(name).copied())
                .collect();
            waves.push(wave_pkgs);
        }

        waves
    }

    /// Fuzzy matches a user prompt to find referenced packages in the workspace.
    pub fn match_packages(&self, query: &str) -> Vec<&PackageDescriptor> {
        let q = query.to_lowercase();
        let trimmed_q = q.trim();

        if trimmed_q == "all"
            || trimmed_q == "workspace"
            || trimmed_q == "monorepo"
            || trimmed_q.is_empty()
        {
            return self.packages.iter().collect();
        }

        let mut matches = Vec::new();
        for pkg in &self.packages {
            let pkg_name_lower = pkg.name.to_lowercase();
            let path_str = pkg.relative_path.to_string_lossy().to_lowercase();

            if q.contains(&pkg_name_lower)
                || pkg_name_lower.contains(&q)
                || q.contains(&path_str)
                || path_str.contains(&q)
            {
                matches.push(pkg);
            }
        }

        if matches.is_empty() {
            // Check domain archetypes
            for pkg in &self.packages {
                let archetype_str = pkg.domain_archetype.display_name().to_lowercase();
                if q.contains(&archetype_str)
                    || (q.contains("service")
                        && pkg.domain_archetype == DomainArchetype::BackendService)
                    || (q.contains("cli") && pkg.domain_archetype == DomainArchetype::CliTool)
                    || (q.contains("worker")
                        && pkg.domain_archetype == DomainArchetype::WorkerQueue)
                    || (q.contains("lib") && pkg.domain_archetype == DomainArchetype::SharedLibrary)
                    || (q.contains("frontend")
                        && pkg.domain_archetype == DomainArchetype::FrontendApp)
                {
                    matches.push(pkg);
                }
            }
        }

        if matches.is_empty() {
            // Default to all packages if ambiguous
            self.packages.iter().collect()
        } else {
            matches
        }
    }
}

/// Scanner responsible for detecting workspace toolchains and classifying package domain archetypes.
pub struct WorkspaceScanner;

impl WorkspaceScanner {
    /// Autonomously scans a path to discover workspace toolchain, member packages, and dependency DAG.
    pub fn scan_workspace(root: &Path) -> WorkspaceDescriptor {
        let toolchain = Self::detect_toolchain(root);
        let mut packages = Self::discover_packages(root, toolchain);

        if packages.is_empty() {
            // Fallback to treat root itself as a single package
            if let Some(standalone_pkg) = Self::create_standalone_package(root) {
                packages.push(standalone_pkg);
            }
        }

        // Build dependency graph
        let mut dependency_graph = HashMap::new();
        let mut total_lines = 0;
        for pkg in &packages {
            dependency_graph.insert(pkg.name.clone(), pkg.dependencies.clone());
            total_lines += pkg.lines_of_code;
        }

        WorkspaceDescriptor {
            root_dir: root.to_path_buf(),
            toolchain,
            packages,
            dependency_graph,
            total_lines_of_code: total_lines,
        }
    }

    /// Autonomously detects the workspace/monorepo toolchain from configuration files.
    pub fn detect_toolchain(root: &Path) -> WorkspaceToolchain {
        if root.join("turbo.json").exists() {
            return WorkspaceToolchain::Turborepo;
        }
        if root.join(".moon/workspace.yml").exists() || root.join(".moon").is_dir() {
            return WorkspaceToolchain::Moonrepo;
        }
        if root.join("nx.json").exists() {
            return WorkspaceToolchain::Nx;
        }
        if root.join("pnpm-workspace.yaml").exists() {
            return WorkspaceToolchain::PnpmWorkspaces;
        }
        if root.join("go.work").exists() {
            return WorkspaceToolchain::GoWorkspace;
        }
        if root.join("settings.gradle").exists() || root.join("settings.gradle.kts").exists() {
            return WorkspaceToolchain::GradleMultiProject;
        }

        // Check Cargo.toml for [workspace]
        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return WorkspaceToolchain::CargoWorkspace;
                }
            }
        }

        // Check package.json for "workspaces"
        let pkg_json = root.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = fs::read_to_string(&pkg_json) {
                if content.contains("\"workspaces\"") {
                    if root.join("yarn.lock").exists() {
                        return WorkspaceToolchain::YarnWorkspaces;
                    }
                    return WorkspaceToolchain::NpmWorkspaces;
                }
            }
        }

        // Check for Python workspaces
        if root.join("uv.workspace").exists() || root.join("pants.toml").exists() {
            return WorkspaceToolchain::PythonMonorepo;
        }

        // Scan common monorepo subdirectories even without a root config file
        if root.join("packages").is_dir()
            || root.join("apps").is_dir()
            || root.join("services").is_dir()
            || root.join("crates").is_dir()
        {
            return WorkspaceToolchain::Turborepo;
        }

        WorkspaceToolchain::Standalone
    }

    /// Discovers all member packages within the workspace.
    fn discover_packages(root: &Path, _toolchain: WorkspaceToolchain) -> Vec<PackageDescriptor> {
        let mut packages = Vec::new();
        let candidate_dirs = [
            "packages", "apps", "services", "crates", "libs", "modules", "src",
        ];

        for dir_name in &candidate_dirs {
            let search_dir = root.join(dir_name);
            if search_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(&search_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(pkg) = Self::inspect_package_dir(root, &path) {
                                packages.push(pkg);
                            }
                        }
                    }
                }
            }
        }

        packages
    }

    /// Inspects a directory to extract metadata, source files, language, and domain archetype.
    fn inspect_package_dir(root: &Path, pkg_dir: &Path) -> Option<PackageDescriptor> {
        let pkg_name = pkg_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed")
            .to_string();

        let rel_path = pkg_dir.strip_prefix(root).unwrap_or(pkg_dir).to_path_buf();

        let mut manifest_file = None;
        let mut source_lang = "python".to_string();
        let mut dependencies = Vec::new();
        let mut domain = DomainArchetype::Unknown;

        // 1. Check package.json
        let pkg_json_path = pkg_dir.join("package.json");
        if pkg_json_path.exists() {
            manifest_file = Some(pkg_json_path.clone());
            source_lang = "typescript".to_string();
            if let Ok(content) = fs::read_to_string(&pkg_json_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json["name"].as_str() {
                        let clean_name = name.trim_start_matches('@').replace('/', "-");
                        let _ = clean_name;
                    }
                    if let Some(deps) = json["dependencies"].as_object() {
                        for dep_name in deps.keys() {
                            dependencies.push(dep_name.clone());
                        }
                    }
                    domain = Self::classify_domain_from_deps(&dependencies, &pkg_name, &rel_path);
                }
            }
        }

        // 2. Check Cargo.toml
        let cargo_path = pkg_dir.join("Cargo.toml");
        if cargo_path.exists() {
            manifest_file = Some(cargo_path.clone());
            source_lang = "rust".to_string();
            if let Ok(content) = fs::read_to_string(&cargo_path) {
                if content.contains("axum") || content.contains("actix-web") {
                    domain = DomainArchetype::BackendService;
                } else if content.contains("clap") {
                    domain = DomainArchetype::CliTool;
                } else if content.contains("tokio") && content.contains("lapin") {
                    domain = DomainArchetype::WorkerQueue;
                }
            }
        }

        // 3. Check pyproject.toml / requirements.txt
        let pyproject_path = pkg_dir.join("pyproject.toml");
        let req_path = pkg_dir.join("requirements.txt");
        if pyproject_path.exists() || req_path.exists() {
            manifest_file = if pyproject_path.exists() {
                Some(pyproject_path.clone())
            } else {
                Some(req_path.clone())
            };
            source_lang = "python".to_string();
            let content = fs::read_to_string(pyproject_path.as_path())
                .or_else(|_| fs::read_to_string(req_path.as_path()))
                .unwrap_or_default();
            if content.contains("fastapi")
                || content.contains("flask")
                || content.contains("django")
            {
                domain = DomainArchetype::BackendService;
            } else if content.contains("click") || content.contains("typer") {
                domain = DomainArchetype::CliTool;
            } else if content.contains("celery") || content.contains("rq") {
                domain = DomainArchetype::WorkerQueue;
            }
        }

        // 4. Check go.mod
        let go_mod_path = pkg_dir.join("go.mod");
        if go_mod_path.exists() {
            manifest_file = Some(go_mod_path.clone());
            source_lang = "go".to_string();
            if let Ok(content) = fs::read_to_string(&go_mod_path) {
                if content.contains("gin-gonic")
                    || content.contains("fiber")
                    || content.contains("echo")
                {
                    domain = DomainArchetype::BackendService;
                } else if content.contains("cobra") {
                    domain = DomainArchetype::CliTool;
                }
            }
        }

        // 5. Fallback domain heuristic from folder name
        if domain == DomainArchetype::Unknown {
            domain = Self::classify_domain_from_path(&rel_path, &pkg_name);
        }

        // Collect source files & LOC
        let mut source_files = Vec::new();
        let mut loc = 0;
        Self::collect_files_recursive(pkg_dir, &mut source_files, &mut loc, &mut source_lang);

        if source_files.is_empty() && manifest_file.is_none() {
            return None;
        }

        Some(PackageDescriptor {
            name: pkg_name,
            root_path: pkg_dir.to_path_buf(),
            relative_path: rel_path,
            manifest_file,
            source_language: source_lang,
            domain_archetype: domain,
            dependencies,
            source_files,
            lines_of_code: loc,
        })
    }

    fn classify_domain_from_deps(
        deps: &[String],
        pkg_name: &str,
        rel_path: &Path,
    ) -> DomainArchetype {
        let deps_str = deps.join(" ").to_lowercase();
        if deps_str.contains("express")
            || deps_str.contains("fastify")
            || deps_str.contains("koa")
            || deps_str.contains("nestjs")
            || deps_str.contains("@nestjs")
        {
            DomainArchetype::BackendService
        } else if deps_str.contains("commander")
            || deps_str.contains("yargs")
            || deps_str.contains("cac")
        {
            DomainArchetype::CliTool
        } else if deps_str.contains("bull")
            || deps_str.contains("bullmq")
            || deps_str.contains("kafkajs")
        {
            DomainArchetype::WorkerQueue
        } else if deps_str.contains("next")
            || deps_str.contains("react")
            || deps_str.contains("vue")
            || deps_str.contains("svelte")
        {
            DomainArchetype::FrontendApp
        } else {
            Self::classify_domain_from_path(rel_path, pkg_name)
        }
    }

    fn classify_domain_from_path(rel_path: &Path, pkg_name: &str) -> DomainArchetype {
        let path_lower = rel_path.to_string_lossy().to_lowercase();
        let name_lower = pkg_name.to_lowercase();

        if path_lower.contains("services")
            || path_lower.contains("api")
            || name_lower.ends_with("-service")
            || name_lower.ends_with("-api")
            || name_lower.contains("server")
        {
            DomainArchetype::BackendService
        } else if path_lower.contains("cli")
            || name_lower.ends_with("-cli")
            || name_lower.contains("tool")
        {
            DomainArchetype::CliTool
        } else if path_lower.contains("worker")
            || name_lower.ends_with("-worker")
            || name_lower.contains("queue")
        {
            DomainArchetype::WorkerQueue
        } else if path_lower.contains("packages")
            || path_lower.contains("libs")
            || path_lower.contains("crates")
            || name_lower.contains("shared")
            || name_lower.contains("core")
            || name_lower.contains("models")
            || name_lower.contains("types")
            || name_lower.contains("utils")
        {
            DomainArchetype::SharedLibrary
        } else if path_lower.contains("apps")
            || name_lower.contains("web")
            || name_lower.contains("frontend")
        {
            DomainArchetype::FrontendApp
        } else {
            DomainArchetype::SharedLibrary
        }
    }

    fn collect_files_recursive(
        dir: &Path,
        files: &mut Vec<PathBuf>,
        loc: &mut usize,
        detected_lang: &mut String,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.starts_with('.')
                    || file_name == "node_modules"
                    || file_name == "target"
                    || file_name == "dist"
                    || file_name == "build"
                {
                    continue;
                }
                if path.is_dir() {
                    Self::collect_files_recursive(&path, files, loc, detected_lang);
                } else if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let is_source = match ext {
                            "rs" => {
                                *detected_lang = "rust".to_string();
                                true
                            }
                            "ts" | "tsx" => {
                                *detected_lang = "typescript".to_string();
                                true
                            }
                            "js" | "jsx" => {
                                *detected_lang = "javascript".to_string();
                                true
                            }
                            "py" => {
                                *detected_lang = "python".to_string();
                                true
                            }
                            "go" => {
                                *detected_lang = "go".to_string();
                                true
                            }
                            "java" => {
                                *detected_lang = "java".to_string();
                                true
                            }
                            "kt" => {
                                *detected_lang = "kotlin".to_string();
                                true
                            }
                            "cpp" | "cc" | "cxx" | "h" | "hpp" => {
                                *detected_lang = "cpp".to_string();
                                true
                            }
                            "cs" => {
                                *detected_lang = "csharp".to_string();
                                true
                            }
                            _ => false,
                        };
                        if is_source {
                            if let Ok(content) = fs::read_to_string(&path) {
                                *loc += content.lines().count();
                            }
                            files.push(path);
                        }
                    }
                }
            }
        }
    }

    fn create_standalone_package(root: &Path) -> Option<PackageDescriptor> {
        let name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("root-project")
            .to_string();

        let mut source_files = Vec::new();
        let mut loc = 0;
        let mut lang = "python".to_string();
        Self::collect_files_recursive(root, &mut source_files, &mut loc, &mut lang);

        if source_files.is_empty() {
            return None;
        }

        Some(PackageDescriptor {
            name,
            root_path: root.to_path_buf(),
            relative_path: PathBuf::from("."),
            manifest_file: None,
            source_language: lang,
            domain_archetype: DomainArchetype::SharedLibrary,
            dependencies: Vec::new(),
            source_files,
            lines_of_code: loc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_turborepo_workspace_detection() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("turbo.json"), "{}").unwrap();

        let packages_dir = root.join("packages").join("core-models");
        fs::create_dir_all(&packages_dir).unwrap();
        fs::write(packages_dir.join("models.py"), "class User: pass\n").unwrap();

        let apps_dir = root.join("apps").join("payment-service");
        fs::create_dir_all(&apps_dir).unwrap();
        fs::write(apps_dir.join("service.py"), "from fastapi import FastAPI\n").unwrap();

        let ws = WorkspaceScanner::scan_workspace(root);
        assert_eq!(ws.toolchain, WorkspaceToolchain::Turborepo);
        assert_eq!(ws.packages.len(), 2);

        let core_pkg = ws
            .packages
            .iter()
            .find(|p| p.name == "core-models")
            .unwrap();
        assert_eq!(core_pkg.domain_archetype, DomainArchetype::SharedLibrary);

        let svc_pkg = ws
            .packages
            .iter()
            .find(|p| p.name == "payment-service")
            .unwrap();
        assert_eq!(svc_pkg.domain_archetype, DomainArchetype::BackendService);
    }

    #[test]
    fn test_topological_waves_ordering() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let pkg_a = PackageDescriptor {
            name: "pkg-shared".to_string(),
            root_path: root.join("packages/pkg-shared"),
            relative_path: PathBuf::from("packages/pkg-shared"),
            manifest_file: None,
            source_language: "typescript".to_string(),
            domain_archetype: DomainArchetype::SharedLibrary,
            dependencies: vec![],
            source_files: vec![],
            lines_of_code: 100,
        };

        let pkg_b = PackageDescriptor {
            name: "pkg-service".to_string(),
            root_path: root.join("apps/pkg-service"),
            relative_path: PathBuf::from("apps/pkg-service"),
            manifest_file: None,
            source_language: "typescript".to_string(),
            domain_archetype: DomainArchetype::BackendService,
            dependencies: vec!["pkg-shared".to_string()],
            source_files: vec![],
            lines_of_code: 250,
        };

        let ws = WorkspaceDescriptor {
            root_dir: root.to_path_buf(),
            toolchain: WorkspaceToolchain::Turborepo,
            packages: vec![pkg_b, pkg_a],
            dependency_graph: HashMap::new(),
            total_lines_of_code: 350,
        };

        let waves = ws.topological_waves();
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0][0].name, "pkg-shared");
        assert_eq!(waves[1][0].name, "pkg-service");
    }

    #[test]
    fn test_fuzzy_package_matching() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let pkg_a = PackageDescriptor {
            name: "auth-service".to_string(),
            root_path: root.join("services/auth-service"),
            relative_path: PathBuf::from("services/auth-service"),
            manifest_file: None,
            source_language: "python".to_string(),
            domain_archetype: DomainArchetype::BackendService,
            dependencies: vec![],
            source_files: vec![],
            lines_of_code: 150,
        };

        let ws = WorkspaceDescriptor {
            root_dir: root.to_path_buf(),
            toolchain: WorkspaceToolchain::Turborepo,
            packages: vec![pkg_a],
            dependency_graph: HashMap::new(),
            total_lines_of_code: 150,
        };

        let matches = ws.match_packages("migrate auth service to rust");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "auth-service");
    }

    #[test]
    fn test_framework_registry_embedded_defaults_and_session_updates() {
        let mut reg = FrameworkRegistry::embedded_defaults();

        // Check embedded default
        assert!(reg
            .get_framework(DomainArchetype::BackendService, "rust")
            .contains("Axum"));

        // User updates session for a novel / unthought-of scenario (e.g. Actix-web + SQLx)
        reg.set_user_override(
            DomainArchetype::BackendService,
            "rust",
            "Actix-Web + SQLx + Tokio",
            vec!["actix-web".to_string(), "sqlx".to_string()],
            Some("User preferred Actix-web over Axum in current session".to_string()),
        );

        // Verification: returns updated session rule
        assert_eq!(
            reg.get_framework(DomainArchetype::BackendService, "rust"),
            "Actix-Web + SQLx + Tokio"
        );

        // Test persistence and reload
        let temp_dir = tempdir().unwrap();
        let saved_file = reg.save_to_dir(temp_dir.path()).unwrap();
        assert!(saved_file.exists());

        let loaded = FrameworkRegistry::load_or_init(temp_dir.path());
        assert_eq!(
            loaded.get_framework(DomainArchetype::BackendService, "rust"),
            "Actix-Web + SQLx + Tokio"
        );
        // Default rule for other languages still preserved
        assert!(loaded
            .get_framework(DomainArchetype::BackendService, "go")
            .contains("Gin"));
    }
}
