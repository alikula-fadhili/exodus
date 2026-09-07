//! Embedded storage layer, GraphStore abstractions, and knowledge persistence for Project Exodus.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use exodus_case::{CaseStatus, FailureCategory, MigrationCase};
use exodus_core::{
    BehavioralContract, CiFailureEventPayload, DeprecationRecord, EsgEdge, EsgNode, ExodusError,
    MigrationOutcome, OperationalDomainTag, OperationalItem, OperationalLifecycleState,
    RepositoryProfile, Result, SdlcIntegrationSettings, TargetLanguageSpecRecord,
};
use exodus_graph::SemanticGraph;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use surrealdb::engine::local::{Db, Mem, SurrealKv};
use surrealdb::Surreal;

pub mod sdlc_catalog;
pub use sdlc_catalog::*;

pub mod dynamic_memory;
pub use dynamic_memory::*;

pub mod skills_discovery;
pub use skills_discovery::*;

pub mod operational_store;
pub use operational_store::*;

/// Persistent record representing an ESG node in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNodeRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub signature: Option<String>,
    pub file_path: String,
    pub is_grounded: bool,
}

/// Persistent record representing an ESG relationship edge in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdgeRecord {
    pub from_id: String,
    pub to_id: String,
    pub relationship: String,
    pub metadata: BTreeMap<String, String>,
}

/// Edge traversal direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Structural difference between two graph snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: Vec<(String, String, String)>,
    pub removed_edges: Vec<(String, String, String)>,
}

/// Storage interface for ESG graph snapshots, node upsert, and relationship queries.
#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn upsert_node(&mut self, node: &GraphNodeRecord) -> Result<()>;
    async fn add_edge(&mut self, edge: &GraphEdgeRecord) -> Result<()>;
    async fn get_node(&self, id: &str) -> Result<Option<GraphNodeRecord>>;
    async fn neighbors(&self, id: &str, direction: GraphDirection) -> Result<Vec<GraphNodeRecord>>;
    async fn create_snapshot(&mut self, snapshot_id: &str, graph: &SemanticGraph) -> Result<()>;
    async fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<SemanticGraph>>;
    async fn diff_snapshots(&self, snap_a: &str, snap_b: &str) -> Result<GraphDiff>;

    // Language-neutral ESG contracts
    async fn upsert_esg_node(&mut self, node: &EsgNode) -> Result<()>;
    async fn get_esg_node(&self, id: &str) -> Result<Option<EsgNode>>;
    async fn list_esg_nodes(&self) -> Result<Vec<EsgNode>>;
    async fn add_esg_edge(&mut self, edge: &EsgEdge) -> Result<()>;
    async fn list_esg_edges(&self) -> Result<Vec<EsgEdge>>;
}

/// Verification record for persistent trajectory tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub run_id: String,
    pub unit_id: String,
    pub outcome: MigrationOutcome,
    pub compiler_ok: bool,
    pub tests_ok: bool,
    pub timestamp: DateTime<Utc>,
}

/// Export manifest for portable backup and rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub case_count: usize,
    pub contract_count: usize,
    pub snapshot_count: usize,
    pub run_count: usize,
    pub esg_node_count: usize,
    pub deprecation_count: usize,
}

/// Report emitted during database rebuild or JSON import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    pub cases_imported: usize,
    pub contracts_imported: usize,
    pub snapshots_imported: usize,
    pub profiles_imported: usize,
    pub deprecations_imported: usize,
    pub errors: Vec<String>,
}

/// Storage interface for migration cases, behavioral contracts, and verification runs.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    async fn save_case(&mut self, case: &MigrationCase) -> Result<()>;
    async fn get_case(&self, case_id: &str) -> Result<Option<MigrationCase>>;
    async fn list_cases(&self) -> Result<Vec<MigrationCase>>;
    async fn search_promoted_cases(
        &self,
        fingerprint: &str,
        category: &FailureCategory,
    ) -> Result<Vec<MigrationCase>>;
    async fn save_contract(&mut self, contract: &BehavioralContract) -> Result<()>;
    async fn get_contract(&self, unit_id: &str) -> Result<Option<BehavioralContract>>;
    async fn list_contracts(&self) -> Result<Vec<BehavioralContract>>;
    async fn save_verification_run(&mut self, run: &VerificationRecord) -> Result<()>;
    async fn save_framework_rule(&mut self, rule: &exodus_toolchain::FrameworkRule) -> Result<()>;
    async fn get_framework_registry(&self) -> Result<exodus_toolchain::FrameworkRegistry>;
    async fn export_all(&self, target_dir: &Path) -> Result<ExportManifest>;
    async fn import_all(&mut self, source_dir: &Path) -> Result<ImportReport>;

    // Language-neutral Repository Profile & Deprecation lifecycle
    async fn save_profile(&mut self, profile: &RepositoryProfile) -> Result<()>;
    async fn get_profile(&self, repo_id: &str) -> Result<Option<RepositoryProfile>>;
    async fn save_deprecation(&mut self, deprecation: &DeprecationRecord) -> Result<()>;
    async fn get_deprecation(&self, id: &str) -> Result<Option<DeprecationRecord>>;
    async fn list_deprecations(&self) -> Result<Vec<DeprecationRecord>>;
}

/// Runtime-editable target-language catalog backed by the selected Exodus store.
///
/// Language specifications are configuration, not a parallel run-state system. Keeping them in
/// the embedded store allows adding or tuning an ecosystem without rebuilding the CLI.
#[async_trait]
pub trait TargetLanguageCatalog: Send + Sync {
    async fn get_language_spec(&self, query: &str) -> Result<Option<TargetLanguageSpecRecord>>;
    async fn list_language_specs(&self) -> Result<Vec<TargetLanguageSpecRecord>>;
    async fn upsert_language_spec(&mut self, spec: &TargetLanguageSpecRecord) -> Result<()>;
    async fn reset_language_specs(&mut self) -> Result<()>;
    async fn ensure_language_specs_seeded(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PersistedStoreState {
    nodes: HashMap<String, GraphNodeRecord>,
    edges: Vec<GraphEdgeRecord>,
    esg_nodes: HashMap<String, EsgNode>,
    esg_edges: Vec<EsgEdge>,
    snapshots: HashMap<String, SemanticGraph>,
    profiles: HashMap<String, RepositoryProfile>,
    deprecations: HashMap<String, DeprecationRecord>,
    cases: HashMap<String, MigrationCase>,
    contracts: HashMap<String, BehavioralContract>,
    runs: Vec<VerificationRecord>,
    frameworks: exodus_toolchain::FrameworkRegistry,
    language_specs: HashMap<String, TargetLanguageSpecRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedStateRecord {
    schema_version: u32,
    state: PersistedStoreState,
}

/// Fast, thread-safe in-memory store for unit tests, CI, and parallel attempt sandboxes.
#[derive(Debug, Clone, Default)]
pub struct MemoryGraphStore {
    nodes: Arc<RwLock<HashMap<String, GraphNodeRecord>>>,
    edges: Arc<RwLock<Vec<GraphEdgeRecord>>>,
    esg_nodes: Arc<RwLock<HashMap<String, EsgNode>>>,
    esg_edges: Arc<RwLock<Vec<EsgEdge>>>,
    snapshots: Arc<RwLock<HashMap<String, SemanticGraph>>>,
    profiles: Arc<RwLock<HashMap<String, RepositoryProfile>>>,
    deprecations: Arc<RwLock<HashMap<String, DeprecationRecord>>>,
    cases: Arc<RwLock<HashMap<String, MigrationCase>>>,
    contracts: Arc<RwLock<HashMap<String, BehavioralContract>>>,
    runs: Arc<RwLock<Vec<VerificationRecord>>>,
    frameworks: Arc<RwLock<exodus_toolchain::FrameworkRegistry>>,
    language_specs: Arc<RwLock<HashMap<String, TargetLanguageSpecRecord>>>,
}

impl MemoryGraphStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn snapshot(&self) -> PersistedStoreState {
        PersistedStoreState {
            nodes: self.nodes.read().unwrap().clone(),
            edges: self.edges.read().unwrap().clone(),
            esg_nodes: self.esg_nodes.read().unwrap().clone(),
            esg_edges: self.esg_edges.read().unwrap().clone(),
            snapshots: self.snapshots.read().unwrap().clone(),
            profiles: self.profiles.read().unwrap().clone(),
            deprecations: self.deprecations.read().unwrap().clone(),
            cases: self.cases.read().unwrap().clone(),
            contracts: self.contracts.read().unwrap().clone(),
            runs: self.runs.read().unwrap().clone(),
            frameworks: self.frameworks.read().unwrap().clone(),
            language_specs: self.language_specs.read().unwrap().clone(),
        }
    }

    fn restore(&self, state: PersistedStoreState) {
        *self.nodes.write().unwrap() = state.nodes;
        *self.edges.write().unwrap() = state.edges;
        *self.esg_nodes.write().unwrap() = state.esg_nodes;
        *self.esg_edges.write().unwrap() = state.esg_edges;
        *self.snapshots.write().unwrap() = state.snapshots;
        *self.profiles.write().unwrap() = state.profiles;
        *self.deprecations.write().unwrap() = state.deprecations;
        *self.cases.write().unwrap() = state.cases;
        *self.contracts.write().unwrap() = state.contracts;
        *self.runs.write().unwrap() = state.runs;
        *self.frameworks.write().unwrap() = state.frameworks;
        *self.language_specs.write().unwrap() = state.language_specs;
    }

    pub fn load_from_disk_sync(&self, path: &Path) {
        let esg_file = path.join("esg_nodes.json");
        if esg_file.is_file() {
            if let Ok(content) = fs::read_to_string(&esg_file) {
                if let Ok(nodes) = serde_json::from_str::<Vec<EsgNode>>(&content) {
                    let mut map = self.esg_nodes.write().unwrap();
                    for node in nodes {
                        map.insert(node.id.clone(), node);
                    }
                }
            }
        }

        let dep_file = path.join("deprecations.json");
        if dep_file.is_file() {
            if let Ok(content) = fs::read_to_string(&dep_file) {
                if let Ok(deps) = serde_json::from_str::<Vec<DeprecationRecord>>(&content) {
                    let mut map = self.deprecations.write().unwrap();
                    for dep in deps {
                        map.insert(dep.id.clone(), dep);
                    }
                }
            }
        }

        let cases_file = path.join("cases.json");
        if cases_file.is_file() {
            if let Ok(content) = fs::read_to_string(&cases_file) {
                if let Ok(cases) = serde_json::from_str::<Vec<MigrationCase>>(&content) {
                    let mut map = self.cases.write().unwrap();
                    for case in cases {
                        map.insert(case.case_id.clone(), case);
                    }
                }
            }
        }

        let contracts_file = path.join("contracts.json");
        if contracts_file.is_file() {
            if let Ok(content) = fs::read_to_string(&contracts_file) {
                if let Ok(contracts) = serde_json::from_str::<Vec<BehavioralContract>>(&content) {
                    let mut map = self.contracts.write().unwrap();
                    for contract in contracts {
                        map.insert(contract.unit_id.clone(), contract);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl GraphStore for MemoryGraphStore {
    async fn upsert_node(&mut self, node: &GraphNodeRecord) -> Result<()> {
        let mut n = self.nodes.write().unwrap();
        n.insert(node.id.clone(), node.clone());
        Ok(())
    }

    async fn add_edge(&mut self, edge: &GraphEdgeRecord) -> Result<()> {
        let mut e = self.edges.write().unwrap();
        e.push(edge.clone());
        Ok(())
    }

    async fn get_node(&self, id: &str) -> Result<Option<GraphNodeRecord>> {
        let n = self.nodes.read().unwrap();
        Ok(n.get(id).cloned())
    }

    async fn neighbors(&self, id: &str, direction: GraphDirection) -> Result<Vec<GraphNodeRecord>> {
        let e = self.edges.read().unwrap();
        let n = self.nodes.read().unwrap();
        let mut neighbor_ids = Vec::new();

        for edge in e.iter() {
            match direction {
                GraphDirection::Outgoing if edge.from_id == id => neighbor_ids.push(&edge.to_id),
                GraphDirection::Incoming if edge.to_id == id => neighbor_ids.push(&edge.from_id),
                GraphDirection::Both => {
                    if edge.from_id == id {
                        neighbor_ids.push(&edge.to_id);
                    } else if edge.to_id == id {
                        neighbor_ids.push(&edge.from_id);
                    }
                }
                _ => {}
            }
        }

        let mut results = Vec::new();
        for nid in neighbor_ids {
            if let Some(node) = n.get(nid) {
                results.push(node.clone());
            }
        }
        Ok(results)
    }

    async fn create_snapshot(&mut self, snapshot_id: &str, graph: &SemanticGraph) -> Result<()> {
        let mut snaps = self.snapshots.write().unwrap();
        snaps.insert(snapshot_id.to_string(), graph.clone());

        // Upsert nodes and edges into store
        let mut n = self.nodes.write().unwrap();
        let mut e = self.edges.write().unwrap();

        for (id, node) in &graph.nodes {
            n.insert(
                id.clone(),
                GraphNodeRecord {
                    id: id.clone(),
                    name: node.name.clone(),
                    kind: format!("{:?}", node.kind),
                    signature: None,
                    file_path: node.file_path.clone(),
                    is_grounded: true,
                },
            );
        }

        for edge in &graph.edges {
            e.push(GraphEdgeRecord {
                from_id: edge.from.clone(),
                to_id: edge.to.clone(),
                relationship: format!("{:?}", edge.relationship),
                metadata: BTreeMap::new(),
            });
        }

        Ok(())
    }

    async fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<SemanticGraph>> {
        let snaps = self.snapshots.read().unwrap();
        Ok(snaps.get(snapshot_id).cloned())
    }

    async fn diff_snapshots(&self, snap_a: &str, snap_b: &str) -> Result<GraphDiff> {
        let snaps = self.snapshots.read().unwrap();
        let a = snaps.get(snap_a).ok_or_else(|| {
            ExodusError::Generic(format!("Snapshot `{snap_a}` not found in store"))
        })?;
        let b = snaps.get(snap_b).ok_or_else(|| {
            ExodusError::Generic(format!("Snapshot `{snap_b}` not found in store"))
        })?;

        let mut added_nodes = Vec::new();
        let mut removed_nodes = Vec::new();

        for id in b.nodes.keys() {
            if !a.nodes.contains_key(id) {
                added_nodes.push(id.clone());
            }
        }
        for id in a.nodes.keys() {
            if !b.nodes.contains_key(id) {
                removed_nodes.push(id.clone());
            }
        }

        let mut added_edges = Vec::new();
        let mut removed_edges = Vec::new();

        let edge_key = |from: &str, to: &str, rel: &exodus_graph::RelationKind| {
            format!("{from}->{to}:{:?}", rel)
        };

        let mut a_edges = HashMap::new();
        for e in &a.edges {
            a_edges.insert(
                edge_key(&e.from, &e.to, &e.relationship),
                (&e.from, &e.to, format!("{:?}", e.relationship)),
            );
        }

        let mut b_edges = HashMap::new();
        for e in &b.edges {
            b_edges.insert(
                edge_key(&e.from, &e.to, &e.relationship),
                (&e.from, &e.to, format!("{:?}", e.relationship)),
            );
        }

        for (k, (from, to, rel)) in &b_edges {
            if !a_edges.contains_key(k) {
                added_edges.push((from.to_string(), to.to_string(), rel.clone()));
            }
        }
        for (k, (from, to, rel)) in &a_edges {
            if !b_edges.contains_key(k) {
                removed_edges.push((from.to_string(), to.to_string(), rel.clone()));
            }
        }

        Ok(GraphDiff {
            added_nodes,
            removed_nodes,
            added_edges,
            removed_edges,
        })
    }

    async fn upsert_esg_node(&mut self, node: &EsgNode) -> Result<()> {
        let mut map = self.esg_nodes.write().unwrap();
        map.insert(node.id.clone(), node.clone());
        Ok(())
    }

    async fn get_esg_node(&self, id: &str) -> Result<Option<EsgNode>> {
        let map = self.esg_nodes.read().unwrap();
        Ok(map.get(id).cloned())
    }

    async fn list_esg_nodes(&self) -> Result<Vec<EsgNode>> {
        let map = self.esg_nodes.read().unwrap();
        Ok(map.values().cloned().collect())
    }

    async fn add_esg_edge(&mut self, edge: &EsgEdge) -> Result<()> {
        let mut list = self.esg_edges.write().unwrap();
        list.push(edge.clone());
        Ok(())
    }

    async fn list_esg_edges(&self) -> Result<Vec<EsgEdge>> {
        let list = self.esg_edges.read().unwrap();
        Ok(list.clone())
    }
}

#[async_trait]
impl KnowledgeStore for MemoryGraphStore {
    async fn save_case(&mut self, case: &MigrationCase) -> Result<()> {
        let mut c = self.cases.write().unwrap();
        c.insert(case.case_id.clone(), case.clone());
        Ok(())
    }

    async fn get_case(&self, case_id: &str) -> Result<Option<MigrationCase>> {
        let c = self.cases.read().unwrap();
        Ok(c.get(case_id).cloned())
    }

    async fn list_cases(&self) -> Result<Vec<MigrationCase>> {
        let c = self.cases.read().unwrap();
        Ok(c.values().cloned().collect())
    }

    async fn search_promoted_cases(
        &self,
        fingerprint: &str,
        category: &FailureCategory,
    ) -> Result<Vec<MigrationCase>> {
        let c = self.cases.read().unwrap();
        let mut matches = Vec::new();

        for case in c.values() {
            if case.status == CaseStatus::Promoted
                && case.failure_category == *category
                && case.structural_fingerprint == fingerprint
            {
                matches.push(case.clone());
            }
        }

        Ok(matches)
    }

    async fn save_contract(&mut self, contract: &BehavioralContract) -> Result<()> {
        let mut c = self.contracts.write().unwrap();
        c.insert(contract.unit_id.clone(), contract.clone());
        Ok(())
    }

    async fn get_contract(&self, unit_id: &str) -> Result<Option<BehavioralContract>> {
        let c = self.contracts.read().unwrap();
        Ok(c.get(unit_id).cloned())
    }

    async fn list_contracts(&self) -> Result<Vec<BehavioralContract>> {
        let c = self.contracts.read().unwrap();
        Ok(c.values().cloned().collect())
    }

    async fn save_verification_run(&mut self, run: &VerificationRecord) -> Result<()> {
        let mut r = self.runs.write().unwrap();
        r.push(run.clone());
        Ok(())
    }

    async fn save_framework_rule(&mut self, rule: &exodus_toolchain::FrameworkRule) -> Result<()> {
        let mut f = self.frameworks.write().unwrap();
        f.upsert_rule(rule.clone());
        Ok(())
    }

    async fn get_framework_registry(&self) -> Result<exodus_toolchain::FrameworkRegistry> {
        let f = self.frameworks.read().unwrap();
        Ok(f.clone())
    }

    async fn save_profile(&mut self, profile: &RepositoryProfile) -> Result<()> {
        let mut map = self.profiles.write().unwrap();
        map.insert(profile.repository_id.clone(), profile.clone());
        Ok(())
    }

    async fn get_profile(&self, repo_id: &str) -> Result<Option<RepositoryProfile>> {
        let map = self.profiles.read().unwrap();
        Ok(map.get(repo_id).cloned())
    }

    async fn save_deprecation(&mut self, deprecation: &DeprecationRecord) -> Result<()> {
        let mut map = self.deprecations.write().unwrap();
        map.insert(deprecation.id.clone(), deprecation.clone());
        Ok(())
    }

    async fn get_deprecation(&self, id: &str) -> Result<Option<DeprecationRecord>> {
        let map = self.deprecations.read().unwrap();
        Ok(map.get(id).cloned())
    }

    async fn list_deprecations(&self) -> Result<Vec<DeprecationRecord>> {
        let map = self.deprecations.read().unwrap();
        Ok(map.values().cloned().collect())
    }

    async fn export_all(&self, target_dir: &Path) -> Result<ExportManifest> {
        fs::create_dir_all(target_dir).map_err(ExodusError::from)?;

        let cases = self.list_cases().await?;
        let cases_file = target_dir.join("cases.json");
        let cases_json = serde_json::to_string_pretty(&cases).map_err(ExodusError::from)?;
        fs::write(cases_file, cases_json).map_err(ExodusError::from)?;

        let contracts = self.list_contracts().await?;
        let contracts_file = target_dir.join("contracts.json");
        let contracts_json = serde_json::to_string_pretty(&contracts).map_err(ExodusError::from)?;
        fs::write(contracts_file, contracts_json).map_err(ExodusError::from)?;

        let esg_nodes = self.list_esg_nodes().await?;
        let esg_file = target_dir.join("esg_nodes.json");
        let esg_json = serde_json::to_string_pretty(&esg_nodes).map_err(ExodusError::from)?;
        fs::write(esg_file, esg_json).map_err(ExodusError::from)?;

        let deprecations = self.list_deprecations().await?;
        let dep_file = target_dir.join("deprecations.json");
        let dep_json = serde_json::to_string_pretty(&deprecations).map_err(ExodusError::from)?;
        fs::write(dep_file, dep_json).map_err(ExodusError::from)?;

        let frameworks = self.get_framework_registry().await?;
        let fw_file = target_dir.join("framework_rules.json");
        let fw_json = serde_json::to_string_pretty(&frameworks.rules).map_err(ExodusError::from)?;
        fs::write(fw_file, fw_json).map_err(ExodusError::from)?;

        let snaps = self.snapshots.read().unwrap();
        let snaps_dir = target_dir.join("snapshots");
        fs::create_dir_all(&snaps_dir).map_err(ExodusError::from)?;
        for (snap_id, graph) in snaps.iter() {
            let snap_file = snaps_dir.join(format!("{}.json", snap_id));
            let snap_json = serde_json::to_string_pretty(graph).map_err(ExodusError::from)?;
            fs::write(snap_file, snap_json).map_err(ExodusError::from)?;
        }

        let runs = self.runs.read().unwrap();
        let runs_file = target_dir.join("runs.json");
        let runs_json = serde_json::to_string_pretty(&*runs).map_err(ExodusError::from)?;
        fs::write(runs_file, runs_json).map_err(ExodusError::from)?;

        let manifest = ExportManifest {
            schema_version: "4.0.0".to_string(),
            created_at: Utc::now(),
            case_count: cases.len(),
            contract_count: contracts.len(),
            snapshot_count: snaps.len(),
            run_count: runs.len(),
            esg_node_count: esg_nodes.len(),
            deprecation_count: deprecations.len(),
        };

        let manifest_file = target_dir.join("export_manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(ExodusError::from)?;
        fs::write(manifest_file, manifest_json).map_err(ExodusError::from)?;

        Ok(manifest)
    }

    async fn import_all(&mut self, source_dir: &Path) -> Result<ImportReport> {
        let mut report = ImportReport {
            cases_imported: 0,
            contracts_imported: 0,
            snapshots_imported: 0,
            profiles_imported: 0,
            deprecations_imported: 0,
            errors: Vec::new(),
        };

        let cases_file = source_dir.join("cases.json");
        if cases_file.is_file() {
            if let Ok(content) = fs::read_to_string(&cases_file) {
                if let Ok(cases) = serde_json::from_str::<Vec<MigrationCase>>(&content) {
                    for case in cases {
                        self.save_case(&case).await?;
                        report.cases_imported += 1;
                    }
                }
            }
        }

        let contracts_file = source_dir.join("contracts.json");
        if contracts_file.is_file() {
            if let Ok(content) = fs::read_to_string(&contracts_file) {
                if let Ok(contracts) = serde_json::from_str::<Vec<BehavioralContract>>(&content) {
                    for contract in contracts {
                        self.save_contract(&contract).await?;
                        report.contracts_imported += 1;
                    }
                }
            }
        }

        let esg_file = source_dir.join("esg_nodes.json");
        if esg_file.is_file() {
            if let Ok(content) = fs::read_to_string(&esg_file) {
                if let Ok(nodes) = serde_json::from_str::<Vec<EsgNode>>(&content) {
                    for node in nodes {
                        self.upsert_esg_node(&node).await?;
                    }
                }
            }
        }

        let dep_file = source_dir.join("deprecations.json");
        if dep_file.is_file() {
            if let Ok(content) = fs::read_to_string(&dep_file) {
                if let Ok(deps) = serde_json::from_str::<Vec<DeprecationRecord>>(&content) {
                    for dep in deps {
                        self.save_deprecation(&dep).await?;
                        report.deprecations_imported += 1;
                    }
                }
            }
        }

        let fw_file = source_dir.join("framework_rules.json");
        if fw_file.is_file() {
            if let Ok(content) = fs::read_to_string(&fw_file) {
                if let Ok(rules) =
                    serde_json::from_str::<Vec<exodus_toolchain::FrameworkRule>>(&content)
                {
                    for rule in rules {
                        self.save_framework_rule(&rule).await?;
                    }
                }
            }
        }

        let legacy_cases_dir = source_dir.join("cases");
        if legacy_cases_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(legacy_cases_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(case) = serde_json::from_str::<MigrationCase>(&content) {
                                self.save_case(&case).await?;
                                report.cases_imported += 1;
                            }
                        }
                    }
                }
            }
        }

        let legacy_contracts_dir = source_dir.join("contracts");
        if legacy_contracts_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(legacy_contracts_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(contract) =
                                serde_json::from_str::<BehavioralContract>(&content)
                            {
                                self.save_contract(&contract).await?;
                                report.contracts_imported += 1;
                            }
                        }
                    }
                }
            }
        }

        let snaps_dir = source_dir.join("snapshots");
        if snaps_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(snaps_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        let snap_id = entry
                            .path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_string();
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(graph) = serde_json::from_str::<SemanticGraph>(&content) {
                                self.create_snapshot(&snap_id, &graph).await?;
                                report.snapshots_imported += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(report)
    }
}

#[async_trait]
impl TargetLanguageCatalog for MemoryGraphStore {
    async fn get_language_spec(&self, query: &str) -> Result<Option<TargetLanguageSpecRecord>> {
        Ok(self
            .language_specs
            .read()
            .unwrap()
            .values()
            .find(|spec| spec.matches_query(query))
            .cloned())
    }

    async fn list_language_specs(&self) -> Result<Vec<TargetLanguageSpecRecord>> {
        let mut specs: Vec<_> = self
            .language_specs
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect();
        specs.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(specs)
    }

    async fn upsert_language_spec(&mut self, spec: &TargetLanguageSpecRecord) -> Result<()> {
        self.language_specs
            .write()
            .unwrap()
            .insert(spec.id.clone(), spec.clone());
        Ok(())
    }

    async fn reset_language_specs(&mut self) -> Result<()> {
        *self.language_specs.write().unwrap() = TargetLanguageSpecRecord::default_specs()
            .into_iter()
            .map(|spec| (spec.id.clone(), spec))
            .collect();
        Ok(())
    }

    async fn ensure_language_specs_seeded(&mut self) -> Result<()> {
        if self.language_specs.read().unwrap().is_empty() {
            self.reset_language_specs().await?;
        }
        Ok(())
    }
}

/// Embedded SurrealDB store managing durable persistence under `.exodus/data/surreal/`.
#[derive(Clone)]
pub struct SurrealGraphStore {
    db_path: PathBuf,
    db: Surreal<Db>,
    memory_backend: MemoryGraphStore,
    operational_backend: EmbeddedOperationalStore,
    schema_version: u32,
    is_durable_file_backed: bool,
}

impl SurrealGraphStore {
    const NAMESPACE: &'static str = "exodus";
    const DATABASE: &'static str = "living_memory";
    const STATE_TABLE: &'static str = "embedded_state";
    const STATE_ID: &'static str = "current";

    fn storage_error(context: &str, error: impl std::fmt::Display) -> ExodusError {
        ExodusError::Generic(format!("{context}: {error}"))
    }

    /// Opens or initializes a local SurrealKV database backed by disk storage.
    pub async fn open(db_path: impl Into<PathBuf>) -> Result<Self> {
        let path = db_path.into();
        fs::create_dir_all(&path).map_err(ExodusError::from)?;
        let endpoint = path.to_string_lossy().into_owned();
        let db = Surreal::new::<SurrealKv>(endpoint)
            .await
            .map_err(|error| Self::storage_error("failed to open embedded SurrealKV", error))?;
        db.use_ns(Self::NAMESPACE)
            .use_db(Self::DATABASE)
            .await
            .map_err(|error| Self::storage_error("failed to select SurrealDB namespace", error))?;

        let mut store = Self {
            db_path: path.clone(),
            db,
            memory_backend: MemoryGraphStore::new(),
            operational_backend: EmbeddedOperationalStore::new(),
            schema_version: 4,
            is_durable_file_backed: true,
        };

        if !store.restore_from_surreal().await? {
            // One-time, read-only import path for artifacts emitted by the pre-Surreal mock.
            store.memory_backend.load_from_disk_sync(&path);
        }
        store.ensure_language_specs_seeded().await?;
        store.flush_to_disk().await?;

        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn is_durable(&self) -> bool {
        self.is_durable_file_backed
    }

    /// Initializes an embedded SurrealDB memory engine for deterministic unit tests.
    pub async fn open_in_memory() -> Result<Self> {
        let db = Surreal::new::<Mem>(()).await.map_err(|error| {
            Self::storage_error("failed to open SurrealDB memory engine", error)
        })?;
        db.use_ns(Self::NAMESPACE)
            .use_db(Self::DATABASE)
            .await
            .map_err(|error| Self::storage_error("failed to select SurrealDB namespace", error))?;
        let mut store = Self {
            db_path: PathBuf::from(":memory:"),
            db,
            memory_backend: MemoryGraphStore::new(),
            operational_backend: EmbeddedOperationalStore::new(),
            schema_version: 4,
            is_durable_file_backed: false,
        };
        store.ensure_language_specs_seeded().await?;
        Ok(store)
    }

    /// Returns current schema migration version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    async fn restore_from_surreal(&self) -> Result<bool> {
        let record: Option<PersistedStateRecord> = self
            .db
            .select((Self::STATE_TABLE, Self::STATE_ID))
            .await
            .map_err(|error| {
                Self::storage_error("failed to load embedded SurrealDB state", error)
            })?;
        if let Some(record) = record {
            self.memory_backend.restore(record.state);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Atomically writes the current state into the embedded SurrealDB engine.
    pub async fn flush_to_disk(&self) -> Result<()> {
        let record = PersistedStateRecord {
            schema_version: self.schema_version,
            state: self.memory_backend.snapshot(),
        };
        let _: Option<PersistedStateRecord> = self
            .db
            .upsert((Self::STATE_TABLE, Self::STATE_ID))
            .content(record)
            .await
            .map_err(|error| {
                Self::storage_error("failed to commit embedded SurrealDB state", error)
            })?;
        Ok(())
    }

    /// Validates round-trip consistency between Rust domain graph and stored graph.
    pub async fn validate_graph_roundtrip(&self, original: &SemanticGraph) -> Result<bool> {
        let snap_id = format!("rt-check-{}", uuid::Uuid::new_v4());
        let mut store = self.clone();
        store.create_snapshot(&snap_id, original).await?;
        let loaded = store.load_snapshot(&snap_id).await?;

        if let Some(reloaded) = loaded {
            let ok = reloaded.nodes.len() == original.nodes.len()
                && reloaded.edges.len() == original.edges.len();
            Ok(ok)
        } else {
            Ok(false)
        }
    }
}

#[async_trait]
impl GraphStore for SurrealGraphStore {
    async fn upsert_node(&mut self, node: &GraphNodeRecord) -> Result<()> {
        self.memory_backend.upsert_node(node).await?;
        self.flush_to_disk().await
    }

    async fn add_edge(&mut self, edge: &GraphEdgeRecord) -> Result<()> {
        self.memory_backend.add_edge(edge).await?;
        self.flush_to_disk().await
    }

    async fn get_node(&self, id: &str) -> Result<Option<GraphNodeRecord>> {
        self.memory_backend.get_node(id).await
    }

    async fn neighbors(&self, id: &str, direction: GraphDirection) -> Result<Vec<GraphNodeRecord>> {
        self.memory_backend.neighbors(id, direction).await
    }

    async fn create_snapshot(&mut self, snapshot_id: &str, graph: &SemanticGraph) -> Result<()> {
        self.memory_backend
            .create_snapshot(snapshot_id, graph)
            .await?;
        self.flush_to_disk().await
    }

    async fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<SemanticGraph>> {
        self.memory_backend.load_snapshot(snapshot_id).await
    }

    async fn diff_snapshots(&self, snap_a: &str, snap_b: &str) -> Result<GraphDiff> {
        self.memory_backend.diff_snapshots(snap_a, snap_b).await
    }

    async fn upsert_esg_node(&mut self, node: &EsgNode) -> Result<()> {
        self.memory_backend.upsert_esg_node(node).await?;
        self.flush_to_disk().await
    }

    async fn get_esg_node(&self, id: &str) -> Result<Option<EsgNode>> {
        self.memory_backend.get_esg_node(id).await
    }

    async fn list_esg_nodes(&self) -> Result<Vec<EsgNode>> {
        self.memory_backend.list_esg_nodes().await
    }

    async fn add_esg_edge(&mut self, edge: &EsgEdge) -> Result<()> {
        self.memory_backend.add_esg_edge(edge).await?;
        self.flush_to_disk().await
    }

    async fn list_esg_edges(&self) -> Result<Vec<EsgEdge>> {
        self.memory_backend.list_esg_edges().await
    }
}

#[async_trait]
impl KnowledgeStore for SurrealGraphStore {
    async fn save_case(&mut self, case: &MigrationCase) -> Result<()> {
        self.memory_backend.save_case(case).await?;
        self.flush_to_disk().await
    }

    async fn get_case(&self, case_id: &str) -> Result<Option<MigrationCase>> {
        self.memory_backend.get_case(case_id).await
    }

    async fn list_cases(&self) -> Result<Vec<MigrationCase>> {
        self.memory_backend.list_cases().await
    }

    async fn search_promoted_cases(
        &self,
        fingerprint: &str,
        category: &FailureCategory,
    ) -> Result<Vec<MigrationCase>> {
        self.memory_backend
            .search_promoted_cases(fingerprint, category)
            .await
    }

    async fn save_contract(&mut self, contract: &BehavioralContract) -> Result<()> {
        self.memory_backend.save_contract(contract).await?;
        self.flush_to_disk().await
    }

    async fn get_contract(&self, unit_id: &str) -> Result<Option<BehavioralContract>> {
        self.memory_backend.get_contract(unit_id).await
    }

    async fn list_contracts(&self) -> Result<Vec<BehavioralContract>> {
        self.memory_backend.list_contracts().await
    }

    async fn save_verification_run(&mut self, run: &VerificationRecord) -> Result<()> {
        self.memory_backend.save_verification_run(run).await?;
        self.flush_to_disk().await
    }

    async fn save_framework_rule(&mut self, rule: &exodus_toolchain::FrameworkRule) -> Result<()> {
        self.memory_backend.save_framework_rule(rule).await?;
        self.flush_to_disk().await
    }

    async fn get_framework_registry(&self) -> Result<exodus_toolchain::FrameworkRegistry> {
        self.memory_backend.get_framework_registry().await
    }

    async fn export_all(&self, target_dir: &Path) -> Result<ExportManifest> {
        self.memory_backend.export_all(target_dir).await
    }

    async fn import_all(&mut self, source_dir: &Path) -> Result<ImportReport> {
        self.memory_backend.import_all(source_dir).await
    }

    async fn save_profile(&mut self, profile: &RepositoryProfile) -> Result<()> {
        self.memory_backend.save_profile(profile).await?;
        self.flush_to_disk().await
    }

    async fn get_profile(&self, repo_id: &str) -> Result<Option<RepositoryProfile>> {
        self.memory_backend.get_profile(repo_id).await
    }

    async fn save_deprecation(&mut self, deprecation: &DeprecationRecord) -> Result<()> {
        self.memory_backend.save_deprecation(deprecation).await?;
        self.flush_to_disk().await
    }

    async fn get_deprecation(&self, id: &str) -> Result<Option<DeprecationRecord>> {
        self.memory_backend.get_deprecation(id).await
    }

    async fn list_deprecations(&self) -> Result<Vec<DeprecationRecord>> {
        self.memory_backend.list_deprecations().await
    }
}

#[async_trait]
impl TargetLanguageCatalog for SurrealGraphStore {
    async fn get_language_spec(&self, query: &str) -> Result<Option<TargetLanguageSpecRecord>> {
        self.memory_backend.get_language_spec(query).await
    }

    async fn list_language_specs(&self) -> Result<Vec<TargetLanguageSpecRecord>> {
        self.memory_backend.list_language_specs().await
    }

    async fn upsert_language_spec(&mut self, spec: &TargetLanguageSpecRecord) -> Result<()> {
        self.memory_backend.upsert_language_spec(spec).await?;
        self.flush_to_disk().await
    }

    async fn reset_language_specs(&mut self) -> Result<()> {
        self.memory_backend.reset_language_specs().await?;
        self.flush_to_disk().await
    }

    async fn ensure_language_specs_seeded(&mut self) -> Result<()> {
        self.memory_backend.ensure_language_specs_seeded().await?;
        self.flush_to_disk().await
    }
}

#[async_trait]
impl OperationalStore for SurrealGraphStore {
    async fn save_operational_item(&mut self, item: &OperationalItem) -> Result<()> {
        self.operational_backend.save_operational_item(item).await
    }

    async fn get_operational_item(&self, id: &str) -> Result<Option<OperationalItem>> {
        self.operational_backend.get_operational_item(id).await
    }

    async fn list_operational_items(&self) -> Result<Vec<OperationalItem>> {
        self.operational_backend.list_operational_items().await
    }

    async fn list_operational_items_by_domain(
        &self,
        domain: OperationalDomainTag,
    ) -> Result<Vec<OperationalItem>> {
        self.operational_backend
            .list_operational_items_by_domain(domain)
            .await
    }

    async fn list_operational_items_by_state(
        &self,
        state: OperationalLifecycleState,
    ) -> Result<Vec<OperationalItem>> {
        self.operational_backend
            .list_operational_items_by_state(state)
            .await
    }

    async fn save_crm_account(&mut self, account: &CrmAccountRecord) -> Result<()> {
        self.operational_backend.save_crm_account(account).await
    }

    async fn get_crm_account(&self, account_id: &str) -> Result<Option<CrmAccountRecord>> {
        self.operational_backend.get_crm_account(account_id).await
    }

    async fn list_crm_accounts(&self) -> Result<Vec<CrmAccountRecord>> {
        self.operational_backend.list_crm_accounts().await
    }

    async fn save_crm_policy(&mut self, policy: &CrmPolicyRule) -> Result<()> {
        self.operational_backend.save_crm_policy(policy).await
    }

    async fn list_crm_policies(&self) -> Result<Vec<CrmPolicyRule>> {
        self.operational_backend.list_crm_policies().await
    }

    async fn evaluate_crm_discount(
        &self,
        account_id: &str,
        requested_discount_pct: f64,
        requester_role: &str,
    ) -> Result<CrmEvaluationReport> {
        self.operational_backend
            .evaluate_crm_discount(account_id, requested_discount_pct, requester_role)
            .await
    }

    async fn save_taxonomy_node(&mut self, node: &TaxonomyNodeRecord) -> Result<()> {
        self.operational_backend.save_taxonomy_node(node).await
    }

    async fn get_taxonomy_node(&self, id: &str) -> Result<Option<TaxonomyNodeRecord>> {
        self.operational_backend.get_taxonomy_node(id).await
    }

    async fn list_taxonomy_nodes(&self) -> Result<Vec<TaxonomyNodeRecord>> {
        self.operational_backend.list_taxonomy_nodes().await
    }

    async fn save_survey_feedback(&mut self, feedback: &SurveyFeedbackRecord) -> Result<()> {
        self.operational_backend
            .save_survey_feedback(feedback)
            .await
    }

    async fn list_survey_feedbacks(&self) -> Result<Vec<SurveyFeedbackRecord>> {
        self.operational_backend.list_survey_feedbacks().await
    }

    async fn match_feedback_to_taxonomy(
        &self,
        feedback_text: &str,
    ) -> Result<Option<(TaxonomyNodeRecord, f64)>> {
        self.operational_backend
            .match_feedback_to_taxonomy(feedback_text)
            .await
    }

    async fn get_sdlc_settings(&self) -> Result<SdlcIntegrationSettings> {
        self.operational_backend.get_sdlc_settings().await
    }

    async fn save_sdlc_settings(&mut self, settings: &SdlcIntegrationSettings) -> Result<()> {
        self.operational_backend.save_sdlc_settings(settings).await
    }

    async fn generate_pipeline_plugin_scaffold(&self) -> Result<HashMap<String, String>> {
        self.operational_backend
            .generate_pipeline_plugin_scaffold()
            .await
    }

    async fn ingest_ci_failure_event(
        &mut self,
        event: CiFailureEventPayload,
    ) -> Result<OperationalItem> {
        self.operational_backend
            .ingest_ci_failure_event(event)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_graph_store_snapshot_roundtrip() {
        let mut store = MemoryGraphStore::new();
        let mut graph = SemanticGraph::new();
        graph.add_node(exodus_graph::SemanticNode {
            id: "function::test".to_string(),
            name: "test".to_string(),
            kind: exodus_graph::NodeKind::Function,
            qualified_name: "test".to_string(),
            file_path: "test.py".to_string(),
            risk_score: 10,
            risk_level: exodus_core::RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        store.create_snapshot("snap-1", &graph).await.unwrap();
        let loaded = store.load_snapshot("snap-1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_surreal_graph_store_initialization() {
        let store = SurrealGraphStore::open_in_memory().await.unwrap();
        assert_eq!(store.schema_version(), 4);
        assert!(!store.is_durable());
    }

    #[tokio::test]
    async fn test_surreal_file_backed_instance_reopen_durability() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("data/surreal");

        // 1. First instance: write ESG node & deprecation record
        {
            let mut store = SurrealGraphStore::open(&db_path).await.unwrap();
            assert!(store.is_durable());

            let esg_node = EsgNode::new(
                "urn:sym:pkg::test_fn",
                exodus_core::LanguageId::new("python"),
                exodus_core::EsgNodeKind::Function,
                "test_fn",
            );
            store.upsert_esg_node(&esg_node).await.unwrap();

            let dep = DeprecationRecord::new(
                "dep-001",
                exodus_core::LanguageId::new("python"),
                "os.popen",
                exodus_core::DeprecationStatus::Deprecated,
                exodus_core::DeprecationEvidenceSource::CompilerWarning,
            );
            store.save_deprecation(&dep).await.unwrap();
            // store dropped here
        }

        // 2. Second instance: open same db_path and verify retrieval
        {
            let store2 = SurrealGraphStore::open(&db_path).await.unwrap();
            let loaded_node = store2.get_esg_node("urn:sym:pkg::test_fn").await.unwrap();
            assert!(loaded_node.is_some());
            assert_eq!(loaded_node.unwrap().name, "test_fn");

            let loaded_dep = store2.get_deprecation("dep-001").await.unwrap();
            assert!(loaded_dep.is_some());
            assert_eq!(loaded_dep.unwrap().target_symbol, "os.popen");
        }
    }

    #[tokio::test]
    async fn test_surreal_graph_store_operational_store() {
        let mut store = SurrealGraphStore::open_in_memory().await.unwrap();

        let payload = exodus_core::DomainPayload::CrmRequest(exodus_core::CrmRequestPayload::new(
            "acc-enterprise-99",
            "MegaCorp",
            500_000.0,
            20.0,
            "Enterprise",
            "SalesLead",
        ));

        let item = OperationalItem::new(
            "Enterprise Renewal Discount",
            "Customer requesting 20% discount on renewal",
            "sales-lead",
            payload,
        );

        let id = item.id.clone();
        store.save_operational_item(&item).await.unwrap();

        let loaded = store.get_operational_item(&id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().domain_tag, OperationalDomainTag::CrmRequest);

        // Verify CRM policy evaluation through SurrealGraphStore
        let eval = store
            .evaluate_crm_discount("acc-enterprise-99", 20.0, "SalesLead")
            .await
            .unwrap();
        assert!(eval.compliant);

        // Verify SDLC scaffolding through SurrealGraphStore
        let scaffold = store.generate_pipeline_plugin_scaffold().await.unwrap();
        assert!(scaffold.contains_key(".github/workflows/exodus-verify.yml"));
    }
}
