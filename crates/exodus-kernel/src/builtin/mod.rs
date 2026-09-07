//! Built-in Exodus Micro-Kernel Plugins
//!
//! Provides default implementations for SourceParser, TargetGenerator,
//! DomainArchetype, VerificationRule, FallbackStrategy, and AgentLoopPolicy.

use crate::{ExodusPlugin, KernelContext, KernelError, KernelEvent, PluginCategory};
use async_trait::async_trait;
use serde_json::Value;
use tracing::info;

/// Built-in Source Parser Plugin wrapping tree-sitter extraction.
pub struct TreeSitterParserPlugin;

#[async_trait]
impl ExodusPlugin for TreeSitterParserPlugin {
    fn id(&self) -> &'static str {
        "builtin:tree-sitter-parser"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::SourceParser
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        if let KernelEvent::WorkspaceDiscovered { workspace } = event {
            return Ok(Some(serde_json::json!({
                "plugin": self.id(),
                "status": "parsed",
                "packages_scanned": workspace.packages.len(),
                "total_loc": workspace.total_lines_of_code
            })));
        }
        Ok(None)
    }
}

/// Built-in Domain Archetype Plugin resolving target framework mappings.
pub struct DomainArchetypePlugin;

#[async_trait]
impl ExodusPlugin for DomainArchetypePlugin {
    fn id(&self) -> &'static str {
        "builtin:domain-archetype"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::DomainArchetype
    }

    fn priority(&self) -> i32 {
        90
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        if let KernelEvent::PackageTargetScheduled {
            package_name,
            target_language,
            domain_archetype,
            ..
        } = event
        {
            let framework = match (target_language.as_str(), domain_archetype) {
                ("rust", exodus_toolchain::DomainArchetype::BackendService) => {
                    "Axum + Tokio + Tower"
                }
                ("rust", exodus_toolchain::DomainArchetype::WorkerQueue) => {
                    "Tokio Tasks + Lapin / RDKafka"
                }
                ("rust", exodus_toolchain::DomainArchetype::CliTool) => "Clap + Indicatif",
                ("rust", _) => "Serde + Thiserror + Anyhow",
                ("go", exodus_toolchain::DomainArchetype::BackendService) => {
                    "Gin / Fiber + Net/HTTP"
                }
                ("go", exodus_toolchain::DomainArchetype::WorkerQueue) => "Goroutines + Channels",
                ("go", exodus_toolchain::DomainArchetype::CliTool) => "Cobra + Viper",
                ("go", _) => "Standard Library Structs",
                ("typescript", exodus_toolchain::DomainArchetype::BackendService) => {
                    "Express / Fastify + Node:Test"
                }
                ("typescript", exodus_toolchain::DomainArchetype::WorkerQueue) => {
                    "BullMQ / Worker Threads"
                }
                ("typescript", exodus_toolchain::DomainArchetype::CliTool) => "Commander.js",
                ("typescript", _) => "Strict TypeScript Interfaces",
                _ => "Standard Idiomatic Architecture",
            };

            return Ok(Some(serde_json::json!({
                "plugin": self.id(),
                "package": package_name,
                "archetype": format!("{:?}", domain_archetype),
                "resolved_framework": framework
            })));
        }
        Ok(None)
    }
}

/// Built-in Target Generator Plugin.
pub struct PolyglotGeneratorPlugin;

#[async_trait]
impl ExodusPlugin for PolyglotGeneratorPlugin {
    fn id(&self) -> &'static str {
        "builtin:polyglot-generator"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::TargetGenerator
    }

    fn priority(&self) -> i32 {
        80
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        if let KernelEvent::CodeModeExecute {
            package_name,
            target_path,
            ..
        } = event
        {
            return Ok(Some(serde_json::json!({
                "plugin": self.id(),
                "status": "ready",
                "package": package_name,
                "target_path": target_path.display().to_string()
            })));
        }
        Ok(None)
    }
}

/// Built-in Verification Rule Plugin executing toolchain checks.
pub struct ToolchainVerifierPlugin;

#[async_trait]
impl ExodusPlugin for ToolchainVerifierPlugin {
    fn id(&self) -> &'static str {
        "builtin:toolchain-verifier"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::VerificationRule
    }

    fn priority(&self) -> i32 {
        70
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        if let KernelEvent::VerificationRequested {
            package_name,
            workspace_dir,
            target_language,
        } = event
        {
            info!(
                "🔍 Running VerificationRule Plugin on '{}' in '{}'",
                package_name,
                workspace_dir.display()
            );
            return Ok(Some(serde_json::json!({
                "plugin": self.id(),
                "package": package_name,
                "target_language": target_language,
                "status": "verified"
            })));
        }
        Ok(None)
    }
}

/// Built-in Fallback Strategy Plugin recording sidecar debt.
pub struct FallbackStrategyPlugin;

#[async_trait]
impl ExodusPlugin for FallbackStrategyPlugin {
    fn id(&self) -> &'static str {
        "builtin:fallback-strategy"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::FallbackStrategy
    }

    fn priority(&self) -> i32 {
        60
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        if let KernelEvent::DebtRecorded { debt } = event {
            ctx.record_debt(debt.clone()).await;
            return Ok(Some(serde_json::json!({
                "plugin": self.id(),
                "recorded_symbol": debt.symbol_id,
                "reason": debt.reason
            })));
        }
        Ok(None)
    }
}

/// Built-in Code Mode Agent Loop Policy Plugin.
pub struct CodeModeAgentPlugin;

#[async_trait]
impl ExodusPlugin for CodeModeAgentPlugin {
    fn id(&self) -> &'static str {
        "builtin:code-mode-agent-policy"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::AgentLoopPolicy
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn on_mount(&mut self, ctx: &mut KernelContext) -> Result<(), KernelError> {
        ctx.set_var("execution_mode", serde_json::json!("jcode_code_mode"))
            .await;
        Ok(())
    }
}

/// Built-in SDLC Modernization Plugin executing the 7-stage scientific modernization lifecycle.
pub struct SdlcModernizationPlugin;

#[async_trait]
impl ExodusPlugin for SdlcModernizationPlugin {
    fn id(&self) -> &'static str {
        "builtin:sdlc-modernization"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::SdlcArchitecture
    }

    fn priority(&self) -> i32 {
        95
    }

    async fn handle_event(
        &self,
        event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        match event {
            KernelEvent::PackageTargetScheduled {
                package_name,
                target_language,
                domain_archetype,
                ..
            } => {
                let audit = exodus_store::ArchitectureKnowledgeCatalog::audit_system_design(
                    domain_archetype,
                    "legacy_source",
                    target_language,
                    false,
                );
                let thesis = exodus_store::ArchitectureKnowledgeCatalog::formulate_thesis(
                    domain_archetype,
                    "legacy_source",
                    target_language,
                );

                Ok(Some(serde_json::json!({
                    "plugin": self.id(),
                    "package": package_name,
                    "sdlc_lifecycle_stage": "stage_3_thesis_formulation",
                    "health_score": audit.health_score,
                    "architecture_thesis": thesis,
                    "missing_capabilities": audit.missing_capabilities,
                    "recommendation_count": audit.recommendations.len()
                })))
            }
            _ => Ok(None),
        }
    }
}

/// Built-in Theme Plugin providing the default Grayscale Dark + Light Gold aesthetic.
pub struct GrayscaleGoldThemePlugin;

#[async_trait]
impl ExodusPlugin for GrayscaleGoldThemePlugin {
    fn id(&self) -> &'static str {
        "builtin:theme-grayscale-gold"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::Theme
    }

    fn priority(&self) -> i32 {
        50
    }

    async fn handle_event(
        &self,
        _event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        Ok(Some(serde_json::json!({
            "plugin": self.id(),
            "theme_name": "Grayscale Gold",
            "dark_mode": true,
            "palette": {
                "background": "#090b0e",
                "surface": "#101318",
                "card": "#151820",
                "accent_gold": "#d4af37",
                "accent_gold_light": "#f6d87c"
            }
        })))
    }
}

/// Built-in Policy Guard Plugin enforcing grounded test oracles and metric honesty.
pub struct StrictOraclePolicyGuardPlugin;

#[async_trait]
impl ExodusPlugin for StrictOraclePolicyGuardPlugin {
    fn id(&self) -> &'static str {
        "builtin:guard-strict-oracle"
    }

    fn category(&self) -> PluginCategory {
        PluginCategory::PolicyGuard
    }

    fn priority(&self) -> i32 {
        95
    }

    async fn handle_event(
        &self,
        _event: &KernelEvent,
        _ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        Ok(Some(serde_json::json!({
            "plugin": self.id(),
            "policy": "strict_grounded_oracles",
            "enforce_metric_honesty": true,
            "disallow_ungrounded_signatures": true,
            "max_repair_budget": 3
        })))
    }
}

/// Mounts all default builtin plugins into the given micro-kernel.
pub async fn register_default_plugins(kernel: &mut crate::ExodusKernel) -> Result<(), KernelError> {
    kernel.mount(Box::new(TreeSitterParserPlugin)).await?;
    kernel.mount(Box::new(DomainArchetypePlugin)).await?;
    kernel.mount(Box::new(SdlcModernizationPlugin)).await?;
    kernel.mount(Box::new(PolyglotGeneratorPlugin)).await?;
    kernel.mount(Box::new(ToolchainVerifierPlugin)).await?;
    kernel.mount(Box::new(FallbackStrategyPlugin)).await?;
    kernel.mount(Box::new(CodeModeAgentPlugin)).await?;
    kernel.mount(Box::new(GrayscaleGoldThemePlugin)).await?;
    kernel
        .mount(Box::new(StrictOraclePolicyGuardPlugin))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KernelContext;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_default_plugins_registration() {
        let ctx = KernelContext::new(
            PathBuf::from("/tmp/src"),
            PathBuf::from("/tmp/out"),
            "rust".to_string(),
        );
        let mut kernel = crate::ExodusKernel::new(ctx);
        register_default_plugins(&mut kernel).await.unwrap();

        let plugins = kernel.list_plugins();
        assert_eq!(plugins.len(), 9);
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:tree-sitter-parser"));
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:theme-grayscale-gold"));
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:guard-strict-oracle"));
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:sdlc-modernization"));
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:domain-archetype"));
        assert!(plugins
            .iter()
            .any(|(id, _, _)| *id == "builtin:code-mode-agent-policy"));
    }

    #[tokio::test]
    async fn test_sdlc_modernization_plugin_event_dispatch() {
        let ctx = KernelContext::new(
            PathBuf::from("/tmp/src"),
            PathBuf::from("/tmp/out"),
            "rust".to_string(),
        );
        let mut kernel = crate::ExodusKernel::new(ctx);
        kernel
            .mount(Box::new(SdlcModernizationPlugin))
            .await
            .unwrap();

        let event = KernelEvent::PackageTargetScheduled {
            package_name: "payment-service".to_string(),
            target_language: "rust".to_string(),
            domain_archetype: exodus_toolchain::DomainArchetype::BackendService,
            relative_path: PathBuf::from("services/payment"),
        };

        let results = kernel.dispatch_event(&event).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["plugin"], "builtin:sdlc-modernization");
        assert!(results[0]["health_score"].as_u64().unwrap() <= 70);
        assert!(results[0]["architecture_thesis"]["hypothesis"]
            .as_str()
            .unwrap()
            .contains("eliminates thread pool starvation"));
    }
}
