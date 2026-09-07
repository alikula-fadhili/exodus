//! Git worktree isolation, leasing, and merge proposal engine for Project Exodus.

use chrono::{DateTime, Utc};
use exodus_core::{ExodusError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

/// State of a worktree lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeStatus {
    Active,
    Recoverable,
    DirtyReviewRequired,
    CleanRemovable,
    Missing,
    OwnershipMismatched,
    /// `git worktree add` itself failed (spawn error or non-zero exit) — the lease record exists
    /// but there is no real isolated worktree behind it. Distinct from `Active` so callers can
    /// tell a genuine isolated worktree apart from this failure, instead of silently proceeding as
    /// if generation/compilation/verification were actually isolated.
    CreationFailed,
}

/// A persisted worktree lease for an isolated migration task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeLease {
    pub schema_version: String,
    pub lease_id: String,
    pub repository_id: String,
    pub task_id: String,
    pub run_id: String,
    pub worktree_path: PathBuf,
    pub branch_name: String,
    pub base_commit: String,
    pub current_commit: Option<String>,
    pub created_at: DateTime<Utc>,
    pub locked: bool,
    pub status: WorktreeStatus,
}

/// Merge proposal generated upon successful verification of an isolated attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeProposal {
    pub schema_version: String,
    pub proposal_id: String,
    pub task_id: String,
    pub run_id: String,
    pub winning_branch: String,
    pub base_commit: String,
    pub final_commit: String,
    pub verified_contracts_count: usize,
    pub migration_debt_count: usize,
    pub diff_summary: String,
    pub reproduction_command: String,
    pub rollback_instructions: String,
    pub created_at: DateTime<Utc>,
    pub approved: bool,
}

/// Git worktree manager ensuring safe, non-destructive isolated migration environments.
pub struct WorktreeManager {
    repo_root: PathBuf,
    worktree_root: PathBuf,
}

impl WorktreeManager {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        let repo_path = repo_root.into();
        let parent = repo_path.parent().unwrap_or(&repo_path);
        let worktree_root = parent.join(".exodus-worktrees");

        Self {
            repo_root: repo_path,
            worktree_root,
        }
    }

    /// Sets custom worktree root directory.
    pub fn with_custom_root(mut self, custom_root: impl Into<PathBuf>) -> Self {
        self.worktree_root = custom_root.into();
        self
    }

    /// Preflight check: queries git status and HEAD commit safely.
    pub fn preflight_check(&self) -> Result<(String, bool)> {
        let head_out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(ExodusError::from)?;

        let head_commit = String::from_utf8_lossy(&head_out.stdout).trim().to_string();

        let status_out = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(ExodusError::from)?;

        let is_dirty = !status_out.stdout.is_empty();
        Ok((head_commit, is_dirty))
    }

    /// Creates an isolated linked Git worktree for a migration task.
    pub fn create_lease(&self, task_id: &str, run_id: &str) -> Result<WorktreeLease> {
        let (base_commit, _) = self.preflight_check()?;
        let lease_id = format!("lease-{}", Uuid::new_v4());
        let repo_name = self
            .repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo");

        let task_worktree_dir = self.worktree_root.join(repo_name).join(task_id);
        fs::create_dir_all(&task_worktree_dir).map_err(ExodusError::from)?;

        let branch_name = format!("exodus/{task_id}/migration");

        // Execute: git worktree add --lock --reason ... <path> -b <branch>
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "--lock",
                "--reason",
                "Project Exodus isolated migration run",
                "-b",
                &branch_name,
                task_worktree_dir.to_str().unwrap(),
            ])
            .current_dir(&self.repo_root)
            .output();

        // A lease is only genuinely `Active` (a real isolated worktree exists) if the command both
        // spawned AND exited successfully — the previous version only checked for a spawn error,
        // silently treating a non-zero exit (e.g. the branch already existing) as success.
        let status = match &output {
            Ok(out) if out.status.success() => WorktreeStatus::Active,
            Ok(out) => {
                tracing::warn!(
                    "git worktree add failed (exit {:?}): {}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr)
                );
                WorktreeStatus::CreationFailed
            }
            Err(e) => {
                tracing::warn!("git worktree add could not be spawned: {e}");
                WorktreeStatus::CreationFailed
            }
        };

        let lease = WorktreeLease {
            schema_version: "1.0.0".to_string(),
            lease_id,
            repository_id: repo_name.to_string(),
            task_id: task_id.to_string(),
            run_id: run_id.to_string(),
            worktree_path: task_worktree_dir,
            branch_name,
            base_commit,
            current_commit: None,
            created_at: Utc::now(),
            locked: true,
            status,
        };

        // Persist lease under .exodus/runs/<run_id>/worktree.json
        let runs_dir = self.repo_root.join(".exodus").join("runs").join(run_id);
        fs::create_dir_all(&runs_dir).map_err(ExodusError::from)?;
        fs::write(
            runs_dir.join("worktree.json"),
            serde_json::to_string_pretty(&lease)?,
        )
        .map_err(ExodusError::from)?;

        Ok(lease)
    }

    /// Generates an approval-ready merge proposal.
    pub fn generate_merge_proposal(
        &self,
        lease: &WorktreeLease,
        verified_contracts: usize,
        debts: usize,
        diff_summary: &str,
    ) -> Result<MergeProposal> {
        let proposal = MergeProposal {
            schema_version: "1.0.0".to_string(),
            proposal_id: format!("proposal-{}", Uuid::new_v4()),
            task_id: lease.task_id.clone(),
            run_id: lease.run_id.clone(),
            winning_branch: lease.branch_name.clone(),
            base_commit: lease.base_commit.clone(),
            final_commit: lease
                .current_commit
                .clone()
                .unwrap_or_else(|| lease.base_commit.clone()),
            verified_contracts_count: verified_contracts,
            migration_debt_count: debts,
            diff_summary: diff_summary.to_string(),
            reproduction_command: format!("exodus verify {}", lease.worktree_path.display()),
            rollback_instructions: format!(
                "git worktree remove --force {}",
                lease.worktree_path.display()
            ),
            created_at: Utc::now(),
            approved: false,
        };

        let runs_dir = self
            .repo_root
            .join(".exodus")
            .join("runs")
            .join(&lease.run_id);
        fs::create_dir_all(&runs_dir).map_err(ExodusError::from)?;
        fs::write(
            runs_dir.join("merge-proposal.json"),
            serde_json::to_string_pretty(&proposal)?,
        )
        .map_err(ExodusError::from)?;

        Ok(proposal)
    }

    /// Lists active and recoverable worktrees.
    pub fn list_leases(&self) -> Result<Vec<WorktreeLease>> {
        let runs_root = self.repo_root.join(".exodus").join("runs");
        let mut leases = Vec::new();

        if runs_root.exists() {
            if let Ok(entries) = fs::read_dir(runs_root) {
                for entry in entries.flatten() {
                    let manifest = entry.path().join("worktree.json");
                    if manifest.exists() {
                        if let Ok(content) = fs::read_to_string(manifest) {
                            if let Ok(lease) = serde_json::from_str::<WorktreeLease>(&content) {
                                leases.push(lease);
                            }
                        }
                    }
                }
            }
        }

        Ok(leases)
    }

    /// Checks whether a worktree has uncommitted changes.
    fn is_worktree_dirty(&self, worktree_path: &PathBuf) -> bool {
        Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree_path)
            .output()
            .map(|out| !out.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Safely cleans up a lease without data loss. Refuses to delete a dirty worktree unless
    /// `force` is set — matching the master prompt's rule that `git worktree remove --force` is
    /// never used during *normal* cleanup, and that dirty/failed worktrees are preserved as
    /// review-required by default. The CLI's `worktree cleanup` maps to `force: false`; `worktree
    /// discard` (which separately requires the caller to repeat the task ID as `--confirm`) maps
    /// to `force: true`.
    pub fn cleanup_lease(&self, task_id: &str, force: bool) -> Result<bool> {
        let leases = self.list_leases()?;
        if let Some(target) = leases.iter().find(|l| l.task_id == task_id) {
            if !force
                && target.worktree_path.exists()
                && self.is_worktree_dirty(&target.worktree_path)
            {
                let mut dirty = target.clone();
                dirty.status = WorktreeStatus::DirtyReviewRequired;
                self.persist_lease(&dirty)?;
                tracing::warn!(
                    "worktree for task `{task_id}` has uncommitted changes — preserved for review, not removed. Use `worktree discard --confirm {task_id}` to force."
                );
                return Ok(false);
            }

            let mut remove_args = vec!["worktree", "remove"];
            if force {
                remove_args.push("--force");
            }
            let worktree_path_str = target.worktree_path.to_str().unwrap();
            remove_args.push(worktree_path_str);
            let _ = Command::new("git")
                .args(&remove_args)
                .current_dir(&self.repo_root)
                .output();

            if target.worktree_path.exists() {
                let _ = fs::remove_dir_all(&target.worktree_path);
            }
            return Ok(true);
        }
        Ok(false)
    }

    fn persist_lease(&self, lease: &WorktreeLease) -> Result<()> {
        let runs_dir = self
            .repo_root
            .join(".exodus")
            .join("runs")
            .join(&lease.run_id);
        fs::create_dir_all(&runs_dir).map_err(ExodusError::from)?;
        fs::write(
            runs_dir.join("worktree.json"),
            serde_json::to_string_pretty(lease)?,
        )
        .map_err(ExodusError::from)?;
        Ok(())
    }

    /// Stages and commits all current changes inside a leased worktree, on its own Exodus branch,
    /// using a per-command identity rather than modifying global Git config. Returns `None` (not
    /// an error) when there is nothing to commit — a no-op unit gate result is not a failure.
    /// Refuses to commit if the staged diff appears to contain an obvious secret pattern.
    pub fn commit_all(&self, lease: &mut WorktreeLease, message: &str) -> Result<Option<String>> {
        let add_out = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&lease.worktree_path)
            .output()
            .map_err(ExodusError::from)?;
        if !add_out.status.success() {
            return Err(ExodusError::Generic(format!(
                "git add failed in worktree {}: {}",
                lease.worktree_path.display(),
                String::from_utf8_lossy(&add_out.stderr)
            )));
        }

        let diff_out = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&lease.worktree_path)
            .output()
            .map_err(ExodusError::from)?;
        let staged_diff = String::from_utf8_lossy(&diff_out.stdout);
        if staged_diff.trim().is_empty() {
            return Ok(None);
        }
        for marker in [
            "BEGIN PRIVATE KEY",
            "BEGIN RSA PRIVATE KEY",
            "ANTHROPIC_API_KEY=",
            "OPENAI_API_KEY=",
            "AKIA",
        ] {
            if staged_diff.contains(marker) {
                return Err(ExodusError::Generic(format!(
                    "refusing to commit: staged diff contains a likely secret marker `{marker}`"
                )));
            }
        }

        let commit_out = Command::new("git")
            .args([
                "-c",
                "user.name=Project Exodus",
                "-c",
                "user.email=exodus@localhost",
                "commit",
                "-m",
                message,
            ])
            .current_dir(&lease.worktree_path)
            .output()
            .map_err(ExodusError::from)?;
        if !commit_out.status.success() {
            return Err(ExodusError::Generic(format!(
                "git commit failed in worktree {}: {}",
                lease.worktree_path.display(),
                String::from_utf8_lossy(&commit_out.stderr)
            )));
        }

        let rev_out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&lease.worktree_path)
            .output()
            .map_err(ExodusError::from)?;
        let commit_hash = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();
        lease.current_commit = Some(commit_hash.clone());
        self.persist_lease(lease)?;
        Ok(Some(commit_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Initializes a real git repository with one commit — `create_lease` genuinely runs
    /// `git worktree add`, so tests need a real repo behind it, not just a bare directory (a bare
    /// directory is exactly the `CreationFailed` case covered separately below).
    fn init_real_repo(dir: &PathBuf) {
        fs::create_dir_all(dir).unwrap();
        let run = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        };
        run(&["init", "-q"]);
        run(&[
            "-c",
            "user.name=T",
            "-c",
            "user.email=t@t.com",
            "commit",
            "--allow-empty",
            "-q",
            "-m",
            "init",
        ]);
    }

    #[test]
    fn test_worktree_manager_lease_and_proposal() {
        let temp_dir = std::env::temp_dir().join(format!("exodus_wt_test_{}", Uuid::new_v4()));
        init_real_repo(&temp_dir);

        let manager = WorktreeManager::new(&temp_dir).with_custom_root(temp_dir.join("wts"));
        let lease = manager.create_lease("task-100", "run-100").unwrap();

        assert_eq!(lease.task_id, "task-100");
        assert_eq!(
            lease.status,
            WorktreeStatus::Active,
            "a real git repo must produce a genuinely Active lease"
        );
        assert!(lease.worktree_path.exists());

        let proposal = manager
            .generate_merge_proposal(&lease, 5, 1, "+ 100 lines Rust")
            .unwrap();

        assert_eq!(proposal.task_id, "task-100");
        assert_eq!(proposal.verified_contracts_count, 5);
        assert!(!proposal.approved);

        let leases = manager.list_leases().unwrap();
        assert!(!leases.is_empty());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_create_lease_reports_failure_honestly_when_not_a_git_repo() {
        let temp_dir =
            std::env::temp_dir().join(format!("exodus_wt_test_nogit_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).unwrap();

        let manager = WorktreeManager::new(&temp_dir).with_custom_root(temp_dir.join("wts"));
        let lease = manager.create_lease("task-nogit", "run-nogit").unwrap();

        assert_eq!(
            lease.status,
            WorktreeStatus::CreationFailed,
            "a failed `git worktree add` must never be silently reported as Active"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_commit_all_creates_real_commit_and_updates_lease() {
        let temp_dir =
            std::env::temp_dir().join(format!("exodus_wt_test_commit_{}", Uuid::new_v4()));
        init_real_repo(&temp_dir);

        let manager = WorktreeManager::new(&temp_dir).with_custom_root(temp_dir.join("wts"));
        let mut lease = manager.create_lease("task-commit", "run-commit").unwrap();
        assert_eq!(lease.status, WorktreeStatus::Active);

        fs::write(lease.worktree_path.join("unit_a.rs"), "pub fn a() {}\n").unwrap();
        let commit_hash = manager
            .commit_all(&mut lease, "verified unit: function::a")
            .unwrap();
        assert!(commit_hash.is_some());
        assert_eq!(lease.current_commit, commit_hash);

        // The commit is real and inspectable via plain git log on the branch.
        let log_out = Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(&lease.worktree_path)
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&log_out.stdout).contains("verified unit"));

        // A second commit_all with no new changes is a no-op, not an error.
        let second = manager.commit_all(&mut lease, "nothing changed").unwrap();
        assert!(second.is_none());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cleanup_preserves_dirty_worktree_unless_forced() {
        let temp_dir =
            std::env::temp_dir().join(format!("exodus_wt_test_dirty_{}", Uuid::new_v4()));
        init_real_repo(&temp_dir);

        let manager = WorktreeManager::new(&temp_dir).with_custom_root(temp_dir.join("wts"));
        let lease = manager.create_lease("task-dirty", "run-dirty").unwrap();
        fs::write(lease.worktree_path.join("uncommitted.rs"), "// dirty\n").unwrap();

        let removed = manager.cleanup_lease("task-dirty", false).unwrap();
        assert!(
            !removed,
            "a dirty worktree must not be removed by a non-forced cleanup"
        );
        assert!(
            lease.worktree_path.exists(),
            "the dirty worktree's files must still be on disk after a refused cleanup"
        );

        let leases = manager.list_leases().unwrap();
        let persisted = leases.iter().find(|l| l.task_id == "task-dirty").unwrap();
        assert_eq!(persisted.status, WorktreeStatus::DirtyReviewRequired);

        let force_removed = manager.cleanup_lease("task-dirty", true).unwrap();
        assert!(
            force_removed,
            "an explicitly forced cleanup must remove even a dirty worktree"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
