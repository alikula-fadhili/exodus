//! Deterministic Python-to-Rust transformation engine for Project Exodus.

use exodus_core::{Diagnostic, MigrationOutcome, Result};
use exodus_fallback::{FallbackGenerator, FallbackRecord, FallbackStrategy};
use exodus_parser::{ClassDef, FunctionDef, ParsedModule, ParsedRepository};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Transformation configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformOptions {
    pub target_rust_edition: String,
    pub derive_common_traits: bool,
    pub generate_stubs_for_unsupported: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            target_rust_edition: "2021".to_string(),
            derive_common_traits: true,
            generate_stubs_for_unsupported: true,
        }
    }
}

/// Request to transform a parsed module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformRequest {
    pub parsed_module: ParsedModule,
    pub options: TransformOptions,
}

/// Result of transforming a module to Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub rust_source: String,
    pub module_name: String,
    pub file_path: PathBuf,
    pub outcome: MigrationOutcome,
    pub fallbacks: Vec<FallbackRecord>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Core deterministic transformation engine.
pub struct TransformationEngine {
    fallback_gen: FallbackGenerator,
}

impl TransformationEngine {
    pub fn new() -> Self {
        Self {
            fallback_gen: FallbackGenerator::new(),
        }
    }

    /// Maps Python type annotations to idiomatic Rust types.
    pub fn map_type(py_type: &str) -> String {
        let trimmed = py_type.trim();
        if trimmed.is_empty() {
            return "()".to_string();
        }

        if trimmed == "int" {
            return "i64".to_string();
        } else if trimmed == "float" {
            return "f64".to_string();
        } else if trimmed == "str" {
            return "String".to_string();
        } else if trimmed == "bool" {
            return "bool".to_string();
        } else if trimmed == "bytes" {
            return "Vec<u8>".to_string();
        } else if trimmed == "None" || trimmed == "NoneType" {
            return "()".to_string();
        } else if trimmed == "Any" {
            return "serde_json::Value".to_string();
        } else if trimmed == "dict" || trimmed == "Dict" {
            return "std::collections::HashMap<String, serde_json::Value>".to_string();
        } else if trimmed == "list" || trimmed == "List" {
            return "Vec<serde_json::Value>".to_string();
        } else if trimmed == "set" || trimmed == "Set" {
            return "std::collections::HashSet<String>".to_string();
        }

        // Handle Optional[T]
        if let Some(inner) = trimmed
            .strip_prefix("Optional[")
            .and_then(|s| s.strip_suffix(']'))
        {
            return format!("Option<{}>", Self::map_type(inner));
        }

        // Handle List[T] / list[T]
        if let Some(inner) = trimmed
            .strip_prefix("List[")
            .or_else(|| trimmed.strip_prefix("list["))
            .and_then(|s| s.strip_suffix(']'))
        {
            return format!("Vec<{}>", Self::map_type(inner));
        }

        // Handle Dict[K, V] / dict[K, V]
        if let Some(inner) = trimmed
            .strip_prefix("Dict[")
            .or_else(|| trimmed.strip_prefix("dict["))
            .and_then(|s| s.strip_suffix(']'))
        {
            let parts: Vec<&str> = inner.splitn(2, ',').collect();
            if parts.len() == 2 {
                return format!(
                    "std::collections::HashMap<{}, {}>",
                    Self::map_type(parts[0]),
                    Self::map_type(parts[1])
                );
            }
        }

        // Handle Set[T] / set[T]
        if let Some(inner) = trimmed
            .strip_prefix("Set[")
            .or_else(|| trimmed.strip_prefix("set["))
            .and_then(|s| s.strip_suffix(']'))
        {
            return format!("std::collections::HashSet<{}>", Self::map_type(inner));
        }

        // Custom class / identifier
        trimmed.to_string()
    }

    /// Infers receiver mutability (`&mut self` vs `&self`) by scanning for field mutation.
    pub fn infer_receiver_mutability(body_snippet: &str) -> &'static str {
        let set_field_re = Regex::new(r#"(?m)^\s*self\.[a-zA-Z0-9_]+\s*=[^=]"#).unwrap();
        let mutate_call_re = Regex::new(
            r#"self\.[a-zA-Z0-9_]+\.(push|insert|clear|pop|remove|retain|extend|append)\("#,
        )
        .unwrap();
        let mutate_method_re =
            Regex::new(r#"self\.(set_|update_|add_|clear_|reset_|increment_)"#).unwrap();

        if set_field_re.is_match(body_snippet)
            || mutate_call_re.is_match(body_snippet)
            || mutate_method_re.is_match(body_snippet)
        {
            "&mut self"
        } else {
            "&self"
        }
    }

    /// Indentation-aware Python statement transformer.
    fn transform_body_code(body: &str, ret_type: &str, _is_async: bool) -> String {
        let mut rust_lines = Vec::new();
        let lines: Vec<&str> = body.lines().collect();

        let print_re = Regex::new(r#"print\((.*)\)"#).unwrap();
        let len_re = Regex::new(r#"len\(([^)]+)\)"#).unwrap();
        let range_re = Regex::new(r#"range\(([^,]+),\s*([^)]+)\)"#).unwrap();
        let range_one_re = Regex::new(r#"range\(([^)]+)\)"#).unwrap();
        let append_re = Regex::new(r#"([a-zA-Z0-9_]+)\.append\(([^)]+)\)"#).unwrap();
        let fstring_re = Regex::new(r#"f"([^"]*)""#).unwrap();
        let placeholder_re = Regex::new(r"\{([^{}]+)\}").unwrap();
        let in_re = Regex::new(r#"^if\s+(.+)\s+in\s+([a-zA-Z0-9_]+):"#).unwrap();
        let call_arg_re = Regex::new(r#"([a-zA-Z0-9_]+)\(([a-zA-Z0-9_]+)\)"#).unwrap();

        let mut declared_vars = HashSet::new();
        let mut indent_stack: Vec<usize> = Vec::new();
        let mut has_unconditional_return = false;

        let base_indent = if lines.len() > 1 {
            lines[1..]
                .iter()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                .min()
                .unwrap_or(0)
        } else {
            0
        };

        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || (trimmed.starts_with("\"\"\"") && trimmed.ends_with("\"\"\""))
            {
                continue;
            }

            let raw_indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let current_indent = if idx == 0 {
                0
            } else {
                raw_indent.saturating_sub(base_indent)
            };

            // Close blocks if indent decreased
            while let Some(&top_indent) = indent_stack.last() {
                if current_indent < top_indent {
                    indent_stack.pop();
                    let close_indent = " ".repeat(indent_stack.len() * 4 + 4);
                    rust_lines.push(format!("{close_indent}}}"));
                } else {
                    break;
                }
            }

            let mut trans = trimmed.to_string();

            // Python f-strings (`f"...{expr}..."`)
            trans = fstring_re
                .replace_all(&trans, |caps: &regex::Captures| {
                    let inner = &caps[1];
                    let mut exprs = Vec::new();
                    let literal = placeholder_re.replace_all(inner, |pc: &regex::Captures| {
                        exprs.push(pc[1].to_string());
                        "{}"
                    });
                    if exprs.is_empty() {
                        format!("\"{literal}\".to_string()")
                    } else {
                        format!("format!(\"{literal}\", {})", exprs.join(", "))
                    }
                })
                .to_string();

            // Dynamic eval reflection fallback
            if trans.contains("eval(") {
                let effective_indent = " ".repeat(indent_stack.len() * 4 + 4);
                rust_lines.push(format!(
                    "{effective_indent}todo!(\"Exodus Migration Debt: DynamicReflectionUnsupported - eval reflection cannot be translated statically\");"
                ));
                has_unconditional_return = true;
                continue;
            }

            // Boolean literals & None
            trans = trans.replace("True", "true");
            trans = trans.replace("False", "false");
            trans = trans.replace(" and ", " && ");
            trans = trans.replace(" or ", " || ");
            trans = trans.replace("not ", "!");

            // Python builtins to Rust
            trans = print_re
                .replace_all(&trans, "println!(\"{}\", $1)")
                .to_string();
            trans = len_re.replace_all(&trans, "$1.len()").to_string();
            trans = range_re.replace_all(&trans, "$1..$2").to_string();
            trans = range_one_re.replace_all(&trans, "0..$1").to_string();
            trans = append_re.replace_all(&trans, "$1.push($2)").to_string();

            // Handle collection membership in conditionals: `if "x" in payload:` -> `if payload.contains_key("x"):`
            if let Some(caps) = in_re.captures(&trans) {
                let key = caps[1].trim();
                let coll = caps[2].trim();
                trans = format!("if {coll}.contains_key({key}):");
            }

            // Function call argument cloning for non-copy identifier passing (avoids use-after-move)
            if trans.contains('(')
                && trans.contains(')')
                && !trans.starts_with("if ")
                && !trans.starts_with("for ")
            {
                trans = call_arg_re
                    .replace_all(&trans, |caps: &regex::Captures| {
                        let fn_name = &caps[1];
                        let arg_name = &caps[2];
                        if fn_name != "println"
                            && fn_name != "print"
                            && fn_name != "len"
                            && fn_name != "range"
                            && fn_name != "vec"
                            && fn_name != "Some"
                            && fn_name != "Ok"
                            && fn_name != "Err"
                            && arg_name != "self"
                            && !arg_name.chars().all(|c| c.is_ascii_digit())
                        {
                            format!("{fn_name}({arg_name}.clone())")
                        } else {
                            format!("{fn_name}({arg_name})")
                        }
                    })
                    .to_string();
            }

            // List literal: result = [] -> let mut result = Vec::new()
            if trans.contains(" = []") {
                trans = trans.replace(" = []", " = Vec::new()");
                if !trans.starts_with("let ") && !trans.starts_with("self.") {
                    let var_name = trans
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_start_matches("let mut ")
                        .to_string();
                    if !declared_vars.contains(&var_name) {
                        declared_vars.insert(var_name);
                        trans = format!("let mut {trans}");
                    }
                }
            } else if trans.contains(" = ")
                && !trans.starts_with("let ")
                && !trans.starts_with("self.")
                && !trans.starts_with("if ")
            {
                let var_name = trans.split_whitespace().next().unwrap_or("").to_string();
                if !declared_vars.contains(&var_name) {
                    declared_vars.insert(var_name.clone());
                    trans = format!("let mut {trans}");
                }
            }

            let effective_indent = " ".repeat(indent_stack.len() * 4 + 4);

            if trans.starts_with("elif ") {
                if let Some(top_indent) = indent_stack.pop() {
                    let close_indent = " ".repeat(indent_stack.len() * 4 + 4);
                    let cond = trans.strip_prefix("elif ").unwrap().trim_end_matches(':');
                    rust_lines.push(format!("{close_indent}}} else if {cond} {{"));
                    indent_stack.push(top_indent);
                }
            } else if trans == "else:" {
                if let Some(top_indent) = indent_stack.pop() {
                    let close_indent = " ".repeat(indent_stack.len() * 4 + 4);
                    rust_lines.push(format!("{close_indent}}} else {{"));
                    indent_stack.push(top_indent);
                }
            } else if trans.starts_with("if ") {
                let cond = trans.strip_prefix("if ").unwrap().trim_end_matches(':');
                rust_lines.push(format!("{effective_indent}if {cond} {{"));
                indent_stack.push(current_indent + 4);
            } else if trans.starts_with("for ") && trans.contains(" in ") {
                let stripped = trans.strip_prefix("for ").unwrap().trim_end_matches(':');
                let parts: Vec<&str> = stripped.split(" in ").collect();
                if parts.len() == 2 {
                    let loop_var = parts[0].trim();
                    let iter_expr = parts[1].trim();
                    let target_var = if declared_vars.contains(loop_var) {
                        format!("{loop_var}_item")
                    } else {
                        loop_var.to_string()
                    };
                    rust_lines.push(format!(
                        "{effective_indent}for {target_var} in {iter_expr} {{"
                    ));
                } else {
                    rust_lines.push(format!("{effective_indent}for {stripped} {{"));
                }
                indent_stack.push(current_indent + 4);
            } else if trans.starts_with("while ") {
                let cond = trans.strip_prefix("while ").unwrap().trim_end_matches(':');
                rust_lines.push(format!("{effective_indent}while {cond} {{"));
                indent_stack.push(current_indent + 4);
            } else if trans.starts_with("return ") {
                let mut expr = trans
                    .strip_prefix("return ")
                    .unwrap()
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                if expr.starts_with('[') && expr.ends_with(']') {
                    expr = format!("vec!{expr}");
                } else if ret_type == "String" {
                    if expr.starts_with('"') && expr.ends_with('"') {
                        expr = format!("{expr}.to_string()");
                    } else if expr.starts_with("self.") {
                        expr = format!("{expr}.clone()");
                    }
                } else if expr.starts_with("self.")
                    && (ret_type.starts_with("Vec<")
                        || ret_type.starts_with("HashMap<")
                        || ret_type.starts_with("HashSet<")
                        || ret_type == "serde_json::Value")
                {
                    expr = format!("{expr}.clone()");
                }
                rust_lines.push(format!("{effective_indent}return {expr};"));
                if indent_stack.is_empty() {
                    has_unconditional_return = true;
                }
            } else if trans == "pass" {
                rust_lines.push(format!("{effective_indent}// pass"));
            } else {
                let stmt = trans.trim_end_matches(';');
                rust_lines.push(format!("{effective_indent}{stmt};"));
            }
        }

        // Close remaining open blocks
        while indent_stack.pop().is_some() {
            let close_indent = " ".repeat(indent_stack.len() * 4 + 4);
            rust_lines.push(format!("{close_indent}}}"));
        }

        // If function returns non-() and has no unconditional top-level return, synthesize default return
        if ret_type != "()" && !has_unconditional_return {
            if let Some(last) = rust_lines.last() {
                if !last.trim_start().starts_with("return ")
                    && !last.trim_start().starts_with("todo!")
                {
                    rust_lines.push("    Default::default()".to_string());
                }
            }
        }

        if rust_lines.is_empty() {
            "    // Default implementation\n".to_string()
        } else {
            let joined = rust_lines.join("\n");
            format!("{joined}\n")
        }
    }

    /// Transforms a single FunctionDef to Rust code.
    fn transform_function(&self, func: &FunctionDef) -> (String, Option<FallbackRecord>) {
        let is_async = func.is_async;
        let async_prefix = if is_async { "async " } else { "" };
        let ret_type = func
            .return_type
            .as_deref()
            .map(Self::map_type)
            .unwrap_or_else(|| "()".to_string());

        let mut param_strs = Vec::new();
        if func.is_method {
            let receiver = Self::infer_receiver_mutability(&func.body_snippet);
            param_strs.push(receiver.to_string());
        }

        for p in &func.parameters {
            let p_type = p
                .type_annotation
                .as_deref()
                .map(Self::map_type)
                .unwrap_or_else(|| "serde_json::Value".to_string());
            param_strs.push(format!("{}: {}", p.name, p_type));
        }

        let params_joined = param_strs.join(", ");
        let ret_clause = if ret_type == "()" {
            String::new()
        } else {
            format!(" -> {ret_type}")
        };

        let mut doc = String::new();
        if let Some(d) = &func.docstring {
            doc.push_str(&format!("/// {}\n", d.replace('\n', "\n/// ")));
        }

        let body_code = Self::transform_body_code(&func.body_snippet, &ret_type, is_async);

        let fb =
            if body_code.contains("todo!(") || body_code.contains("DynamicReflectionUnsupported") {
                Some(FallbackRecord {
                    symbol_id: format!("function::{}", func.name),
                    source_location: Some(func.evidence.span.clone()),
                    construct: "dynamic_eval_reflection".to_string(),
                    strategy: FallbackStrategy::TypedFailureStub,
                    reason: "Dynamic eval reflection cannot be translated statically".to_string(),
                    confidence: 0.0,
                    verification_status: MigrationOutcome::Degraded,
                    human_review_required: true,
                    generated_code: body_code.clone(),
                })
            } else {
                None
            };

        let code = format!(
            "{doc}pub {async_prefix}fn {}({}){} {{\n{}}}\n",
            func.name, params_joined, ret_clause, body_code
        );

        (code, fb)
    }

    /// Transforms a ClassDef to a Rust struct and impl block.
    fn transform_class(&self, class: &ClassDef) -> (String, Vec<FallbackRecord>) {
        let mut out = String::new();
        let mut fallbacks = Vec::new();

        if let Some(doc) = &class.docstring {
            out.push_str(&format!("/// {}\n", doc.replace('\n', "\n/// ")));
        }

        out.push_str(
            "#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]\n",
        );
        out.push_str(&format!("pub struct {} {{\n", class.name));

        // Fields
        for field in &class.fields {
            out.push_str(&format!("    pub {}: serde_json::Value,\n", field));
        }
        if class.fields.is_empty() {
            let mut init_fields: Vec<(String, String)> = Vec::new();
            if let Some(init_m) = class.methods.iter().find(|m| m.name == "__init__") {
                for p in &init_m.parameters {
                    let p_type = p
                        .type_annotation
                        .as_deref()
                        .map(Self::map_type)
                        .unwrap_or_else(|| "serde_json::Value".to_string());
                    if !init_fields.iter().any(|(n, _)| n == &p.name) {
                        init_fields.push((p.name.clone(), p_type));
                    }
                }
            }
            if init_fields.is_empty() {
                out.push_str("    // No explicit fields\n");
            } else {
                for (fname, ftype) in &init_fields {
                    out.push_str(&format!("    pub {}: {},\n", fname, ftype));
                }
            }
        }
        out.push_str("}\n\n");

        // Impl block
        out.push_str(&format!("impl {} {{\n", class.name));

        for method in &class.methods {
            if method.name == "__init__" {
                let mut params = Vec::new();
                let mut field_inits = Vec::new();
                for p in &method.parameters {
                    let p_type = p
                        .type_annotation
                        .as_deref()
                        .map(Self::map_type)
                        .unwrap_or_else(|| "serde_json::Value".to_string());
                    params.push(format!("{}: {}", p.name, p_type));
                    field_inits.push(format!("            {}: {},", p.name, p.name));
                }
                out.push_str(&format!(
                    "    pub fn new({}) -> Self {{\n        Self {{\n{}\n        }}\n    }}\n\n",
                    params.join(", "),
                    field_inits.join("\n")
                ));
            } else {
                let (m_code, fb) = self.transform_function(method);
                if let Some(fb) = fb {
                    fallbacks.push(fb);
                }
                let indented = m_code
                    .lines()
                    .map(|l| format!("    {l}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                out.push_str(&format!("{indented}\n\n"));
            }
        }

        out.push_str("}\n");

        (out, fallbacks)
    }

    /// Transforms a parsed module to Rust source code.
    pub fn transform_module(&self, request: &TransformRequest) -> Result<TransformResult> {
        let module = &request.parsed_module;
        let mut rust_source = String::new();
        let mut fallbacks = Vec::new();
        let diagnostics = module.diagnostics.clone();

        rust_source.push_str(&format!(
            "//! Transformed Rust module `{}` generated by Project Exodus.\n\n",
            module.module_name
        ));

        // Common imports
        rust_source
            .push_str("#[allow(unused_imports)]\nuse std::collections::{HashMap, HashSet};\n");
        rust_source.push_str("#[allow(unused_imports)]\nuse serde::{Serialize, Deserialize};\n");
        rust_source.push_str("#[allow(unused_imports)]\nuse crate::*;\n\n");

        // Transform unsupported constructs into explicit fallback stubs
        for unsupp in &module.unsupported_constructs {
            let safe_kind = unsupp
                .construct_kind
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let fb = self.fallback_gen.generate_stub(
                &format!("{}::{}", module.module_name, safe_kind),
                &format!("{safe_kind}_fallback"),
                "_args: serde_json::Value",
                "Result<(), String>",
                &unsupp.reason,
                Some(unsupp.evidence.span.clone()),
            );
            rust_source.push_str(&fb.generated_code);
            rust_source.push('\n');
            fallbacks.push(fb);
        }

        // Transform classes
        for class in &module.classes {
            let (cls_code, cls_fallbacks) = self.transform_class(class);
            rust_source.push_str(&cls_code);
            rust_source.push('\n');
            fallbacks.extend(cls_fallbacks);
        }

        // Transform top-level functions
        for func in &module.functions {
            let (fn_code, fn_fallback) = self.transform_function(func);
            rust_source.push_str(&fn_code);
            rust_source.push('\n');
            if let Some(fb) = fn_fallback {
                fallbacks.push(fb);
            }
        }

        let outcome = if !fallbacks.is_empty() {
            if fallbacks
                .iter()
                .any(|f| f.strategy == FallbackStrategy::TypedFailureStub)
            {
                MigrationOutcome::Degraded
            } else {
                MigrationOutcome::Compatible
            }
        } else {
            MigrationOutcome::Verified
        };

        Ok(TransformResult {
            rust_source,
            module_name: module.module_name.clone(),
            file_path: module.file_path.clone(),
            outcome,
            fallbacks,
            diagnostics,
        })
    }

    /// Transforms an entire parsed repository.
    pub fn transform_repository(
        &self,
        parsed_repo: &ParsedRepository,
    ) -> Result<Vec<TransformResult>> {
        let mut results = Vec::new();
        let options = TransformOptions::default();

        for module in &parsed_repo.modules {
            let req = TransformRequest {
                parsed_module: module.clone(),
                options: options.clone(),
            };
            results.push(self.transform_module(&req)?);
        }

        Ok(results)
    }
}

impl Default for TransformationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_parser::{PythonParser, SourceParser};
    use std::path::Path;

    #[test]
    fn test_map_types() {
        assert_eq!(TransformationEngine::map_type("int"), "i64");
        assert_eq!(TransformationEngine::map_type("str"), "String");
        assert_eq!(
            TransformationEngine::map_type("Optional[int]"),
            "Option<i64>"
        );
        assert_eq!(TransformationEngine::map_type("List[str]"), "Vec<String>");
        assert_eq!(
            TransformationEngine::map_type("Dict[str, int]"),
            "std::collections::HashMap<String, i64>"
        );
        assert_eq!(
            TransformationEngine::map_type("dict"),
            "std::collections::HashMap<String, serde_json::Value>"
        );
    }

    #[test]
    fn test_receiver_mutability_inference() {
        let read_only = "return self.count";
        assert_eq!(
            TransformationEngine::infer_receiver_mutability(read_only),
            "&self"
        );

        let mutating = "self.count = self.count + 1\nreturn self.count";
        assert_eq!(
            TransformationEngine::infer_receiver_mutability(mutating),
            "&mut self"
        );

        let push_mutating = "self.items.push(item)";
        assert_eq!(
            TransformationEngine::infer_receiver_mutability(push_mutating),
            "&mut self"
        );
    }

    #[test]
    fn test_fstring_translation() {
        let py_code = r#"
def summarize(user_id: str, orders: list) -> str:
    return f"User {user_id} has {len(orders)} orders"

def greet() -> str:
    return f"hello world"
"#;
        let parser = PythonParser::new();
        let parsed = parser.parse_source(Path::new("m.py"), py_code).unwrap();
        let engine = TransformationEngine::new();
        let result = engine
            .transform_module(&TransformRequest {
                parsed_module: parsed,
                options: TransformOptions::default(),
            })
            .unwrap();

        assert!(
            result
                .rust_source
                .contains(r#"format!("User {} has {} orders", user_id, orders.len())"#),
            "actual source:\n{}",
            result.rust_source
        );
        assert!(
            !result.rust_source.contains("f\""),
            "no raw f-string syntax should remain"
        );
        assert!(result.rust_source.contains(r#""hello world".to_string()"#));
    }

    #[test]
    fn test_transform_module_clean() {
        let py_code = r#"
def add(a: int, b: int) -> int:
    return a + b

class Counter:
    def __init__(self, start: int):
        self.count = start

    def get_count(self) -> int:
        return self.count
"#;
        let parser = PythonParser::new();
        let parsed = parser
            .parse_source(Path::new("counter.py"), py_code)
            .unwrap();

        let engine = TransformationEngine::new();
        let req = TransformRequest {
            parsed_module: parsed,
            options: TransformOptions::default(),
        };

        let result = engine.transform_module(&req).unwrap();
        assert!(result
            .rust_source
            .contains("pub fn add(a: i64, b: i64) -> i64"));
        assert!(result.rust_source.contains("pub struct Counter"));
        assert!(result
            .rust_source
            .contains("pub fn new(start: i64) -> Self"));
        assert!(result
            .rust_source
            .contains("pub fn get_count(&self) -> i64"));
        assert_eq!(result.outcome, MigrationOutcome::Verified);
    }

    #[test]
    fn test_transform_bank_account() {
        let py_code = r#"
class BankAccount:
    """A simple bank account."""
    def __init__(self, owner: str, balance: int = 0):
        self.owner = owner
        self.balance = balance

    def deposit(self, amount: int) -> int:
        self.balance = self.balance + amount
        return self.balance

    def withdraw(self, amount: int) -> int:
        if self.balance >= amount:
            self.balance = self.balance - amount
            return self.balance
        return -1
"#;
        let parser = PythonParser::new();
        let parsed = parser
            .parse_source(Path::new("bank_account.py"), py_code)
            .unwrap();

        let engine = TransformationEngine::new();
        let req = TransformRequest {
            parsed_module: parsed,
            options: TransformOptions::default(),
        };

        let result = engine.transform_module(&req).unwrap();
        assert!(result.rust_source.contains("pub struct BankAccount"));
        assert!(result
            .rust_source
            .contains("pub fn deposit(&mut self, amount: i64) -> i64"));
        assert!(result
            .rust_source
            .contains("pub fn withdraw(&mut self, amount: i64) -> i64"));
        assert!(result.rust_source.contains("if self.balance >= amount {"));
        assert!(result.rust_source.contains("return -1;"));
        assert_eq!(result.outcome, MigrationOutcome::Verified);
    }
}
