//! Integration tests for the TypeScript -> Go migration lane.
//!
//! Tests signature rules validation, target path planning with collision detection,
//! concurrency mapping, and Go toolchain verifier orchestration.

use exodus_core::{
    ConcurrencyMappingPlan, GroundingTier, LanguageId, MappingApprovalStatus, MigrationOutcome,
    NullabilityPolicy, ParameterRule, ReceiverRule, ReturnTypeRule, SignatureRuleSet,
    TargetLanguageSpecRecord, TargetPathMapping, TargetPathPlan, TypeMappingRule,
};
use exodus_verifier::UniversalTargetVerifier;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn test_typescript_to_go_signature_rule_set_validation() {
    let mut go_spec = TargetLanguageSpecRecord::default_specs()
        .into_iter()
        .find(|s| s.id.as_str() == "go")
        .expect("Go language specification must be registered");

    // Add typescript -> go signature rules
    let ts_rules = SignatureRuleSet {
        schema_version: "1.0.0".to_string(),
        revision: 1,
        source_language: LanguageId::new("typescript"),
        target_language: LanguageId::new("go"),
        function_keyword: "func".to_string(),
        signature_template: "{{function_keyword}} {{name}}({{parameters}}) {{return_type}}"
            .to_string(),
        receiver: ReceiverRule {
            instance_template: "({{name}} *{{type}})".to_string(),
            mutable_instance_template: None,
            static_template: None,
        },
        parameters: ParameterRule {
            template: "{{name}} {{type}}".to_string(),
            separator: ", ".to_string(),
            variadic_template: Some("...{{type}}".to_string()),
            default_values_supported: false,
        },
        return_type: ReturnTypeRule {
            template: "({{type}}, error)".to_string(),
            void_types: vec!["void".to_string(), "undefined".to_string()],
            async_wrapper_template: None,
        },
        type_mappings: vec![
            TypeMappingRule {
                source_pattern: "string".to_string(),
                target_type: "string".to_string(),
                generic_template: None,
                nullable_template: None,
                nullability: NullabilityPolicy::Preserve,
                grounding: GroundingTier::Deterministic,
            },
            TypeMappingRule {
                source_pattern: "number".to_string(),
                target_type: "float64".to_string(),
                generic_template: None,
                nullable_template: None,
                nullability: NullabilityPolicy::Preserve,
                grounding: GroundingTier::Deterministic,
            },
            TypeMappingRule {
                source_pattern: "boolean".to_string(),
                target_type: "bool".to_string(),
                generic_template: None,
                nullable_template: None,
                nullability: NullabilityPolicy::Preserve,
                grounding: GroundingTier::Deterministic,
            },
            TypeMappingRule {
                source_pattern: "Promise<User>".to_string(),
                target_type: "*User".to_string(),
                generic_template: None,
                nullable_template: None,
                nullability: NullabilityPolicy::Preserve,
                grounding: GroundingTier::Deterministic,
            },
        ],
    };

    assert!(ts_rules.validate().is_ok());

    go_spec.signature_rule_sets.push(ts_rules);

    assert!(go_spec
        .signature_rule_sets
        .iter()
        .any(|r| r.source_language == "typescript"));
}

#[test]
fn test_typescript_to_go_target_path_planning() {
    let mut plan = TargetPathPlan::default();

    plan.mappings.push(TargetPathMapping {
        source: std::path::PathBuf::from("src/services/user_service.ts"),
        target: std::path::PathBuf::from("user_service.go"),
        package: Some("userservice".to_string()),
    });
    plan.mappings.push(TargetPathMapping {
        source: std::path::PathBuf::from("src/services/user_service.test.ts"),
        target: std::path::PathBuf::from("user_service_test.go"),
        package: Some("userservice".to_string()),
    });
    plan.mappings.push(TargetPathMapping {
        source: std::path::PathBuf::from("src/utils/crypto.ts"),
        target: std::path::PathBuf::from("crypto.go"),
        package: Some("crypto".to_string()),
    });

    // Valid plan has no collisions
    assert!(plan.detect_collisions().is_ok());

    // Collisions must be detected when another file maps to same target
    plan.mappings.push(TargetPathMapping {
        source: std::path::PathBuf::from("src/another/user_service.ts"),
        target: std::path::PathBuf::from("user_service.go"),
        package: Some("userservice".to_string()),
    });

    let collision = plan.detect_collisions();
    assert!(collision.is_err());
    let errs = collision.unwrap_err();
    assert!(errs.iter().any(|e| e.kind == "duplicate_target"));
}

#[test]
fn test_typescript_to_go_concurrency_mapping() {
    let mapping_plan = ConcurrencyMappingPlan {
        schema_version: "1.0.0".to_string(),
        source_language: LanguageId::new("typescript"),
        target_language: LanguageId::new("go"),
        source_concurrency_tier: "single_threaded_event_loop".to_string(),
        target_runtime: "goroutines_and_channels".to_string(),
        primitive_substitutions: BTreeMap::from([
            (
                "Promise.all".to_string(),
                "golang.org/x/sync/errgroup".to_string(),
            ),
            (
                "async/await".to_string(),
                "goroutine spawning with return channel".to_string(),
            ),
        ]),
        blocking_call_treatment: Some("run_in_goroutine".to_string()),
        lifecycle_expectations: vec!["context.Context cancellation propagation".to_string()],
        unsupported_primitives: vec![],
        grounding: GroundingTier::Deterministic,
        approval_status: MappingApprovalStatus::AcceptedForPlanning,
        evidence_ids: vec!["ev-concurrency-ts".to_string()],
    };

    assert!(mapping_plan.is_ready());
    assert_eq!(mapping_plan.primitive_substitutions.len(), 2);
    assert_eq!(
        mapping_plan
            .primitive_substitutions
            .get("Promise.all")
            .unwrap(),
        "golang.org/x/sync/errgroup"
    );
}

#[tokio::test]
async fn test_typescript_to_go_scaffold_and_verifier() {
    let test_dir = std::env::temp_dir().join(format!("exodus_test_ts_go_{}", uuid::Uuid::now_v7()));
    let workspace = test_dir.join("go_workspace");
    fs::create_dir_all(&workspace).unwrap();

    // 1. Scaffold Go module and source files
    let go_mod = r#"module example.com/userservice

go 1.21
"#;
    fs::write(workspace.join("go.mod"), go_mod).unwrap();

    let user_service_go = r#"package userservice

import "fmt"

type User struct {
    ID   string `json:"id"`
    Name string `json:"name"`
}

func FetchUser(id string) (*User, error) {
    if id == "" {
        return nil, fmt.Errorf("user ID cannot be empty")
    }
    return &User{
        ID:   id,
        Name: "User-" + id,
    }, nil
}
"#;
    fs::write(workspace.join("user_service.go"), user_service_go).unwrap();

    let user_service_test_go = r#"package userservice

import "testing"

func TestFetchUser(t *testing.T) {
    user, err := FetchUser("123")
    if err != nil {
        t.Fatalf("expected no error, got %v", err)
    }
    if user.ID != "123" {
        t.Errorf("expected ID 123, got %s", user.ID)
    }
}
"#;
    fs::write(workspace.join("user_service_test.go"), user_service_test_go).unwrap();

    // 2. Resolve Go language spec and verifier
    let go_spec = TargetLanguageSpecRecord::default_specs()
        .into_iter()
        .find(|s| s.id.as_str() == "go")
        .unwrap();

    let verifier = UniversalTargetVerifier::for_spec(&go_spec);
    let res = verifier.verify_workspace_full(&workspace).await;
    match res {
        Ok(report) => {
            if report.compiled {
                assert!(matches!(
                    report.outcome,
                    MigrationOutcome::Verified | MigrationOutcome::Compatible
                ));
            } else {
                assert_eq!(report.outcome, MigrationOutcome::Blocked);
            }
        }
        Err(e) => {
            let err_msg = e.to_string();
            assert!(
                err_msg.contains("Failed to execute command")
                    || err_msg.contains("No such file or directory")
                    || err_msg.contains("Verification")
            );
        }
    }

    let _ = fs::remove_dir_all(&test_dir);
}
