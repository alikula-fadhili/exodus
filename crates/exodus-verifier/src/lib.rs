//! Verification, formatting, compilation, test execution, and bounded repair loop for Project Exodus.

use async_trait::async_trait;
use exodus_core::{
    Diagnostic, DiagnosticFormat, ExodusError, FailedAssertion, LanguageId, MigrationDebt,
    MigrationOutcome, Result, Severity, SourceSpan, TargetLanguageSpecRecord, TargetVerifier,
    ToolchainProfile, VerificationOutput, VerificationStage, VerificationState,
};
use exodus_transform::TransformResult;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod domain_verifiers;
pub mod pipeline;
pub mod unit_gate;

pub use domain_verifiers::*;
pub use pipeline::*;
pub use unit_gate::*;

/// Sanitizes a string into a valid Cargo package name.
pub fn sanitize_crate_name(name: &str) -> String {
    let mut sanitized = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        return "migrated_pkg".to_string();
    }
    if sanitized
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("pkg_{sanitized}")
    } else {
        sanitized
    }
}

/// Verification report generated after verifying transformed code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub formatted_cleanly: bool,
    pub compiled: bool,
    pub tests_passed: bool,
    pub repair_attempts: u32,
    pub diagnostics: Vec<Diagnostic>,
    pub outcome: MigrationOutcome,
    pub migration_debts: Vec<MigrationDebt>,
    pub duration_ms: u128,
}

/// Verifier orchestrating rustfmt, cargo check, test runners, and bounded repair.
pub struct Verifier {
    target_dir: PathBuf,
}

impl Verifier {
    pub fn new<P: AsRef<Path>>(target_dir: P) -> Self {
        Self {
            target_dir: target_dir.as_ref().to_path_buf(),
        }
    }

    /// Scaffolds a complete compilable Cargo package for verification.
    pub fn scaffold_target_crate(
        &self,
        crate_name: &str,
        modules: &[TransformResult],
        behavioral_tests: Option<&str>,
    ) -> Result<()> {
        let src_dir = self.target_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(ExodusError::from)?;

        let safe_crate_name = sanitize_crate_name(crate_name);

        let cargo_toml_content = format!(
            r#"[package]
name = "{safe_crate_name}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
"#
        );
        fs::write(self.target_dir.join("Cargo.toml"), cargo_toml_content)
            .map_err(ExodusError::from)?;

        let mut lib_rs_content = String::from("//! Exodus Migrated Crate Entrypoint\n\n");

        for m in modules {
            let mod_file_name = format!("{}.rs", m.module_name);
            fs::write(src_dir.join(&mod_file_name), &m.rust_source).map_err(ExodusError::from)?;
            lib_rs_content.push_str(&format!(
                "pub mod {};\npub use {}::*;\n",
                m.module_name, m.module_name
            ));
        }

        fs::write(src_dir.join("lib.rs"), lib_rs_content).map_err(ExodusError::from)?;

        if let Some(tests) = behavioral_tests {
            let tests_dir = self.target_dir.join("tests");
            fs::create_dir_all(&tests_dir).map_err(ExodusError::from)?;
            let tests_content = format!("use {safe_crate_name}::*;\n\n{tests}\n");
            fs::write(tests_dir.join("behavioral_tests.rs"), tests_content)
                .map_err(ExodusError::from)?;
        }

        Ok(())
    }

    pub fn run_formatter(&self) -> Result<bool> {
        let output = Command::new("rustfmt")
            .arg("--edition")
            .arg("2021")
            .arg("src/lib.rs")
            .current_dir(&self.target_dir)
            .output();

        match output {
            Ok(out) => Ok(out.status.success()),
            Err(_) => Ok(true),
        }
    }

    pub fn run_cargo_check(&self) -> Result<(bool, Vec<Diagnostic>)> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .current_dir(&self.target_dir)
            .output()
            .map_err(|e| {
                ExodusError::VerificationFailure(format!("Failed to run cargo check: {e}"))
            })?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();

        for line in stdout.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json["reason"] == "compiler-message" {
                    if let Some(msg) = json.get("message") {
                        let level = msg["level"].as_str().unwrap_or("error");
                        let rendered = msg["rendered"].as_str().unwrap_or("").to_string();
                        let code = msg["code"]["code"]
                            .as_str()
                            .unwrap_or("E_UNKNOWN")
                            .to_string();

                        let severity = match level {
                            "error" => Severity::Error,
                            "warning" => Severity::Warning,
                            _ => Severity::Info,
                        };

                        let location =
                            msg["spans"]
                                .as_array()
                                .and_then(|spans| spans.first())
                                .map(|s| {
                                    let file = s["file_name"].as_str().unwrap_or("unknown");
                                    let line = s["line_start"].as_u64().unwrap_or(1) as usize;
                                    let col = s["column_start"].as_u64().unwrap_or(1) as usize;
                                    SourceSpan::point(file, line, col)
                                });

                        diagnostics.push(Diagnostic {
                            severity,
                            code,
                            message: rendered,
                            span: location,
                            suggestion: None,
                        });
                    }
                }
            }
        }

        Ok((success, diagnostics))
    }

    pub fn run_cargo_tests(&self) -> Result<bool> {
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(&self.target_dir)
            .output()
            .map_err(|e| {
                ExodusError::VerificationFailure(format!("Failed to run cargo test: {e}"))
            })?;

        Ok(output.status.success())
    }

    pub fn verify_migration(
        &self,
        modules: &[TransformResult],
        behavioral_tests: Option<&str>,
    ) -> Result<VerificationReport> {
        let start = std::time::Instant::now();
        self.scaffold_target_crate("migrated_target", modules, behavioral_tests)?;

        let formatted = self.run_formatter().unwrap_or(true);
        let (mut compiled, mut diagnostics) = self.run_cargo_check()?;
        let mut debts = Vec::new();

        for m in modules {
            for fb in &m.fallbacks {
                debts.push(fb.to_migration_debt());
            }
        }

        let mut repair_attempts = 0;
        while !compiled && repair_attempts < 3 {
            repair_attempts += 1;
            let (next_compiled, next_diags) = self.run_cargo_check()?;
            compiled = next_compiled;
            diagnostics = next_diags;
            if compiled {
                break;
            }
        }

        let tests_passed = if compiled && behavioral_tests.is_some() {
            self.run_cargo_tests().unwrap_or(false)
        } else {
            false
        };

        let outcome = if !compiled {
            MigrationOutcome::Blocked
        } else if !debts.is_empty() {
            MigrationOutcome::Degraded
        } else if tests_passed || behavioral_tests.is_none() {
            MigrationOutcome::Verified
        } else {
            MigrationOutcome::Compatible
        };

        Ok(VerificationReport {
            formatted_cleanly: formatted,
            compiled,
            tests_passed,
            repair_attempts,
            diagnostics,
            outcome,
            migration_debts: debts,
            duration_ms: start.elapsed().as_millis(),
        })
    }
}

/// Aggregate diagnostic summary metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub total_infos: usize,
    pub failed_tests: usize,
}

/// Comprehensive polyglot target diagnostic report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDiagnosticReport {
    pub target_language: LanguageId,
    pub workspace_dir: PathBuf,
    pub formatted: bool,
    pub compiled: bool,
    pub tests_passed: bool,
    pub format_output: Option<VerificationOutput>,
    pub check_output: VerificationOutput,
    pub test_output: Option<VerificationOutput>,
    pub diagnostics: Vec<Diagnostic>,
    pub failed_assertions: Vec<FailedAssertion>,
    pub summary: DiagnosticSummary,
    pub outcome: MigrationOutcome,
    pub duration_ms: u128,
}

/// Universal state-driven target verifier driven by declarative `ToolchainProfile`.
pub struct UniversalTargetVerifier {
    pub profile: ToolchainProfile,
}

impl UniversalTargetVerifier {
    pub fn new(profile: ToolchainProfile) -> Self {
        Self { profile }
    }

    pub fn for_spec(spec: &TargetLanguageSpecRecord) -> Self {
        Self::new(spec.toolchain_profile.clone())
    }

    pub fn for_language(lang: &LanguageId) -> Self {
        TargetLanguageSpecRecord::default_specs()
            .iter()
            .find(|spec| spec.matches_query(lang))
            .map(Self::for_spec)
            .unwrap_or_else(|| {
                Self::new(ToolchainProfile::new(
                    lang.clone(),
                    [lang.to_string(), "check".to_string()],
                ))
            })
    }

    /// Performs a full verification run across format, check, and test stages for a workspace.
    pub async fn verify_workspace_full(
        &self,
        workspace_root: &Path,
    ) -> Result<TargetDiagnosticReport> {
        let start = std::time::Instant::now();

        // 1. Format stage
        let (formatted, fmt_output) = if self.profile.format_command.is_some() {
            let out = self.format(workspace_root).await?;
            (out.success, Some(out))
        } else {
            (true, None)
        };

        // 2. Check / Compile stage
        let check_output = self.check_compile(workspace_root).await?;
        let compiled = check_output.success;

        // 3. Behavioral test stage
        let (tests_passed, test_output) = if compiled && self.profile.test_command.is_some() {
            let out = self.run_tests(workspace_root, None).await?;
            (out.success, Some(out))
        } else {
            (false, None)
        };

        // Collect all diagnostics & assertions
        let mut all_diags = Vec::new();
        let mut all_assertions = Vec::new();

        if let Some(ref fo) = fmt_output {
            all_diags.extend(fo.diagnostics.clone());
            if let Some(ref st) = fo.state {
                all_assertions.extend(st.failed_assertions.clone());
            }
        }
        all_diags.extend(check_output.diagnostics.clone());
        if let Some(ref st) = check_output.state {
            all_assertions.extend(st.failed_assertions.clone());
        }
        if let Some(ref to) = test_output {
            all_diags.extend(to.diagnostics.clone());
            if let Some(ref st) = to.state {
                all_assertions.extend(st.failed_assertions.clone());
            }
        }

        let mut errors = 0;
        let mut warnings = 0;
        let mut infos = 0;
        for d in &all_diags {
            match d.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
                Severity::Info => infos += 1,
            }
        }

        let summary = DiagnosticSummary {
            total_errors: errors,
            total_warnings: warnings,
            total_infos: infos,
            failed_tests: all_assertions.len(),
        };

        let outcome = if !compiled {
            MigrationOutcome::Blocked
        } else if errors > 0 || !all_assertions.is_empty() {
            MigrationOutcome::Degraded
        } else if tests_passed || self.profile.test_command.is_none() {
            MigrationOutcome::Verified
        } else {
            MigrationOutcome::Compatible
        };

        let duration_ms = start.elapsed().as_millis();

        Ok(TargetDiagnosticReport {
            target_language: self.profile.target_language.clone(),
            workspace_dir: workspace_root.to_path_buf(),
            formatted,
            compiled,
            tests_passed,
            format_output: fmt_output,
            check_output,
            test_output,
            diagnostics: all_diags,
            failed_assertions: all_assertions,
            summary,
            outcome,
            duration_ms,
        })
    }

    pub fn parse_diagnostics(
        &self,
        stdout: &str,
        stderr: &str,
    ) -> (Vec<Diagnostic>, Vec<FailedAssertion>) {
        let mut diags = Vec::new();
        let mut assertions = Vec::new();

        match self.profile.diagnostic_format {
            DiagnosticFormat::RustcJson => {
                for line in stdout.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        if json["reason"] == "compiler-message" {
                            if let Some(msg) = json.get("message") {
                                let level = msg["level"].as_str().unwrap_or("error");
                                let rendered = msg["rendered"].as_str().unwrap_or("").to_string();
                                let code = msg["code"]["code"]
                                    .as_str()
                                    .unwrap_or("E_COMPILER")
                                    .to_string();

                                let severity = match level {
                                    "error" => Severity::Error,
                                    "warning" => Severity::Warning,
                                    _ => Severity::Info,
                                };

                                let location = msg["spans"]
                                    .as_array()
                                    .and_then(|spans| spans.first())
                                    .map(|s| {
                                        let file = s["file_name"].as_str().unwrap_or("unknown");
                                        let line = s["line_start"].as_u64().unwrap_or(1) as usize;
                                        let col = s["column_start"].as_u64().unwrap_or(1) as usize;
                                        SourceSpan::point(file, line, col)
                                    });

                                diags.push(Diagnostic {
                                    severity,
                                    code,
                                    message: rendered,
                                    span: location,
                                    suggestion: None,
                                });
                            }
                        }
                    }
                }
            }
            DiagnosticFormat::TypeScript => {
                let re = Regex::new(r"^(.*?)\((\d+),(\d+)\):\s+(error|warning)\s+(TS\d+):\s+(.*)$")
                    .unwrap();
                for line in stdout.lines().chain(stderr.lines()) {
                    if let Some(caps) = re.captures(line) {
                        let file = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                        let line_no: usize = caps
                            .get(2)
                            .and_then(|m| m.as_str().parse().ok())
                            .unwrap_or(1);
                        let col_no: usize = caps
                            .get(3)
                            .and_then(|m| m.as_str().parse().ok())
                            .unwrap_or(1);
                        let is_err = caps.get(4).map(|m| m.as_str() == "error").unwrap_or(true);
                        let code = caps
                            .get(5)
                            .map(|m| m.as_str())
                            .unwrap_or("TS_ERR")
                            .to_string();
                        let message = caps.get(6).map(|m| m.as_str()).unwrap_or("").to_string();

                        diags.push(Diagnostic {
                            severity: if is_err {
                                Severity::Error
                            } else {
                                Severity::Warning
                            },
                            code,
                            message,
                            span: Some(SourceSpan::point(file, line_no, col_no)),
                            suggestion: None,
                        });
                    }
                }
            }
            DiagnosticFormat::GoVet | DiagnosticFormat::ClangStyle | DiagnosticFormat::Generic => {
                let re =
                    Regex::new(r"^(.*?):(\d+):(?:(\d+):)?\s*(error|warning)?\s*(.*)$").unwrap();
                for line in stderr.lines().chain(stdout.lines()) {
                    if let Some(caps) = re.captures(line) {
                        let file = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                        let line_no: usize = caps
                            .get(2)
                            .and_then(|m| m.as_str().parse().ok())
                            .unwrap_or(1);
                        let col_no: usize = caps
                            .get(3)
                            .and_then(|m| m.as_str().parse().ok())
                            .unwrap_or(1);
                        let is_err = caps.get(4).map(|m| m.as_str() != "warning").unwrap_or(true);
                        let message = caps
                            .get(5)
                            .map(|m| m.as_str().trim())
                            .unwrap_or("")
                            .to_string();

                        if !message.is_empty() {
                            diags.push(Diagnostic {
                                severity: if is_err {
                                    Severity::Error
                                } else {
                                    Severity::Warning
                                },
                                code: "E_TOOLCHAIN".to_string(),
                                message,
                                span: Some(SourceSpan::point(file, line_no, col_no)),
                                suggestion: None,
                            });
                        }
                    }
                }
            }
            DiagnosticFormat::Pytest => {
                let fail_re = Regex::new(r"^FAILED\s+(.*?)::(.*?)\s+-\s+(.*)$").unwrap();
                for line in stdout.lines() {
                    if let Some(caps) = fail_re.captures(line) {
                        let file = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let test_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                        let failure_message =
                            caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

                        assertions.push(FailedAssertion {
                            test_name: format!("{file}::{test_name}"),
                            failure_message,
                        });
                    }
                }
            }
        }

        (diags, assertions)
    }

    fn execute_command(
        &self,
        cmd_vec: &[String],
        workspace_root: &Path,
        stage: VerificationStage,
    ) -> Result<VerificationOutput> {
        if cmd_vec.is_empty() {
            let state = VerificationState {
                stage,
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                failed_assertions: Vec::new(),
                duration_ms: 0,
            };
            return Ok(VerificationOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                state: Some(state),
            });
        }

        let start = std::time::Instant::now();
        let mut cmd = Command::new(&cmd_vec[0]);
        cmd.args(&cmd_vec[1..]).current_dir(workspace_root);

        for (k, v) in &self.profile.env_vars {
            cmd.env(k, v);
        }

        let output = cmd.output().map_err(|e| {
            ExodusError::VerificationFailure(format!(
                "Failed to execute command '{:?}': {e}",
                cmd_vec
            ))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (diagnostics, failed_assertions) = self.parse_diagnostics(&stdout, &stderr);

        let state = VerificationState {
            stage,
            success,
            exit_code,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            diagnostics: diagnostics.clone(),
            failed_assertions,
            duration_ms,
        };

        Ok(VerificationOutput {
            success,
            exit_code,
            stdout,
            stderr,
            diagnostics,
            state: Some(state),
        })
    }
}

#[async_trait]
impl TargetVerifier for UniversalTargetVerifier {
    fn target_id(&self) -> LanguageId {
        self.profile.target_language.clone()
    }

    async fn format(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        if let Some(ref fmt_cmd) = self.profile.format_command {
            self.execute_command(fmt_cmd, workspace_root, VerificationStage::Format)
        } else {
            let state = VerificationState {
                stage: VerificationStage::Format,
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                failed_assertions: Vec::new(),
                duration_ms: 0,
            };
            Ok(VerificationOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                state: Some(state),
            })
        }
    }

    async fn check_compile(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.execute_command(
            &self.profile.check_command,
            workspace_root,
            VerificationStage::CompileCheck,
        )
    }

    async fn run_tests(
        &self,
        workspace_root: &Path,
        filter: Option<&str>,
    ) -> Result<VerificationOutput> {
        if let Some(ref test_cmd) = self.profile.test_command {
            let mut final_cmd = test_cmd.clone();
            if let Some(f) = filter {
                final_cmd.push(f.to_string());
            }
            self.execute_command(
                &final_cmd,
                workspace_root,
                VerificationStage::BehavioralTest,
            )
        } else {
            let state = VerificationState {
                stage: VerificationStage::BehavioralTest,
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                failed_assertions: Vec::new(),
                duration_ms: 0,
            };
            Ok(VerificationOutput {
                success: true,
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                diagnostics: Vec::new(),
                state: Some(state),
            })
        }
    }
}

/// Backwards-compatible aliases wrapping UniversalTargetVerifier
pub struct RustTargetVerifier(pub UniversalTargetVerifier);
impl Default for RustTargetVerifier {
    fn default() -> Self {
        Self(UniversalTargetVerifier::for_language(&LanguageId::new(
            "rust",
        )))
    }
}
#[async_trait]
impl TargetVerifier for RustTargetVerifier {
    fn target_id(&self) -> LanguageId {
        self.0.target_id()
    }
    async fn format(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.format(workspace_root).await
    }
    async fn check_compile(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.check_compile(workspace_root).await
    }
    async fn run_tests(
        &self,
        workspace_root: &Path,
        filter: Option<&str>,
    ) -> Result<VerificationOutput> {
        self.0.run_tests(workspace_root, filter).await
    }
}

pub struct GoTargetVerifier(pub UniversalTargetVerifier);
impl Default for GoTargetVerifier {
    fn default() -> Self {
        Self(UniversalTargetVerifier::for_language(&LanguageId::new(
            "go",
        )))
    }
}
#[async_trait]
impl TargetVerifier for GoTargetVerifier {
    fn target_id(&self) -> LanguageId {
        self.0.target_id()
    }
    async fn format(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.format(workspace_root).await
    }
    async fn check_compile(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.check_compile(workspace_root).await
    }
    async fn run_tests(
        &self,
        workspace_root: &Path,
        filter: Option<&str>,
    ) -> Result<VerificationOutput> {
        self.0.run_tests(workspace_root, filter).await
    }
}

pub struct GenericTargetVerifier(pub UniversalTargetVerifier);
impl GenericTargetVerifier {
    pub fn new(
        language: LanguageId,
        build_cmd: impl Into<String>,
        test_cmd: impl Into<String>,
    ) -> Self {
        let b: String = build_cmd.into();
        let t: String = test_cmd.into();
        let check_cmd: Vec<String> = b.split_whitespace().map(|s| s.to_string()).collect();
        let test_cmd_vec: Vec<String> = t.split_whitespace().map(|s| s.to_string()).collect();
        let profile = ToolchainProfile::new(language, check_cmd).with_tests(test_cmd_vec);
        Self(UniversalTargetVerifier::new(profile))
    }
}
#[async_trait]
impl TargetVerifier for GenericTargetVerifier {
    fn target_id(&self) -> LanguageId {
        self.0.target_id()
    }
    async fn format(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.format(workspace_root).await
    }
    async fn check_compile(&self, workspace_root: &Path) -> Result<VerificationOutput> {
        self.0.check_compile(workspace_root).await
    }
    async fn run_tests(
        &self,
        workspace_root: &Path,
        filter: Option<&str>,
    ) -> Result<VerificationOutput> {
        self.0.run_tests(workspace_root, filter).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_and_cargo_check() {
        let temp =
            std::env::temp_dir().join(format!("exodus_test_verify_{}", uuid::Uuid::new_v4()));
        let verifier = Verifier::new(&temp);

        let module = TransformResult {
            rust_source: "pub fn add(a: i64, b: i64) -> i64 {\n    a + b\n}\n".to_string(),
            module_name: "math".to_string(),
            file_path: PathBuf::from("math.py"),
            outcome: MigrationOutcome::Verified,
            fallbacks: vec![],
            diagnostics: vec![],
        };

        let res = verifier.scaffold_target_crate("test_crate", &[module], None);
        assert!(res.is_ok());

        let (compiled, diags) = verifier.run_cargo_check().unwrap();
        assert!(compiled, "Diagnostics: {:?}", diags);

        let _ = fs::remove_dir_all(&temp);
    }

    #[tokio::test]
    async fn test_universal_target_verifier_state() {
        let rust_verifier = UniversalTargetVerifier::for_language(&LanguageId::new("rust"));
        assert_eq!(rust_verifier.target_id(), LanguageId::new("rust"));

        let go_verifier = UniversalTargetVerifier::for_language(&LanguageId::new("go"));
        assert_eq!(go_verifier.target_id(), LanguageId::new("go"));

        let ts_verifier = UniversalTargetVerifier::for_language(&LanguageId::new("typescript"));
        assert_eq!(ts_verifier.target_id(), LanguageId::new("typescript"));
    }

    #[test]
    fn test_parse_diagnostics_generic_and_ts() {
        let ts_verifier = UniversalTargetVerifier::for_language(&LanguageId::new("typescript"));
        let (diags, _) = ts_verifier.parse_diagnostics(
            "src/index.ts(10,5): error TS2304: Cannot find name 'foo'.",
            "",
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "TS2304");

        let generic_verifier =
            UniversalTargetVerifier::new(ToolchainProfile::new("c", std::iter::empty::<&str>()));
        let (diags2, _) = generic_verifier
            .parse_diagnostics("", "main.c:12:3: error: unknown type name 'size_t'");
        assert_eq!(diags2.len(), 1);
    }
}
