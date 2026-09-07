//! Extensible Target Language Emitter Strategies & Registry for Project Exodus.

use crate::workspace::{DomainArchetype, PackageDescriptor};
use exodus_core::TargetLanguageSpecRecord;
use std::path::Path;

/// Trait governing target language scaffolding, entrypoint exports, manifest synthesis,
/// directory layout conventions, ecosystem-appropriate naming standards, and toolchain integration.
pub trait TargetLanguageEmitter: Send + Sync {
    /// Canonical target language identifier (e.g. "rust", "go", "zig", "kotlin", "typescript", "python", "csharp", "dart").
    fn name(&self) -> &str;

    /// File extension for code files in this target language (e.g. "rs", "go", "zig", "kt", "ts", "py", "cs", "dart").
    fn extension(&self) -> &str;

    /// Preferred source subdirectory (e.g. "src", ".", "lib", "src/main/kotlin").
    fn source_dir(&self) -> &str;

    /// Primary package manifest name (e.g. "Cargo.toml", "go.mod", "build.zig", "build.gradle.kts", "package.json", "pyproject.toml", "pubspec.yaml").
    fn manifest_name(&self) -> &str;

    /// Default package entrypoint filename (e.g. "lib.rs", "doc.go", "lib.zig", "Main.kt", "index.ts", "__init__.py", "main.dart").
    fn entrypoint_filename(&self) -> &str;

    /// Ecosystem-accurate container terminology: "Crate" for Rust, "Package" for Go / Dart / Python / TS / Kotlin / Zig, "Project" for C#.
    fn ecosystem_container_term(&self) -> &str;

    /// Collection directory convention for multi-package workspaces based on language and domain archetype
    /// (e.g. "packages" or "services" for Go/Dart/Node, "crates" for Rust, "src" for C#).
    fn collection_dir_name(&self, archetype: DomainArchetype) -> &str {
        match archetype {
            DomainArchetype::BackendService => "services",
            DomainArchetype::CliTool | DomainArchetype::FrontendApp => "apps",
            _ => {
                if self.name() == "rust" {
                    "crates"
                } else {
                    "packages"
                }
            }
        }
    }

    /// Contextual semantic label for migrated units (e.g. "Service Route / Endpoint", "CLI Command", "Task Consumer").
    fn contextual_unit_label(&self, archetype: DomainArchetype, unit_name: &str) -> String {
        match archetype {
            DomainArchetype::BackendService => {
                format!("API Service Route / Endpoint `{unit_name}`")
            }
            DomainArchetype::CliTool => {
                format!("CLI Command / Subcommand `{unit_name}`")
            }
            DomainArchetype::WorkerQueue => {
                format!("Worker Consumer / Queue Task `{unit_name}`")
            }
            DomainArchetype::FrontendApp => {
                format!("UI View / Client Component `{unit_name}`")
            }
            DomainArchetype::SharedLibrary => {
                format!("Domain Entity / Schema Module `{unit_name}`")
            }
            DomainArchetype::Unknown => {
                format!("Migrated Unit `{unit_name}`")
            }
        }
    }

    /// Generates root entrypoint export file connecting all migrated modules in a package.
    fn generate_entrypoint(
        &self,
        pkg_name: &str,
        archetype: DomainArchetype,
        migrated_module_names: &[String],
    ) -> String;

    /// Generates deterministic module template/stub when LLM is offline or in fallback mode.
    fn generate_deterministic_module_source(
        &self,
        module_name: &str,
        source_path: &Path,
        archetype: DomainArchetype,
    ) -> String;

    /// Generates target package manifest (e.g. Cargo.toml, go.mod, build.zig, build.gradle.kts, package.json, pubspec.yaml).
    fn generate_package_manifest(
        &self,
        pkg: &PackageDescriptor,
        all_packages: &[PackageDescriptor],
        is_standalone: bool,
    ) -> (String, String);

    /// Generates top-level workspace manifest (e.g. Cargo.toml [workspace], go.work, pnpm-workspace.yaml).
    fn generate_workspace_manifest(
        &self,
        packages: &[&PackageDescriptor],
    ) -> Option<(String, String)>;

    /// Generates ecosystem-specific version lock/manager files (e.g. .go-version, .python-version, rust-toolchain.toml, .zig-version).
    fn generate_version_files(&self, version_str: &str) -> Vec<(String, String)>;

    /// Returns recommended quickstart commands for the target language.
    fn quickstart_command(&self) -> &str;

    /// Generates modern cloud-native SDLC scaffolding files (.env.example, Dockerfile, ci.yml, health probes).
    fn generate_sdlc_scaffolding(
        &self,
        pkg_name: &str,
        archetype: DomainArchetype,
    ) -> Vec<(String, String)> {
        let mut files = Vec::new();
        files.push((
            ".env.example".to_string(),
            format!("PORT=8080\nENVIRONMENT=development\nAPP_NAME={pkg_name}\n"),
        ));

        let is_service = matches!(
            archetype,
            DomainArchetype::BackendService | DomainArchetype::WorkerQueue
        );

        if is_service {
            files.push((
                ".github/workflows/ci.yml".to_string(),
                format!(
                    "name: CI Pipeline\non: [push, pull_request]\njobs:\n  build-and-test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - name: Run Checks\n        run: echo 'Running automated checks for {pkg_name}'\n"
                ),
            ));
        }

        files
    }
}

// ----------------------------------------------------------------------------
// Generic State-Driven Language Emitter
// ----------------------------------------------------------------------------

/// Unified generic language emitter executing pure template transformations from a `TargetLanguageSpecRecord`.
#[derive(Debug, Clone)]
pub struct GenericLanguageEmitter {
    pub spec: TargetLanguageSpecRecord,
}

impl GenericLanguageEmitter {
    pub fn new(spec: TargetLanguageSpecRecord) -> Self {
        Self { spec }
    }
}

impl TargetLanguageEmitter for GenericLanguageEmitter {
    fn name(&self) -> &str {
        &self.spec.id
    }

    fn extension(&self) -> &str {
        &self.spec.file_extension
    }

    fn source_dir(&self) -> &str {
        &self.spec.source_dir
    }

    fn manifest_name(&self) -> &str {
        &self.spec.manifest_name
    }

    fn entrypoint_filename(&self) -> &str {
        &self.spec.entrypoint_filename
    }

    fn ecosystem_container_term(&self) -> &str {
        &self.spec.ecosystem_container_term
    }

    fn generate_entrypoint(
        &self,
        pkg_name: &str,
        _archetype: DomainArchetype,
        migrated_module_names: &[String],
    ) -> String {
        let safe_pkg = pkg_name.replace('-', "_");
        let modules_exports = self
            .spec
            .module_export_template
            .as_deref()
            .map(|template| {
                migrated_module_names
                    .iter()
                    .map(|module_name| template.replace("{{module_name}}", module_name))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        self.spec
            .entrypoint_template
            .replace("{{pkg_name}}", pkg_name)
            .replace("{{safe_pkg}}", &safe_pkg)
            .replace("{{modules_exports}}", &modules_exports)
    }

    fn generate_deterministic_module_source(
        &self,
        module_name: &str,
        source_path: &Path,
        _archetype: DomainArchetype,
    ) -> String {
        let safe_pkg = module_name.replace('-', "_");
        self.spec
            .module_stub_template
            .replace("{{module_name}}", module_name)
            .replace("{{safe_pkg}}", &safe_pkg)
            .replace("{{source_path}}", &source_path.display().to_string())
    }

    fn generate_package_manifest(
        &self,
        pkg: &PackageDescriptor,
        all_packages: &[PackageDescriptor],
        is_standalone: bool,
    ) -> (String, String) {
        let safe_pkg = pkg.name.replace('-', "_");
        let safe_name = pkg.name.replace('_', "-");

        let mut manifest_content = self
            .spec
            .package_manifest_template
            .replace("{{pkg_name}}", &safe_name)
            .replace("{{safe_pkg}}", &safe_pkg);
        if !is_standalone {
            if let Some(template) = &self.spec.internal_dependency_template {
                let dependencies = pkg
                    .dependencies
                    .iter()
                    .filter_map(|dependency| {
                        all_packages
                            .iter()
                            .find(|candidate| &candidate.name == dependency)
                    })
                    .map(|dependency| {
                        template
                            .replace("{{dependency_name}}", &dependency.name)
                            .replace("{{dependency_path}}", &dependency.name)
                    })
                    .collect::<Vec<_>>();
                if !dependencies.is_empty() {
                    manifest_content.push('\n');
                    manifest_content.push_str(&dependencies.join("\n"));
                    manifest_content.push('\n');
                }
            }
        }

        (self.spec.manifest_name.clone(), manifest_content)
    }

    fn generate_workspace_manifest(
        &self,
        packages: &[&PackageDescriptor],
    ) -> Option<(String, String)> {
        let filename = self.spec.workspace_manifest_filename.as_ref()?;
        let template = self.spec.workspace_manifest_template.as_ref()?;

        let member_template = self.spec.workspace_member_template.as_deref()?;
        let members_str = packages
            .iter()
            .map(|package| {
                member_template
                    .replace("{{package_path}}", &package.name)
                    .replace("{{package_name}}", &package.name)
            })
            .collect::<Vec<_>>()
            .join("");

        let content = template
            .replace("{{pkg_name}}", "migrated-workspace")
            .replace("{{workspace_members}}", &members_str);

        Some((filename.clone(), content))
    }

    fn generate_version_files(&self, version_str: &str) -> Vec<(String, String)> {
        let mut files = Vec::new();
        for (filename, template) in &self.spec.version_files {
            files.push((
                filename.clone(),
                template.replace("{{version}}", version_str),
            ));
        }
        files
    }

    fn quickstart_command(&self) -> &str {
        &self.spec.quickstart_command
    }
}

// ----------------------------------------------------------------------------
// Target Language Registry
// ----------------------------------------------------------------------------

/// Central lookup and factory for target language emitters.
pub struct TargetLanguageRegistry;

impl TargetLanguageRegistry {
    /// Resolves the appropriate emitter for a given language identifier or alias.
    pub fn get(lang: &str) -> exodus_core::Result<Box<dyn TargetLanguageEmitter>> {
        let specs = TargetLanguageSpecRecord::default_specs();
        let query = lang.trim().to_lowercase();
        if let Some(spec) = specs.iter().find(|s| s.matches_query(&query)) {
            return Ok(Box::new(GenericLanguageEmitter::new(spec.clone())));
        }
        Err(exodus_core::ExodusError::UnsupportedTargetLanguage(format!(
            "`{lang}`; available targets: {}",
            specs
                .iter()
                .map(|spec| spec.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }

    /// Resolves an emitter from an explicit `TargetLanguageSpecRecord`.
    pub fn from_spec(spec: TargetLanguageSpecRecord) -> Box<dyn TargetLanguageEmitter> {
        Box::new(GenericLanguageEmitter::new(spec))
    }

    /// Returns list of all supported target language names.
    pub fn supported_languages() -> &'static [&'static str] {
        &[
            "rust",
            "go",
            "zig",
            "kotlin",
            "typescript",
            "javascript",
            "python",
            "csharp",
            "dart",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_registry_resolution_and_terms() {
        let go_emitter = TargetLanguageRegistry::get("go").unwrap();
        assert_eq!(go_emitter.name(), "go");
        assert_eq!(go_emitter.ecosystem_container_term(), "Package");
        assert_eq!(
            go_emitter.collection_dir_name(DomainArchetype::SharedLibrary),
            "packages"
        );
        assert_eq!(
            go_emitter.collection_dir_name(DomainArchetype::BackendService),
            "services"
        );

        let dart_emitter = TargetLanguageRegistry::get("dart").unwrap();
        assert_eq!(dart_emitter.name(), "dart");
        assert_eq!(dart_emitter.ecosystem_container_term(), "Package");
        assert_eq!(dart_emitter.manifest_name(), "pubspec.yaml");

        let rust_emitter = TargetLanguageRegistry::get("rust").unwrap();
        assert_eq!(rust_emitter.ecosystem_container_term(), "Crate");
        assert_eq!(
            rust_emitter.collection_dir_name(DomainArchetype::SharedLibrary),
            "crates"
        );
    }

    #[test]
    fn test_contextual_unit_labels() {
        let emitter = TargetLanguageRegistry::get("go").unwrap();
        let label = emitter.contextual_unit_label(DomainArchetype::BackendService, "auth_handler");
        assert!(label.contains("API Service Route / Endpoint"));

        let cli_label = emitter.contextual_unit_label(DomainArchetype::CliTool, "migrate_cmd");
        assert!(cli_label.contains("CLI Command"));
    }

    #[test]
    fn test_polyglot_generic_emitters() {
        let zig = TargetLanguageRegistry::get("zig").unwrap();
        assert_eq!(zig.name(), "zig");
        assert_eq!(zig.manifest_name(), "build.zig");
        assert_eq!(zig.extension(), "zig");

        let kotlin = TargetLanguageRegistry::get("kotlin").unwrap();
        assert_eq!(kotlin.name(), "kotlin");
        assert_eq!(kotlin.manifest_name(), "build.gradle.kts");
        assert_eq!(kotlin.extension(), "kt");
    }

    #[test]
    fn unknown_target_is_rejected_without_fallback() {
        let error = TargetLanguageRegistry::get("not-a-language")
            .err()
            .expect("unknown target must fail");
        assert!(matches!(
            error,
            exodus_core::ExodusError::UnsupportedTargetLanguage(_)
        ));
    }
}
