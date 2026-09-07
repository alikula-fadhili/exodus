//! Interrupted migration run state persistence when budget or process halts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationInterrupted {
    pub run_id: String,
    pub timestamp: String,
    pub reason: String,
    pub spent_usd: f64,
    pub budget_limit_usd: f64,
    pub completed_units: Vec<String>,
    pub unfinished_units: Vec<String>,
}

impl MigrationInterrupted {
    pub fn budget_exceeded(
        run_id: impl Into<String>,
        spent_usd: f64,
        budget_limit_usd: f64,
        completed_units: Vec<String>,
        unfinished_units: Vec<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reason: "budget_exceeded".to_string(),
            spent_usd,
            budget_limit_usd,
            completed_units,
            unfinished_units,
        }
    }

    pub fn save(&self, runs_dir: &Path) -> std::io::Result<PathBuf> {
        let run_path = runs_dir.join(&self.run_id);
        std::fs::create_dir_all(&run_path)?;
        let file_path = run_path.join("interrupted.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&file_path, json)?;
        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_save_interrupted_run() {
        let temp = tempdir().unwrap();
        let interrupted = MigrationInterrupted::budget_exceeded(
            "run-test-01",
            1.002,
            1.00,
            vec!["unit_a".into()],
            vec!["unit_b".into(), "unit_c".into()],
        );

        let path = interrupted.save(temp.path()).unwrap();
        assert!(path.exists());

        let raw = std::fs::read_to_string(&path).unwrap();
        let loaded: MigrationInterrupted = serde_json::from_str(&raw).unwrap();
        assert_eq!(loaded.reason, "budget_exceeded");
        assert_eq!(loaded.unfinished_units.len(), 2);
    }
}
