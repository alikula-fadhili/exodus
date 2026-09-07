//! Per-unit migration gate (master prompt §11): resolve dependencies, generate/compile the
//! smallest valid target component for a single [`VerificationBoundary`], run its behavioral
//! contract if one is grounded, apply bounded repair on failure, localize failures into the Case
//! Engine, and report a real (never fabricated) verification result.

use crate::Verifier;
use chrono::Utc;
use exodus_agent::{AgentBounds, BoundedAgent, MockAgentProvider};
use exodus_case::{CaseCaptureInput, CaseEngine, FailureCategory};
use exodus_core::{BehavioralContract, ExodusError, MigrationOutcome, Result, VerificationStatus};
use exodus_graph::{NodeKind, SemanticGraph, VerificationBoundary};
use exodus_parser::{ParsedModule, ParsedRepository};
use exodus_transform::{TransformOptions, TransformRequest, TransformationEngine};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The result of running a single [`VerificationBoundary`] through the per-unit gate. Serializes
/// directly to `.exodus/contracts/<unit-id>/verification.json` (schema: `schemas/verification.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitVerificationResult {
    pub schema_version: String,
    pub verification_id: String,
    pub unit_id: String,
    pub boundary_kind: String, // "unit" | "cluster"
    pub cluster_members: Vec<String>,
    pub contract_id: Option<String>,
    pub compiled: bool,
    pub diagnostics_count: usize,
    pub assertions_total: usize,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub failed_assertion_case_id: Option<String>,
    pub repair_attempts: u32,
    pub outcome: MigrationOutcome,
    pub case_id: Option<String>,
    pub rustc_version: String,
    pub verified_at: String,
}

/// Everything the gate needs to resolve, generate, compile, and verify one boundary.
pub struct UnitGateContext<'a> {
    pub run_id: &'a str,
    pub repo: &'a ParsedRepository,
    pub graph: &'a SemanticGraph,
    /// Where generated `.exodus/contracts/<unit-id>/{behavioral-contract,verification}.json`
    /// artifacts are written.
    pub contracts_dir: PathBuf,
    /// Where the target crate for this attempt is scaffolded and compiled — the smallest valid
    /// target component, not the whole repository.
    pub work_dir: PathBuf,
    pub case_engine: &'a CaseEngine,
    /// A grounded contract for this boundary's primary unit, if one exists (see
    /// `load_fixture_contract`). `None` means no grounded oracle is available for this unit —
    /// the gate still compiles and reports honestly, it just cannot claim behavioral verification.
    pub contract: Option<BehavioralContract>,
    pub repair_bounds: AgentBounds,
}

pub fn unit_id_for(boundary: &VerificationBoundary) -> String {
    let mut ids = boundary.node_ids();
    ids.sort();
    ids.join("+")
}

/// A unit ID (`"function::math_ops::add"`, or a `+`-joined cluster of them) contains characters
/// that aren't valid in a Rust module identifier. Derives a stable, collision-resistant one so
/// each unit gets its own `src/<name>.rs` when multiple verified units accumulate into one growing
/// target crate (see `pipeline::run_gated_migration`), instead of every unit colliding on the same
/// generic module name.
pub fn module_name_for(unit_id: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_underscore = false;
    for c in unit_id.chars() {
        if c.is_alphanumeric() {
            sanitized.push(c);
            last_was_underscore = false;
        } else if !last_was_underscore {
            sanitized.push('_');
            last_was_underscore = true;
        }
    }
    format!("unit_{sanitized}")
}

/// Fixed crate name for a single unit's isolated scratch build. Deliberately distinct from every
/// possible `module_name_for` output: if the crate and its one module shared a name, a test's
/// `use <name>::*;` would be genuinely ambiguous between "the module" and "the crate itself"
/// (Rust integration tests can always refer to the crate under test by its own name) and fail to
/// compile with E0659 regardless of whether the migrated code is correct.
const UNIT_SCRATCH_CRATE_NAME: &str = "exodus_unit_under_test";

/// Pulls the `FunctionDef`/`ClassDef`s that make up a boundary out of the full parsed repository,
/// plus any `Type` node the boundary's members directly depend on (so a function referencing a
/// class it doesn't itself define still has that class in scope) — this is what "compile the
/// smallest valid target component" means in practice: not just the failing/target symbol, but
/// whatever it structurally needs to type-check on its own.
fn build_synthetic_module(
    boundary: &VerificationBoundary,
    repo: &ParsedRepository,
    graph: &SemanticGraph,
    module_name: &str,
) -> ParsedModule {
    let mut wanted: HashSet<String> = boundary.node_ids().into_iter().collect();

    // A boundary member calling `p.get_price()` depends on the `Product` *method*, not the type
    // node directly — pulling in that method (which drags its whole containing class along, via
    // the class-inclusion pass below) or a directly-depended type is what makes the unit
    // compilable in isolation.
    let mut extra: Vec<String> = Vec::new();
    for id in &wanted {
        for dep in graph.dependencies_of(id) {
            if graph
                .get_node(&dep)
                .is_some_and(|n| matches!(n.kind, NodeKind::Type | NodeKind::Method))
            {
                extra.push(dep);
            }
        }
    }
    wanted.extend(extra);

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut seen_classes: HashSet<String> = HashSet::new();

    for module in &repo.modules {
        for func in &module.functions {
            if wanted.contains(&format!("function::{}", func.qualified_name)) {
                functions.push(func.clone());
            }
        }
        for class in &module.classes {
            let class_wanted = wanted.contains(&format!("type::{}", class.qualified_name));
            let method_wanted = class
                .methods
                .iter()
                .any(|m| wanted.contains(&format!("method::{}", m.qualified_name)));
            if (class_wanted || method_wanted) && seen_classes.insert(class.qualified_name.clone())
            {
                classes.push(class.clone());
            }
        }
    }

    ParsedModule {
        file_path: PathBuf::from(format!("{module_name}.py")),
        module_name: module_name.to_string(),
        imports: Vec::new(),
        functions,
        classes,
        calls: Vec::new(),
        assignments: Vec::new(),
        unsupported_constructs: Vec::new(),
        diagnostics: Vec::new(),
    }
}

/// Parses `cargo test`'s default text output for per-test pass/fail lines
/// (`test <name> ... ok|FAILED`). Cargo's harness output format is stable enough to rely on
/// without pulling in the unstable `--format json` flag.
fn parse_test_results(stdout: &str) -> (usize, usize, Vec<String>) {
    let re = Regex::new(r"(?m)^test (\S+) \.\.\. (ok|FAILED)$").unwrap();
    let mut passed = 0;
    let mut failed_names = Vec::new();
    for cap in re.captures_iter(stdout) {
        if &cap[2] == "ok" {
            passed += 1;
        } else {
            failed_names.push(cap[1].to_string());
        }
    }
    (passed, failed_names.len(), failed_names)
}

/// Builds a `tests/behavioral_tests.rs` harness from a grounded contract: one `#[test]` per
/// assertion, comparing the migrated unit's actual output against the grounded `expected` value.
/// Assumes assertions target a single top-level function named `unit_id`'s final segment; this is
/// sufficient for the pure/stateful function contracts this pass grounds (see M3 fixtures).
/// `#[tokio::test]` async tests can call both async and sync migrated units — a sync call inside
/// an async test body needs no `.await` and behaves identically to a plain `#[test]`. This lets
/// one harness shape cover both without the gate needing to track async-ness separately from what
/// the grounded contract's `input` expression already encodes (an async assertion's `input`
/// includes its own trailing `.await`, authored alongside the differential-execution capture that
/// produced `expected` — see `scripts/ground_fixture_contracts.py`).
fn build_harness(contract: &BehavioralContract, crate_name: &str, module_name: &str) -> String {
    // From an integration test in `tests/`, the crate under test is reached by its own crate
    // name, not directly by its internal module path.
    let mut out = format!("use {crate_name}::{module_name}::*;\n\n");
    for (i, assertion) in contract.assertions.iter().enumerate() {
        // `assert_eq!`'s message argument is itself parsed as a format string by the macro — the
        // literal fixed text here ("assertion `{}` ...") is the only part that gets macro-parsed;
        // `case_id`/`oracle`/`evidence` are passed as separate `{:?}`-quoted string *arguments*,
        // so any `{`/`}`/quotes they happen to contain (real evidence text, e.g. a captured path
        // like `{models,service}.py`) is just literal data, never re-parsed as a format string.
        out.push_str(&format!(
            "#[tokio::test]\nasync fn contract_assertion_{i}_{case}() {{\n    let actual = format!(\"{{:?}}\", {call});\n    assert_eq!(actual, {expected:?}, \"assertion `{{}}` (oracle: {{}}, evidence: {{}})\", {case_id:?}, {oracle:?}, {evidence:?});\n}}\n\n",
            i = i,
            case = sanitize_ident(&assertion.case_id),
            call = assertion.input,
            expected = assertion.expected,
            case_id = assertion.case_id,
            oracle = assertion.oracle.to_string(),
            evidence = assertion.evidence,
        ));
    }
    out
}

fn sanitize_ident(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Loads a fixture-authored, differential-execution-grounded contract for `unit_id` from
/// `<fixture_root>/contracts.json`, if present. That file is authored once per fixture by actually
/// running the Python source (see `scripts/ground_fixture_contracts.py` and
/// `docs/reviews/unit-verification-audit.md`), never invented from a signature.
pub fn load_fixture_contract(fixture_root: &Path, unit_id: &str) -> Option<BehavioralContract> {
    let path = fixture_root.join("contracts.json");
    let content = std::fs::read_to_string(path).ok()?;
    let all: Vec<BehavioralContract> = serde_json::from_str(&content).ok()?;
    all.into_iter().find(|c| c.unit_id == unit_id)
}

/// Runs the full per-unit migration gate for one boundary: resolve deps, generate the smallest
/// compilable component, compile, run the contract if grounded, repair on failure (bounded), and
/// localize any unresolved failure into a Migration Case. Never labels a unit `Verified` merely
/// because it compiled.
pub async fn run_unit_gate(
    boundary: &VerificationBoundary,
    ctx: &UnitGateContext<'_>,
) -> Result<(UnitVerificationResult, exodus_transform::TransformResult)> {
    let unit_id = unit_id_for(boundary);
    let module_name = module_name_for(&unit_id);
    let is_cluster = boundary.is_cluster();
    let cluster_members = if is_cluster {
        boundary.node_ids()
    } else {
        Vec::new()
    };

    let synthetic_module = build_synthetic_module(boundary, ctx.repo, ctx.graph, &module_name);
    let engine = TransformationEngine::new();
    let transform_result = engine.transform_module(&TransformRequest {
        parsed_module: synthetic_module,
        options: TransformOptions::default(),
    })?;

    let has_stub_fallback = transform_result.fallbacks.iter().any(|f| {
        matches!(
            f.strategy,
            exodus_fallback::FallbackStrategy::TypedFailureStub
        )
    });
    let has_any_fallback = !transform_result.fallbacks.is_empty();

    // `work_dir` is a scratch build directory that may be reused sequentially across units in the
    // same run (see `pipeline::run_gated_migration`) — clear any previous unit's generated source
    // so a stale, no-longer-referenced module file never lingers on disk.
    let _ = std::fs::remove_dir_all(ctx.work_dir.join("src"));
    let _ = std::fs::remove_dir_all(ctx.work_dir.join("tests"));
    std::fs::create_dir_all(&ctx.work_dir).map_err(ExodusError::from)?;
    let verifier = Verifier::new(&ctx.work_dir);

    let harness = ctx
        .contract
        .as_ref()
        .map(|c| build_harness(c, UNIT_SCRATCH_CRATE_NAME, &module_name));
    verifier.scaffold_target_crate(
        UNIT_SCRATCH_CRATE_NAME,
        std::slice::from_ref(&transform_result),
        harness.as_deref(),
    )?;
    let _ = verifier.run_formatter();

    let (mut compiled, mut diagnostics) = verifier.run_cargo_check()?;
    let mut repair_attempts = 0u32;

    if !compiled {
        let mut agent = BoundedAgent::new(MockAgentProvider::new(), ctx.repair_bounds.clone());
        let error_snippet = diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let (_, outcome, _debt) = agent
            .repair_symbol(&unit_id, "// unit under verification", &error_snippet, None)
            .await;
        repair_attempts = 1;
        if outcome == MigrationOutcome::Compatible {
            let (re_compiled, re_diags) = verifier.run_cargo_check().unwrap_or((false, vec![]));
            compiled = re_compiled;
            diagnostics = re_diags;
        }
    }

    let mut assertions_total = ctx.contract.as_ref().map_or(0, |c| c.assertions.len());
    let mut assertions_passed = 0usize;
    let mut assertions_failed = 0usize;
    let mut failed_assertion_case_id: Option<String> = None;
    let mut failed_test_names: Vec<String> = Vec::new();
    // True when `cargo test` itself couldn't produce a test binary at all (e.g. a bug in the
    // generated harness) as opposed to producing one that ran assertions and failed some — these
    // must not be conflated: 0 passed + 0 failed is "nothing was actually verified," never success.
    let mut harness_build_failed = false;

    if compiled && ctx.contract.is_some() {
        let shared_target = std::env::temp_dir().join("exodus_shared_target");
        let output = std::process::Command::new("cargo")
            .arg("test")
            .env("CARGO_TARGET_DIR", &shared_target)
            .current_dir(&ctx.work_dir)
            .output()
            .map_err(ExodusError::from)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (passed, failed, names) = parse_test_results(&stdout);
        assertions_passed = passed;
        assertions_failed = failed;
        assertions_total = assertions_total.max(passed + failed);
        failed_test_names = names;
        failed_assertion_case_id = failed_test_names.first().cloned();
        if !output.status.success() && passed == 0 && failed == 0 {
            harness_build_failed = true;
            let stderr = String::from_utf8_lossy(&output.stderr);
            diagnostics.push(exodus_core::Diagnostic::error(
                "E_HARNESS_BUILD",
                format!("behavioral test harness failed to build or run: {stderr}"),
                None,
            ));
        }
    }

    let outcome = if !compiled {
        MigrationOutcome::Blocked
    } else if ctx.contract.is_none() {
        // Compiles, but there is no grounded behavioral evidence to verify against — this is
        // explicit, reported debt, never presented as verified behavior.
        MigrationOutcome::Degraded
    } else if !harness_build_failed
        && assertions_failed == 0
        && assertions_passed > 0
        && assertions_passed == assertions_total
    {
        if has_stub_fallback {
            MigrationOutcome::Degraded
        } else if has_any_fallback {
            MigrationOutcome::Compatible
        } else {
            MigrationOutcome::Verified
        }
    } else if has_stub_fallback || has_any_fallback {
        MigrationOutcome::Degraded
    } else {
        MigrationOutcome::Blocked
    };

    let mut case_id = None;
    if matches!(outcome, MigrationOutcome::Blocked) {
        let subgraph = boundary
            .node_ids()
            .first()
            .map(|id| ctx.graph.relevant_subgraph(id));
        let failure_category = if !compiled {
            FailureCategory::SyntaxError
        } else {
            FailureCategory::BehavioralAssertionFailed
        };
        let diagnostic_text = if !compiled {
            diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            format!("Failed assertions: {}", failed_test_names.join(", "))
        };
        let (source_obs, target_obs) =
            if let (Some(contract), Some(test_name)) = (&ctx.contract, &failed_assertion_case_id) {
                // Harness test names are `contract_assertion_<i>_<sanitized case_id>` — match by
                // substring rather than reconstructing the exact index prefix.
                let assertion = contract
                    .assertions
                    .iter()
                    .find(|a| test_name.contains(&sanitize_ident(&a.case_id)));
                (
                    assertion.map(|a| a.expected.clone()),
                    Some(diagnostic_text.clone()),
                )
            } else {
                (None, None)
            };

        let captured = ctx.case_engine.capture_failure(CaseCaptureInput {
            run_id: ctx.run_id,
            failure_category,
            unit_id: &unit_id,
            source_language: "Python 3.11",
            target_language: "Rust 2021",
            graph: subgraph.as_ref(),
            diagnostic: Some(&diagnostic_text),
            failed_assertion: failed_assertion_case_id.as_deref(),
            source_observation: source_obs.as_deref(),
            target_observation: target_obs.as_deref(),
        })?;
        case_id = Some(captured.case_id);
    }

    let rustc_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let verification_id = format!("verify-{}", uuid::Uuid::new_v4());
    let result = UnitVerificationResult {
        schema_version: "1.0.0".to_string(),
        verification_id,
        unit_id: unit_id.clone(),
        boundary_kind: if is_cluster { "cluster" } else { "unit" }.to_string(),
        cluster_members,
        contract_id: ctx.contract.as_ref().map(|c| c.contract_id.clone()),
        compiled,
        diagnostics_count: diagnostics.len(),
        assertions_total,
        assertions_passed,
        assertions_failed,
        failed_assertion_case_id,
        repair_attempts,
        outcome,
        case_id,
        rustc_version,
        verified_at: Utc::now().to_rfc3339(),
    };

    write_artifacts(ctx, &unit_id, &result, has_stub_fallback)?;
    Ok((result, transform_result))
}

fn write_artifacts(
    ctx: &UnitGateContext<'_>,
    unit_id: &str,
    result: &UnitVerificationResult,
    _has_stub_fallback: bool,
) -> Result<()> {
    let unit_dir = ctx.contracts_dir.join(unit_id);
    std::fs::create_dir_all(&unit_dir).map_err(ExodusError::from)?;

    if let Some(contract) = &ctx.contract {
        let mut final_contract = contract.clone();
        final_contract.verification_status = match result.outcome {
            MigrationOutcome::Verified | MigrationOutcome::Compatible => VerificationStatus::Passed,
            MigrationOutcome::Degraded => VerificationStatus::Degraded,
            MigrationOutcome::Blocked => VerificationStatus::Failed,
        };
        std::fs::write(
            unit_dir.join("behavioral-contract.json"),
            serde_json::to_string_pretty(&final_contract)?,
        )
        .map_err(ExodusError::from)?;
    }

    std::fs::write(
        unit_dir.join("verification.json"),
        serde_json::to_string_pretty(result)?,
    )
    .map_err(ExodusError::from)?;

    Ok(())
}
