//! Migration debt records accrued during expenditure breaches and fallback halts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebtReason {
    BudgetExceeded { spent_usd: f64, limit_usd: f64 },
    CompilationFailure { error_log: String },
    DependencyBlocked { missing_dep: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationDebtEntry {
    pub node_id: String,
    pub package_name: String,
    pub wave_index: usize,
    pub reason: DebtReason,
    pub target_files: Vec<PathBuf>,
    pub timestamp: String,
}

impl MigrationDebtEntry {
    pub fn budget_exceeded(
        node_id: String,
        package_name: String,
        wave_index: usize,
        spent_usd: f64,
        limit_usd: f64,
    ) -> Self {
        Self {
            node_id,
            package_name,
            wave_index,
            reason: DebtReason::BudgetExceeded {
                spent_usd,
                limit_usd,
            },
            target_files: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Flushes recorded migration debt entries to `.exodus/debt.json`.
pub fn flush_debt_ledger(debt: &[MigrationDebtEntry], output_path: &Path) -> std::io::Result<()> {
    if debt.is_empty() {
        return Ok(());
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(debt)?;
    std::fs::write(output_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_debt_ledger_flushing() {
        let temp = tempdir().unwrap();
        let debt_file = temp.path().join(".exodus/debt.json");

        let entries = vec![MigrationDebtEntry::budget_exceeded(
            "function::calc::compute".to_string(),
            "core-pkg".to_string(),
            1,
            5.02,
            5.00,
        )];

        flush_debt_ledger(&entries, &debt_file).unwrap();
        assert!(debt_file.exists());

        let content = std::fs::read_to_string(&debt_file).unwrap();
        let loaded: Vec<MigrationDebtEntry> = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].node_id, "function::calc::compute");
    }
}
