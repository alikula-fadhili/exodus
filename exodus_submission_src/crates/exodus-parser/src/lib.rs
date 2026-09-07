//! Tree-sitter Python parser and AST symbol extraction for Project Exodus.

use exodus_core::{
    Diagnostic, ExodusError, LanguageId, Result, SourceEvidence, SourceLocation, SourceSpan,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser, Tree};

pub mod universal;
pub use universal::*;
pub mod tree_sitter_adapter;
pub use tree_sitter_adapter::*;

/// Registers the three Phase 1 Tree-sitter adapters through one generic implementation.
pub async fn register_builtin_source_adapters(
    registry: &mut exodus_core::LanguageAdapterRegistry,
) -> exodus_core::Result<()> {
    registry
        .register_source_adapter(std::sync::Arc::new(PythonSourceAdapter::new()))
        .await?;
    registry
        .register_source_adapter(std::sync::Arc::new(TypeScriptSourceAdapter::new()))
        .await?;
    registry
        .register_source_adapter(std::sync::Arc::new(GoSourceAdapter::new()))
        .await?;
    Ok(())
}

/// Extracted function or method parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
    pub evidence: SourceEvidence,
}

/// Extracted function or method definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub qualified_name: String,
    pub is_async: bool,
    pub is_method: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub decorators: Vec<String>,
    pub docstring: Option<String>,
    pub body_snippet: String,
    pub evidence: SourceEvidence,
}

/// Extracted class definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassDef {
    pub name: String,
    pub qualified_name: String,
    pub base_classes: Vec<String>,
    pub decorators: Vec<String>,
    pub docstring: Option<String>,
    pub methods: Vec<FunctionDef>,
    pub fields: Vec<String>,
    pub evidence: SourceEvidence,
}

/// Extracted import statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportStatement {
    pub module: String,
    pub imported_names: Vec<(String, Option<String>)>, // (name, alias)
    pub is_from_import: bool,
    pub evidence: SourceEvidence,
}

/// Extracted function/method call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: String,
    pub caller_scope: String,
    pub arguments: Vec<String>,
    pub evidence: SourceEvidence,
}

/// Extracted assignment statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub target: String,
    pub value_snippet: String,
    pub type_annotation: Option<String>,
    pub scope: String,
    pub evidence: SourceEvidence,
}

/// Detected unsupported dynamic or complex construct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedConstruct {
    pub construct_kind: String,
    pub reason: String,
    pub snippet: String,
    pub scope: String,
    pub evidence: SourceEvidence,
}

/// Complete parsed representation of a single Python source module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModule {
    pub file_path: PathBuf,
    pub module_name: String,
    pub imports: Vec<ImportStatement>,
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub calls: Vec<CallExpr>,
    pub assignments: Vec<Assignment>,
    pub unsupported_constructs: Vec<UnsupportedConstruct>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Detected HTTP/gRPC route endpoint in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRouteEvidence {
    pub method: String,
    pub path: String,
    pub handler_name: String,
    pub file_path: PathBuf,
}

/// Detected database access or query usage in the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseUsageEvidence {
    pub driver_or_orm: String,
    pub has_raw_sql: bool,
    pub queries_detected: Vec<String>,
    pub file_path: PathBuf,
}

/// Deep Research Report produced during Stage 1 & 2 analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeepResearchReport {
    pub total_modules: usize,
    pub total_functions: usize,
    pub total_classes: usize,
    pub detected_frameworks: Vec<String>,
    pub detected_routes: Vec<HttpRouteEvidence>,
    pub detected_databases: Vec<DatabaseUsageEvidence>,
    pub detected_sdks: Vec<String>,
    pub inferred_archetype: String,
}

impl DeepResearchReport {
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "🔬 Deep Codebase Analysis & Research Findings:\n   • Modules: {} | Functions: {} | Classes: {}\n   • Inferred Archetype: {}\n",
            self.total_modules, self.total_functions, self.total_classes, self.inferred_archetype
        );
        if !self.detected_frameworks.is_empty() {
            out.push_str(&format!(
                "   • Frameworks: {}\n",
                self.detected_frameworks.join(", ")
            ));
        }
        if !self.detected_routes.is_empty() {
            out.push_str(&format!(
                "   • HTTP Routes ({} detected):\n",
                self.detected_routes.len()
            ));
            for r in self.detected_routes.iter().take(4) {
                out.push_str(&format!(
                    "      - [{}] {} -> `{}`\n",
                    r.method, r.path, r.handler_name
                ));
            }
        }
        if !self.detected_databases.is_empty() {
            out.push_str(&format!(
                "   • Database Drivers: {}\n",
                self.detected_databases
                    .iter()
                    .map(|d| d.driver_or_orm.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.detected_sdks.is_empty() {
            out.push_str(&format!(
                "   • External Cloud SDKs: {}\n",
                self.detected_sdks.join(", ")
            ));
        }
        out
    }
}

/// Complete parsed representation of a repository.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedRepository {
    pub root_path: PathBuf,
    pub modules: Vec<ParsedModule>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParsedRepository {
    /// Perform deep AST, framework, route, and database dependency research.
    pub fn perform_deep_research(&self) -> DeepResearchReport {
        let mut total_fns = 0;
        let mut total_cls = 0;
        let mut frameworks = Vec::new();
        let mut routes = Vec::new();
        let mut dbs = Vec::new();
        let mut sdks = Vec::new();

        for m in &self.modules {
            total_fns += m.functions.len();
            total_cls += m.classes.len();

            // 1. Scan imports for frameworks and cloud SDKs
            for imp in &m.imports {
                let mod_name = imp.module.to_lowercase();
                if (mod_name.contains("fastapi")
                    || mod_name.contains("flask")
                    || mod_name.contains("django")
                    || mod_name.contains("axum")
                    || mod_name.contains("express"))
                    && !frameworks.contains(&imp.module)
                {
                    frameworks.push(imp.module.clone());
                }
                if mod_name.contains("sqlalchemy")
                    || mod_name.contains("sqlite3")
                    || mod_name.contains("psycopg")
                    || mod_name.contains("sqlx")
                    || mod_name.contains("diesel")
                    || mod_name.contains("pymongo")
                {
                    let db_name = imp.module.clone();
                    if !dbs
                        .iter()
                        .any(|d: &DatabaseUsageEvidence| d.driver_or_orm == db_name)
                    {
                        dbs.push(DatabaseUsageEvidence {
                            driver_or_orm: db_name,
                            has_raw_sql: false,
                            queries_detected: Vec::new(),
                            file_path: m.file_path.clone(),
                        });
                    }
                }
                if (mod_name.contains("boto3")
                    || mod_name.contains("redis")
                    || mod_name.contains("celery")
                    || mod_name.contains("requests")
                    || mod_name.contains("httpx"))
                    && !sdks.contains(&imp.module)
                {
                    sdks.push(imp.module.clone());
                }
            }

            // 2. Scan function decorators for routes
            for f in &m.functions {
                for dec in &f.decorators {
                    let d_lower = dec.to_lowercase();
                    if d_lower.contains(".get(")
                        || d_lower.contains(".post(")
                        || d_lower.contains(".put(")
                        || d_lower.contains(".delete(")
                        || d_lower.contains(".route(")
                    {
                        let method = if d_lower.contains(".get") {
                            "GET"
                        } else if d_lower.contains(".post") {
                            "POST"
                        } else if d_lower.contains(".put") {
                            "PUT"
                        } else if d_lower.contains(".delete") {
                            "DELETE"
                        } else {
                            "HTTP"
                        };
                        let path = dec
                            .split('(')
                            .nth(1)
                            .and_then(|s| s.split(')').next())
                            .unwrap_or("/")
                            .trim_matches('\"')
                            .trim_matches('\'')
                            .to_string();
                        routes.push(HttpRouteEvidence {
                            method: method.to_string(),
                            path,
                            handler_name: f.name.clone(),
                            file_path: m.file_path.clone(),
                        });
                    }
                }
            }
        }

        let inferred_archetype = if !routes.is_empty()
            || frameworks
                .iter()
                .any(|f| f.to_lowercase().contains("fastapi") || f.to_lowercase().contains("flask"))
        {
            "BackendService".to_string()
        } else if sdks.iter().any(|s| s.to_lowercase().contains("celery")) {
            "WorkerQueue".to_string()
        } else {
            "SharedLibrary".to_string()
        };

        DeepResearchReport {
            total_modules: self.modules.len(),
            total_functions: total_fns,
            total_classes: total_cls,
            detected_frameworks: frameworks,
            detected_routes: routes,
            detected_databases: dbs,
            detected_sdks: sdks,
            inferred_archetype,
        }
    }
}

/// Interface for source language parsers.
pub trait SourceParser {
    fn language(&self) -> LanguageId;
    fn parse_source(&self, file_path: &Path, source_code: &str) -> Result<ParsedModule>;
    fn parse_repository(&self, repo_path: &Path) -> Result<ParsedRepository>;
}

/// Python source parser powered by Tree-sitter.
pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }

    /// True when a `function_definition` node is an `async def`. This tree-sitter-python grammar
    /// version represents `async def foo():` as an ordinary `function_definition` node with an
    /// `async` keyword *child* — there is no distinct `async_function_definition` node kind (the
    /// grammar's node-kinds.json for older/other grammar versions did define one, which is why
    /// that name still appears in match arms elsewhere in this file; checking for it via
    /// `node.kind()` alone never matches against the currently vendored grammar and silently
    /// classified every async function as synchronous).
    fn node_is_async(node: Node) -> bool {
        let mut cursor = node.walk();
        let is_async = node.children(&mut cursor).any(|c| c.kind() == "async");
        is_async
    }

    fn create_evidence(node: Node, file_path: &Path, source: &str) -> SourceEvidence {
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        let start = SourceLocation::new(start_pos.row + 1, start_pos.column + 1, node.start_byte());
        let end = SourceLocation::new(end_pos.row + 1, end_pos.column + 1, node.end_byte());
        let snippet = source
            .get(node.start_byte()..node.end_byte())
            .map(String::from);

        SourceEvidence {
            span: SourceSpan::new(file_path, start, end),
            snippet,
            ast_node_kind: node.kind().to_string(),
        }
    }

    fn node_text<'a>(node: Node<'a>, source: &'a str) -> &'a str {
        source.get(node.start_byte()..node.end_byte()).unwrap_or("")
    }

    fn extract_docstring(node: Node, source: &str) -> Option<String> {
        let body = node.child_by_field_name("body")?;
        let first_stmt = body.named_child(0)?;
        if first_stmt.kind() == "expression_statement" {
            let expr = first_stmt.named_child(0)?;
            if expr.kind() == "string" {
                let text = Self::node_text(expr, source);
                return Some(text.trim_matches(|c| c == '"' || c == '\'').to_string());
            }
        }
        None
    }

    fn extract_decorators(node: Node, source: &str) -> Vec<String> {
        let mut decorators = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "decorator" {
                decorators.push(Self::node_text(child, source).trim().to_string());
            }
        }
        decorators
    }

    fn extract_parameters(params_node: Node, file_path: &Path, source: &str) -> Vec<Parameter> {
        let mut params = Vec::new();
        let mut cursor = params_node.walk();
        for child in params_node.named_children(&mut cursor) {
            match child.kind() {
                "identifier" => {
                    let name = Self::node_text(child, source).to_string();
                    if name != "self" && name != "cls" {
                        params.push(Parameter {
                            name,
                            type_annotation: None,
                            default_value: None,
                            evidence: Self::create_evidence(child, file_path, source),
                        });
                    }
                }
                "typed_parameter" => {
                    let name_node = child
                        .child_by_field_name("name")
                        .or_else(|| child.named_child(0));
                    let type_node = child
                        .child_by_field_name("type")
                        .or_else(|| child.named_child(1));
                    if let Some(name_node) = name_node {
                        let name = Self::node_text(name_node, source).to_string();
                        if name != "self" && name != "cls" {
                            let type_annotation =
                                type_node.map(|t| Self::node_text(t, source).to_string());
                            params.push(Parameter {
                                name,
                                type_annotation,
                                default_value: None,
                                evidence: Self::create_evidence(child, file_path, source),
                            });
                        }
                    }
                }
                "default_parameter" => {
                    let name_node = child
                        .child_by_field_name("name")
                        .or_else(|| child.named_child(0));
                    let val_node = child
                        .child_by_field_name("value")
                        .or_else(|| child.named_child(1));
                    if let Some(name_node) = name_node {
                        let name = Self::node_text(name_node, source).to_string();
                        if name != "self" && name != "cls" {
                            let default_value =
                                val_node.map(|v| Self::node_text(v, source).to_string());
                            params.push(Parameter {
                                name,
                                type_annotation: None,
                                default_value,
                                evidence: Self::create_evidence(child, file_path, source),
                            });
                        }
                    }
                }
                "typed_default_parameter" => {
                    let name_node = child
                        .child_by_field_name("name")
                        .or_else(|| child.named_child(0));
                    let type_node = child
                        .child_by_field_name("type")
                        .or_else(|| child.named_child(1));
                    let val_node = child
                        .child_by_field_name("value")
                        .or_else(|| child.named_child(2));
                    if let Some(name_node) = name_node {
                        let name = Self::node_text(name_node, source).to_string();
                        if name != "self" && name != "cls" {
                            let type_annotation =
                                type_node.map(|t| Self::node_text(t, source).to_string());
                            let default_value =
                                val_node.map(|v| Self::node_text(v, source).to_string());
                            params.push(Parameter {
                                name,
                                type_annotation,
                                default_value,
                                evidence: Self::create_evidence(child, file_path, source),
                            });
                        }
                    }
                }
                "list_splat_pattern" | "dictionary_splat_pattern" => {
                    let name = Self::node_text(child, source).to_string();
                    params.push(Parameter {
                        name,
                        type_annotation: None,
                        default_value: None,
                        evidence: Self::create_evidence(child, file_path, source),
                    });
                }
                _ => {}
            }
        }
        params
    }

    fn parse_function_node(
        node: Node,
        file_path: &Path,
        source: &str,
        parent_scope: &str,
        is_async: bool,
        is_method: bool,
    ) -> Option<FunctionDef> {
        let name_node = node.child_by_field_name("name")?;
        let name = Self::node_text(name_node, source).to_string();
        let qualified_name = if parent_scope.is_empty() {
            name.clone()
        } else {
            format!("{parent_scope}::{name}")
        };

        let return_type = node
            .child_by_field_name("return_type")
            .map(|n| Self::node_text(n, source).to_string());

        let parameters = node
            .child_by_field_name("parameters")
            .map(|p| Self::extract_parameters(p, file_path, source))
            .unwrap_or_default();

        let docstring = Self::extract_docstring(node, source);
        let decorators = Self::extract_decorators(node, source);
        let body_snippet = node
            .child_by_field_name("body")
            .map(|b| Self::node_text(b, source).to_string())
            .unwrap_or_default();

        Some(FunctionDef {
            name,
            qualified_name,
            is_async,
            is_method,
            parameters,
            return_type,
            decorators,
            docstring,
            body_snippet,
            evidence: Self::create_evidence(node, file_path, source),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn traverse_ast(
        node: Node,
        file_path: &Path,
        source: &str,
        scope: &str,
        imports: &mut Vec<ImportStatement>,
        functions: &mut Vec<FunctionDef>,
        classes: &mut Vec<ClassDef>,
        calls: &mut Vec<CallExpr>,
        assignments: &mut Vec<Assignment>,
        unsupported: &mut Vec<UnsupportedConstruct>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // 1. Detect dynamic calls & unsupported reflection
        if node.kind() == "call" {
            if let Some(func_node) = node.child_by_field_name("function") {
                let callee_text = Self::node_text(func_node, source);
                if callee_text == "eval"
                    || callee_text == "exec"
                    || callee_text == "globals"
                    || callee_text == "locals"
                    || callee_text == "__import__"
                {
                    unsupported.push(UnsupportedConstruct {
                        construct_kind: "dynamic_code_execution".to_string(),
                        reason: format!("Dynamic runtime invocation `{callee_text}()` is unsupported in static Rust."),
                        snippet: Self::node_text(node, source).to_string(),
                        scope: scope.to_string(),
                        evidence: Self::create_evidence(node, file_path, source),
                    });
                }

                let mut args = Vec::new();
                if let Some(args_node) = node.child_by_field_name("arguments") {
                    let mut arg_cursor = args_node.walk();
                    for arg_child in args_node.named_children(&mut arg_cursor) {
                        args.push(Self::node_text(arg_child, source).to_string());
                    }
                }

                calls.push(CallExpr {
                    callee: callee_text.to_string(),
                    caller_scope: scope.to_string(),
                    arguments: args,
                    evidence: Self::create_evidence(node, file_path, source),
                });
            }
        }

        // 2. Process node definition
        match node.kind() {
            "import_statement" => {
                let mut names = Vec::new();
                let mut c = node.walk();
                for child in node.named_children(&mut c) {
                    if child.kind() == "dotted_name" {
                        names.push((Self::node_text(child, source).to_string(), None));
                    } else if child.kind() == "aliased_import" {
                        let orig = child
                            .child_by_field_name("name")
                            .map(|n| Self::node_text(n, source).to_string());
                        let alias = child
                            .child_by_field_name("alias")
                            .map(|a| Self::node_text(a, source).to_string());
                        if let Some(orig) = orig {
                            names.push((orig, alias));
                        }
                    }
                }
                let module_path = names.first().map(|n| n.0.clone()).unwrap_or_default();
                imports.push(ImportStatement {
                    module: module_path,
                    imported_names: names,
                    is_from_import: false,
                    evidence: Self::create_evidence(node, file_path, source),
                });
            }
            "import_from_statement" => {
                let module_node = node.child_by_field_name("module_name");
                let module_name_str = module_node
                    .map(|m| Self::node_text(m, source).to_string())
                    .unwrap_or_default();

                let mut names = Vec::new();
                let mut c = node.walk();
                for child in node.named_children(&mut c) {
                    if child.kind() == "dotted_name" && Some(child) != module_node {
                        names.push((Self::node_text(child, source).to_string(), None));
                    } else if child.kind() == "aliased_import" {
                        let orig = child
                            .child_by_field_name("name")
                            .map(|n| Self::node_text(n, source).to_string());
                        let alias = child
                            .child_by_field_name("alias")
                            .map(|a| Self::node_text(a, source).to_string());
                        if let Some(orig) = orig {
                            names.push((orig, alias));
                        }
                    }
                }

                imports.push(ImportStatement {
                    module: module_name_str,
                    imported_names: names,
                    is_from_import: true,
                    evidence: Self::create_evidence(node, file_path, source),
                });
            }
            "function_definition" | "async_function_definition" => {
                let is_async = Self::node_is_async(node);
                let is_method = scope.contains("::");
                if let Some(func_def) =
                    Self::parse_function_node(node, file_path, source, scope, is_async, is_method)
                {
                    let func_scope = func_def.qualified_name.clone();
                    if !is_method {
                        functions.push(func_def);
                    }

                    // Traverse inside function body
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for child in body.named_children(&mut body_cursor) {
                            Self::traverse_ast(
                                child,
                                file_path,
                                source,
                                &func_scope,
                                imports,
                                functions,
                                classes,
                                calls,
                                assignments,
                                unsupported,
                                diagnostics,
                            );
                        }
                    }
                    return;
                }
            }
            "class_definition" => {
                let name_node = node.child_by_field_name("name");
                if let Some(name_node) = name_node {
                    let class_name = Self::node_text(name_node, source).to_string();
                    let qualified_name = if scope.is_empty() {
                        class_name.clone()
                    } else {
                        format!("{scope}::{class_name}")
                    };

                    let mut base_classes = Vec::new();
                    if let Some(superclasses) = node.child_by_field_name("superclasses") {
                        let mut sc_cursor = superclasses.walk();
                        for sc in superclasses.named_children(&mut sc_cursor) {
                            base_classes.push(Self::node_text(sc, source).to_string());
                        }
                    }

                    let decorators = Self::extract_decorators(node, source);
                    let docstring = Self::extract_docstring(node, source);

                    let mut methods = Vec::new();
                    let mut fields = Vec::new();

                    if let Some(body) = node.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for member in body.named_children(&mut body_cursor) {
                            if member.kind() == "function_definition"
                                || member.kind() == "async_function_definition"
                            {
                                let is_async = Self::node_is_async(member);
                                if let Some(m) = Self::parse_function_node(
                                    member,
                                    file_path,
                                    source,
                                    &qualified_name,
                                    is_async,
                                    true,
                                ) {
                                    methods.push(m);
                                }
                            } else if member.kind() == "assignment" {
                                if let Some(left) = member.child_by_field_name("left") {
                                    fields.push(Self::node_text(left, source).to_string());
                                }
                            }
                        }
                    }

                    classes.push(ClassDef {
                        name: class_name,
                        qualified_name: qualified_name.clone(),
                        base_classes,
                        decorators,
                        docstring,
                        methods,
                        fields,
                        evidence: Self::create_evidence(node, file_path, source),
                    });

                    // Traverse inside class body for calls/assignments
                    if let Some(body) = node.child_by_field_name("body") {
                        let mut body_cursor = body.walk();
                        for child in body.named_children(&mut body_cursor) {
                            Self::traverse_ast(
                                child,
                                file_path,
                                source,
                                &qualified_name,
                                imports,
                                functions,
                                classes,
                                calls,
                                assignments,
                                unsupported,
                                diagnostics,
                            );
                        }
                    }
                    return;
                }
            }
            "assignment" => {
                let left = node.child_by_field_name("left");
                let right = node.child_by_field_name("right");
                let type_node = node.child_by_field_name("type");

                if let (Some(left), Some(right)) = (left, right) {
                    let target = Self::node_text(left, source).to_string();
                    let value_snippet = Self::node_text(right, source).to_string();
                    let type_annotation = type_node.map(|t| Self::node_text(t, source).to_string());

                    assignments.push(Assignment {
                        target,
                        value_snippet,
                        type_annotation,
                        scope: scope.to_string(),
                        evidence: Self::create_evidence(node, file_path, source),
                    });
                }
            }
            "ERROR" => {
                diagnostics.push(Diagnostic::error(
                    "P001_SYNTAX_ERROR",
                    format!("Syntax error encountered in {}", file_path.display()),
                    Some(SourceSpan::point(
                        file_path,
                        node.start_position().row + 1,
                        node.start_position().column + 1,
                    )),
                ));
            }
            _ => {}
        }

        // Recursive walk for other container nodes
        let mut child_cursor = node.walk();
        for child in node.named_children(&mut child_cursor) {
            Self::traverse_ast(
                child,
                file_path,
                source,
                scope,
                imports,
                functions,
                classes,
                calls,
                assignments,
                unsupported,
                diagnostics,
            );
        }
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for PythonParser {
    fn language(&self) -> LanguageId {
        LanguageId::new("python")
    }

    fn parse_source(&self, file_path: &Path, source_code: &str) -> Result<ParsedModule> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| ExodusError::ParseError {
                file: file_path.display().to_string(),
                message: format!("Failed to set tree-sitter python language: {e:?}"),
            })?;

        let tree: Tree =
            parser
                .parse(source_code, None)
                .ok_or_else(|| ExodusError::ParseError {
                    file: file_path.display().to_string(),
                    message: "Tree-sitter parse produced None".to_string(),
                })?;

        let root_node = tree.root_node();
        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("module")
            .to_string();

        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut calls = Vec::new();
        let mut assignments = Vec::new();
        let mut unsupported_constructs = Vec::new();
        let mut diagnostics = Vec::new();

        Self::traverse_ast(
            root_node,
            file_path,
            source_code,
            &module_name,
            &mut imports,
            &mut functions,
            &mut classes,
            &mut calls,
            &mut assignments,
            &mut unsupported_constructs,
            &mut diagnostics,
        );

        Ok(ParsedModule {
            file_path: file_path.to_path_buf(),
            module_name,
            imports,
            functions,
            classes,
            calls,
            assignments,
            unsupported_constructs,
            diagnostics,
        })
    }

    fn parse_repository(&self, repo_path: &Path) -> Result<ParsedRepository> {
        let mut modules = Vec::new();
        let mut diagnostics = Vec::new();

        if !repo_path.exists() {
            return Err(ExodusError::ParseError {
                file: repo_path.display().to_string(),
                message: "Repository path does not exist".to_string(),
            });
        }

        let mut entries = Vec::new();
        Self::collect_python_files(repo_path, &mut entries);

        for path in entries {
            match fs::read_to_string(&path) {
                Ok(content) => match self.parse_source(&path, &content) {
                    Ok(parsed_mod) => {
                        diagnostics.extend(parsed_mod.diagnostics.clone());
                        modules.push(parsed_mod);
                    }
                    Err(e) => {
                        diagnostics.push(Diagnostic::error(
                            "P002_FILE_PARSE_FAILED",
                            format!("Failed to parse {}: {e}", path.display()),
                            Some(SourceSpan::point(&path, 1, 1)),
                        ));
                    }
                },
                Err(e) => {
                    diagnostics.push(Diagnostic::error(
                        "P003_FILE_READ_FAILED",
                        format!("Failed to read {}: {e}", path.display()),
                        Some(SourceSpan::point(&path, 1, 1)),
                    ));
                }
            }
        }

        Ok(ParsedRepository {
            root_path: repo_path.to_path_buf(),
            modules,
            diagnostics,
        })
    }
}

impl PythonParser {
    fn collect_python_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.')
                        && name != "__pycache__"
                        && name != "node_modules"
                        && name != "target"
                    {
                        Self::collect_python_files(&path, files);
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("py") {
                    files.push(path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_core::SourceLanguageAdapter;

    #[test]
    fn test_python_parse_function_and_class() {
        let code = r#"
import math
from typing import Optional

def add(a: int, b: int = 10) -> int:
    """Add two numbers."""
    return a + b

class Calculator:
    def __init__(self, initial: int = 0):
        self.total = initial

    def compute(self, x: int) -> int:
        return add(self.total, x)
"#;
        let parser = PythonParser::new();
        let parsed = parser.parse_source(Path::new("calc.py"), code).unwrap();

        assert_eq!(parsed.functions.len(), 1);
        assert_eq!(parsed.functions[0].name, "add");
        assert_eq!(parsed.functions[0].parameters.len(), 2);
        assert_eq!(parsed.functions[0].return_type, Some("int".to_string()));

        assert_eq!(parsed.classes.len(), 1);
        assert_eq!(parsed.classes[0].name, "Calculator");
        assert_eq!(parsed.classes[0].methods.len(), 2);
    }

    #[test]
    fn test_async_function_and_method_detected() {
        let code = r#"
async def fetch(resource_id: str) -> str:
    return "x"

class Client:
    async def get(self, url: str) -> str:
        return url

    def sync_method(self) -> int:
        return 1
"#;
        let parser = PythonParser::new();
        let parsed = parser.parse_source(Path::new("client.py"), code).unwrap();

        assert_eq!(parsed.functions.len(), 1);
        assert!(
            parsed.functions[0].is_async,
            "top-level `async def` must be detected as async"
        );

        assert_eq!(parsed.classes[0].methods.len(), 2);
        let get_method = parsed.classes[0]
            .methods
            .iter()
            .find(|m| m.name == "get")
            .unwrap();
        assert!(
            get_method.is_async,
            "`async def` method must be detected as async"
        );
        let sync_method = parsed.classes[0]
            .methods
            .iter()
            .find(|m| m.name == "sync_method")
            .unwrap();
        assert!(
            !sync_method.is_async,
            "a plain `def` method must not be flagged async"
        );
    }

    #[test]
    fn test_python_unsupported_construct_detection() {
        let code = r#"
def dynamic_eval(expr: str):
    return eval(expr)
"#;
        let parser = PythonParser::new();
        let parsed = parser.parse_source(Path::new("dynamic.py"), code).unwrap();

        assert_eq!(parsed.unsupported_constructs.len(), 1);
        assert_eq!(
            parsed.unsupported_constructs[0].construct_kind,
            "dynamic_code_execution"
        );
    }

    #[tokio::test]
    async fn test_universal_source_adapter_python() {
        let adapter = PythonSourceAdapter::new();
        let code = r#"
import os

def calculate(x: int) -> int:
    return x * 2

class Worker:
    def execute(self) -> None:
        pass
"#;
        let (nodes, edges, _, _) = adapter
            .parse_file(Path::new("."), Path::new("worker.py"), code)
            .await
            .unwrap();

        assert!(nodes.iter().any(|n| n.name == "calculate"));
        assert!(nodes.iter().any(|n| n.name == "Worker"));
        assert!(edges
            .iter()
            .any(|e| e.relation == exodus_core::EsgRelation::Contains));
    }

    #[tokio::test]
    async fn test_universal_source_adapter_typescript() {
        let adapter = TypeScriptSourceAdapter::new();
        let code = r#"
import { Request } from 'express';

export interface UserDTO {
    id: string;
    email: string;
}

export class UserService {
    async function getUser(id: string) {
        return { id };
    }
}
"#;
        let (nodes, edges, _, _) = adapter
            .parse_file(Path::new("."), Path::new("user.ts"), code)
            .await
            .unwrap();

        assert!(nodes.iter().any(|n| n.name == "UserDTO"));
        assert!(nodes.iter().any(|n| n.name == "UserService"));
        assert!(edges
            .iter()
            .any(|e| e.relation == exodus_core::EsgRelation::Imports));
    }

    #[tokio::test]
    async fn test_universal_source_adapter_go() {
        let adapter = GoSourceAdapter::new();
        let code = r#"
package service

import "fmt"

type Config struct {
    Port int
}

func Start() error {
    return nil
}
"#;
        let (nodes, edges, _, _) = adapter
            .parse_file(Path::new("."), Path::new("service.go"), code)
            .await
            .unwrap();

        assert!(nodes.iter().any(|n| n.name == "Config"));
        assert!(nodes.iter().any(|n| n.name == "Start"));
        assert!(edges
            .iter()
            .any(|e| e.relation == exodus_core::EsgRelation::Contains));
    }

    #[tokio::test]
    async fn test_universal_source_adapter_custom_grammar() {
        let mut registry = GrammarRegistry::new();
        let mut custom = LanguageGrammarSpec::new("elixir", &["ex", "exs"]);
        custom.function_pattern = Some(r"def\s+([a-zA-Z_][a-zA-Z0-9_?!]*)\s*\(".into());
        registry.register(custom);

        let adapter = UniversalSourceAdapter::new().with_grammar_registry(registry);
        let code = r#"
def calculate(a, b) do
  a + b
end
"#;
        let (nodes, _, _, _) = adapter
            .parse_file(Path::new("."), Path::new("math.ex"), code)
            .await
            .unwrap();

        assert!(nodes.iter().any(|n| n.name == "calculate"));
    }

    #[tokio::test]
    async fn test_register_builtin_universal_adapter() {
        let mut registry = exodus_core::LanguageAdapterRegistry::new();
        register_builtin_source_adapters(&mut registry)
            .await
            .unwrap();
    }
}
