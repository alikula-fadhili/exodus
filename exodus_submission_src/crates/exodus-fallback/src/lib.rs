//! Explicit fallback generation and migration debt accounting for Project Exodus.

use exodus_core::{MigrationDebt, MigrationOutcome, SourceSpan};
use serde::{Deserialize, Serialize};

/// Strategy used when a legacy construct cannot be directly transformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FallbackStrategy {
    /// Expand into equivalent standard Rust structures.
    ExpandedSyntax,
    /// Wrap in a compatibility adapter or emulation trait.
    CompatibilityWrapper,
    /// Represent dynamic types using `serde_json::Value` (recorded as debt).
    DynamicValue,
    /// Emit an explicit typed failure / todo! stub.
    TypedFailureStub,
    /// Mark for manual human architect review.
    ManualReview,
}

impl std::fmt::Display for FallbackStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpandedSyntax => write!(f, "ExpandedSyntax"),
            Self::CompatibilityWrapper => write!(f, "CompatibilityWrapper"),
            Self::DynamicValue => write!(f, "DynamicValue"),
            Self::TypedFailureStub => write!(f, "TypedFailureStub"),
            Self::ManualReview => write!(f, "ManualReview"),
        }
    }
}

/// A first-class fallback record representing generated fallback code and debt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FallbackRecord {
    pub symbol_id: String,
    pub source_location: Option<SourceSpan>,
    pub construct: String,
    pub strategy: FallbackStrategy,
    pub reason: String,
    pub confidence: f32,
    pub verification_status: MigrationOutcome,
    pub human_review_required: bool,
    pub generated_code: String,
}

impl FallbackRecord {
    pub fn to_migration_debt(&self) -> MigrationDebt {
        MigrationDebt {
            symbol_id: self.symbol_id.clone(),
            description: format!(
                "Fallback [{}] for construct `{}`",
                self.strategy, self.construct
            ),
            reason: self.reason.clone(),
            location: self.source_location.clone(),
            fallback_strategy: self.strategy.to_string(),
            confidence_score: self.confidence,
            requires_human_review: self.human_review_required,
        }
    }
}

/// Fallback generator producing safe, explicit Rust fallback stubs.
pub struct FallbackGenerator;

impl FallbackGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generates an explicit typed failure stub.
    pub fn generate_stub(
        &self,
        symbol_id: &str,
        fn_name: &str,
        params: &str,
        return_type: &str,
        reason: &str,
        source_location: Option<SourceSpan>,
    ) -> FallbackRecord {
        let ret_sig = if return_type.is_empty() || return_type == "()" {
            String::new()
        } else {
            format!(" -> {return_type}")
        };

        let generated_code = format!(
            "// Exodus Fallback Stub: {}\n// Reason: {}\npub fn {}({}){} {{\n    todo!(\"Exodus Migration Debt: {}\");\n}}\n",
            symbol_id, reason, fn_name, params, ret_sig, reason
        );

        FallbackRecord {
            symbol_id: symbol_id.to_string(),
            source_location,
            construct: fn_name.to_string(),
            strategy: FallbackStrategy::TypedFailureStub,
            reason: reason.to_string(),
            confidence: 1.0,
            verification_status: MigrationOutcome::Degraded,
            human_review_required: true,
            generated_code,
        }
    }

    /// Generates a dynamic value fallback adapter.
    pub fn generate_dynamic_value_adapter(
        &self,
        symbol_id: &str,
        field_or_fn: &str,
        reason: &str,
        source_location: Option<SourceSpan>,
    ) -> FallbackRecord {
        let generated_code = format!(
            "// Exodus Migration Debt: Dynamic typing mapped to serde_json::Value\npub type {}Dynamic = serde_json::Value;\n",
            field_or_fn
        );

        FallbackRecord {
            symbol_id: symbol_id.to_string(),
            source_location,
            construct: field_or_fn.to_string(),
            strategy: FallbackStrategy::DynamicValue,
            reason: reason.to_string(),
            confidence: 0.8,
            verification_status: MigrationOutcome::Compatible,
            human_review_required: false,
            generated_code,
        }
    }

    /// Generates a compatibility wrapper trait/struct.
    pub fn generate_compatibility_wrapper(
        &self,
        symbol_id: &str,
        trait_or_struct_name: &str,
        methods: &[(&str, &str, &str)], // (method_name, args, ret)
        reason: &str,
        source_location: Option<SourceSpan>,
    ) -> FallbackRecord {
        let mut code = format!(
            "// Exodus Compatibility Wrapper for {}\n// Reason: {}\npub trait {}Compat {{\n",
            symbol_id, reason, trait_or_struct_name
        );
        for (m, a, r) in methods {
            let ret = if r.is_empty() || *r == "()" {
                String::new()
            } else {
                format!(" -> {r}")
            };
            let args_part = if a.is_empty() {
                String::new()
            } else {
                format!(", {a}")
            };
            code.push_str(&format!("    fn {}(&self{}){};\n", m, args_part, ret));
        }
        code.push_str("}\n");

        FallbackRecord {
            symbol_id: symbol_id.to_string(),
            source_location,
            construct: trait_or_struct_name.to_string(),
            strategy: FallbackStrategy::CompatibilityWrapper,
            reason: reason.to_string(),
            confidence: 0.9,
            verification_status: MigrationOutcome::Compatible,
            human_review_required: false,
            generated_code: code,
        }
    }
}

impl Default for FallbackGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_generator() {
        let generator = FallbackGenerator::new();
        let fallback = generator.generate_stub(
            "calc::eval",
            "eval_expr",
            "expr: &str",
            "Result<i32, String>",
            "Dynamic evaluation unsupported in Rust",
            None,
        );

        assert_eq!(fallback.strategy, FallbackStrategy::TypedFailureStub);
        assert_eq!(fallback.verification_status, MigrationOutcome::Degraded);
        assert!(fallback.generated_code.contains("todo!"));

        let debt = fallback.to_migration_debt();
        assert!(debt.requires_human_review);
    }
}
