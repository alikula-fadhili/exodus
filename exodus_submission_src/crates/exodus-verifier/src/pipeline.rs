//! Orchestrates a full gated migration run: parse → build the ESG → compute dependency-ordered
//! verification boundaries → run the per-unit gate for each → accumulate verified units into a
//! growing target crate inside an Exodus-owned worktree → commit atomically on each success →
//! generate an approval-ready merge proposal. Never mutates the caller's primary working tree.

use crate::unit_gate::{load_fixture_contract, run_unit_gate, unit_id_for};
use crate::{UnitGateContext, UnitVerificationResult, Verifier};
use exodus_agent::AgentBounds;
use exodus_case::CaseEngine;
use exodus_core::{MigrationOutcome, Result};
use exodus_graph::SemanticGraph;
use exodus_parser::{PythonParser, SourceParser};
use exodus_transform::TransformResult;
use exodus_worktree::{MergeProposal, WorktreeManager};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Verification-hierarchy summary of one gated migration run: unit-level results stay distinct
/// from the module-integration result computed by (re-)compiling the accumulated target crate —
/// unit success is never presented as repository success (master prompt §11).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatedMigrationSummary {
    pub run_id: String,
    pub task_id: String,
    pub branch_name: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub unit_results: Vec<UnitVerificationResult>,
    pub verified_unit_count: usize,
    pub compatible_unit_count: usize,
    pub degraded_unit_count: usize,
    pub blocked_unit_count: usize,
    pub grounded_contract_count: usize,
    /// Whether the accumulated crate — all units that individually passed their own gate — still
    /// compiles as a whole. Distinct from, and computed after, unit-level results.
    pub module_integration_passed: bool,
    pub commits: Vec<String>,
    pub merge_proposal: Option<MergeProposal>,
}

/// Runs the full gated migration pipeline for `source`, inside an Exodus-owned worktree of
/// `repo_root`. `fixture_contracts_root` is where `contracts.json` is looked up for grounding (see
/// `unit_gate::load_fixture_contract`) — pass `source` itself when the fixture and contract file
/// live in the same directory, as this pass's grounded fixtures do.
pub async fn run_gated_migration(
    source: &Path,
    repo_root: &Path,
    fixture_contracts_root: Option<&Path>,
    task_id: &str,
    run_id: &str,
) -> Result<GatedMigrationSummary> {
    let parser = PythonParser::new();
    let repo = parser.parse_repository(source)?;
    let graph = SemanticGraph::from_parsed_repository(&repo);
    let boundaries = graph.verification_units();

    let contracts_dir = repo_root.join(".exodus").join("contracts");
    let knowledge_dir = repo_root.join(".exodus").join("knowledge");
    let case_engine = CaseEngine::new(&knowledge_dir);

    let worktree_manager = WorktreeManager::new(repo_root);
    let mut lease = worktree_manager.create_lease(task_id, run_id)?;
    let worktree_ok = lease.status == exodus_worktree::WorktreeStatus::Active;

    let target_crate_dir = if worktree_ok {
        lease.worktree_path.join("migrated_target")
    } else {
        // Real worktree creation failed (e.g. no git repo in this environment) — fall back to a
        // clearly-scoped scratch directory so the gate can still run and report honestly, rather
        // than silently pretending isolation succeeded.
        repo_root.join(".exodus").join("work").join(run_id)
    };
    let unit_scratch_dir = target_crate_dir.join("_unit_scratch");

    let mut unit_results = Vec::new();
    let mut accumulated_modules: Vec<TransformResult> = Vec::new();
    // Module-integration re-compiles use only the accumulated *source* — behavioral tests were
    // already run per-unit by the gate above, so no test harness needs to accumulate here too.
    let accumulated_harness = String::new();
    let mut commits = Vec::new();
    let mut grounded_contract_count = 0usize;

    for boundary in &boundaries {
        let unit_id = unit_id_for(boundary);
        let contract =
            fixture_contracts_root.and_then(|root| load_fixture_contract(root, &unit_id));
        if contract.is_some() {
            grounded_contract_count += 1;
        }

        let ctx = UnitGateContext {
            run_id,
            repo: &repo,
            graph: &graph,
            contracts_dir: contracts_dir.clone(),
            work_dir: unit_scratch_dir.clone(),
            case_engine: &case_engine,
            contract,
            repair_bounds: AgentBounds::default(),
        };

        let (result, transform_result) = run_unit_gate(boundary, &ctx).await?;
        let verified = matches!(
            result.outcome,
            MigrationOutcome::Verified | MigrationOutcome::Compatible
        );
        unit_results.push(result);

        if verified {
            // Re-scaffold the *accumulated* set of every previously and newly verified unit into
            // the growing target crate and confirm it still compiles as a whole — this is the
            // module-integration check, run incrementally as each unit is added, kept distinct
            // from the unit's own already-passed isolated gate result.
            accumulated_modules.push(transform_result);
            let integration_verifier = Verifier::new(&target_crate_dir);
            integration_verifier.scaffold_target_crate(
                "exodus_migrated_target",
                &accumulated_modules,
                if accumulated_harness.is_empty() {
                    None
                } else {
                    Some(accumulated_harness.as_str())
                },
            )?;
            let _ = integration_verifier.run_formatter();
            let (integration_compiled, _diags) = integration_verifier.run_cargo_check()?;

            if integration_compiled && worktree_ok {
                let message = format!(
                    "Verified unit: {unit_id}\n\nOutcome: {:?}\nRun: {run_id}",
                    unit_results.last().unwrap().outcome
                );
                match worktree_manager.commit_all(&mut lease, &message) {
                    Ok(Some(hash)) => commits.push(hash),
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("failed to commit verified unit `{unit_id}`: {e}");
                    }
                }
            } else if !integration_compiled {
                // The unit passed in isolation but breaks the accumulated crate as a whole (a
                // real cross-unit interaction, e.g. a duplicate symbol) — drop it from the
                // accumulated set so the target crate stays in its last known-good state, and
                // downgrade the just-recorded unit result to reflect that module integration
                // failed even though its own unit gate passed.
                accumulated_modules.pop();
                if let Some(last) = unit_results.last_mut() {
                    last.outcome = MigrationOutcome::Blocked;
                }
            }
        }
    }

    let module_integration_passed = if accumulated_modules.is_empty() {
        false
    } else {
        let integration_verifier = Verifier::new(&target_crate_dir);
        let (compiled, _) = integration_verifier
            .run_cargo_check()
            .unwrap_or((false, vec![]));
        compiled
    };

    let verified_unit_count = unit_results
        .iter()
        .filter(|r| r.outcome == MigrationOutcome::Verified)
        .count();
    let compatible_unit_count = unit_results
        .iter()
        .filter(|r| r.outcome == MigrationOutcome::Compatible)
        .count();
    let degraded_unit_count = unit_results
        .iter()
        .filter(|r| r.outcome == MigrationOutcome::Degraded)
        .count();
    let blocked_unit_count = unit_results
        .iter()
        .filter(|r| r.outcome == MigrationOutcome::Blocked)
        .count();

    let merge_proposal = if worktree_ok {
        let diff_summary = format!(
            "{} unit(s) verified/compatible, {} degraded, {} blocked across {} commit(s)",
            verified_unit_count + compatible_unit_count,
            degraded_unit_count,
            blocked_unit_count,
            commits.len()
        );
        Some(worktree_manager.generate_merge_proposal(
            &lease,
            verified_unit_count + compatible_unit_count,
            degraded_unit_count,
            &diff_summary,
        )?)
    } else {
        None
    };

    Ok(GatedMigrationSummary {
        run_id: run_id.to_string(),
        task_id: task_id.to_string(),
        branch_name: worktree_ok.then(|| lease.branch_name.clone()),
        worktree_path: worktree_ok.then(|| lease.worktree_path.clone()),
        unit_results,
        verified_unit_count,
        compatible_unit_count,
        degraded_unit_count,
        blocked_unit_count,
        grounded_contract_count,
        module_integration_passed,
        commits,
        merge_proposal,
    })
}
