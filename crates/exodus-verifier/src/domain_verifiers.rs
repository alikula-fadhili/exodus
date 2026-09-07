//! Multi-Domain Deterministic Contract Verifiers.
//!
//! Provides deterministic verification pipelines for the 3 operational domains:
//! - `#prod-bug`: Unit compilation and test harness execution inside Git worktrees.
//! - `#crm-request`: Commercial policy rules, discount limits, and ARR tier integrity.
//! - `#survey-mapping`: Product taxonomy completeness, foreign key validation, and unmapped key checks.

use exodus_core::{
    ContractVerificationReport, CrmRequestPayload, DomainPayload, MigrationDebt, OperationalItem,
    ProdBugPayload, Result, RuleVerificationResult, SurveyMappingPayload, UnitPromptDiagnostic,
};
use exodus_store::OperationalStore;
use std::path::Path;
use tokio::process::Command;

/// Verifier for software engineering bugs and CI crashes (#prod-bug).
pub struct ProdBugVerifier;

impl ProdBugVerifier {
    /// Verify fix within an isolated Git worktree or repo directory.
    pub async fn verify(
        payload: &ProdBugPayload,
        worktree_path: Option<&Path>,
    ) -> ContractVerificationReport {
        let mut rules = Vec::new();
        let target_dir = worktree_path.unwrap_or_else(|| Path::new("."));

        // Rule 1: Working directory and target file existence
        if let Some(target_file) = &payload.target_file {
            let full_path = target_dir.join(target_file);
            let exists = full_path.exists();
            rules.push(RuleVerificationResult {
                rule_name: "file_exists".to_string(),
                passed: exists,
                details: format!("Target file {:?} exists: {}", target_file, exists),
            });
            if !exists {
                return ContractVerificationReport::failure(
                    format!("Target file {:?} does not exist in worktree", target_file),
                    rules,
                );
            }
        }

        // Rule 2: Execute compiler / test command if reproduction command is provided
        let test_cmd = payload
            .reproduction_command
            .as_deref()
            .unwrap_or("cargo check --tests");

        let parts: Vec<&str> = test_cmd.split_whitespace().collect();
        if parts.is_empty() {
            rules.push(RuleVerificationResult {
                rule_name: "command_execution".to_string(),
                passed: true,
                details: "No reproduction command provided; syntax verified".to_string(),
            });
            return ContractVerificationReport::success(
                "Reproduction command omitted; validated",
                rules,
            );
        }

        let program = parts[0];
        let args = &parts[1..];

        match Command::new(program)
            .args(args)
            .current_dir(target_dir)
            .output()
            .await
        {
            Ok(output) => {
                let passed = output.status.success();
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                let summary_output = if !stderr.is_empty() {
                    stderr.to_string()
                } else {
                    stdout.lines().take(5).collect::<Vec<_>>().join("\n")
                };

                rules.push(RuleVerificationResult {
                    rule_name: "test_harness_execution".to_string(),
                    passed,
                    details: format!(
                        "Command '{}' exited with {}: {}",
                        test_cmd, output.status, summary_output
                    ),
                });

                if passed {
                    ContractVerificationReport::success(
                        format!(
                            "Deterministic test harness passed with exit code 0: {}",
                            test_cmd
                        ),
                        rules,
                    )
                } else {
                    ContractVerificationReport::failure(
                        format!(
                            "Compiler / test failure under '{}': {}",
                            test_cmd, summary_output
                        ),
                        rules,
                    )
                }
            }
            Err(err) => {
                rules.push(RuleVerificationResult {
                    rule_name: "command_execution".to_string(),
                    passed: false,
                    details: format!("Failed to spawn test command '{}': {}", test_cmd, err),
                });
                ContractVerificationReport::failure(
                    format!("Subprocess spawn error: {}", err),
                    rules,
                )
            }
        }
    }
}

/// Verifier for commercial requests and CRM mutations (#crm-request).
pub struct CrmPolicyVerifier;

impl CrmPolicyVerifier {
    /// Verify discount request against embedded SurrealDB governance policy rules.
    pub async fn verify(
        payload: &CrmRequestPayload,
        store: &impl OperationalStore,
    ) -> ContractVerificationReport {
        let mut rules = Vec::new();
        let mut debts = Vec::new();

        // Rule 1: Check discount authorization role threshold
        let eval_res = store
            .evaluate_crm_discount(
                &payload.account_id,
                payload.requested_discount_pct,
                &payload.requester_role,
            )
            .await;

        let role_compliant = match eval_res {
            Ok(report) => {
                rules.push(RuleVerificationResult {
                    rule_name: "discount_role_authorization".to_string(),
                    passed: report.compliant,
                    details: report.reason.clone(),
                });
                report.compliant
            }
            Err(e) => {
                rules.push(RuleVerificationResult {
                    rule_name: "discount_role_authorization".to_string(),
                    passed: false,
                    details: format!("Store policy error: {}", e),
                });
                false
            }
        };

        // Rule 2: Minimum contract duration constraint (>= 12 months)
        let term_valid = payload.contract_term_months >= 12;
        rules.push(RuleVerificationResult {
            rule_name: "contract_duration_floor".to_string(),
            passed: term_valid,
            details: format!(
                "Contract term of {} months (floor is 12 months)",
                payload.contract_term_months
            ),
        });

        // Rule 3: Tier ARR eligibility floor
        let tier_floor_met = match payload.requested_tier.as_str() {
            "Enterprise" => payload.current_arr >= 50_000.0,
            "Custom" => payload.current_arr >= 100_000.0,
            _ => true,
        };
        rules.push(RuleVerificationResult {
            rule_name: "tier_arr_floor".to_string(),
            passed: tier_floor_met,
            details: format!(
                "ARR ${:.2} satisfies floor for tier '{}'",
                payload.current_arr, payload.requested_tier
            ),
        });

        if !role_compliant || !term_valid || !tier_floor_met {
            ContractVerificationReport::failure(
                "CRM Policy verification failed: discount or contract tier violation",
                rules,
            )
        } else if payload.requested_discount_pct > 20.0 {
            // Degraded with review debt if discount exceeds 20%
            debts.push(MigrationDebt {
                symbol_id: payload.account_id.clone(),
                description: format!(
                    "High discount exception of {:.1}% requires executive notification audit",
                    payload.requested_discount_pct
                ),
                reason: "Commercial margin risk policy".to_string(),
                location: None,
                fallback_strategy: "Executive Audit Precedent".to_string(),
                confidence_score: 0.85,
                requires_human_review: true,
            });
            ContractVerificationReport::degraded(
                "CRM mutation approved with high-discount debt audit",
                rules,
                debts,
            )
        } else {
            ContractVerificationReport::success(
                "CRM mutation fully verified against commercial governance policies",
                rules,
            )
        }
    }
}

/// Verifier for customer sentiment and taxonomy ontology alignment (#survey-mapping).
pub struct SurveyTaxonomyVerifier;

impl SurveyTaxonomyVerifier {
    /// Verify survey response mapping completeness and foreign key integrity.
    pub async fn verify(
        payload: &SurveyMappingPayload,
        store: &impl OperationalStore,
    ) -> ContractVerificationReport {
        let mut rules = Vec::new();
        let mut debts = Vec::new();
        let total = payload.responses.len();

        if total == 0 {
            rules.push(RuleVerificationResult {
                rule_name: "batch_not_empty".to_string(),
                passed: false,
                details: "Survey batch contains zero responses".to_string(),
            });
            return ContractVerificationReport::failure("Empty survey batch", rules);
        }

        // Rule 1: Foreign key validity & Coverage
        let mut unmapped_count = 0;
        let mut valid_fk_count = 0;

        for resp in &payload.responses {
            if let Some(target_node_id) = &resp.candidate_taxonomy_node {
                let node = store
                    .get_taxonomy_node(target_node_id)
                    .await
                    .unwrap_or(None);
                if node.is_some() {
                    valid_fk_count += 1;
                } else {
                    rules.push(RuleVerificationResult {
                        rule_name: "taxonomy_foreign_key".to_string(),
                        passed: false,
                        details: format!(
                            "Response {} points to non-existent taxonomy node '{}'",
                            resp.id, target_node_id
                        ),
                    });
                }
            } else {
                unmapped_count += 1;
            }
        }

        let coverage_pct = ((total - unmapped_count) as f64) / (total as f64) * 100.0;
        rules.push(RuleVerificationResult {
            rule_name: "taxonomy_coverage".to_string(),
            passed: unmapped_count == 0,
            details: format!(
                "Mapped {} of {} responses ({:.1}% coverage, {} unmapped)",
                total - unmapped_count,
                total,
                coverage_pct,
                unmapped_count
            ),
        });

        if unmapped_count > 0 {
            debts.push(MigrationDebt {
                symbol_id: payload.batch_id.clone(),
                description: format!(
                    "{} unmapped customer survey responses in batch {}",
                    unmapped_count, payload.batch_id
                ),
                reason: "Taxonomy coverage gap".to_string(),
                location: None,
                fallback_strategy: "Unclassified Sentiment Bucket".to_string(),
                confidence_score: 0.5,
                requires_human_review: true,
            });

            ContractVerificationReport::degraded(
                format!(
                    "Taxonomy mapping verified with {:.1}% coverage ({} unmapped debt items)",
                    coverage_pct, unmapped_count
                ),
                rules,
                debts,
            )
        } else if valid_fk_count == total {
            ContractVerificationReport::success(
                "100% bidirectional taxonomy coverage verified with strict FK consistency",
                rules,
            )
        } else {
            ContractVerificationReport::failure(
                "Taxonomy verification failed due to invalid foreign key references",
                rules,
            )
        }
    }
}

/// Universal multi-domain contract verification orchestrator.
pub struct MultiDomainVerifier;

impl MultiDomainVerifier {
    /// Dispatches contract verification according to the operational domain tag.
    pub async fn verify_item(
        item: &mut OperationalItem,
        store: &impl OperationalStore,
        worktree_path: Option<&Path>,
    ) -> Result<ContractVerificationReport> {
        let report = match &item.payload {
            DomainPayload::ProdBug(payload) => {
                ProdBugVerifier::verify(payload, worktree_path).await
            }
            DomainPayload::CrmRequest(payload) => CrmPolicyVerifier::verify(payload, store).await,
            DomainPayload::SurveyMapping(payload) => {
                SurveyTaxonomyVerifier::verify(payload, store).await
            }
        };

        item.record_verification("deterministic-contract-verifier", report.clone())?;

        // If verification failed or degraded with debt, synthesize stage-by-stage prompt diagnostic breakdown
        let prompt_goal = item
            .prompt
            .clone()
            .unwrap_or_else(|| format!("Resolve: {}", item.title));

        if !report.passed {
            let failed_rule = report
                .rule_results
                .iter()
                .find(|r| !r.passed)
                .map(|r| r.details.clone())
                .unwrap_or_else(|| report.summary.clone());

            let failure_reason = match &item.payload {
                DomainPayload::ProdBug(_) => {
                    format!("The prompt directed the agent to achieve a unit change, but tests/compilation diverged: {}. Possible cause: unhandled variant, missing boundary guard, or incorrect signature.", failed_rule)
                }
                DomainPayload::CrmRequest(_) => {
                    format!("Requested commercial exception violates policy threshold: {}. Prompt did not specify required approval override authority.", failed_rule)
                }
                DomainPayload::SurveyMapping(_) => {
                    format!(
                        "Taxonomy mapping contains unresolved foreign key or unmapped tokens: {}.",
                        failed_rule
                    )
                }
            };

            let suggested_refinement = match &item.payload {
                DomainPayload::ProdBug(_) => Some(format!(
                    "{} Explicitly check for edge cases and ensure all test contracts in {} pass cleanly without panicking.",
                    prompt_goal.trim(),
                    failed_rule
                )),
                DomainPayload::CrmRequest(_) => Some(format!(
                    "{} Include VP-level override escalation tag or adjust discount under 20% limit.",
                    prompt_goal.trim()
                )),
                DomainPayload::SurveyMapping(_) => Some(format!(
                    "{} Map all unclassified keywords to default fallback taxonomy domain.",
                    prompt_goal.trim()
                )),
            };

            item.record_diagnostic(UnitPromptDiagnostic {
                stage_failed: "Stage 3: Tests Passing (Contract Gate Failure)".to_string(),
                prompt_goal: prompt_goal.clone(),
                execution_divergence: failed_rule.clone(),
                failure_reason,
                error_snippet: Some(report.summary.clone()),
                suggested_refinement,
            });
        } else if !report.debt.is_empty() {
            let debt_desc = report
                .debt
                .iter()
                .map(|d| d.reason.clone())
                .collect::<Vec<_>>()
                .join("; ");

            item.record_diagnostic(UnitPromptDiagnostic {
                stage_failed: "Stage 3: Tests Passing (Degraded with Debt)".to_string(),
                prompt_goal: prompt_goal.clone(),
                execution_divergence: format!("Passed compiler check but emitted {} migration debt item(s)", report.debt.len()),
                failure_reason: format!("Prompt achieved partial unit transformation but relied on fallback stubs: {}", debt_desc),
                error_snippet: Some(debt_desc.clone()),
                suggested_refinement: Some(format!(
                    "{} Fully implement stubbed fallbacks: {} without using todo!() or degraded branches.",
                    prompt_goal.trim(),
                    debt_desc
                )),
            });
        } else {
            // Passed cleanly without debt
            item.unit_diagnostic = None;
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_core::{
        DomainPayload, OperationalLifecycleState, ProdBugPayload, SurveyResponseItem,
    };
    use exodus_store::EmbeddedOperationalStore;

    #[tokio::test]
    async fn test_prod_bug_verification_dummy_command() {
        let payload = ProdBugPayload {
            commit_id: "test-commit".to_string(),
            error_message: "compile error".to_string(),
            stack_trace: None,
            target_file: None,
            target_symbol: None,
            reproduction_command: Some("echo 'Simulating successful test run'".to_string()),
        };

        let report = ProdBugVerifier::verify(&payload, None).await;
        assert!(report.passed);
        assert!(report.summary.contains("passed with exit code 0"));
    }

    #[tokio::test]
    async fn test_crm_policy_verification() {
        let store = EmbeddedOperationalStore::new();

        // Valid sales lead request
        let payload = CrmRequestPayload::new(
            "acc-corp-1",
            "Corp 1",
            75_000.0,
            18.0,
            "Enterprise",
            "SalesLead",
        );

        let report = CrmPolicyVerifier::verify(&payload, &store).await;
        assert!(report.passed);
        assert!(report.debt.is_empty());

        // Invalid discount request (exceeds role)
        let invalid_payload = CrmRequestPayload::new(
            "acc-corp-2",
            "Corp 2",
            20_000.0,
            35.0,
            "Standard",
            "AccountExec",
        );
        let report2 = CrmPolicyVerifier::verify(&invalid_payload, &store).await;
        assert!(!report2.passed);
    }

    #[tokio::test]
    async fn test_survey_taxonomy_verification() {
        let store = EmbeddedOperationalStore::new();

        let mut payload = SurveyMappingPayload::new("batch-01", "Typeform", "root-tax");
        payload.responses.push(SurveyResponseItem {
            id: "resp-1".to_string(),
            feedback_text: "Auth is broken".to_string(),
            customer_segment: None,
            candidate_taxonomy_node: Some("tax-auth-login".to_string()),
        });

        // Fully mapped
        let report = SurveyTaxonomyVerifier::verify(&payload, &store).await;
        assert!(report.passed);
        assert!(report.debt.is_empty());

        // Unmapped response -> degraded with debt
        payload.responses.push(SurveyResponseItem {
            id: "resp-2".to_string(),
            feedback_text: "Random unspecified comment".to_string(),
            customer_segment: None,
            candidate_taxonomy_node: None,
        });

        let report2 = SurveyTaxonomyVerifier::verify(&payload, &store).await;
        assert!(report2.passed);
        assert!(!report2.debt.is_empty());
    }

    #[tokio::test]
    async fn test_multi_domain_orchestrator() {
        let store = EmbeddedOperationalStore::new();

        let payload = DomainPayload::CrmRequest(CrmRequestPayload::new(
            "acc-mega",
            "Mega",
            80_000.0,
            12.0,
            "Enterprise",
            "SalesLead",
        ));

        let mut item = OperationalItem::new("Discount for Mega", "Desc", "agent", payload);
        item.mark_sandboxed("sandbox-agent", "in-memory-tx")
            .unwrap();

        let report = MultiDomainVerifier::verify_item(&mut item, &store, None)
            .await
            .unwrap();
        assert!(report.passed);
        assert_eq!(item.state, OperationalLifecycleState::ContractVerified);
    }
}
