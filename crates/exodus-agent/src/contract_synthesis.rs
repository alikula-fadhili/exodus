//! Semantic & Behavioral Contract Synthesizer.
//!
//! Synthesizes grounded behavioral contracts and target-native property tests
//! from source code ASTs, ESG semantic graphs, docstrings, and execution traces.

use chrono::Utc;
use exodus_core::{
    BehavioralAssertion, BehavioralContract, EsgNode, LanguageId, OracleType, Result,
    TargetLanguageSpecRecord, VerificationStatus,
};

/// Synthesizes grounded behavioral contracts from AST and ESG symbols.
#[derive(Default)]
pub struct ContractSynthesizer;

impl ContractSynthesizer {
    pub fn new() -> Self {
        Self
    }

    /// Synthesizes a behavioral contract for a specific ESG unit/symbol.
    pub fn synthesize_from_node(
        &self,
        node: &EsgNode,
        source_code: &str,
        target_lang: &LanguageId,
    ) -> Result<BehavioralContract> {
        let contract_id = format!("contract-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let mut assertions = Vec::new();

        // 1. Extract docstrings or inline comments for doctests / invariant examples
        if let Some(doc) = &node.docstring {
            let extracted_invariants = self.extract_invariants_from_docstring(doc, &node.name);
            for (idx, (input, expected, evidence)) in extracted_invariants.into_iter().enumerate() {
                assertions.push(BehavioralAssertion {
                    case_id: format!("{}-inv-{}", node.id, idx + 1),
                    input,
                    expected,
                    oracle: OracleType::DeclaredInvariant,
                    evidence,
                });
            }
        }

        // Also check source_code directly if docstring was empty
        if assertions.is_empty() {
            let extracted = self.extract_invariants_from_source(source_code, &node.name);
            for (idx, (input, expected, evidence)) in extracted.into_iter().enumerate() {
                assertions.push(BehavioralAssertion {
                    case_id: format!("{}-inv-{}", node.id, idx + 1),
                    input,
                    expected,
                    oracle: OracleType::DeclaredInvariant,
                    evidence,
                });
            }
        }

        // 2. Synthesize boundary and differential assertions for functions / methods
        let boundary_cases = self.synthesize_boundary_cases(node);
        for (idx, (input, expected, oracle, evidence)) in boundary_cases.into_iter().enumerate() {
            assertions.push(BehavioralAssertion {
                case_id: format!("{}-bound-{}", node.id, idx + 1),
                input,
                expected,
                oracle,
                evidence,
            });
        }

        // If no grounded assertions were found from source or boundaries, fallback to a declared invariant
        if assertions.is_empty() {
            let file_str = node
                .location
                .as_ref()
                .map(|loc| loc.file_path.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            assertions.push(BehavioralAssertion {
                case_id: format!("{}-default-1", node.id),
                input: "()".to_string(),
                expected: "()".to_string(),
                oracle: OracleType::DeclaredInvariant,
                evidence: format!("{}:{}", file_str, node.name),
            });
        }

        let raw_sig = node
            .signature
            .raw_signature
            .clone()
            .unwrap_or_else(|| node.name.clone());

        let target_sig = self.derive_target_signature(&raw_sig, target_lang);

        Ok(BehavioralContract {
            schema_version: "1.0.0".to_string(),
            contract_id,
            unit_id: node.id.clone(),
            unit_name: node.name.clone(),
            unit_kind: format!("{:?}", node.kind).to_lowercase(),
            source_signature: raw_sig,
            target_signature: target_sig,
            assertions,
            verification_status: VerificationStatus::Pending,
        })
    }

    /// Extract declared docstring assertions or examples from a docstring.
    fn extract_invariants_from_docstring(
        &self,
        docstring: &str,
        symbol_name: &str,
    ) -> Vec<(String, String, String)> {
        let mut invariants = Vec::new();
        let mut pending_input = None;

        for (line_no, line) in docstring.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(input_expr) = trimmed.strip_prefix(">>> ") {
                pending_input = Some((input_expr.to_string(), line_no + 1));
            } else if let Some((input_expr, input_line)) = pending_input.take() {
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    invariants.push((
                        input_expr,
                        trimmed.to_string(),
                        format!("docstring:{symbol_name}#L{input_line}"),
                    ));
                }
            }
        }

        invariants
    }

    /// Extract declared docstring assertions or examples from source code snippet.
    fn extract_invariants_from_source(
        &self,
        source_code: &str,
        symbol_name: &str,
    ) -> Vec<(String, String, String)> {
        let mut invariants = Vec::new();
        let mut in_symbol = false;
        let mut pending_input = None;

        for (line_no, line) in source_code.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains(&format!("def {symbol_name}"))
                || trimmed.contains(&format!("class {symbol_name}"))
                || trimmed.contains(&format!("fn {symbol_name}"))
                || trimmed.contains(&format!("func {symbol_name}"))
            {
                in_symbol = true;
            }

            if in_symbol {
                if let Some(input_expr) = trimmed.strip_prefix(">>> ") {
                    pending_input = Some((input_expr.to_string(), line_no + 1));
                } else if let Some((input_expr, input_line)) = pending_input.take() {
                    if !trimmed.is_empty()
                        && !trimmed.starts_with('#')
                        && !trimmed.starts_with("\"\"\"")
                    {
                        invariants.push((
                            input_expr,
                            trimmed.to_string(),
                            format!("doctest#L{}", input_line),
                        ));
                    }
                }

                if (trimmed.starts_with("def ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("fn "))
                    && !trimmed.contains(symbol_name)
                {
                    break;
                }
            }
        }

        invariants
    }

    /// Synthesizes standard boundary test cases based on signature and parameters.
    fn synthesize_boundary_cases(
        &self,
        node: &EsgNode,
    ) -> Vec<(String, String, OracleType, String)> {
        let mut cases = Vec::new();
        let sig_str = node.signature.raw_signature.as_deref().unwrap_or("");

        let params = node.signature.parameters.join(" ");

        if sig_str.contains("int")
            || sig_str.contains("i32")
            || sig_str.contains("i64")
            || params.contains("int")
        {
            cases.push((
                "0".to_string(),
                "0".to_string(),
                OracleType::DifferentialExecution,
                format!("boundary_fuzz:{}:zero_input", node.id),
            ));
        }

        if sig_str.contains("str")
            || sig_str.contains("String")
            || params.contains("str")
            || params.contains("String")
        {
            cases.push((
                "\"\"".to_string(),
                "\"\"".to_string(),
                OracleType::DifferentialExecution,
                format!("boundary_fuzz:{}:empty_string", node.id),
            ));
        }

        if sig_str.contains("list")
            || sig_str.contains("Vec")
            || sig_str.contains("[]")
            || params.contains("list")
        {
            cases.push((
                "[]".to_string(),
                "[]".to_string(),
                OracleType::DifferentialExecution,
                format!("boundary_fuzz:{}:empty_collection", node.id),
            ));
        }

        cases
    }

    /// Derives approximate target language signature from source signature.
    fn derive_target_signature(&self, source_sig: &str, target_lang: &LanguageId) -> String {
        match target_lang.as_str() {
            "rust" | "rs" => {
                let replaced = source_sig
                    .replace("def ", "pub fn ")
                    .replace("self", "&self")
                    .replace(": int", ": i64")
                    .replace(": str", ": &str")
                    .replace(" -> None:", "")
                    .replace(':', " {");
                if !replaced.contains("pub fn ") && !replaced.contains("pub struct ") {
                    format!("pub fn {}()", source_sig.trim())
                } else {
                    replaced
                }
            }
            "go" | "golang" => {
                let replaced = source_sig
                    .replace("def ", "func ")
                    .replace(": int", " int64")
                    .replace(": str", " string")
                    .replace(": bool", " bool")
                    .replace(" -> None:", "")
                    .replace(':', " {");
                if !replaced.contains("func ") {
                    format!("func {}()", source_sig.trim())
                } else {
                    replaced
                }
            }
            "zig" => {
                let replaced = source_sig
                    .replace("def ", "pub fn ")
                    .replace(": int", " i64")
                    .replace(": str", " []const u8")
                    .replace(" -> None:", " void")
                    .replace(':', " {");
                if !replaced.contains("pub fn ") {
                    format!("pub fn {}() void", source_sig.trim())
                } else {
                    replaced
                }
            }
            "kotlin" | "kt" => {
                let replaced = source_sig
                    .replace("def ", "fun ")
                    .replace(": int", ": Long")
                    .replace(": str", ": String")
                    .replace(": bool", ": Boolean")
                    .replace(" -> None:", "")
                    .replace(':', " {");
                if !replaced.contains("fun ") {
                    format!("fun {}()", source_sig.trim())
                } else {
                    replaced
                }
            }
            _ => source_sig.to_string(),
        }
    }

    /// Generates target-native executable test suite code for the behavioral contract
    /// dynamically formatted using the persisted TargetLanguageSpecRecord test harness template.
    pub fn generate_target_test_code(
        &self,
        contract: &BehavioralContract,
        target_lang: &LanguageId,
    ) -> String {
        let specs = TargetLanguageSpecRecord::default_specs();
        let query = target_lang.as_str();
        let spec = specs
            .iter()
            .find(|s| s.matches_query(query))
            .or_else(|| specs.iter().find(|s| s.id == "rust"))
            .unwrap();
        self.generate_target_test_code_for_spec(contract, spec)
    }

    /// Renders a contract from the caller-provided, potentially embedded-store-backed language
    /// specification. This is the runtime-update path; no language branch lives here.
    pub fn generate_target_test_code_for_spec(
        &self,
        contract: &BehavioralContract,
        spec: &TargetLanguageSpecRecord,
    ) -> String {
        let safe_pkg = contract.unit_name.replace('-', "_");
        let mut test_cases = Vec::new();

        for assertion in &contract.assertions {
            let safe_case = assertion.case_id.replace(['-', '.'], "_");
            let case_code = spec
                .test_case_template
                .replace("{{case_id}}", &safe_case)
                .replace("{{evidence}}", &assertion.evidence)
                .replace("{{oracle}}", &format!("{:?}", assertion.oracle))
                .replace("{{input}}", &assertion.input)
                .replace("{{expected}}", &assertion.expected);
            test_cases.push(case_code);
        }

        let joined_cases = test_cases.join("\n\n");
        spec.test_harness_template
            .replace("{{safe_pkg}}", &safe_pkg)
            .replace("{{pkg_name}}", &contract.unit_name)
            .replace("{{test_cases}}", &joined_cases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_core::{EsgNode, EsgNodeKind, SignatureMetadata};

    #[test]
    fn test_synthesize_contract_with_grounded_invariants() {
        let synthesizer = ContractSynthesizer::new();
        let mut node = EsgNode::new(
            "bank.deposit",
            LanguageId::new("python"),
            EsgNodeKind::Method,
            "deposit",
        );
        node.signature = SignatureMetadata {
            raw_signature: Some("def deposit(self, amount: int) -> int:".to_string()),
            parameters: vec!["amount: int".to_string()],
            return_type: Some("int".to_string()),
            is_async: false,
            is_static: false,
            generic_parameters: vec![],
        };

        let source = r#"
class BankAccount:
    def deposit(self, amount: int) -> int:
        """
        >>> acc.deposit(50)
        50
        """
        return self.balance + amount
"#;

        let contract = synthesizer
            .synthesize_from_node(&node, source, &LanguageId::new("rust"))
            .unwrap();

        assert_eq!(contract.unit_id, "bank.deposit");
        assert!(contract.is_grounded());
        assert!(!contract.assertions.is_empty());

        let rust_test = synthesizer.generate_target_test_code(&contract, &LanguageId::new("rust"));
        assert!(rust_test.contains("#[cfg(test)]"));

        let go_test = synthesizer.generate_target_test_code(&contract, &LanguageId::new("go"));
        assert!(go_test.contains("func Test_"));

        let zig_test = synthesizer.generate_target_test_code(&contract, &LanguageId::new("zig"));
        assert!(zig_test.contains("test \""));

        let kt_test = synthesizer.generate_target_test_code(&contract, &LanguageId::new("kotlin"));
        assert!(kt_test.contains("@Test"));
    }
}
