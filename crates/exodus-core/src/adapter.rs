//! Multi-language adapter abstractions, verifiers, declarative toolchains, and dynamic registry.

use crate::deprecation::DeprecationRecord;
use crate::esg::{EsgEdge, EsgNode, LanguageId};
use crate::profile::RepositoryProfile;
use crate::{BehavioralContract, Diagnostic, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Diagnostic output format supported by language toolchains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticFormat {
    Generic,
    RustcJson,
    ClangStyle,
    GoVet,
    Pytest,
    TypeScript,
}

/// Verification stage executed during verification gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VerificationStage {
    Format,
    CompileCheck,
    BehavioralTest,
}

/// Assertion failure record extracted from test runner execution.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FailedAssertion {
    pub test_name: String,
    pub failure_message: String,
}

/// Comprehensive state captured after a toolchain verification step.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationState {
    pub stage: VerificationStage,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<Diagnostic>,
    pub failed_assertions: Vec<FailedAssertion>,
    pub duration_ms: u64,
}

/// Declarative profile specifying a target toolchain's execution parameters.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolchainProfile {
    pub target_language: LanguageId,
    pub format_command: Option<Vec<String>>,
    pub check_command: Vec<String>,
    pub test_command: Option<Vec<String>>,
    pub diagnostic_format: DiagnosticFormat,
    pub env_vars: HashMap<String, String>,
}

/// Nullability behavior for a source-to-target type rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullabilityPolicy {
    Reject,
    Preserve,
    Wrap,
}

/// One ordered, source-specific type mapping.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TypeMappingRule {
    pub source_pattern: String,
    pub target_type: String,
    pub generic_template: Option<String>,
    pub nullable_template: Option<String>,
    pub nullability: NullabilityPolicy,
    pub grounding: crate::GroundingTier,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReceiverRule {
    pub instance_template: String,
    pub mutable_instance_template: Option<String>,
    pub static_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ParameterRule {
    pub template: String,
    pub separator: String,
    pub variadic_template: Option<String>,
    pub default_values_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReturnTypeRule {
    pub template: String,
    pub void_types: Vec<String>,
    pub async_wrapper_template: Option<String>,
}

/// Versioned rules for rendering one source-language signature into a target language.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignatureRuleSet {
    pub schema_version: String,
    pub revision: u32,
    pub source_language: LanguageId,
    pub target_language: LanguageId,
    pub function_keyword: String,
    pub signature_template: String,
    pub receiver: ReceiverRule,
    pub parameters: ParameterRule,
    pub return_type: ReturnTypeRule,
    pub type_mappings: Vec<TypeMappingRule>,
}

impl SignatureRuleSet {
    pub fn validate(&self) -> Result<()> {
        for placeholder in [
            "{{function_keyword}}",
            "{{name}}",
            "{{parameters}}",
            "{{return_type}}",
        ] {
            if !self.signature_template.contains(placeholder) {
                return Err(crate::ExodusError::Generic(format!(
                    "signature rule {} -> {} is missing required placeholder {placeholder}",
                    self.source_language, self.target_language
                )));
            }
        }
        for placeholder in ["{{name}}", "{{type}}"] {
            if !self.parameters.template.contains(placeholder) {
                return Err(crate::ExodusError::Generic(format!(
                    "parameter template is missing required placeholder {placeholder}"
                )));
            }
        }
        let mut patterns = std::collections::BTreeSet::new();
        for mapping in &self.type_mappings {
            let pattern = mapping.source_pattern.trim().to_lowercase();
            if pattern.is_empty() || !patterns.insert(pattern.clone()) {
                return Err(crate::ExodusError::Generic(format!(
                    "duplicate or empty type mapping `{pattern}`"
                )));
            }
        }
        Ok(())
    }
}

/// Comprehensive data-driven specification for a target language ecosystem.
/// Persisted in living memory / database to drive generic scaffolding, emitting, and verification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TargetLanguageSpecRecord {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub file_extension: String,
    pub source_dir: String,
    pub manifest_name: String,
    pub entrypoint_filename: String,
    pub ecosystem_container_term: String,
    pub entrypoint_template: String,
    pub module_stub_template: String,
    #[serde(default)]
    pub module_export_template: Option<String>,
    pub package_manifest_template: String,
    pub workspace_manifest_filename: Option<String>,
    pub workspace_manifest_template: Option<String>,
    #[serde(default)]
    pub workspace_member_template: Option<String>,
    #[serde(default)]
    pub internal_dependency_template: Option<String>,
    pub version_files: Vec<(String, String)>,
    pub quickstart_command: String,
    pub test_harness_template: String,
    #[serde(default)]
    pub test_case_template: String,
    pub toolchain_profile: ToolchainProfile,
    #[serde(default)]
    pub signature_rule_sets: Vec<SignatureRuleSet>,
}

impl TargetLanguageSpecRecord {
    /// Checks if this language specification matches a given language query or alias.
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if self.id.to_lowercase() == q || self.name.to_lowercase() == q {
            return true;
        }
        self.aliases.iter().any(|a| a.to_lowercase() == q)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(crate::ExodusError::Generic(
                "target language id cannot be empty".to_string(),
            ));
        }
        for rules in &self.signature_rule_sets {
            if rules.target_language != self.id.as_str() {
                return Err(crate::ExodusError::Generic(format!(
                    "signature rules target `{}` does not match language specification `{}`",
                    rules.target_language, self.id
                )));
            }
            rules.validate()?;
        }
        Ok(())
    }

    pub fn signature_rules_for(&self, source: &LanguageId) -> Option<&SignatureRuleSet> {
        self.signature_rule_sets
            .iter()
            .filter(|rules| &rules.source_language == source)
            .max_by_key(|rules| rules.revision)
    }

    /// Pre-seeded target language specification registry defaults.
    pub fn default_specs() -> Vec<TargetLanguageSpecRecord> {
        let mut specs = vec![
            TargetLanguageSpecRecord {
                id: "rust".to_string(),
                name: "Rust".to_string(),
                aliases: vec!["rs".to_string()],
                file_extension: "rs".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "Cargo.toml".to_string(),
                entrypoint_filename: "lib.rs".to_string(),
                ecosystem_container_term: "Crate".to_string(),
                entrypoint_template: "//! Autonomous Project Exodus Migrated Target Crate\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "//! Migrated Rust module `{{module_name}}`\n//! Migrated from {{source_path}}\n\npub fn placeholder() {}\n".to_string(),
                module_export_template: Some("pub mod {{module_name}};".to_string()),
                package_manifest_template: "[package]\nname = \"{{pkg_name}}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = { version = \"1.0\", features = [\"derive\"] }\nserde_json = \"1.0\"\ntokio = { version = \"1\", features = [\"full\"] }\nchrono = { version = \"0.4\", features = [\"serde\"] }\n".to_string(),
                workspace_manifest_filename: Some("Cargo.toml".to_string()),
                workspace_manifest_template: Some("[workspace]\nmembers = [\n{{workspace_members}}]\nresolver = \"2\"\n".to_string()),
                workspace_member_template: Some("    \"{{package_path}}\",\n".to_string()),
                internal_dependency_template: Some("{{dependency_name}} = { path = \"../{{dependency_path}}\" }".to_string()),
                version_files: vec![("rust-toolchain.toml".to_string(), "[toolchain]\nchannel = \"stable\"\ncomponents = [\"rustfmt\", \"clippy\"]\n".to_string())],
                quickstart_command: "cargo check\ncargo test".to_string(),
                test_harness_template: "#[cfg(test)]\nmod tests {\n    use super::*;\n\n{{test_cases}}\n}\n".to_string(),
                test_case_template: "    #[test]\n    fn test_{{case_id}}() {\n        // Evidence: {{evidence}}\n        // Oracle: {{oracle}}\n        // Input: {{input}}\n        // Expected: {{expected}}\n    }".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new(
                    "rust",
                    ["cargo", "check", "--message-format=json"],
                )
                .with_format(["rustfmt", "--check"])
                .with_tests(["cargo", "test"])
                .with_diagnostic_format(DiagnosticFormat::RustcJson),
            },
            TargetLanguageSpecRecord {
                id: "go".to_string(),
                name: "Go".to_string(),
                aliases: vec!["golang".to_string()],
                file_extension: "go".to_string(),
                source_dir: ".".to_string(),
                manifest_name: "go.mod".to_string(),
                entrypoint_filename: "doc.go".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "// Package {{safe_pkg}} provides migrated functionality.\npackage {{safe_pkg}}\n".to_string(),
                module_stub_template: "// Migrated Go module `{{module_name}}`\n// Migrated from {{source_path}}\npackage {{safe_pkg}}\n".to_string(),
                module_export_template: None,
                package_manifest_template: "module {{pkg_name}}\n\ngo 1.22\n".to_string(),
                workspace_manifest_filename: Some("go.work".to_string()),
                workspace_manifest_template: Some("go 1.22\n\nuse (\n{{workspace_members}})\n".to_string()),
                workspace_member_template: Some("    ./{{package_path}}\n".to_string()),
                internal_dependency_template: None,
                version_files: vec![(".go-version".to_string(), "1.22\n".to_string())],
                quickstart_command: "go test ./...".to_string(),
                test_harness_template: "package {{safe_pkg}}\n\nimport \"testing\"\n\n{{test_cases}}\n".to_string(),
                test_case_template: "func Test_{{case_id}}(t *testing.T) {\n    // Evidence: {{evidence}}\n    // Oracle: {{oracle}}\n    // Input: {{input}}\n    // Expected: {{expected}}\n}".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("go", ["go", "vet", "./..."])
                    .with_format(["gofmt", "-l"])
                    .with_tests(["go", "test", "./..."])
                    .with_diagnostic_format(DiagnosticFormat::GoVet),
            },
            TargetLanguageSpecRecord {
                id: "zig".to_string(),
                name: "Zig".to_string(),
                aliases: vec![],
                file_extension: "zig".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "build.zig".to_string(),
                entrypoint_filename: "lib.zig".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "//! Migrated Zig package for `{{pkg_name}}`\n\nconst std = @import(\"std\");\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "//! Migrated Zig module `{{module_name}}`\nconst std = @import(\"std\");\n\n// Migrated from {{source_path}}\n".to_string(),
                module_export_template: Some("pub const {{module_name}} = @import(\"{{module_name}}.zig\");".to_string()),
                package_manifest_template: "const std = @import(\"std\");\n\npub fn build(b: *std.Build) void {\n    const target = b.standardTargetOptions(.{});\n    const optimize = b.standardOptimizeOption(.{});\n\n    const lib = b.addStaticLibrary(.{\n        .name = \"{{pkg_name}}\",\n        .root_source_file = b.path(\"src/lib.zig\"),\n        .target = target,\n        .optimize = optimize,\n    });\n    b.installArtifact(lib);\n\n    const main_tests = b.addTest(.{\n        .root_source_file = b.path(\"src/lib.zig\"),\n        .target = target,\n        .optimize = optimize,\n    });\n    const run_main_tests = b.addRunArtifact(main_tests);\n    const test_step = b.step(\"test\", \"Run library tests\");\n    test_step.dependOn(&run_main_tests.step);\n}\n".to_string(),
                workspace_manifest_filename: None,
                workspace_manifest_template: None,
                workspace_member_template: None,
                internal_dependency_template: None,
                version_files: vec![(".zig-version".to_string(), "0.13.0\n".to_string())],
                quickstart_command: "zig build\nzig build test".to_string(),
                test_harness_template: "const std = @import(\"std\");\n\n{{test_cases}}\n".to_string(),
                test_case_template: "test \"{{case_id}}\" {\n    // Evidence: {{evidence}}\n    // Oracle: {{oracle}}\n    // Input: {{input}}\n    // Expected: {{expected}}\n}".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("zig", ["zig", "build"])
                    .with_format(["zig", "fmt", "--check", "src/lib.zig"])
                    .with_tests(["zig", "build", "test"])
                    .with_diagnostic_format(DiagnosticFormat::ClangStyle),
            },
            TargetLanguageSpecRecord {
                id: "kotlin".to_string(),
                name: "Kotlin".to_string(),
                aliases: vec!["kt".to_string()],
                file_extension: "kt".to_string(),
                source_dir: "src/main/kotlin".to_string(),
                manifest_name: "build.gradle.kts".to_string(),
                entrypoint_filename: "Main.kt".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "package {{safe_pkg}}\n\n// Autonomous Project Exodus Migrated Target (Kotlin)\n\nfun main() {\n    println(\"Exodus Kotlin Target: {{pkg_name}}\")\n}\n".to_string(),
                module_stub_template: "// Migrated Kotlin module `{{module_name}}`\n// Migrated from {{source_path}}\n\npackage {{safe_pkg}}\n\nclass {{module_name}}Module {\n    // Deterministic fallback stub\n}\n".to_string(),
                module_export_template: None,
                package_manifest_template: "plugins {\n    kotlin(\"jvm\") version \"1.9.22\"\n    application\n}\n\ngroup = \"com.exodus\"\nversion = \"0.1.0\"\n\nrepositories {\n    mavenCentral()\n}\n\ndependencies {\n    implementation(kotlin(\"stdlib\"))\n    testImplementation(\"org.junit.jupiter:junit-jupiter:5.10.0\")\n}\n\ntasks.test {\n    useJUnitPlatform()\n}\n\napplication {\n    mainClass.set(\"{{safe_pkg}}.MainKt\")\n}\n".to_string(),
                workspace_manifest_filename: Some("settings.gradle.kts".to_string()),
                workspace_manifest_template: Some("rootProject.name = \"{{pkg_name}}\"\n".to_string()),
                workspace_member_template: None,
                internal_dependency_template: None,
                version_files: vec![("gradle.properties".to_string(), "kotlin.version=1.9.22\norg.gradle.jvmargs=-Xmx2048m\n".to_string())],
                quickstart_command: "gradle build\ngradle test".to_string(),
                test_harness_template: "package {{safe_pkg}}\n\nimport org.junit.jupiter.api.Test\nimport org.junit.jupiter.api.Assertions.*\n\nclass BehavioralContractTest {\n{{test_cases}}\n}\n".to_string(),
                test_case_template: "    @Test\n    fun test_{{case_id}}() {\n        // Evidence: {{evidence}}\n        // Oracle: {{oracle}}\n        // Input: {{input}}\n        // Expected: {{expected}}\n    }".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new(
                    "kotlin",
                    ["gradle", "compileKotlin"],
                )
                .with_format(["ktlint"])
                .with_tests(["gradle", "test"]),
            },
            TargetLanguageSpecRecord {
                id: "typescript".to_string(),
                name: "TypeScript".to_string(),
                aliases: vec!["ts".to_string()],
                file_extension: "ts".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "package.json".to_string(),
                entrypoint_filename: "index.ts".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "// Autonomous Project Exodus Migrated Target (TypeScript)\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "// Migrated TypeScript module `{{module_name}}`\n// Migrated from {{source_path}}\n\nexport const placeholder = () => {};\n".to_string(),
                module_export_template: Some("export * from './{{module_name}}';".to_string()),
                package_manifest_template: "{\n  \"name\": \"{{pkg_name}}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": {\n    \"build\": \"tsc\",\n    \"test\": \"vitest run\"\n  },\n  \"devDependencies\": {\n    \"typescript\": \"^5.3.0\",\n    \"vitest\": \"^1.0.0\"\n  }\n}\n".to_string(),
                workspace_manifest_filename: Some("package.json".to_string()),
                workspace_manifest_template: Some("{\n  \"private\": true,\n  \"workspaces\": [\n{{workspace_members}}  ]\n}\n".to_string()),
                workspace_member_template: Some("    \"{{package_path}}\",\n".to_string()),
                internal_dependency_template: None,
                version_files: vec![("tsconfig.json".to_string(), "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"NodeNext\",\n    \"strict\": true,\n    \"esModuleInterop\": true,\n    \"skipLibCheck\": true\n  }\n}\n".to_string())],
                quickstart_command: "npm test".to_string(),
                test_harness_template: "import { describe, it, expect } from 'vitest';\n\ndescribe('Behavioral Contracts', () => {\n{{test_cases}}\n});\n".to_string(),
                test_case_template: "  it('test_{{case_id}}', () => {\n    // Evidence: {{evidence}}\n    // Oracle: {{oracle}}\n    // Input: {{input}}\n    // Expected: {{expected}}\n  });".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("typescript", ["tsc", "--noEmit"])
                    .with_format(["prettier", "--check", "."])
                    .with_tests(["npm", "test"])
                    .with_diagnostic_format(DiagnosticFormat::TypeScript),
            },
            TargetLanguageSpecRecord {
                id: "javascript".to_string(),
                name: "JavaScript".to_string(),
                aliases: vec!["js".to_string(), "node".to_string()],
                file_extension: "js".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "package.json".to_string(),
                entrypoint_filename: "index.js".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "// Autonomous Project Exodus Migrated Target (JavaScript)\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "// Migrated JavaScript module `{{module_name}}`\n// Migrated from {{source_path}}\n\nexport const placeholder = () => {};\n".to_string(),
                module_export_template: Some("export * from './{{module_name}}';".to_string()),
                package_manifest_template: "{\n  \"name\": \"{{pkg_name}}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": {\n    \"test\": \"node --test\"\n  }\n}\n".to_string(),
                workspace_manifest_filename: Some("package.json".to_string()),
                workspace_manifest_template: Some("{\n  \"private\": true,\n  \"workspaces\": [\n{{workspace_members}}  ]\n}\n".to_string()),
                workspace_member_template: Some("    \"{{package_path}}\",\n".to_string()),
                internal_dependency_template: None,
                version_files: vec![(".node-version".to_string(), "20\n".to_string())],
                quickstart_command: "npm test".to_string(),
                test_harness_template: "import test from 'node:test';\nimport assert from 'node:assert/strict';\n\n{{test_cases}}\n".to_string(),
                test_case_template: "test('test_{{case_id}}', () => {\n    // Evidence: {{evidence}}\n    // Oracle: {{oracle}}\n    // Input: {{input}}\n    // Expected: {{expected}}\n});".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("javascript", ["node", "--check"]),
            },
            TargetLanguageSpecRecord {
                id: "python".to_string(),
                name: "Python".to_string(),
                aliases: vec!["py".to_string()],
                file_extension: "py".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "pyproject.toml".to_string(),
                entrypoint_filename: "__init__.py".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "\"\"\"Autonomous Project Exodus Migrated Target Package.\"\"\"\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "\"\"\"Migrated module {{module_name}} from {{source_path}}.\"\"\"\n\ndef placeholder():\n    pass\n".to_string(),
                module_export_template: Some("from . import {{module_name}}".to_string()),
                package_manifest_template: "[build-system]\nrequires = [\"setuptools>=61.0\"]\nbuild-backend = \"setuptools.build_meta\"\n\n[project]\nname = \"{{pkg_name}}\"\nversion = \"0.1.0\"\n".to_string(),
                workspace_manifest_filename: None,
                workspace_manifest_template: None,
                workspace_member_template: None,
                internal_dependency_template: None,
                version_files: vec![(".python-version".to_string(), "3.11\n".to_string())],
                quickstart_command: "pytest".to_string(),
                test_harness_template: "import pytest\n\n{{test_cases}}\n".to_string(),
                test_case_template: "def test_{{case_id}}():\n    # Evidence: {{evidence}}\n    # Oracle: {{oracle}}\n    # Input: {{input}}\n    # Expected: {{expected}}\n    pass\n".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("python", ["mypy", "."])
                    .with_format(["black", "--check", "."])
                    .with_tests(["pytest"])
                    .with_diagnostic_format(DiagnosticFormat::Pytest),
            },
            TargetLanguageSpecRecord {
                id: "csharp".to_string(),
                name: "C#".to_string(),
                aliases: vec!["cs".to_string(), "c#".to_string(), "dotnet".to_string()],
                file_extension: "cs".to_string(),
                source_dir: "src".to_string(),
                manifest_name: "{{pkg_name}}.csproj".to_string(),
                entrypoint_filename: "Program.cs".to_string(),
                ecosystem_container_term: "Project".to_string(),
                entrypoint_template: "namespace {{safe_pkg}};\n\npublic static class Program {\n    public static void Main() => System.Console.WriteLine(\"Exodus C# Target\");\n}\n".to_string(),
                module_stub_template: "// Migrated C# class `{{module_name}}`\nnamespace {{safe_pkg}};\n\npublic class {{module_name}} {\n}\n".to_string(),
                module_export_template: None,
                package_manifest_template: "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <Nullable>enable</Nullable>\n  </PropertyGroup>\n</Project>\n".to_string(),
                workspace_manifest_filename: Some("Solution.sln".to_string()),
                workspace_manifest_template: None,
                workspace_member_template: None,
                internal_dependency_template: None,
                version_files: vec![("global.json".to_string(), "{\n  \"sdk\": { \"version\": \"8.0.100\" }\n}\n".to_string())],
                quickstart_command: "dotnet test".to_string(),
                test_harness_template: "using Xunit;\n\nnamespace {{safe_pkg}}.Tests;\n\npublic class BehavioralContractTests {\n{{test_cases}}\n}\n".to_string(),
                test_case_template: "    [Fact]\n    public void Test_{{case_id}}() {\n        // Evidence: {{evidence}}\n        // Oracle: {{oracle}}\n        // Input: {{input}}\n        // Expected: {{expected}}\n    }".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("csharp", ["dotnet", "build"]),
            },
            TargetLanguageSpecRecord {
                id: "dart".to_string(),
                name: "Dart".to_string(),
                aliases: vec!["flutter".to_string()],
                file_extension: "dart".to_string(),
                source_dir: "lib".to_string(),
                manifest_name: "pubspec.yaml".to_string(),
                entrypoint_filename: "{{safe_pkg}}.dart".to_string(),
                ecosystem_container_term: "Package".to_string(),
                entrypoint_template: "// Autonomous Project Exodus Migrated Target (Dart)\nlibrary {{safe_pkg}};\n\n{{modules_exports}}\n".to_string(),
                module_stub_template: "// Migrated Dart module `{{module_name}}`\nlibrary {{safe_pkg}};\n\nvoid placeholder() {}\n".to_string(),
                module_export_template: None,
                package_manifest_template: "name: {{safe_pkg}}\ndescription: Migrated Dart package\nversion: 0.1.0\nenvironment:\n  sdk: '>=3.0.0 <4.0.0'\ndev_dependencies:\n  test: ^1.24.0\n".to_string(),
                workspace_manifest_filename: None,
                workspace_manifest_template: None,
                workspace_member_template: None,
                internal_dependency_template: None,
                version_files: vec![],
                quickstart_command: "dart test".to_string(),
                test_harness_template: "import 'package:test/test.dart';\n\nvoid main() {\n{{test_cases}}\n}\n".to_string(),
                test_case_template: "  test('{{case_id}}', () {\n    // Evidence: {{evidence}}\n    // Oracle: {{oracle}}\n    // Input: {{input}}\n    // Expected: {{expected}}\n  });".to_string(),
                signature_rule_sets: Vec::new(),
                toolchain_profile: ToolchainProfile::new("dart", ["dart", "analyze"]),
            },
        ];
        for spec in &mut specs {
            spec.signature_rule_sets = Self::builtin_signature_rules(&spec.id);
        }
        specs
    }

    fn builtin_signature_rules(target: &str) -> Vec<SignatureRuleSet> {
        let grounded = crate::GroundingTier::Deterministic;
        let mapping = |source: &str, target: &str| TypeMappingRule {
            source_pattern: source.to_string(),
            target_type: target.to_string(),
            generic_template: None,
            nullable_template: None,
            nullability: NullabilityPolicy::Reject,
            grounding: grounded,
        };
        match target {
            "rust" => vec![SignatureRuleSet {
                schema_version: "1.0.0".to_string(),
                revision: 1,
                source_language: LanguageId::new("python"),
                target_language: LanguageId::new("rust"),
                function_keyword: "pub fn".to_string(),
                signature_template: "{{function_keyword}} {{name}}({{parameters}}){{return_type}}"
                    .to_string(),
                receiver: ReceiverRule {
                    instance_template: "&self".to_string(),
                    mutable_instance_template: Some("&mut self".to_string()),
                    static_template: None,
                },
                parameters: ParameterRule {
                    template: "{{name}}: {{type}}".to_string(),
                    separator: ", ".to_string(),
                    variadic_template: None,
                    default_values_supported: false,
                },
                return_type: ReturnTypeRule {
                    template: " -> {{type}}".to_string(),
                    void_types: vec!["none".to_string(), "void".to_string()],
                    async_wrapper_template: None,
                },
                type_mappings: vec![
                    mapping("int", "i64"),
                    mapping("str", "&str"),
                    mapping("bool", "bool"),
                    mapping("float", "f64"),
                    TypeMappingRule {
                        source_pattern: "list".to_string(),
                        target_type: "Vec<{{inner}}>".to_string(),
                        generic_template: Some("Vec<{{inner}}>".to_string()),
                        nullable_template: Some("Option<{{type}}>".to_string()),
                        nullability: NullabilityPolicy::Wrap,
                        grounding: grounded,
                    },
                ],
            }],
            "go" => vec![SignatureRuleSet {
                schema_version: "1.0.0".to_string(),
                revision: 1,
                source_language: LanguageId::new("typescript"),
                target_language: LanguageId::new("go"),
                function_keyword: "func".to_string(),
                signature_template: "{{function_keyword}} {{name}}({{parameters}}){{return_type}}"
                    .to_string(),
                receiver: ReceiverRule {
                    instance_template: "receiver any".to_string(),
                    mutable_instance_template: None,
                    static_template: None,
                },
                parameters: ParameterRule {
                    template: "{{name}} {{type}}".to_string(),
                    separator: ", ".to_string(),
                    variadic_template: Some("{{name}} ...{{type}}".to_string()),
                    default_values_supported: false,
                },
                return_type: ReturnTypeRule {
                    template: " {{type}}".to_string(),
                    void_types: vec!["void".to_string(), "undefined".to_string()],
                    async_wrapper_template: None,
                },
                type_mappings: vec![
                    mapping("number", "float64"),
                    mapping("string", "string"),
                    mapping("boolean", "bool"),
                    mapping("unknown", "any"),
                    TypeMappingRule {
                        source_pattern: "array".to_string(),
                        target_type: "[]{{inner}}".to_string(),
                        generic_template: Some("[]{{inner}}".to_string()),
                        nullable_template: Some("*{{type}}".to_string()),
                        nullability: NullabilityPolicy::Wrap,
                        grounding: grounded,
                    },
                ],
            }],
            _ => Vec::new(),
        }
    }
}

impl ToolchainProfile {
    pub fn new<L, I, S>(target_language: L, check_command: I) -> Self
    where
        L: Into<LanguageId>,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            target_language: target_language.into(),
            format_command: None,
            check_command: check_command.into_iter().map(Into::into).collect(),
            test_command: None,
            diagnostic_format: DiagnosticFormat::Generic,
            env_vars: HashMap::new(),
        }
    }

    pub fn with_format<I, S>(mut self, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.format_command = Some(command.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_tests<I, S>(mut self, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.test_command = Some(command.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_diagnostic_format(mut self, format: DiagnosticFormat) -> Self {
        self.diagnostic_format = format;
        self
    }
}

/// Output of a code generation or rendering phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedArtifact {
    pub relative_path: std::path::PathBuf,
    pub content: String,
}

/// Output produced by native verification tools (compiler, linter, tests).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VerificationOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<Diagnostic>,
    pub state: Option<VerificationState>,
}

/// Abstract contract for parsing and extracting language-neutral ESG semantics from source code.
#[async_trait]
pub trait SourceLanguageAdapter: Send + Sync {
    /// Language ID this adapter handles.
    fn language_id(&self) -> LanguageId;

    /// File extensions supported by this source adapter (e.g. `["py"]`, `["ts", "tsx"]`, `["go"]`).
    fn supported_extensions(&self) -> &[&'static str];

    /// Determines if this adapter can process a given file.
    fn can_handle_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.supported_extensions().contains(&ext)
        } else {
            false
        }
    }

    /// Parses a single file and extracts ESG nodes, edges, diagnostics, and deprecation candidates.
    async fn parse_file(
        &self,
        repo_root: &Path,
        rel_path: &Path,
        content: &str,
    ) -> Result<(
        Vec<EsgNode>,
        Vec<EsgEdge>,
        Vec<Diagnostic>,
        Vec<DeprecationRecord>,
    )>;

    /// Profiles the repository's architecture, dependencies, build/test commands, and concurrency.
    async fn profile_repository(&self, repo_root: &Path) -> Result<RepositoryProfile>;
}

/// Abstract contract for rendering ESG semantics and manifests into target-native source code.
#[async_trait]
pub trait TargetLanguageAdapter: Send + Sync {
    /// Target language ID.
    fn target_id(&self) -> LanguageId;

    /// Generates target package manifests (e.g. `Cargo.toml`, `go.mod`, `package.json`).
    async fn generate_manifests(
        &self,
        profile: &RepositoryProfile,
    ) -> Result<Vec<RenderedArtifact>>;

    /// Renders an ESG node or cluster into target-native code.
    async fn render_node(
        &self,
        node: &EsgNode,
        contract: Option<&BehavioralContract>,
    ) -> Result<RenderedArtifact>;
}

/// Abstract contract for verifying generated code against target toolchains.
#[async_trait]
pub trait TargetVerifier: Send + Sync {
    /// Target language ID.
    fn target_id(&self) -> LanguageId;

    /// Formats code in workspace according to language idioms.
    async fn format(&self, workspace_root: &Path) -> Result<VerificationOutput>;

    /// Runs compiler checks or type verification.
    async fn check_compile(&self, workspace_root: &Path) -> Result<VerificationOutput>;

    /// Executes behavioral tests with optional name filter.
    async fn run_tests(
        &self,
        workspace_root: &Path,
        filter: Option<&str>,
    ) -> Result<VerificationOutput>;

    /// Verifies behavioral contract assertions for a given symbol.
    async fn verify_contract(
        &self,
        workspace_root: &Path,
        contract: &BehavioralContract,
    ) -> Result<VerificationOutput> {
        self.run_tests(workspace_root, Some(&contract.unit_id))
            .await
    }
}

/// Thread-safe dynamic registry of language adapters and target verifiers.
#[derive(Clone, Default)]
pub struct LanguageAdapterRegistry {
    source_adapters: HashMap<String, Arc<dyn SourceLanguageAdapter>>,
    target_adapters: HashMap<LanguageId, Arc<dyn TargetLanguageAdapter>>,
    target_verifiers: HashMap<LanguageId, Arc<dyn TargetVerifier>>,
}

impl LanguageAdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source adapter and indexes it by its supported file extensions.
    pub async fn register_source_adapter(
        &mut self,
        adapter: Arc<dyn SourceLanguageAdapter>,
    ) -> Result<()> {
        for ext in adapter.supported_extensions() {
            let key = ext.to_lowercase();
            if self.source_adapters.contains_key(&key) {
                return Err(crate::ExodusError::Generic(format!(
                    "Source adapter for extension '.{}' already registered",
                    key
                )));
            }
        }
        for ext in adapter.supported_extensions() {
            self.source_adapters
                .insert(ext.to_lowercase(), adapter.clone());
        }
        Ok(())
    }

    /// Finds a registered source adapter for a specific file path.
    pub fn find_source_adapter_for_path(
        &self,
        path: &Path,
    ) -> Option<Arc<dyn SourceLanguageAdapter>> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        self.source_adapters.get(&ext).cloned()
    }

    /// Registers a target language adapter.
    pub async fn register_target_adapter(
        &mut self,
        adapter: Arc<dyn TargetLanguageAdapter>,
    ) -> Result<()> {
        self.target_adapters.insert(adapter.target_id(), adapter);
        Ok(())
    }

    /// Finds a registered target language adapter.
    pub fn get_target_adapter(
        &self,
        target_id: &LanguageId,
    ) -> Option<Arc<dyn TargetLanguageAdapter>> {
        self.target_adapters.get(target_id).cloned()
    }

    /// Registers a target verifier.
    pub async fn register_target_verifier(
        &mut self,
        verifier: Arc<dyn TargetVerifier>,
    ) -> Result<()> {
        self.target_verifiers.insert(verifier.target_id(), verifier);
        Ok(())
    }

    /// Finds a registered target verifier.
    pub fn get_target_verifier(&self, target_id: &LanguageId) -> Option<Arc<dyn TargetVerifier>> {
        self.target_verifiers.get(target_id).cloned()
    }
}
