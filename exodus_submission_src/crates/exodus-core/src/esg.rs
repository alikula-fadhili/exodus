use crate::SourceSpan;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

/// Extensible identifier for source and target programming languages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LanguageId(String);

impl LanguageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().trim().to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for LanguageId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for LanguageId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for LanguageId {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for LanguageId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for LanguageId {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<String> for LanguageId {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for LanguageId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for LanguageId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Three-tier grounding hierarchy for semantic facts, symbols, and inferred mappings.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum GroundingTier {
    /// Inferred by LLM or heuristics without native proof.
    #[default]
    Provisional,
    /// Extracted deterministically from AST, compiler diagnostics, or lockfiles.
    Deterministic,
    /// Proved by native compiler checks and grounded behavioral oracle execution.
    Verified,
}

impl GroundingTier {
    pub fn is_grounded(&self) -> bool {
        matches!(self, Self::Deterministic | Self::Verified)
    }
}

impl fmt::Display for GroundingTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provisional => write!(f, "provisional"),
            Self::Deterministic => write!(f, "deterministic"),
            Self::Verified => write!(f, "verified"),
        }
    }
}

/// Visibility classification across OOP and procedural languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Public,
    PackagePrivate,
    Protected,
    Private,
}

/// Universal node kind for the Exodus Semantic Graph (ESG).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EsgNodeKind {
    Repository,
    Workspace,
    Package,
    Module,
    Namespace,
    Type,
    Interface,
    Trait,
    Class,
    Function,
    Method,
    Field,
    Constant,
    Endpoint,
    EventHandler,
    DatabaseEntity,
    Configuration,
    Dependency,
    BuildTarget,
    TestTarget,
    UnsupportedConstruct,
    Deprecation,
    Custom(String),
}

impl fmt::Display for EsgNodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repository => write!(f, "repository"),
            Self::Workspace => write!(f, "workspace"),
            Self::Package => write!(f, "package"),
            Self::Module => write!(f, "module"),
            Self::Namespace => write!(f, "namespace"),
            Self::Type => write!(f, "type"),
            Self::Interface => write!(f, "interface"),
            Self::Trait => write!(f, "trait"),
            Self::Class => write!(f, "class"),
            Self::Function => write!(f, "function"),
            Self::Method => write!(f, "method"),
            Self::Field => write!(f, "field"),
            Self::Constant => write!(f, "constant"),
            Self::Endpoint => write!(f, "endpoint"),
            Self::EventHandler => write!(f, "event_handler"),
            Self::DatabaseEntity => write!(f, "database_entity"),
            Self::Configuration => write!(f, "configuration"),
            Self::Dependency => write!(f, "dependency"),
            Self::BuildTarget => write!(f, "build_target"),
            Self::TestTarget => write!(f, "test_target"),
            Self::UnsupportedConstruct => write!(f, "unsupported_construct"),
            Self::Deprecation => write!(f, "deprecation"),
            Self::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

/// Universal edge relationship kind for the ESG.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EsgRelation {
    Contains,
    Imports,
    Calls,
    Implements,
    Inherits,
    Reads,
    Writes,
    Produces,
    Consumes,
    DependsOn,
    Replaces,
    DeprecatedBy,
    VerifiedBy,
    GeneratedFrom,
    Custom(String),
}

impl fmt::Display for EsgRelation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contains => write!(f, "contains"),
            Self::Imports => write!(f, "imports"),
            Self::Calls => write!(f, "calls"),
            Self::Implements => write!(f, "implements"),
            Self::Inherits => write!(f, "inherits"),
            Self::Reads => write!(f, "reads"),
            Self::Writes => write!(f, "writes"),
            Self::Produces => write!(f, "produces"),
            Self::Consumes => write!(f, "consumes"),
            Self::DependsOn => write!(f, "depends_on"),
            Self::Replaces => write!(f, "replaces"),
            Self::DeprecatedBy => write!(f, "deprecated_by"),
            Self::VerifiedBy => write!(f, "verified_by"),
            Self::GeneratedFrom => write!(f, "generated_from"),
            Self::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

/// Universal symbol signature metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SignatureMetadata {
    pub raw_signature: Option<String>,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_static: bool,
    pub generic_parameters: Vec<String>,
}

/// Language-neutral semantic node in the ESG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EsgNode {
    pub id: String,
    pub language: LanguageId,
    pub kind: EsgNodeKind,
    pub name: String,
    pub visibility: Visibility,
    pub location: Option<SourceSpan>,
    pub signature: SignatureMetadata,
    pub docstring: Option<String>,
    pub grounding: GroundingTier,
    pub language_attributes: serde_json::Value,
}

impl EsgNode {
    pub fn new(
        id: impl Into<String>,
        language: LanguageId,
        kind: EsgNodeKind,
        name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            language,
            kind,
            name: name.into(),
            visibility: Visibility::Public,
            location: None,
            signature: SignatureMetadata::default(),
            docstring: None,
            grounding: GroundingTier::Deterministic,
            language_attributes: serde_json::json!({}),
        }
    }
}

/// Language-neutral semantic edge in the ESG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EsgEdge {
    pub from_id: String,
    pub to_id: String,
    pub relation: EsgRelation,
    pub grounding: GroundingTier,
    pub metadata: BTreeMap<String, String>,
}

impl EsgEdge {
    pub fn new(
        from_id: impl Into<String>,
        to_id: impl Into<String>,
        relation: EsgRelation,
    ) -> Self {
        Self {
            from_id: from_id.into(),
            to_id: to_id.into(),
            relation,
            grounding: GroundingTier::Deterministic,
            metadata: BTreeMap::new(),
        }
    }
}
