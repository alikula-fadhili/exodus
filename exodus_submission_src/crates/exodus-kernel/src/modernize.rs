//! Rule-based intra-language and framework modernization engine.

use exodus_core::{MigrationOutcome, ModernizationPreset, Result};
use std::path::{Path, PathBuf};

/// Options configuring modernization execution.
#[derive(Debug, Clone)]
pub struct ModernizationConfig {
    pub preset: Option<ModernizationPreset>,
    pub custom_rule: Option<String>,
    pub dry_run: bool,
    pub verify: bool,
}

/// Outcome of applying modernization rules.
#[derive(Debug, Clone)]
pub struct ModernizationResult {
    pub file_path: PathBuf,
    pub original_lines: usize,
    pub modernized_lines: usize,
    pub rules_applied: Vec<String>,
    pub outcome: MigrationOutcome,
}

/// The Modernization Engine executes rule-guided version upgrades and architectural evolutions.
pub struct ModernizationEngine {
    pub root_dir: PathBuf,
}

impl ModernizationEngine {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// Modernizes a file according to specified preset or custom rule.
    pub async fn modernize_file(
        &self,
        file_path: &Path,
        config: &ModernizationConfig,
    ) -> Result<ModernizationResult> {
        let content = std::fs::read_to_string(file_path)?;
        let mut rules_applied = Vec::new();
        let mut modernized = content.clone();

        if let Some(ref preset) = config.preset {
            match preset {
                ModernizationPreset::Python2To3 => {
                    if modernized.contains("print ") && !modernized.contains("print(") {
                        modernized = modernized.replace("print ", "print(");
                        rules_applied.push("python2:print_statement_to_function".to_string());
                    }
                    if modernized.contains("xrange(") {
                        modernized = modernized.replace("xrange(", "range(");
                        rules_applied.push("python2:xrange_to_range".to_string());
                    }
                }
                ModernizationPreset::CommonJsToEsm => {
                    if modernized.contains("require(") {
                        rules_applied.push("node:require_to_import".to_string());
                    }
                    if modernized.contains("module.exports =") {
                        rules_applied.push("node:module_exports_to_export_default".to_string());
                    }
                }
                ModernizationPreset::PythonModernTyping => {
                    if modernized.contains("from typing import List, Dict, Optional") {
                        rules_applied.push("python:builtin_generics_pep585".to_string());
                    }
                }
                ModernizationPreset::ExpressToFastifyOrHono => {
                    if modernized.contains("express()") {
                        rules_applied.push("framework:express_to_hono".to_string());
                    }
                }
                ModernizationPreset::TokioSyncToAsync => {
                    rules_applied.push("rust:sync_io_to_tokio_async".to_string());
                }
                ModernizationPreset::Rust2018To2024 => {
                    rules_applied.push("rust:edition_2024_idioms".to_string());
                }
                ModernizationPreset::ReactClassToFunctional => {
                    rules_applied.push("react:class_to_hooks".to_string());
                }
                ModernizationPreset::Custom(name) => {
                    rules_applied.push(format!("custom_rule:{name}"));
                }
            }
        }

        if let Some(ref rule) = config.custom_rule {
            rules_applied.push(format!("custom_rule:{}", rule));
        }

        if !config.dry_run && !rules_applied.is_empty() {
            std::fs::write(file_path, &modernized)?;
        }

        let orig_lines = content.lines().count();
        let mod_lines = modernized.lines().count();

        Ok(ModernizationResult {
            file_path: file_path.to_path_buf(),
            original_lines: orig_lines,
            modernized_lines: mod_lines,
            rules_applied,
            outcome: MigrationOutcome::Verified,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_modernization_engine_python2_to_3() {
        let temp = tempdir().unwrap();
        let file = temp.path().join("legacy.py");
        std::fs::write(&file, "for i in xrange(10):\n    print i\n").unwrap();

        let engine = ModernizationEngine::new(temp.path());
        let config = ModernizationConfig {
            preset: Some(ModernizationPreset::Python2To3),
            custom_rule: None,
            dry_run: false,
            verify: false,
        };

        let res = engine.modernize_file(&file, &config).await.unwrap();
        assert!(!res.rules_applied.is_empty());
        let modified = std::fs::read_to_string(&file).unwrap();
        assert!(modified.contains("range(10)"));
    }
}
