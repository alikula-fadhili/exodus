//! Repository profile and architectural characterization models.

use crate::esg::{GroundingTier, LanguageId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

/// Concurrency model classification and runtime primitives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConcurrencyProfile {
    pub uses_async: bool,
    pub uses_threads: bool,
    pub uses_actors_or_channels: bool,
    pub uses_locks_or_mutexes: bool,
    pub uses_event_loop: bool,
    pub detected_frameworks: Vec<String>,
    pub blocking_call_symbols: Vec<String>,
}

/// Memory model and resource lifecycle characterization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MemoryModelProfile {
    pub has_garbage_collector: bool,
    pub relies_on_reference_cycles: bool,
    pub relies_on_mutable_aliasing: bool,
    pub has_explicit_ownership: bool,
    pub requires_manual_cleanup: bool,
}

/// Review state for a source-to-target concurrency mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MappingApprovalStatus {
    #[default]
    Pending,
    AcceptedForPlanning,
    Rejected,
}

/// A versioned, evidence-bearing plan for translating concurrency semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConcurrencyMappingPlan {
    pub schema_version: String,
    pub source_language: LanguageId,
    pub target_language: LanguageId,
    pub source_concurrency_tier: String,
    pub target_runtime: String,
    pub primitive_substitutions: BTreeMap<String, String>,
    pub blocking_call_treatment: Option<String>,
    pub lifecycle_expectations: Vec<String>,
    pub unsupported_primitives: Vec<String>,
    pub grounding: GroundingTier,
    pub approval_status: MappingApprovalStatus,
    pub evidence_ids: Vec<String>,
}

impl ConcurrencyMappingPlan {
    pub fn is_ready(&self) -> bool {
        self.unsupported_primitives.is_empty()
            && (self.grounding.is_grounded()
                || self.approval_status == MappingApprovalStatus::AcceptedForPlanning)
    }
}

/// One canonical source-to-target path mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetPathMapping {
    pub source: PathBuf,
    pub target: PathBuf,
    pub package: Option<String>,
}

/// Deterministic path-planning failure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PathCollisionError {
    pub kind: String,
    pub target: PathBuf,
    pub sources: Vec<PathBuf>,
    pub message: String,
}

/// Collision-checked preservation plan for repository-relative output paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetPathPlan {
    pub schema_version: String,
    pub target_root: PathBuf,
    pub case_sensitive: bool,
    pub reserved_names: BTreeSet<String>,
    pub mappings: Vec<TargetPathMapping>,
}

impl Default for TargetPathPlan {
    fn default() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            target_root: PathBuf::from("."),
            case_sensitive: true,
            reserved_names: BTreeSet::new(),
            mappings: Vec::new(),
        }
    }
}

impl TargetPathPlan {
    fn normalized_key(&self, path: &Path) -> String {
        let key = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => Some(value.to_string_lossy()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("/");
        if self.case_sensitive {
            key
        } else {
            key.to_lowercase()
        }
    }

    /// Returns every collision in deterministic order without mutating the plan.
    pub fn detect_collisions(&self) -> std::result::Result<(), Vec<PathCollisionError>> {
        let mut errors = Vec::new();
        let mut by_target: BTreeMap<String, Vec<&TargetPathMapping>> = BTreeMap::new();

        for mapping in &self.mappings {
            if mapping.target.is_absolute()
                || mapping.target.components().any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                errors.push(PathCollisionError {
                    kind: "target_root_escape".to_string(),
                    target: mapping.target.clone(),
                    sources: vec![mapping.source.clone()],
                    message: "target path must remain relative to the approved output root"
                        .to_string(),
                });
                continue;
            }

            if let Some(name) = mapping.target.file_name().and_then(|name| name.to_str()) {
                let candidate = if self.case_sensitive {
                    name.to_string()
                } else {
                    name.to_lowercase()
                };
                if self.reserved_names.contains(&candidate) {
                    errors.push(PathCollisionError {
                        kind: "reserved_name".to_string(),
                        target: mapping.target.clone(),
                        sources: vec![mapping.source.clone()],
                        message: format!("target filename `{name}` is reserved"),
                    });
                }
            }

            by_target
                .entry(self.normalized_key(&mapping.target))
                .or_default()
                .push(mapping);
        }

        for mappings in by_target.values().filter(|mappings| mappings.len() > 1) {
            let target = mappings[0].target.clone();
            let mut sources = mappings
                .iter()
                .map(|mapping| mapping.source.clone())
                .collect::<Vec<_>>();
            sources.sort();
            errors.push(PathCollisionError {
                kind: "duplicate_target".to_string(),
                target,
                sources,
                message: "multiple source paths map to the same normalized target".to_string(),
            });
        }

        let keys = by_target.keys().cloned().collect::<Vec<_>>();
        for key in &keys {
            let prefix = format!("{key}/");
            if let Some(descendant) = keys.iter().find(|candidate| candidate.starts_with(&prefix)) {
                errors.push(PathCollisionError {
                    kind: "file_directory_collision".to_string(),
                    target: PathBuf::from(key),
                    sources: by_target[key]
                        .iter()
                        .chain(by_target[descendant].iter())
                        .map(|mapping| mapping.source.clone())
                        .collect(),
                    message: "a target path is both a file and a directory prefix".to_string(),
                });
            }
        }

        errors.sort();
        errors.dedup();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Comprehensive, language-neutral profile of a repository's architecture and conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepositoryProfile {
    pub repository_id: String,
    pub snapshot_id: String,
    pub primary_language: LanguageId,
    pub detected_languages: Vec<LanguageId>,
    pub package_roots: Vec<PathBuf>,
    pub canonical_modules: Vec<String>,
    pub manifests: Vec<PathBuf>,
    pub lockfiles: Vec<PathBuf>,
    pub external_dependencies: BTreeMap<String, String>,
    pub build_commands: Vec<String>,
    pub test_commands: Vec<String>,
    pub ci_workflows: Vec<PathBuf>,
    pub environment_variables: Vec<String>,
    pub concurrency_profile: ConcurrencyProfile,
    pub memory_profile: MemoryModelProfile,
    #[serde(default)]
    pub concurrency_mapping_plan: Option<ConcurrencyMappingPlan>,
    #[serde(default)]
    pub target_path_plan: TargetPathPlan,
    pub total_source_files: usize,
    pub total_lines_of_code: usize,
}

impl RepositoryProfile {
    pub fn new(repository_id: impl Into<String>, primary_language: LanguageId) -> Self {
        Self {
            repository_id: repository_id.into(),
            snapshot_id: uuid::Uuid::now_v7().to_string(),
            primary_language,
            detected_languages: Vec::new(),
            package_roots: Vec::new(),
            canonical_modules: Vec::new(),
            manifests: Vec::new(),
            lockfiles: Vec::new(),
            external_dependencies: BTreeMap::new(),
            build_commands: Vec::new(),
            test_commands: Vec::new(),
            ci_workflows: Vec::new(),
            environment_variables: Vec::new(),
            concurrency_profile: ConcurrencyProfile::default(),
            memory_profile: MemoryModelProfile::default(),
            concurrency_mapping_plan: None,
            target_path_plan: TargetPathPlan::default(),
            total_source_files: 0,
            total_lines_of_code: 0,
        }
    }
}
