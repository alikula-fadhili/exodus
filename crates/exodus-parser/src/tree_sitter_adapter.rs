//! Generic Tree-sitter-backed source adapters for the built-in Phase 1 languages.

use async_trait::async_trait;
use exodus_core::{
    DeprecationRecord, Diagnostic, EsgEdge, EsgNode, EsgNodeKind, EsgRelation, ExodusError,
    LanguageId, RepositoryProfile, Result, Severity, SignatureMetadata, SourceLanguageAdapter,
    SourceLocation, SourceSpan,
};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser};

/// Static grammar facts that cannot be persisted because a Tree-sitter `Language` is executable
/// parser code. Everything else is handled once by `TreeSitterSourceAdapter`.
pub trait BuiltinTreeSitterGrammar: Send + Sync + 'static {
    fn id() -> &'static str;
    fn extensions() -> &'static [&'static str];
    fn language() -> Language;
    fn manifests() -> &'static [&'static str];

    fn function_kind(kind: &str) -> bool;
    fn type_kind(kind: &str) -> Option<EsgNodeKind>;
    fn import_kind(kind: &str) -> bool;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PythonGrammar;

impl BuiltinTreeSitterGrammar for PythonGrammar {
    fn id() -> &'static str {
        "python"
    }

    fn extensions() -> &'static [&'static str] {
        &["py", "pyi"]
    }

    fn language() -> Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn manifests() -> &'static [&'static str] {
        &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "poetry.lock",
        ]
    }

    fn function_kind(kind: &str) -> bool {
        kind == "function_definition"
    }

    fn type_kind(kind: &str) -> Option<EsgNodeKind> {
        (kind == "class_definition").then_some(EsgNodeKind::Class)
    }

    fn import_kind(kind: &str) -> bool {
        matches!(kind, "import_statement" | "import_from_statement")
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TypeScriptGrammar;

impl BuiltinTreeSitterGrammar for TypeScriptGrammar {
    fn id() -> &'static str {
        "typescript"
    }

    fn extensions() -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn language() -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn manifests() -> &'static [&'static str] {
        &[
            "package.json",
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
        ]
    }

    fn function_kind(kind: &str) -> bool {
        matches!(
            kind,
            "function_declaration" | "method_definition" | "generator_function_declaration"
        )
    }

    fn type_kind(kind: &str) -> Option<EsgNodeKind> {
        match kind {
            "class_declaration" => Some(EsgNodeKind::Class),
            "interface_declaration" => Some(EsgNodeKind::Interface),
            "type_alias_declaration" => Some(EsgNodeKind::Type),
            _ => None,
        }
    }

    fn import_kind(kind: &str) -> bool {
        kind == "import_statement"
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GoGrammar;

impl BuiltinTreeSitterGrammar for GoGrammar {
    fn id() -> &'static str {
        "go"
    }

    fn extensions() -> &'static [&'static str] {
        &["go"]
    }

    fn language() -> Language {
        tree_sitter_go::LANGUAGE.into()
    }

    fn manifests() -> &'static [&'static str] {
        &["go.mod", "go.sum", "go.work"]
    }

    fn function_kind(kind: &str) -> bool {
        matches!(kind, "function_declaration" | "method_declaration")
    }

    fn type_kind(kind: &str) -> Option<EsgNodeKind> {
        (kind == "type_spec").then_some(EsgNodeKind::Type)
    }

    fn import_kind(kind: &str) -> bool {
        kind == "import_declaration"
    }
}

/// One adapter implementation parameterized by the executable grammar facts above.
#[derive(Debug, Clone, Copy, Default)]
pub struct TreeSitterSourceAdapter<G> {
    grammar: PhantomData<G>,
}

pub type PythonSourceAdapter = TreeSitterSourceAdapter<PythonGrammar>;
pub type TypeScriptSourceAdapter = TreeSitterSourceAdapter<TypeScriptGrammar>;
pub type GoSourceAdapter = TreeSitterSourceAdapter<GoGrammar>;

impl<G: BuiltinTreeSitterGrammar> TreeSitterSourceAdapter<G> {
    pub fn new() -> Self {
        Self {
            grammar: PhantomData,
        }
    }

    fn source_span(rel_path: &Path, node: Node<'_>) -> SourceSpan {
        let start = node.start_position();
        let end = node.end_position();
        SourceSpan::new(
            rel_path,
            SourceLocation::new(start.row + 1, start.column + 1, node.start_byte()),
            SourceLocation::new(end.row + 1, end.column + 1, node.end_byte()),
        )
    }

    fn node_text<'a>(node: Node<'_>, bytes: &'a [u8]) -> &'a str {
        node.utf8_text(bytes).unwrap_or_default()
    }

    fn symbol_name(node: Node<'_>, bytes: &[u8]) -> Option<String> {
        node.child_by_field_name("name")
            .map(|name| Self::node_text(name, bytes).trim().to_string())
            .filter(|name| !name.is_empty())
    }

    fn go_type_kind(node: Node<'_>) -> EsgNodeKind {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "struct_type" => return EsgNodeKind::Class,
                "interface_type" => return EsgNodeKind::Interface,
                _ => {}
            }
        }
        EsgNodeKind::Type
    }

    fn visit(
        node: Node<'_>,
        bytes: &[u8],
        rel_path: &Path,
        module_id: &str,
        nodes: &mut Vec<EsgNode>,
        edges: &mut Vec<EsgEdge>,
    ) {
        if G::function_kind(node.kind()) {
            if let Some(name) = Self::symbol_name(node, bytes) {
                let is_method = node.kind() == "method_definition"
                    || node.kind() == "method_declaration"
                    || node
                        .parent()
                        .is_some_and(|parent| parent.kind().contains("class"));
                let kind = if is_method {
                    EsgNodeKind::Method
                } else {
                    EsgNodeKind::Function
                };
                let id = format!("{}::{}::{}", kind, rel_path.display(), name);
                let raw = Self::node_text(node, bytes);
                let mut symbol = EsgNode::new(&id, LanguageId::new(G::id()), kind, name);
                symbol.location = Some(Self::source_span(rel_path, node));
                symbol.signature = SignatureMetadata {
                    raw_signature: raw.lines().next().map(str::trim).map(ToOwned::to_owned),
                    is_async: raw.trim_start().starts_with("async "),
                    ..SignatureMetadata::default()
                };
                nodes.push(symbol);
                edges.push(EsgEdge::new(module_id, id, EsgRelation::Contains));
            }
        } else if let Some(mut kind) = G::type_kind(node.kind()) {
            if G::id() == "go" {
                kind = Self::go_type_kind(node);
            }
            if let Some(name) = Self::symbol_name(node, bytes) {
                let id = format!("{}::{}::{}", kind, rel_path.display(), name);
                let mut symbol = EsgNode::new(&id, LanguageId::new(G::id()), kind, name);
                symbol.location = Some(Self::source_span(rel_path, node));
                nodes.push(symbol);
                edges.push(EsgEdge::new(module_id, id, EsgRelation::Contains));
            }
        } else if G::import_kind(node.kind()) {
            let imported = Self::node_text(node, bytes).trim();
            if !imported.is_empty() {
                edges.push(EsgEdge::new(
                    module_id,
                    format!("import::{}", imported),
                    EsgRelation::Imports,
                ));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::visit(child, bytes, rel_path, module_id, nodes, edges);
        }
    }
}

#[async_trait]
impl<G: BuiltinTreeSitterGrammar> SourceLanguageAdapter for TreeSitterSourceAdapter<G> {
    fn language_id(&self) -> LanguageId {
        LanguageId::new(G::id())
    }

    fn supported_extensions(&self) -> &[&'static str] {
        G::extensions()
    }

    async fn parse_file(
        &self,
        _repo_root: &Path,
        rel_path: &Path,
        content: &str,
    ) -> Result<(
        Vec<EsgNode>,
        Vec<EsgEdge>,
        Vec<Diagnostic>,
        Vec<DeprecationRecord>,
    )> {
        let mut parser = Parser::new();
        parser
            .set_language(&G::language())
            .map_err(|error| ExodusError::ParseError {
                file: rel_path.display().to_string(),
                message: format!("failed to load {} Tree-sitter grammar: {error}", G::id()),
            })?;
        let tree = parser
            .parse(content, None)
            .ok_or_else(|| ExodusError::ParseError {
                file: rel_path.display().to_string(),
                message: "Tree-sitter returned no syntax tree".to_string(),
            })?;

        let module_id = format!("module::{}", rel_path.display());
        let mut module = EsgNode::new(
            &module_id,
            LanguageId::new(G::id()),
            EsgNodeKind::Module,
            rel_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("module"),
        );
        module.location = Some(SourceSpan::point(rel_path, 1, 1));

        let mut nodes = vec![module];
        let mut edges = Vec::new();
        Self::visit(
            tree.root_node(),
            content.as_bytes(),
            rel_path,
            &module_id,
            &mut nodes,
            &mut edges,
        );

        let diagnostics = tree
            .root_node()
            .has_error()
            .then(|| Diagnostic {
                severity: Severity::Warning,
                code: "W_TREE_SITTER_RECOVERY".to_string(),
                message: format!(
                    "{} syntax contained recoverable parse errors; extracted nodes remain deterministic",
                    G::id()
                ),
                span: Some(Self::source_span(rel_path, tree.root_node())),
                suggestion: None,
            })
            .into_iter()
            .collect();

        Ok((nodes, edges, diagnostics, Vec::new()))
    }

    async fn profile_repository(&self, repo_root: &Path) -> Result<RepositoryProfile> {
        let repository_id = repo_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace");
        let mut profile = RepositoryProfile::new(repository_id, LanguageId::new(G::id()));
        profile.detected_languages.push(LanguageId::new(G::id()));
        profile.manifests = G::manifests()
            .iter()
            .map(PathBuf::from)
            .filter(|path| repo_root.join(path).is_file())
            .collect();
        profile.lockfiles = profile
            .manifests
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains("lock") || name == "go.sum")
            })
            .cloned()
            .collect();
        Ok(profile)
    }
}
