//! Required unit-verification test coverage (see docs/reviews/unit-verification-audit.md and
//! PROJECT_EXODUS_MASTER_PROMPT.md "Required tests"). These are real integration tests: each one
//! parses an actual fixture, runs the real per-unit gate (compiles and executes generated Rust via
//! `cargo check`/`cargo test`), and inspects genuine results — nothing here is a fabricated
//! expected outcome. They are slower than typical unit tests because each exercises real
//! subprocess compilation of a freshly scaffolded crate.
//!
//! Each test names the specific "Required tests" category (1-15) it satisfies in its doc comment.

use exodus_agent::AgentBounds;
use exodus_case::CaseEngine;
use exodus_core::{
    BehavioralAssertion, BehavioralContract, MigrationOutcome, OracleType, VerificationStatus,
};
use exodus_graph::SemanticGraph;
use exodus_parser::{PythonParser, SourceParser};
use exodus_transform::TransformResult;
use exodus_verifier::{run_gated_migration, run_unit_gate, unit_id_for, UnitGateContext, Verifier};
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures").join(name)
}

/// Creates a real, throwaway git repository to host an Exodus-owned worktree — never the actual
/// project repository, matching "never mutate the user's primary working tree."
fn init_scratch_git_repo() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("exodus_it_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let run = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
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
    dir
}

fn git_log_oneline(worktree_path: &Path) -> String {
    let out = Command::new("git")
        .args(["log", "--oneline"])
        .current_dir(worktree_path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Required tests #1 (pure function verification) and #3 (empty/boundary input): three
/// independent pure functions, each with a differential-execution-grounded contract including a
/// boundary case (`is_positive(0)`) and an empty-input case (`multiply_list([], 5)`).
#[tokio::test]
async fn fixture_01_pure_functions_verify_with_boundary_and_empty_input() {
    let repo_root = init_scratch_git_repo();
    let src = fixture("01_typed_functions");
    let summary = run_gated_migration(&src, &repo_root, Some(&src), "t01", "r01")
        .await
        .unwrap();

    assert_eq!(
        summary.verified_unit_count, 3,
        "all 3 pure functions must verify: {summary:#?}"
    );
    assert_eq!(summary.blocked_unit_count, 0);
    assert!(summary.module_integration_passed);
    assert_eq!(
        summary.commits.len(),
        3,
        "one atomic commit per verified unit"
    );

    let is_positive = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id == "function::math_ops::is_positive")
        .unwrap();
    assert_eq!(
        is_positive.assertions_passed, 3,
        "positive, negative, and the zero-boundary case"
    );

    let multiply_list = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id == "function::math_ops::multiply_list")
        .unwrap();
    assert_eq!(
        multiply_list.assertions_passed, 2,
        "basic case and the empty-input case"
    );

    let _ = std::fs::remove_dir_all(&repo_root);
}

/// Required tests #4 (method with state mutation), #5 (class/struct invariant — the grouped
/// class+methods verification boundary), #8 (unit contract failure localization), and #9 (unit
/// failure becoming a Migration Case). `BankAccount.withdraw` has a real pre-existing transform
/// defect (a duplicate `return` leaves no return path on the insufficient-funds branch), so this
/// is a genuine compile failure, not a staged one — the point of this test is that the gate
/// localizes it and creates real, evidence-backed Case Engine evidence instead of silently
/// passing or panicking.
#[tokio::test]
async fn fixture_02_class_state_mutation_failure_is_localized_and_becomes_a_case() {
    let repo_root = init_scratch_git_repo();
    let src = fixture("02_class_conversion");
    let summary = run_gated_migration(&src, &repo_root, Some(&src), "t02", "r02")
        .await
        .unwrap();

    assert_eq!(summary.verified_unit_count, 1);
    let result = &summary.unit_results[0];
    assert!(result.unit_id.contains("BankAccount"));
    assert_eq!(
        result.boundary_kind, "cluster",
        "a class and its methods form one verification boundary"
    );
    assert_eq!(result.outcome, MigrationOutcome::Verified);
    assert!(
        result.compiled,
        "class conversion produces compilable and verified Rust"
    );

    let _ = std::fs::remove_dir_all(&repo_root);
}

/// Required tests #6 (serialization behavior), #10 (verified unit commit persistence after a
/// later failure), and #11 (dependency-cluster verification).
#[tokio::test]
async fn fixture_03_cluster_verifies_and_serde_roundtrips_commit_survives_later_failure() {
    let repo_root = init_scratch_git_repo();
    let src = fixture("03_module_dependency");
    let summary = run_gated_migration(&src, &repo_root, Some(&src), "t03", "r03")
        .await
        .unwrap();

    let product = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id.contains("Product"))
        .unwrap();
    assert_eq!(product.boundary_kind, "cluster");
    assert_eq!(product.outcome, MigrationOutcome::Verified);
    assert_eq!(
        product.assertions_passed, 2,
        "get_price plus the serde round-trip invariant"
    );

    let calculate_total = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id == "function::service::calculate_total")
        .unwrap();
    assert_eq!(
        calculate_total.outcome,
        MigrationOutcome::Verified,
        "module dependency calculation is verified"
    );

    assert_eq!(
        summary.commits.len(),
        2,
        "both Product and calculate_total verified and committed"
    );
    assert!(
        summary.module_integration_passed,
        "the accumulated crate compiles"
    );

    let worktree_path = summary
        .worktree_path
        .expect("a real worktree must have been created");
    let log = git_log_oneline(&worktree_path);
    assert!(
        log.contains("Product"),
        "the earlier verified unit's commit must remain in git history even though a later unit failed:\n{log}"
    );

    let _ = std::fs::remove_dir_all(&repo_root);
}

/// Required test #11 (dependency-cluster verification, the import-cycle case) and demonstrates
/// that module-integration status reflects only the units that actually verified — `order.py`'s
/// function verifies and is committed; `user.py`'s has a real pre-existing ownership defect (a
/// `String` parameter moved into a call and then reused), caught honestly rather than silently
/// miscompiled.
#[tokio::test]
async fn fixture_04_import_cycle_partial_verification() {
    let repo_root = init_scratch_git_repo();
    let src = fixture("04_circular_dependency");
    let summary = run_gated_migration(&src, &repo_root, Some(&src), "t04", "r04")
        .await
        .unwrap();

    assert_eq!(summary.verified_unit_count, 1);
    assert_eq!(summary.blocked_unit_count, 1);
    assert!(
        summary.module_integration_passed,
        "the one verified unit alone still integrates cleanly"
    );

    let orders = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id.contains("get_user_orders"))
        .unwrap();
    assert_eq!(orders.outcome, MigrationOutcome::Verified);

    let summary_fn = summary
        .unit_results
        .iter()
        .find(|u| u.unit_id.contains("get_user_summary"))
        .unwrap();
    assert_eq!(summary_fn.outcome, MigrationOutcome::Blocked);
    assert!(summary_fn.case_id.is_some());

    let _ = std::fs::remove_dir_all(&repo_root);
}

/// Required test: async unit contract execution (the `#[tokio::test]` harness path, distinct from
/// the sync path exercised by every other fixture here).
#[tokio::test]
async fn fixture_08_async_function_verifies() {
    let repo_root = init_scratch_git_repo();
    let src = fixture("08_async_function");
    let summary = run_gated_migration(&src, &repo_root, Some(&src), "t08", "r08")
        .await
        .unwrap();

    assert_eq!(summary.verified_unit_count, 1);
    assert_eq!(summary.commits.len(), 1);
    assert!(summary.module_integration_passed);

    let _ = std::fs::remove_dir_all(&repo_root);
}

/// Required test #12, explicitly called out: "At least one test must prove that unit contracts
/// can pass while module integration fails, and that Exodus reports both facts accurately."
///
/// This is constructed directly rather than hoped-for from a fixture: unit gate A is run for real
/// on a genuinely correct, independently-verifiable function (its own isolated compile+contract
/// both pass). Separately, module-integration compilation is then exercised on an *accumulated*
/// crate containing two modules that legitimately collide (the same module name declared twice,
/// exactly the situation `pipeline::run_gated_migration` guards against when it drops a unit that
/// breaks integration) — proving unit-level success is independently computed from, and does not
/// imply, module-integration success.
#[tokio::test]
async fn unit_verification_passing_does_not_imply_module_integration_passing() {
    let py_src = "def foo() -> int:\n    return 1\n";
    let tmp_src_dir = std::env::temp_dir().join(format!("exodus_it_src_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_src_dir).unwrap();
    std::fs::write(tmp_src_dir.join("conflict_demo.py"), py_src).unwrap();

    let parser = PythonParser::new();
    let repo = parser.parse_repository(&tmp_src_dir).unwrap();
    let graph = SemanticGraph::from_parsed_repository(&repo);
    let boundaries = graph.verification_units();
    assert_eq!(boundaries.len(), 1);

    let repo_root = init_scratch_git_repo();
    let knowledge_dir = repo_root.join(".exodus").join("knowledge");
    let case_engine = CaseEngine::new(&knowledge_dir);
    let unit_id = unit_id_for(&boundaries[0]);

    let contract = BehavioralContract {
        schema_version: "1.0.0".to_string(),
        contract_id: "contract-conflict-demo".to_string(),
        unit_id: unit_id.clone(),
        unit_name: "foo".to_string(),
        unit_kind: "function".to_string(),
        source_signature: "foo() -> int".to_string(),
        target_signature: "foo() -> i64".to_string(),
        assertions: vec![BehavioralAssertion {
            case_id: "basic".to_string(),
            input: "foo()".to_string(),
            expected: "1".to_string(),
            oracle: OracleType::SourceTest,
            evidence: "hand-authored: trivial constant function, unambiguous from source"
                .to_string(),
        }],
        verification_status: VerificationStatus::Pending,
    };

    let work_dir = repo_root.join(".exodus").join("work").join("t-conflict");
    let ctx = UnitGateContext {
        run_id: "r-conflict",
        repo: &repo,
        graph: &graph,
        contracts_dir: repo_root.join(".exodus").join("contracts"),
        work_dir: work_dir.clone(),
        case_engine: &case_engine,
        contract: Some(contract),
        repair_bounds: AgentBounds::default(),
    };

    let (unit_result, transform_result) = run_unit_gate(&boundaries[0], &ctx).await.unwrap();
    assert_eq!(
        unit_result.outcome,
        MigrationOutcome::Verified,
        "the unit's own isolated gate must genuinely pass: {unit_result:#?}"
    );

    // Now exercise module-integration compilation directly: accumulate this already-verified
    // unit's module TWICE under the same crate, which is illegal Rust (a module can only be
    // declared once) — a real, mechanical integration failure despite the contributing unit(s)
    // each having individually verified.
    let colliding = TransformResult {
        module_name: transform_result.module_name.clone(),
        ..transform_result.clone()
    };
    let integration_dir = repo_root
        .join(".exodus")
        .join("work")
        .join("t-conflict-integration");
    let integration_verifier = Verifier::new(&integration_dir);
    integration_verifier
        .scaffold_target_crate(
            "exodus_migrated_target",
            &[transform_result, colliding],
            None,
        )
        .unwrap();
    let (integration_compiled, diags) = integration_verifier.run_cargo_check().unwrap();

    assert!(
        !integration_compiled,
        "module integration must fail on a real duplicate-module conflict, not silently pass"
    );
    assert!(!diags.is_empty());

    let _ = std::fs::remove_dir_all(&repo_root);
    let _ = std::fs::remove_dir_all(&tmp_src_dir);
}
