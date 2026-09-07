//! Universal knowledge-driven source adapter for Project Exodus.
//! Extracts language-neutral ESG semantics from ANY programming language using
//! extensible grammar patterns, manifest heuristics, and LLM semantic evaluation.

use async_trait::async_trait;
use exodus_core::{
    DeprecationRecord, Diagnostic, EsgEdge, EsgNode, EsgNodeKind, EsgRelation, ExodusError,
    LanguageId, RepositoryProfile, Result, SourceLanguageAdapter, SourceSpan,
};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Dynamic grammar specification for extracting symbols from a specific language syntax.
#[derive(Debug, Clone)]
pub struct LanguageGrammarSpec {
    pub language_id: LanguageId,
    pub extensions: Vec<String>,
    pub function_pattern: Option<String>,
    pub class_pattern: Option<String>,
    pub interface_pattern: Option<String>,
    pub import_pattern: Option<String>,
    pub comment_prefix: String,
}

impl LanguageGrammarSpec {
    pub fn new(lang: impl Into<String>, extensions: &[&str]) -> Self {
        Self {
            language_id: LanguageId::new(lang),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            function_pattern: None,
            class_pattern: None,
            interface_pattern: None,
            import_pattern: None,
            comment_prefix: "//".to_string(),
        }
    }

    pub fn with_function_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.function_pattern = Some(pattern.into());
        self
    }

    pub fn with_class_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.class_pattern = Some(pattern.into());
        self
    }

    pub fn with_interface_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.interface_pattern = Some(pattern.into());
        self
    }

    pub fn with_import_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.import_pattern = Some(pattern.into());
        self
    }

    pub fn with_comment_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.comment_prefix = prefix.into();
        self
    }
}

/// Registry of known and dynamic language grammar specifications.
#[derive(Debug, Clone)]
pub struct GrammarRegistry {
    specs: HashMap<LanguageId, LanguageGrammarSpec>,
    ext_map: HashMap<String, LanguageId>,
}

impl GrammarRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            specs: HashMap::new(),
            ext_map: HashMap::new(),
        };
        reg.register(
            LanguageGrammarSpec::new("python", &["py", "pyi"])
                .with_function_pattern(r"(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"class\s+([a-zA-Z_][a-zA-Z0-9_]*)(?:\s*\((.*?)\))?\s*:")
                .with_import_pattern(
                    r"(?:from\s+([a-zA-Z0-9_\.]+)\s+import|import\s+([a-zA-Z0-9_\.]+))",
                )
                .with_comment_prefix("#"),
        );
        reg.register(
            LanguageGrammarSpec::new("typescript", &["ts", "tsx", "js", "jsx"])
                .with_function_pattern(r"(?:export\s+)?(?:async\s+)?(?:function\s+([a-zA-Z_][a-zA-Z0-9_]*)|const\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(?:async\s*)?\()")
                .with_class_pattern(r"(?:export\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_interface_pattern(r"(?:export\s+)?(?:interface|type)\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r#"import\s+.*?\s+from\s+['"]([^'"]+)['"]"#),
        );
        reg.register(
            LanguageGrammarSpec::new("go", &["go"])
                .with_function_pattern(r"func\s+(?:\(.*?\)\s+)?([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"type\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+struct")
                .with_interface_pattern(r"type\s+([a-zA-Z_][a-zA-Z0-9_]*)\s+interface")
                .with_import_pattern(r#"import\s+(?:\(\s*([\s\S]*?)\s*\)|"([^"]+)")"#),
        );
        reg.register(
            LanguageGrammarSpec::new("rust", &["rs"])
                .with_function_pattern(
                    r"(?:pub(?:\(.*?\))?\s+)?(?:async\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)",
                )
                .with_class_pattern(
                    r"(?:pub(?:\(.*?\))?\s+)?(?:struct|enum)\s+([a-zA-Z_][a-zA-Z0-9_]*)",
                )
                .with_interface_pattern(r"(?:pub(?:\(.*?\))?\s+)?trait\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r"use\s+([a-zA-Z0-9_:]+)"),
        );
        reg.register(
            LanguageGrammarSpec::new("java", &["java"])
                .with_function_pattern(r"(?:public|protected|private|static|\s)+[\w\<\>\[\]]+\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"(?:public\s+)?(?:abstract\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_interface_pattern(r"(?:public\s+)?interface\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r"import\s+([a-zA-Z0-9_\.]+);")
                .with_comment_prefix("//"),
        );
        reg.register(
            LanguageGrammarSpec::new("csharp", &["cs"])
                .with_function_pattern(r"(?:public|protected|private|internal|static|async|\s)+[\w\<\>\[\]\?]+\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"(?:public\s+)?(?:partial\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_interface_pattern(r"(?:public\s+)?interface\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r"using\s+([a-zA-Z0-9_\.]+);")
                .with_comment_prefix("//"),
        );
        reg.register(
            LanguageGrammarSpec::new("zig", &["zig"])
                .with_function_pattern(r"pub\s+fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"pub\s+const\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*struct")
                .with_import_pattern(r#"const\s+\w+\s*=\s*@import\("([^"]+)"\);"#)
                .with_comment_prefix("//"),
        );
        reg.register(
            LanguageGrammarSpec::new("kotlin", &["kt", "kts"])
                .with_function_pattern(r"(?:fun|override\s+fun)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"(?:data\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_interface_pattern(r"interface\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r"import\s+([a-zA-Z0-9_\.]+)")
                .with_comment_prefix("//"),
        );
        reg.register(
            LanguageGrammarSpec::new("dart", &["dart"])
                .with_function_pattern(r"(?:void|Future|[\w\<\>]+)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                .with_class_pattern(r"class\s+([a-zA-Z_][a-zA-Z0-9_]*)")
                .with_import_pattern(r#"import\s+['"]([^'"]+)['"];"#)
                .with_comment_prefix("//"),
        );
        reg
    }

    pub fn register(&mut self, spec: LanguageGrammarSpec) {
        let lang = spec.language_id.clone();
        for ext in &spec.extensions {
            self.ext_map.insert(ext.clone(), lang.clone());
        }
        self.specs.insert(lang, spec);
    }

    pub fn get_by_language(&self, lang: &LanguageId) -> Option<&LanguageGrammarSpec> {
        self.specs.get(lang)
    }

    pub fn get_by_extension(&self, ext: &str) -> Option<&LanguageGrammarSpec> {
        self.ext_map.get(ext).and_then(|lang| self.specs.get(lang))
    }
}

impl Default for GrammarRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook for LLM-driven semantic extraction for novel/unregistered languages.
#[async_trait]
pub trait LlmSemanticEvaluator: Send + Sync {
    async fn extract_semantics(
        &self,
        rel_path: &Path,
        content: &str,
    ) -> Result<(Vec<EsgNode>, Vec<EsgEdge>, Vec<Diagnostic>)>;
}

/// Universal source adapter parsing any registered or novel language into ESG nodes and edges.
pub struct UniversalSourceAdapter {
    grammar_registry: GrammarRegistry,
    llm_evaluator: Option<Arc<dyn LlmSemanticEvaluator>>,
}

impl UniversalSourceAdapter {
    pub fn new() -> Self {
        Self {
            grammar_registry: GrammarRegistry::new(),
            llm_evaluator: None,
        }
    }

    pub fn with_grammar_registry(mut self, registry: GrammarRegistry) -> Self {
        self.grammar_registry = registry;
        self
    }

    pub fn with_llm_evaluator(mut self, evaluator: Arc<dyn LlmSemanticEvaluator>) -> Self {
        self.llm_evaluator = Some(evaluator);
        self
    }
}

impl Default for UniversalSourceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceLanguageAdapter for UniversalSourceAdapter {
    fn language_id(&self) -> LanguageId {
        LanguageId::new("universal")
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &[
            "py", "pyi", "ts", "tsx", "js", "jsx", "go", "rs", "java", "kt", "zig", "cpp", "c",
            "h", "cs", "rb", "swift", "ex", "exs",
        ]
    }

    fn can_handle_file(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.grammar_registry.get_by_extension(ext).is_some() || self.llm_evaluator.is_some()
        } else {
            false
        }
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
        let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let spec_opt = self.grammar_registry.get_by_extension(ext);

        if let Some(spec) = spec_opt {
            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            let diagnostics = Vec::new();
            let deprecations = Vec::new();

            let module_id = format!("module::{}", rel_path.display());
            let mut module_node = EsgNode::new(
                &module_id,
                spec.language_id.clone(),
                EsgNodeKind::Module,
                rel_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("module"),
            );
            module_node.location = Some(SourceSpan::point(rel_path, 1, 1));
            nodes.push(module_node);

            // 1. Classes / Structs
            if let Some(ref pattern) = spec.class_pattern {
                if let Ok(re) = Regex::new(pattern) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if let Some(caps) = re.captures(line) {
                            if let Some(name_match) = caps.get(1) {
                                let name = name_match.as_str().trim();
                                let node_id = format!("class::{}::{}", rel_path.display(), name);
                                let mut class_node = EsgNode::new(
                                    &node_id,
                                    spec.language_id.clone(),
                                    EsgNodeKind::Class,
                                    name,
                                );
                                class_node.location =
                                    Some(SourceSpan::point(rel_path, line_idx + 1, 1));
                                nodes.push(class_node);
                                edges.push(EsgEdge::new(
                                    &module_id,
                                    &node_id,
                                    EsgRelation::Contains,
                                ));
                            }
                        }
                    }
                }
            }

            // 2. Interfaces / Traits / Types
            if let Some(ref pattern) = spec.interface_pattern {
                if let Ok(re) = Regex::new(pattern) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if let Some(caps) = re.captures(line) {
                            if let Some(name_match) = caps.get(1) {
                                let name = name_match.as_str().trim();
                                let node_id =
                                    format!("interface::{}::{}", rel_path.display(), name);
                                let mut iface_node = EsgNode::new(
                                    &node_id,
                                    spec.language_id.clone(),
                                    EsgNodeKind::Interface,
                                    name,
                                );
                                iface_node.location =
                                    Some(SourceSpan::point(rel_path, line_idx + 1, 1));
                                nodes.push(iface_node);
                                edges.push(EsgEdge::new(
                                    &module_id,
                                    &node_id,
                                    EsgRelation::Contains,
                                ));
                            }
                        }
                    }
                }
            }

            // 3. Functions
            if let Some(ref pattern) = spec.function_pattern {
                if let Ok(re) = Regex::new(pattern) {
                    for (line_idx, line) in content.lines().enumerate() {
                        if let Some(caps) = re.captures(line) {
                            let name = caps
                                .get(1)
                                .or_else(|| caps.get(2))
                                .map(|m| m.as_str().trim());
                            if let Some(name_str) = name {
                                let node_id = format!("func::{}::{}", rel_path.display(), name_str);
                                let mut fn_node = EsgNode::new(
                                    &node_id,
                                    spec.language_id.clone(),
                                    EsgNodeKind::Function,
                                    name_str,
                                );
                                fn_node.location =
                                    Some(SourceSpan::point(rel_path, line_idx + 1, 1));
                                nodes.push(fn_node);
                                edges.push(EsgEdge::new(
                                    &module_id,
                                    &node_id,
                                    EsgRelation::Contains,
                                ));
                            }
                        }
                    }
                }
            }

            // 4. Imports
            if let Some(ref pattern) = spec.import_pattern {
                if let Ok(re) = Regex::new(pattern) {
                    for line in content.lines() {
                        if let Some(caps) = re.captures(line) {
                            let imp = caps
                                .get(1)
                                .or_else(|| caps.get(2))
                                .map(|m| m.as_str().trim());
                            if let Some(imp_str) = imp {
                                let target_id = format!("module::{}", imp_str);
                                edges.push(EsgEdge::new(
                                    &module_id,
                                    &target_id,
                                    EsgRelation::Imports,
                                ));
                            }
                        }
                    }
                }
            }

            Ok((nodes, edges, diagnostics, deprecations))
        } else if let Some(ref llm) = self.llm_evaluator {
            let (nodes, edges, diags) = llm.extract_semantics(rel_path, content).await?;
            Ok((nodes, edges, diags, Vec::new()))
        } else {
            Err(ExodusError::Generic(format!(
                "No grammar spec or LLM evaluator registered for file: {}",
                rel_path.display()
            )))
        }
    }

    async fn profile_repository(&self, repo_root: &Path) -> Result<RepositoryProfile> {
        let repo_name = repo_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");

        let mut primary_lang = LanguageId::new("generic");
        if repo_root.join("Cargo.toml").exists() {
            primary_lang = LanguageId::new("rust");
        } else if repo_root.join("go.mod").exists() {
            primary_lang = LanguageId::new("go");
        } else if repo_root.join("package.json").exists() {
            primary_lang = LanguageId::new("typescript");
        } else if repo_root.join("pyproject.toml").exists()
            || repo_root.join("requirements.txt").exists()
            || repo_root.join("setup.py").exists()
        {
            primary_lang = LanguageId::new("python");
        }

        let mut profile = RepositoryProfile::new(repo_name, primary_lang);
        for manifest in &[
            "Cargo.toml",
            "go.mod",
            "package.json",
            "pyproject.toml",
            "requirements.txt",
            "pom.xml",
            "build.gradle",
            "build.zig",
        ] {
            if repo_root.join(manifest).exists() {
                profile.manifests.push(PathBuf::from(manifest));
            }
        }

        Ok(profile)
    }
}
