//! Deterministic Pre-Flight Migration Budget & Risk Estimator.

use exodus_graph::SemanticGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingRule {
    pub tier_name: String,
    pub module_count: usize,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRiskBreakdown {
    pub low_risk_modules: usize,
    pub medium_risk_modules: usize,
    pub high_risk_modules: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationBudgetPreview {
    pub total_modules: usize,
    pub total_files: usize,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
    pub estimated_cost_usd: f64,
    pub recommended_budget_usd: f64,
    pub risk_breakdown: MigrationRiskBreakdown,
    pub model_routing: Vec<ModelRoutingRule>,
    pub high_cost_modules: Vec<String>,
}

impl MigrationBudgetPreview {
    pub fn format_card(&self) -> String {
        let mut s = String::new();
        s.push_str("PROJECT EXODUS: MIGRATION BUDGET & RISK PREVIEW\n");
        s.push_str("════════════════════════════════════════════════════════════\n");
        s.push_str(&format!(
            "Modules / Files   : {:<4} modules | {:<4} source files\n",
            self.total_modules, self.total_files
        ));
        s.push_str(&format!(
            "Estimated Tokens  : {:<10} input / {:<10} output\n",
            format_tokens(self.estimated_input_tokens),
            format_tokens(self.estimated_output_tokens)
        ));
        s.push_str(&format!(
            "Estimated Cost    : ${:.2} USD (Recommended Budget: ${:.2} USD)\n\n",
            self.estimated_cost_usd, self.recommended_budget_usd
        ));

        s.push_str("MIGRATION RISK PROFILE\n");
        s.push_str("────────────────────────────────────────────────────────────\n");
        s.push_str(&format!(
            "LOW    : {:<3} modules (isolated leaf nodes, pure functions)\n",
            self.risk_breakdown.low_risk_modules
        ));
        s.push_str(&format!(
            "MEDIUM : {:<3} modules (cross-module dependencies, state mut)\n",
            self.risk_breakdown.medium_risk_modules
        ));
        s.push_str(&format!(
            "HIGH   : {:<3} modules (circular dependency clusters, dynamic types)\n\n",
            self.risk_breakdown.high_risk_modules
        ));

        s.push_str("EXPLAINABLE MODEL ROUTING\n");
        s.push_str("────────────────────────────────────────────────────────────\n");
        for r in &self.model_routing {
            s.push_str(&format!(
                "{:<10}: {:<3} modules -> {}\n",
                r.tier_name, r.module_count, r.rationale
            ));
        }

        if !self.high_cost_modules.is_empty() {
            s.push_str("\nHIGH-COMPLEXITY MODULE HOTSPOTS:\n");
            for h in &self.high_cost_modules {
                s.push_str(&format!("  • {}\n", h));
            }
        }
        s.push_str("════════════════════════════════════════════════════════════\n");
        s
    }
}

fn format_tokens(t: u64) -> String {
    if t >= 1_000_000 {
        format!("{:.2}M", t as f64 / 1_000_000.0)
    } else if t >= 1_000 {
        format!("{:.1}k", t as f64 / 1_000.0)
    } else {
        t.to_string()
    }
}

pub struct CostEstimator;

impl CostEstimator {
    /// Deterministically estimates token consumption, risk breakdown, and explainable model routing from the ESG.
    pub fn estimate_graph(graph: &SemanticGraph, total_files: usize) -> MigrationBudgetPreview {
        let units = graph.verification_units();
        let total_modules = units.len().max(1);
        let cycles = graph.detect_cycles();

        // Estimate token density: bytes / 3.6 * (1 + mu_context)
        let mut raw_bytes = 0u64;
        for node in graph.nodes.values() {
            let snippet_len = (node.qualified_name.len() * 45).max(150);
            raw_bytes += snippet_len as u64;
        }
        if raw_bytes == 0 {
            raw_bytes = (total_files.max(1) * 3200) as u64;
        }

        let input_tokens = ((raw_bytes as f64 / 3.6) * 1.35) as u64;
        let output_tokens = (input_tokens as f64 * 0.30) as u64;

        // Effective pricing with prompt caching ($0.50/1M in, $2.00/1M out blended)
        let effective_in = input_tokens as f64 * (0.15 * 0.90 + 0.85 * 0.10); // 90% discount on cache hits
        let estimated_cost = ((effective_in * 0.50 / 1_000_000.0)
            + (output_tokens as f64 * 2.00 / 1_000_000.0))
            .max(0.05);
        let recommended_budget = (estimated_cost * 1.35).round().max(0.10);

        let mut low_risk = 0;
        let mut med_risk = 0;
        let mut high_risk = 0;
        let mut high_cost_modules = Vec::new();

        for u in &units {
            let is_cyclic = u.is_cluster()
                || cycles
                    .iter()
                    .any(|c| c.iter().any(|n| u.node_ids().contains(n)));
            if is_cyclic {
                high_risk += 1;
                let name = if u.is_cluster() {
                    format!("cluster:[{}]", u.node_ids().join(", "))
                } else {
                    u.node_ids()
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "unit".to_string())
                };
                high_cost_modules.push(name);
            } else if u.node_ids().len() > 1 {
                med_risk += 1;
            } else {
                low_risk += 1;
            }
        }

        let model_routing = vec![
            ModelRoutingRule {
                tier_name: "Standard".to_string(),
                module_count: low_risk,
                rationale: "Leaf & isolated modules (high-throughput single-pass)".to_string(),
            },
            ModelRoutingRule {
                tier_name: "Reasoning".to_string(),
                module_count: med_risk,
                rationale: "Inter-module dependencies and state mutation".to_string(),
            },
            ModelRoutingRule {
                tier_name: "Frontier".to_string(),
                module_count: high_risk,
                rationale: "High-centrality cycle clusters and complex resolution".to_string(),
            },
        ];

        MigrationBudgetPreview {
            total_modules,
            total_files: total_files.max(1),
            estimated_input_tokens: input_tokens,
            estimated_output_tokens: output_tokens,
            estimated_cost_usd: (estimated_cost * 100.0).round() / 100.0,
            recommended_budget_usd: (recommended_budget * 100.0).round() / 100.0,
            risk_breakdown: MigrationRiskBreakdown {
                low_risk_modules: low_risk,
                medium_risk_modules: med_risk,
                high_risk_modules: high_risk,
            },
            model_routing,
            high_cost_modules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimator_card() {
        let graph = SemanticGraph::new();
        let preview = CostEstimator::estimate_graph(&graph, 10);
        let card = preview.format_card();
        assert!(card.contains("MIGRATION BUDGET & RISK PREVIEW"));
        assert!(card.contains("Standard"));
        assert!(card.contains("Reasoning"));
    }
}
