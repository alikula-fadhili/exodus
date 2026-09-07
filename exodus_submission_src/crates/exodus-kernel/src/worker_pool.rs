//! High-Throughput Parallel Subagent Worker Pool with Budget Guard Enforcement
//!
//! Spawns dedicated Rust async worker tasks for packages within monorepo dependency waves
//! without thread contention, enforcing budget limits and clean cancellation.

use crate::sdk::{CodeModeBundle, CodeModeRunner, ExodusSDK, SdkVerificationResult};
use crate::{KernelContext, KernelError};
use exodus_cost::{BudgetGuard, MigrationInterrupted};
use exodus_toolchain::PackageDescriptor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

/// Result of an individual subagent worker executing a package migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerMigrationOutcome {
    pub package_name: String,
    pub relative_path: PathBuf,
    pub success: bool,
    pub verification: Option<SdkVerificationResult>,
    pub error_message: Option<String>,
}

/// Dispatches high-throughput parallel subagents across package waves with budget protection.
pub struct SubagentWorkerPool {
    max_concurrency: usize,
    budget_guard: Arc<BudgetGuard>,
}

impl Default for SubagentWorkerPool {
    fn default() -> Self {
        Self::new(num_cpus(), Arc::new(BudgetGuard::unlimited()))
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

impl SubagentWorkerPool {
    pub fn new(max_concurrency: usize, budget_guard: Arc<BudgetGuard>) -> Self {
        Self {
            max_concurrency: max_concurrency.max(1),
            budget_guard,
        }
    }

    pub fn budget_guard(&self) -> &Arc<BudgetGuard> {
        &self.budget_guard
    }

    /// Executes an entire wave of packages in parallel using dedicated subagent workers.
    pub async fn execute_wave_parallel<F, Fut>(
        &self,
        ctx: &KernelContext,
        wave_index: usize,
        packages: &[PackageDescriptor],
        bundle_generator: F,
    ) -> Result<Vec<WorkerMigrationOutcome>, KernelError>
    where
        F: Fn(PackageDescriptor, ExodusSDK) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = Result<CodeModeBundle, KernelError>> + Send + 'static,
    {
        info!(
            "🚀 Spawning Wave {} parallel subagents across {} package(s) (concurrency limit: {}, budget limit: ${:.2})",
            wave_index,
            packages.len(),
            self.max_concurrency,
            self.budget_guard.limit_usd()
        );

        let cancel_token = CancellationToken::new();
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));

        for pkg in packages {
            let pkg_clone = pkg.clone();
            let ctx_clone = ctx.clone();
            let target_root = ctx.output_dir.join(&pkg.relative_path);
            let sdk = ExodusSDK::new(ctx_clone, target_root);
            let gen = bundle_generator.clone();
            let sem = semaphore.clone();
            let budget = self.budget_guard.clone();
            let token = cancel_token.clone();

            join_set.spawn(async move {
                tokio::select! {
                    _ = token.cancelled() => {
                        warn!("🛑 [Worker: {}] Cancelled before execution due to budget limit", pkg_clone.name);
                        Ok(WorkerMigrationOutcome {
                            package_name: pkg_clone.name,
                            relative_path: pkg_clone.relative_path,
                            success: false,
                            verification: None,
                            error_message: Some("Cancelled: budget limit reached".to_string()),
                        })
                    }
                    permit = sem.acquire() => {
                        let _permit = permit.map_err(|e| {
                            KernelError::WorkerPoolError(format!("Semaphore acquisition failed: {}", e))
                        })?;

                        if token.is_cancelled() {
                            return Ok(WorkerMigrationOutcome {
                                package_name: pkg_clone.name,
                                relative_path: pkg_clone.relative_path,
                                success: false,
                                verification: None,
                                error_message: Some("Cancelled: budget limit reached".to_string()),
                            });
                        }

                        let pkg_name = pkg_clone.name.clone();
                        let rel_path = pkg_clone.relative_path.clone();

                        info!("🤖 [Worker: {}] Commencing package processing...", pkg_name);

                        // Simulated turn token accounting (approx 3,500 in, 800 out)
                        if let Err(e) = budget.record_usage(3500, 800, 500, 2000) {
                            warn!("🚨 Budget breached during worker execution: {}", e);
                            token.cancel();
                            return Ok(WorkerMigrationOutcome {
                                package_name: pkg_name,
                                relative_path: rel_path,
                                success: false,
                                verification: None,
                                error_message: Some(format!("Budget limit reached: {}", e)),
                            });
                        }

                        let bundle_res = gen(pkg_clone, sdk.clone()).await;
                        match bundle_res {
                            Ok(bundle) => {
                                let exec_res = CodeModeRunner::execute_bundle(&sdk, &bundle).await;
                                match exec_res {
                                    Ok(verification) => {
                                        let success = verification.success;
                                        info!(
                                            "✅ [Worker: {}] Finished with status: success={}, passed_tests={}",
                                            pkg_name, success, verification.passed_tests
                                        );
                                        Ok(WorkerMigrationOutcome {
                                            package_name: pkg_name,
                                            relative_path: rel_path,
                                            success,
                                            verification: Some(verification),
                                            error_message: None,
                                        })
                                    }
                                    Err(err) => {
                                        error!("❌ [Worker: {}] Execution failed: {}", pkg_name, err);
                                        Ok(WorkerMigrationOutcome {
                                            package_name: pkg_name,
                                            relative_path: rel_path,
                                            success: false,
                                            verification: None,
                                            error_message: Some(err.to_string()),
                                        })
                                    }
                                }
                            }
                            Err(err) => {
                                error!("❌ [Worker: {}] Bundle generation failed: {}", pkg_name, err);
                                Ok(WorkerMigrationOutcome {
                                    package_name: pkg_name,
                                    relative_path: rel_path,
                                    success: false,
                                    verification: None,
                                    error_message: Some(err.to_string()),
                                })
                            }
                        }
                    }
                }
            });
        }

        let mut outcomes = Vec::new();
        let mut completed_units = Vec::new();
        let mut unfinished_units = Vec::new();

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(outcome)) => {
                    if outcome.success {
                        completed_units.push(outcome.package_name.clone());
                    } else {
                        unfinished_units.push(outcome.package_name.clone());
                    }
                    outcomes.push(outcome);
                }
                Ok(Err(err)) => return Err(err),
                Err(join_err) => {
                    return Err(KernelError::WorkerPoolError(format!(
                        "Worker task join error: {}",
                        join_err
                    )));
                }
            }
        }

        // If budget was tripped, record interrupted run state
        if self.budget_guard.is_tripped() {
            let runs_dir = ctx.output_dir.join("runs");
            let interrupted = MigrationInterrupted::budget_exceeded(
                &ctx.session_id,
                self.budget_guard.current_spend_usd(),
                self.budget_guard.limit_usd(),
                completed_units,
                unfinished_units,
            );
            let _ = interrupted.save(&runs_dir);
        }

        // Sort outcomes deterministically by package name
        outcomes.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_toolchain::DomainArchetype;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_parallel_worker_wave_execution() {
        let temp = tempdir().unwrap();
        let ctx = KernelContext::new(
            temp.path().to_path_buf(),
            temp.path().join("output"),
            "rust".to_string(),
        );
        let pool = SubagentWorkerPool::new(2, Arc::new(BudgetGuard::unlimited()));

        let pkgs = vec![
            PackageDescriptor {
                name: "pkg-a".to_string(),
                root_path: temp.path().join("pkg-a"),
                relative_path: PathBuf::from("pkg-a"),
                manifest_file: Some(temp.path().join("pkg-a/setup.py")),
                source_language: "python".to_string(),
                domain_archetype: DomainArchetype::SharedLibrary,
                dependencies: vec![],
                source_files: vec![],
                lines_of_code: 50,
            },
            PackageDescriptor {
                name: "pkg-b".to_string(),
                root_path: temp.path().join("pkg-b"),
                relative_path: PathBuf::from("pkg-b"),
                manifest_file: Some(temp.path().join("pkg-b/setup.py")),
                source_language: "python".to_string(),
                domain_archetype: DomainArchetype::BackendService,
                dependencies: vec!["pkg-a".to_string()],
                source_files: vec![],
                lines_of_code: 100,
            },
        ];

        let outcomes = pool
            .execute_wave_parallel(&ctx, 0, &pkgs, |pkg, _sdk| async move {
                Ok(CodeModeBundle {
                    package_name: pkg.name.clone(),
                    target_language: "rust".to_string(),
                    target_files: std::collections::HashMap::new(),
                    manifests: std::collections::HashMap::new(),
                    unit_tests: std::collections::HashMap::new(),
                    recorded_debts: vec![],
                    execution_notes: None,
                })
            })
            .await
            .unwrap();

        assert_eq!(outcomes.len(), 2);
        assert_eq!(outcomes[0].package_name, "pkg-a");
        assert_eq!(outcomes[1].package_name, "pkg-b");
    }

    #[tokio::test]
    async fn test_worker_pool_budget_cancellation() {
        let temp = tempdir().unwrap();
        let ctx = KernelContext::new(
            temp.path().to_path_buf(),
            temp.path().join("output"),
            "rust".to_string(),
        );
        // Tiny budget ($0.00001) to force immediate budget trip
        let pool = SubagentWorkerPool::new(2, Arc::new(BudgetGuard::new(0.00001)));

        let pkgs = vec![
            PackageDescriptor {
                name: "pkg-1".to_string(),
                root_path: temp.path().join("pkg-1"),
                relative_path: PathBuf::from("pkg-1"),
                manifest_file: Some(temp.path().join("pkg-1/setup.py")),
                source_language: "python".to_string(),
                domain_archetype: DomainArchetype::SharedLibrary,
                dependencies: vec![],
                source_files: vec![],
                lines_of_code: 50,
            },
            PackageDescriptor {
                name: "pkg-2".to_string(),
                root_path: temp.path().join("pkg-2"),
                relative_path: PathBuf::from("pkg-2"),
                manifest_file: Some(temp.path().join("pkg-2/setup.py")),
                source_language: "python".to_string(),
                domain_archetype: DomainArchetype::SharedLibrary,
                dependencies: vec![],
                source_files: vec![],
                lines_of_code: 50,
            },
        ];

        let outcomes = pool
            .execute_wave_parallel(&ctx, 0, &pkgs, |pkg, _sdk| async move {
                Ok(CodeModeBundle {
                    package_name: pkg.name.clone(),
                    target_language: "rust".to_string(),
                    target_files: std::collections::HashMap::new(),
                    manifests: std::collections::HashMap::new(),
                    unit_tests: std::collections::HashMap::new(),
                    recorded_debts: vec![],
                    execution_notes: None,
                })
            })
            .await
            .unwrap();

        assert!(pool.budget_guard().is_tripped());
        assert_eq!(outcomes.len(), 2);
    }
}
