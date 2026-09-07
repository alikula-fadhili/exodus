//! Exodus Semantic Graph (ESG) structures, node definitions, and graph algorithms.

use exodus_core::{ExodusError, Result, RiskLevel, SourceEvidence};
use exodus_parser::ParsedRepository;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};

/// Semantic node category in the ESG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Repository,
    Module,
    Type,
    Function,
    Method,
    Parameter,
    State,
    ExternalDependency,
    UnsupportedConstruct,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Relationship between nodes in the ESG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    Contains,
    Imports,
    Calls,
    Reads,
    Writes,
    Returns,
    Accepts,
    Inherits,
    DependsOn,
    Implements,
    UnknownRelation,
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A discrete node in the Exodus Semantic Graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub qualified_name: String,
    pub file_path: String,
    pub risk_score: u8, // 1 - 100
    pub risk_level: RiskLevel,
    pub evidence: Option<SourceEvidence>,
    pub metadata: HashMap<String, String>,
}

impl SemanticNode {
    /// Content-derived hash of this node's current source evidence.
    ///
    /// `id` (`kind::qualified_name`) is the node's *stable identity* — it survives a re-parse of
    /// unchanged code and is what contracts, cases, and commits key off of. It does not, however,
    /// change when only the function/method/class *body* changes. `content_hash` is the
    /// complementary signal: it changes whenever the underlying source text changes, so a unit
    /// gate can tell "same identity, but the implementation moved since the last verified run"
    /// apart from "genuinely unchanged, verified evidence still applies."
    pub fn content_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.kind.hash(&mut hasher);
        self.qualified_name.hash(&mut hasher);
        match self.evidence.as_ref().and_then(|e| e.snippet.as_deref()) {
            Some(snippet) => snippet.hash(&mut hasher),
            None => self.file_path.hash(&mut hasher),
        }
        format!("{:016x}", hasher.finish())
    }
}

/// A directed relationship edge in the Exodus Semantic Graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticEdge {
    pub from: String,
    pub to: String,
    pub relationship: RelationKind,
    pub evidence: Option<SourceEvidence>,
}

/// The smallest boundary at which migration and verification can honestly happen: either a single
/// node that compiles/verifies in isolation, or — when nodes form a dependency cycle that cannot be
/// isolated — the smallest strongly-connected cluster that must be migrated and verified together.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationBoundary {
    /// A single node that can be migrated and verified independently of its SCC-mates.
    Unit { node_id: String },
    /// Multiple nodes that form a strongly connected component and therefore cannot be compiled or
    /// verified independently — the cluster itself is the smallest valid verification boundary.
    Cluster {
        node_ids: Vec<String>,
        reason: String,
    },
}

impl VerificationBoundary {
    /// All node IDs contained in this boundary (one for `Unit`, all cluster members for `Cluster`).
    pub fn node_ids(&self) -> Vec<String> {
        match self {
            Self::Unit { node_id } => vec![node_id.clone()],
            Self::Cluster { node_ids, .. } => node_ids.clone(),
        }
    }

    pub fn is_cluster(&self) -> bool {
        matches!(self, Self::Cluster { .. })
    }
}

/// High-level architecture summary produced from the ESG.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchitectureSummary {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub module_count: usize,
    pub type_count: usize,
    pub function_count: usize,
    pub unsupported_count: usize,
    pub circular_dependencies: Vec<Vec<String>>,
    pub high_risk_nodes: Vec<String>,
}

/// The core language-neutral in-memory Exodus Semantic Graph.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SemanticGraph {
    pub nodes: HashMap<String, SemanticNode>,
    pub edges: Vec<SemanticEdge>,
}

impl SemanticGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SemanticNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: SemanticEdge) {
        self.edges.push(edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&SemanticNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut SemanticNode> {
        self.nodes.get_mut(id)
    }

    pub fn fan_out(&self, node_id: &str) -> usize {
        self.edges.iter().filter(|e| e.from == node_id).count()
    }

    pub fn fan_in(&self, node_id: &str) -> usize {
        self.edges.iter().filter(|e| e.to == node_id).count()
    }

    pub fn dependencies_of(&self, node_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| {
                e.from == node_id
                    && matches!(
                        e.relationship,
                        RelationKind::Calls
                            | RelationKind::DependsOn
                            | RelationKind::Imports
                            | RelationKind::Inherits
                    )
            })
            .map(|e| e.to.clone())
            .collect()
    }

    pub fn dependents_of(&self, node_id: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| {
                e.to == node_id
                    && matches!(
                        e.relationship,
                        RelationKind::Calls
                            | RelationKind::DependsOn
                            | RelationKind::Imports
                            | RelationKind::Inherits
                    )
            })
            .map(|e| e.from.clone())
            .collect()
    }

    /// Computes Strongly Connected Components (SCCs) using Tarjan's algorithm.
    pub fn find_sccs(&self) -> Vec<Vec<String>> {
        let mut index = 0;
        let mut stack = Vec::new();
        let mut on_stack = HashSet::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut lowlink: HashMap<String, usize> = HashMap::new();
        let mut sccs = Vec::new();

        // Build adjacency map for dependency edges
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.edges {
            if matches!(
                edge.relationship,
                RelationKind::Calls
                    | RelationKind::DependsOn
                    | RelationKind::Imports
                    | RelationKind::Inherits
            ) {
                adj.entry(&edge.from).or_default().push(&edge.to);
            }
        }

        for node_id in self.nodes.keys() {
            if !indices.contains_key(node_id) {
                Self::strongconnect(
                    node_id,
                    &adj,
                    &mut index,
                    &mut stack,
                    &mut on_stack,
                    &mut indices,
                    &mut lowlink,
                    &mut sccs,
                );
            }
        }

        sccs
    }

    #[allow(clippy::too_many_arguments)]
    fn strongconnect<'a>(
        v: &'a str,
        adj: &HashMap<&str, Vec<&'a str>>,
        index: &mut usize,
        stack: &mut Vec<&'a str>,
        on_stack: &mut HashSet<&'a str>,
        indices: &mut HashMap<String, usize>,
        lowlink: &mut HashMap<String, usize>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(v.to_string(), *index);
        lowlink.insert(v.to_string(), *index);
        *index += 1;
        stack.push(v);
        on_stack.insert(v);

        if let Some(neighbors) = adj.get(v) {
            for &w in neighbors {
                if !indices.contains_key(w) {
                    Self::strongconnect(w, adj, index, stack, on_stack, indices, lowlink, sccs);
                    let w_low = lowlink[w];
                    let v_low = lowlink.get_mut(v).unwrap();
                    *v_low = (*v_low).min(w_low);
                } else if on_stack.contains(w) {
                    let w_idx = indices[w];
                    let v_low = lowlink.get_mut(v).unwrap();
                    *v_low = (*v_low).min(w_idx);
                }
            }
        }

        if lowlink[v] == indices[v] {
            let mut scc = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack.remove(w);
                scc.push(w.to_string());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                sccs.push(scc);
            }
        }
    }

    /// Detects dependency cycles in the graph (SCCs with size > 1).
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        self.find_sccs()
    }

    /// Extracts the minimal subgraph relevant to a single node: the node itself, its immediate
    /// (1-hop) dependencies and dependents, and only the edges directly connecting them. Used to
    /// scope Case Engine capture and repair-agent context to what a unit failure actually
    /// implicates, instead of serializing the entire repository graph.
    pub fn relevant_subgraph(&self, node_id: &str) -> SemanticGraph {
        let mut sub = SemanticGraph::new();
        let Some(center) = self.nodes.get(node_id) else {
            return sub;
        };

        let mut related_ids: HashSet<String> = HashSet::new();
        related_ids.insert(node_id.to_string());
        for id in self
            .dependencies_of(node_id)
            .into_iter()
            .chain(self.dependents_of(node_id))
        {
            related_ids.insert(id);
        }

        sub.add_node(center.clone());
        for id in &related_ids {
            if id != node_id {
                if let Some(n) = self.nodes.get(id) {
                    sub.add_node(n.clone());
                }
            }
        }

        for edge in &self.edges {
            if related_ids.contains(&edge.from) && related_ids.contains(&edge.to) {
                sub.add_edge(edge.clone());
            }
        }

        sub
    }

    /// Decomposes the graph's migratable nodes (functions, methods, and types/classes) into
    /// dependency-ordered verification boundaries: a single node wherever it can be verified in
    /// isolation, or the smallest strongly-connected cluster of migratable nodes when a cycle makes
    /// finer isolation impossible. Boundaries are emitted so that, on the acyclic portion of the
    /// graph, a boundary's dependencies appear before it — mirroring `topological_sort`.
    pub fn verification_units(&self) -> Vec<VerificationBoundary> {
        fn is_migratable(kind: NodeKind) -> bool {
            matches!(kind, NodeKind::Function | NodeKind::Method | NodeKind::Type)
        }

        fn find(parent: &mut HashMap<String, String>, x: &str) -> String {
            let mut root = x.to_string();
            while parent[&root] != root {
                root = parent[&root].clone();
            }
            let mut cur = x.to_string();
            while parent[&cur] != root {
                let next = parent[&cur].clone();
                parent.insert(cur, root.clone());
                cur = next;
            }
            root
        }

        fn union(parent: &mut HashMap<String, String>, a: &str, b: &str) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent.insert(ra, rb);
            }
        }

        let migratable_ids: Vec<String> = self
            .nodes
            .values()
            .filter(|n| is_migratable(n.kind))
            .map(|n| n.id.clone())
            .collect();
        let mut parent: HashMap<String, String> = migratable_ids
            .iter()
            .map(|id| (id.clone(), id.clone()))
            .collect();

        // Union nodes that are part of a real dependency cycle (SCC) — they cannot be compiled or
        // verified independently of each other.
        let mut cycle_reason_members: HashSet<String> = HashSet::new();
        for scc in self.find_sccs() {
            let members: Vec<String> = scc
                .into_iter()
                .filter(|id| parent.contains_key(id))
                .collect();
            if members.len() > 1 {
                for pair in members.windows(2) {
                    union(&mut parent, &pair[0], &pair[1]);
                }
                cycle_reason_members.extend(members);
            }
        }

        // Union every method with its containing class — a method cannot compile outside its
        // struct's `impl` block, so a class and its methods are always one verification boundary
        // regardless of whether any cycle is involved.
        let mut class_reason_members: HashSet<String> = HashSet::new();
        for edge in &self.edges {
            if edge.relationship != RelationKind::Contains {
                continue;
            }
            let is_class_to_method = self
                .nodes
                .get(&edge.from)
                .is_some_and(|n| n.kind == NodeKind::Type)
                && self
                    .nodes
                    .get(&edge.to)
                    .is_some_and(|n| n.kind == NodeKind::Method);
            if is_class_to_method {
                union(&mut parent, &edge.from, &edge.to);
                class_reason_members.insert(edge.from.clone());
                class_reason_members.insert(edge.to.clone());
            }
        }

        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for id in &migratable_ids {
            let root = find(&mut parent, id);
            groups.entry(root).or_default().push(id.clone());
        }

        let order = self.topological_sort().unwrap_or_default();
        let order_rank: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();

        let mut group_list: Vec<Vec<String>> = groups.into_values().collect();
        for members in &mut group_list {
            members.sort();
        }
        // Order groups by the minimum topological rank among their members: dependency-respecting
        // on the acyclic portion of the graph, deterministic otherwise.
        group_list.sort_by_key(|members| {
            members
                .iter()
                .filter_map(|id| order_rank.get(id.as_str()))
                .min()
                .copied()
                .unwrap_or(usize::MAX)
        });

        group_list
            .into_iter()
            .map(|node_ids| {
                if node_ids.len() == 1 {
                    VerificationBoundary::Unit {
                        node_id: node_ids.into_iter().next().unwrap(),
                    }
                } else {
                    let is_cycle = node_ids.iter().any(|id| cycle_reason_members.contains(id));
                    let is_class = node_ids.iter().any(|id| class_reason_members.contains(id));
                    let member_count = node_ids.len();
                    let reason = match (is_cycle, is_class) {
                        (true, true) => format!(
                            "smallest independently compilable unit — {member_count} nodes are linked by both a dependency cycle and class/method membership and cannot be verified in isolation"
                        ),
                        (true, false) => format!(
                            "smallest independently compilable dependency cluster — {member_count} nodes form a strongly connected component and cannot be verified in isolation"
                        ),
                        (false, true) => format!(
                            "class and its methods compile as a single unit — {member_count} nodes cannot exist independently of their struct's impl block"
                        ),
                        (false, false) => format!("{member_count} nodes grouped for verification"),
                    };
                    VerificationBoundary::Cluster { node_ids, reason }
                }
            })
            .collect()
    }

    /// Computes Kahn's topological sort for DAG nodes (ordering dependencies first).
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        for id in self.nodes.keys() {
            in_degree.insert(id.clone(), 0);
            adj.insert(id.clone(), Vec::new());
        }

        for edge in &self.edges {
            if self.nodes.contains_key(&edge.from) && self.nodes.contains_key(&edge.to) {
                // Dependency: if A depends on B (e.g. A calls B), B should be migrated before A.
                // Edge: B -> A
                adj.entry(edge.to.clone())
                    .or_default()
                    .push(edge.from.clone());
                *in_degree.entry(edge.from.clone()).or_default() += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted = Vec::new();

        while let Some(u) = queue.pop_front() {
            sorted.push(u.clone());
            if let Some(neighbors) = adj.get(&u) {
                for v in neighbors {
                    if let Some(deg) = in_degree.get_mut(v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v.clone());
                        }
                    }
                }
            }
        }

        // If cycle exists, append remaining nodes deterministically
        if sorted.len() < self.nodes.len() {
            for id in self.nodes.keys() {
                if !sorted.contains(id) {
                    sorted.push(id.clone());
                }
            }
        }

        Ok(sorted)
    }

    /// Computes composite risk scores (1 - 100) and risk levels for all nodes.
    pub fn compute_risk_scores(&mut self) {
        let cycles = self.detect_cycles();
        let cycle_nodes: HashSet<String> = cycles.into_iter().flatten().collect();

        let node_ids: Vec<String> = self.nodes.keys().cloned().collect();

        for id in node_ids {
            let fan_in = self.fan_in(&id);
            let fan_out = self.fan_out(&id);
            let in_cycle = cycle_nodes.contains(&id);

            if let Some(node) = self.nodes.get_mut(&id) {
                let mut score: u32 = 10; // Baseline score

                // Factor 1: Fan-in (impact on dependents)
                score += (fan_in as u32 * 5).min(25);

                // Factor 2: Fan-out (coupling to dependencies)
                score += (fan_out as u32 * 5).min(25);

                // Factor 3: In a cyclic dependency
                if in_cycle {
                    score += 30;
                }

                // Factor 4: Node kind risk
                match node.kind {
                    NodeKind::UnsupportedConstruct => score += 40,
                    NodeKind::ExternalDependency => score += 15,
                    _ => {}
                }

                let final_score = (score.min(100)) as u8;
                node.risk_score = final_score;
                node.risk_level = match final_score {
                    0..=30 => RiskLevel::Low,
                    31..=60 => RiskLevel::Medium,
                    61..=85 => RiskLevel::High,
                    _ => RiskLevel::Critical,
                };
            }
        }
    }

    /// Builds an ESG instance from a parsed repository representation.
    pub fn from_parsed_repository(parsed: &ParsedRepository) -> Self {
        let mut graph = Self::new();
        // Populated as functions/methods are added below, then used in a second pass to resolve
        // `CallExpr.callee` (raw call-site text, e.g. "get_price" or "obj.get_price") into real
        // qualified node IDs. This can only run after every module's nodes exist, since a call may
        // reference a function defined in a module visited later in `parsed.modules`.
        let mut qualified_id_by_name: HashMap<String, String> = HashMap::new();
        let mut candidates_by_bare_name: HashMap<String, Vec<String>> = HashMap::new();

        // 1. Add Repository Node
        let repo_id = "repo::root".to_string();
        graph.add_node(SemanticNode {
            id: repo_id.clone(),
            name: parsed.root_path.display().to_string(),
            kind: NodeKind::Repository,
            qualified_name: "root".to_string(),
            file_path: parsed.root_path.display().to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        // 2. Add Modules, Types, Functions, Unsupported Constructs
        for module in &parsed.modules {
            let mod_id = format!("module::{}", module.module_name);
            graph.add_node(SemanticNode {
                id: mod_id.clone(),
                name: module.module_name.clone(),
                kind: NodeKind::Module,
                qualified_name: module.module_name.clone(),
                file_path: module.file_path.display().to_string(),
                risk_score: 10,
                risk_level: RiskLevel::Low,
                evidence: None,
                metadata: HashMap::new(),
            });

            graph.add_edge(SemanticEdge {
                from: repo_id.clone(),
                to: mod_id.clone(),
                relationship: RelationKind::Contains,
                evidence: None,
            });

            // Add classes / types
            for class in &module.classes {
                let class_id = format!("type::{}", class.qualified_name);
                graph.add_node(SemanticNode {
                    id: class_id.clone(),
                    name: class.name.clone(),
                    kind: NodeKind::Type,
                    qualified_name: class.qualified_name.clone(),
                    file_path: module.file_path.display().to_string(),
                    risk_score: 20,
                    risk_level: RiskLevel::Low,
                    evidence: Some(class.evidence.clone()),
                    metadata: HashMap::new(),
                });

                graph.add_edge(SemanticEdge {
                    from: mod_id.clone(),
                    to: class_id.clone(),
                    relationship: RelationKind::Contains,
                    evidence: Some(class.evidence.clone()),
                });

                for base in &class.base_classes {
                    let base_id = format!("type::{base}");
                    graph.add_edge(SemanticEdge {
                        from: class_id.clone(),
                        to: base_id,
                        relationship: RelationKind::Inherits,
                        evidence: Some(class.evidence.clone()),
                    });
                }

                for method in &class.methods {
                    let method_id = format!("method::{}", method.qualified_name);
                    graph.add_node(SemanticNode {
                        id: method_id.clone(),
                        name: method.name.clone(),
                        kind: NodeKind::Method,
                        qualified_name: method.qualified_name.clone(),
                        file_path: module.file_path.display().to_string(),
                        risk_score: 15,
                        risk_level: RiskLevel::Low,
                        evidence: Some(method.evidence.clone()),
                        metadata: HashMap::new(),
                    });

                    graph.add_edge(SemanticEdge {
                        from: class_id.clone(),
                        to: method_id.clone(),
                        relationship: RelationKind::Contains,
                        evidence: Some(method.evidence.clone()),
                    });

                    qualified_id_by_name.insert(method.qualified_name.clone(), method_id.clone());
                    candidates_by_bare_name
                        .entry(method.name.clone())
                        .or_default()
                        .push(method_id);
                }
            }

            // Add standalone functions
            for func in &module.functions {
                let func_id = format!("function::{}", func.qualified_name);
                graph.add_node(SemanticNode {
                    id: func_id.clone(),
                    name: func.name.clone(),
                    kind: NodeKind::Function,
                    qualified_name: func.qualified_name.clone(),
                    file_path: module.file_path.display().to_string(),
                    risk_score: 15,
                    risk_level: RiskLevel::Low,
                    evidence: Some(func.evidence.clone()),
                    metadata: HashMap::new(),
                });

                graph.add_edge(SemanticEdge {
                    from: mod_id.clone(),
                    to: func_id.clone(),
                    relationship: RelationKind::Contains,
                    evidence: Some(func.evidence.clone()),
                });

                qualified_id_by_name.insert(func.qualified_name.clone(), func_id.clone());
                candidates_by_bare_name
                    .entry(func.name.clone())
                    .or_default()
                    .push(func_id);
            }

            // Add unsupported constructs
            for unsupp in &module.unsupported_constructs {
                let unsupp_id = format!(
                    "unsupported::{}::{}",
                    module.module_name, unsupp.construct_kind
                );
                graph.add_node(SemanticNode {
                    id: unsupp_id.clone(),
                    name: unsupp.construct_kind.clone(),
                    kind: NodeKind::UnsupportedConstruct,
                    qualified_name: format!("{}::{}", unsupp.scope, unsupp.construct_kind),
                    file_path: module.file_path.display().to_string(),
                    risk_score: 85,
                    risk_level: RiskLevel::High,
                    evidence: Some(unsupp.evidence.clone()),
                    metadata: HashMap::from([
                        ("reason".to_string(), unsupp.reason.clone()),
                        ("snippet".to_string(), unsupp.snippet.clone()),
                    ]),
                });

                graph.add_edge(SemanticEdge {
                    from: mod_id.clone(),
                    to: unsupp_id,
                    relationship: RelationKind::DependsOn,
                    evidence: Some(unsupp.evidence.clone()),
                });
            }
        }

        // 3. Resolve module-level imports into real Imports edges. Deferred for the same reason as
        // calls below: the target module may be visited later in `parsed.modules`. Imports of
        // external/stdlib packages that were never parsed as part of this repository are skipped
        // rather than creating a dangling edge.
        for module in &parsed.modules {
            let from_mod_id = format!("module::{}", module.module_name);
            for import in &module.imports {
                let to_mod_id = format!("module::{}", import.module);
                if to_mod_id != from_mod_id && graph.nodes.contains_key(&to_mod_id) {
                    graph.add_edge(SemanticEdge {
                        from: from_mod_id.clone(),
                        to: to_mod_id,
                        relationship: RelationKind::Imports,
                        evidence: Some(import.evidence.clone()),
                    });
                }
            }
        }

        // 4. Resolve calls into real Calls edges. This runs after every module's nodes exist (a
        // call may reference a function defined in a module visited later above) and after a call
        // site's raw callee text (a bare name like "get_price", or an attribute expression like
        // "product.get_price") is resolved to the qualified node ID actually built above — a
        // literal `format!("function::{}", call.callee)` (the previous approach) could never match
        // any real node, because every function/method qualified_name always carries a
        // module/class prefix that raw call-site text never includes.
        for module in &parsed.modules {
            for call in &module.calls {
                let caller_id = qualified_id_by_name
                    .get(&call.caller_scope)
                    .cloned()
                    .unwrap_or_else(|| {
                        // Module-level call (not inside any function/method).
                        format!("module::{}", module.module_name)
                    });

                let bare_callee = call.callee.rsplit('.').next().unwrap_or(&call.callee);
                let Some(candidates) = candidates_by_bare_name.get(bare_callee) else {
                    // Unresolvable (stdlib call, external dependency, or a name Exodus doesn't
                    // track) — skip rather than create a dangling edge to a node that never existed.
                    continue;
                };

                // Prefer a candidate defined in the caller's own module (the common case for a
                // same-file call); otherwise fall back to the first candidate in sorted order for
                // determinism. This is a best-effort heuristic, not full semantic name resolution
                // (no import-alias tracking) — ambiguous cross-module calls to same-named symbols
                // may resolve to the wrong candidate.
                let module_prefix = format!("::{}::", module.module_name);
                let mut sorted_candidates = candidates.clone();
                sorted_candidates.sort();
                let callee_id = sorted_candidates
                    .iter()
                    .find(|id| id.contains(&module_prefix))
                    .or_else(|| sorted_candidates.first())
                    .cloned();

                if let Some(callee_id) = callee_id {
                    graph.add_edge(SemanticEdge {
                        from: caller_id,
                        to: callee_id,
                        relationship: RelationKind::Calls,
                        evidence: Some(call.evidence.clone()),
                    });
                }
            }
        }

        graph.compute_risk_scores();
        graph
    }

    /// Serializes semantic graph to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(ExodusError::from)
    }

    /// Deserializes semantic graph from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(ExodusError::from)
    }

    /// Generates high-level architectural metrics.
    pub fn generate_architecture_summary(&self) -> ArchitectureSummary {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();
        let module_count = self
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Module)
            .count();
        let type_count = self
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Type)
            .count();
        let function_count = self
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Function || n.kind == NodeKind::Method)
            .count();
        let unsupported_count = self
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::UnsupportedConstruct)
            .count();
        let circular_dependencies = self.detect_cycles();
        let high_risk_nodes = self
            .nodes
            .values()
            .filter(|n| n.risk_level >= RiskLevel::High)
            .map(|n| n.id.clone())
            .collect();

        ArchitectureSummary {
            total_nodes,
            total_edges,
            module_count,
            type_count,
            function_count,
            unsupported_count,
            circular_dependencies,
            high_risk_nodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_cycle_detection() {
        let mut graph = SemanticGraph::new();

        graph.add_node(SemanticNode {
            id: "A".to_string(),
            name: "A".to_string(),
            kind: NodeKind::Function,
            qualified_name: "A".to_string(),
            file_path: "a.py".to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_node(SemanticNode {
            id: "B".to_string(),
            name: "B".to_string(),
            kind: NodeKind::Function,
            qualified_name: "B".to_string(),
            file_path: "b.py".to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });

        graph.add_edge(SemanticEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        graph.add_edge(SemanticEdge {
            from: "B".to_string(),
            to: "A".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let cycles = graph.detect_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);

        graph.compute_risk_scores();
        assert!(graph.get_node("A").unwrap().risk_score > 30);
    }

    #[test]
    fn test_graph_topological_sort() {
        let mut graph = SemanticGraph::new();
        for id in ["leaf", "service", "entrypoint"] {
            graph.add_node(SemanticNode {
                id: id.to_string(),
                name: id.to_string(),
                kind: NodeKind::Function,
                qualified_name: id.to_string(),
                file_path: "f.py".to_string(),
                risk_score: 10,
                risk_level: RiskLevel::Low,
                evidence: None,
                metadata: HashMap::new(),
            });
        }

        // entrypoint calls service, service calls leaf
        graph.add_edge(SemanticEdge {
            from: "entrypoint".to_string(),
            to: "service".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "service".to_string(),
            to: "leaf".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let sorted = graph.topological_sort().unwrap();
        let leaf_idx = sorted.iter().position(|x| x == "leaf").unwrap();
        let service_idx = sorted.iter().position(|x| x == "service").unwrap();
        let entry_idx = sorted.iter().position(|x| x == "entrypoint").unwrap();

        // Dependencies before callers: leaf < service < entrypoint
        assert!(leaf_idx < service_idx);
        assert!(service_idx < entry_idx);
    }

    fn function_node(id: &str, qualified_name: &str) -> SemanticNode {
        SemanticNode {
            id: id.to_string(),
            name: id.to_string(),
            kind: NodeKind::Function,
            qualified_name: qualified_name.to_string(),
            file_path: "f.py".to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_verification_units_isolates_single_nodes() {
        let mut graph = SemanticGraph::new();
        for id in ["leaf", "service", "entrypoint"] {
            graph.add_node(function_node(id, id));
        }
        graph.add_edge(SemanticEdge {
            from: "entrypoint".to_string(),
            to: "service".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "service".to_string(),
            to: "leaf".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let units = graph.verification_units();
        assert_eq!(units.len(), 3);
        assert!(units.iter().all(|u| !u.is_cluster()));

        let leaf_pos = units
            .iter()
            .position(|u| u.node_ids() == vec!["leaf".to_string()])
            .unwrap();
        let entry_pos = units
            .iter()
            .position(|u| u.node_ids() == vec!["entrypoint".to_string()])
            .unwrap();
        assert!(leaf_pos < entry_pos);
    }

    #[test]
    fn test_verification_units_groups_cycles_into_clusters() {
        let mut graph = SemanticGraph::new();
        graph.add_node(function_node("A", "A"));
        graph.add_node(function_node("B", "B"));
        graph.add_edge(SemanticEdge {
            from: "A".to_string(),
            to: "B".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "B".to_string(),
            to: "A".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let units = graph.verification_units();
        assert_eq!(units.len(), 1);
        assert!(units[0].is_cluster());
        let mut ids = units[0].node_ids();
        ids.sort();
        assert_eq!(ids, vec!["A".to_string(), "B".to_string()]);
        match &units[0] {
            VerificationBoundary::Cluster { reason, .. } => {
                assert!(reason.contains("strongly connected"))
            }
            VerificationBoundary::Unit { .. } => panic!("expected a cluster"),
        }
    }

    #[test]
    fn test_verification_units_groups_class_with_its_methods() {
        let mut graph = SemanticGraph::new();
        graph.add_node(SemanticNode {
            id: "type::bank_account::BankAccount".to_string(),
            name: "BankAccount".to_string(),
            kind: NodeKind::Type,
            qualified_name: "bank_account::BankAccount".to_string(),
            file_path: "bank_account.py".to_string(),
            risk_score: 10,
            risk_level: RiskLevel::Low,
            evidence: None,
            metadata: HashMap::new(),
        });
        for method in ["deposit", "withdraw"] {
            let method_id = format!("method::bank_account::BankAccount::{method}");
            graph.add_node(SemanticNode {
                id: method_id.clone(),
                name: method.to_string(),
                kind: NodeKind::Method,
                qualified_name: format!("bank_account::BankAccount::{method}"),
                file_path: "bank_account.py".to_string(),
                risk_score: 10,
                risk_level: RiskLevel::Low,
                evidence: None,
                metadata: HashMap::new(),
            });
            graph.add_edge(SemanticEdge {
                from: "type::bank_account::BankAccount".to_string(),
                to: method_id,
                relationship: RelationKind::Contains,
                evidence: None,
            });
        }
        // An unrelated free function must not get pulled into the class's boundary.
        graph.add_node(function_node("function::other::helper", "other::helper"));

        let units = graph.verification_units();
        assert_eq!(units.len(), 2);

        let class_boundary = units
            .iter()
            .find(|u| {
                u.node_ids()
                    .contains(&"type::bank_account::BankAccount".to_string())
            })
            .unwrap();
        assert!(class_boundary.is_cluster());
        let mut ids = class_boundary.node_ids();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "method::bank_account::BankAccount::deposit".to_string(),
                "method::bank_account::BankAccount::withdraw".to_string(),
                "type::bank_account::BankAccount".to_string(),
            ]
        );
        match class_boundary {
            VerificationBoundary::Cluster { reason, .. } => {
                assert!(reason.contains("impl block"))
            }
            VerificationBoundary::Unit { .. } => panic!("expected a cluster"),
        }

        let helper_boundary = units
            .iter()
            .find(|u| u.node_ids() == vec!["function::other::helper".to_string()])
            .unwrap();
        assert!(!helper_boundary.is_cluster());
    }

    #[test]
    fn test_relevant_subgraph_includes_only_one_hop_neighborhood() {
        let mut graph = SemanticGraph::new();
        for id in ["far", "leaf", "service", "entrypoint"] {
            graph.add_node(function_node(id, id));
        }
        // far -> leaf -> service -> entrypoint (calls chain)
        graph.add_edge(SemanticEdge {
            from: "leaf".to_string(),
            to: "far".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "service".to_string(),
            to: "leaf".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });
        graph.add_edge(SemanticEdge {
            from: "entrypoint".to_string(),
            to: "service".to_string(),
            relationship: RelationKind::Calls,
            evidence: None,
        });

        let sub = graph.relevant_subgraph("service");
        let mut ids: Vec<&String> = sub.nodes.keys().collect();
        ids.sort();
        // service's 1-hop neighborhood: itself, its dependency (leaf), its dependent (entrypoint) —
        // "far" is two hops away and must not be pulled in.
        assert_eq!(
            ids,
            vec![
                &"entrypoint".to_string(),
                &"leaf".to_string(),
                &"service".to_string()
            ]
        );
        assert!(sub.get_node("service").is_some());
        assert!(sub.get_node("far").is_none());
    }

    #[test]
    fn test_content_hash_stable_across_reparse_changes_on_edit() {
        let mut node = function_node("function::f", "f");
        node.evidence = Some(SourceEvidence {
            span: exodus_core::SourceSpan::point("f.py", 1, 0),
            snippet: Some("def f(): return 1".to_string()),
            ast_node_kind: "function_definition".to_string(),
        });
        let hash_before = node.content_hash();

        // Re-parsing identical source with the same qualified name reproduces the same node.
        let mut reparsed = node.clone();
        assert_eq!(reparsed.content_hash(), hash_before);

        // Editing the body changes the content hash even though `id`/`qualified_name` do not.
        reparsed.evidence = Some(SourceEvidence {
            span: exodus_core::SourceSpan::point("f.py", 1, 0),
            snippet: Some("def f(): return 2".to_string()),
            ast_node_kind: "function_definition".to_string(),
        });
        assert_ne!(reparsed.content_hash(), hash_before);
        assert_eq!(reparsed.id, node.id);
    }
}
