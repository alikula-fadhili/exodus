//! Project Exodus Micro-Kernel (`exodus-kernel`)
//!
//! Provides a Cordis-inspired micro-kernel plugin architecture, dynamic lifecycle orchestration,
//! and `jcode` Code Mode execution engine.

pub mod builtin;
pub mod ledger;
pub mod modernize;
pub mod sdk;
pub mod worker_pool;

use async_trait::async_trait;
use exodus_core::MigrationDebt;
use exodus_graph::SemanticGraph;
use exodus_toolchain::{DomainArchetype, WorkspaceDescriptor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum KernelError {
    #[error("Plugin '{0}' is already mounted")]
    PluginAlreadyMounted(String),

    #[error("Plugin '{0}' not found in registry")]
    PluginNotFound(String),

    #[error("Plugin error in '{0}': {1}")]
    PluginExecutionError(String, String),

    #[error("SDK error: {0}")]
    SdkError(String),

    #[error("Worker pool error: {0}")]
    WorkerPoolError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Category of an Exodus plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCategory {
    SourceParser,     // AST/Tree-sitter symbol extractors
    TargetGenerator,  // Code generators (Axum, Tokio, Clap, Serde, Gin, etc.)
    DomainArchetype,  // Framework & archetype mapping logic
    SdlcArchitecture, // SDLC modern system design & architectural thesis review
    AgentLoopPolicy,  // Code Mode, Step-by-Step, or Autonomous policy
    VerificationRule, // Compiler and test runners
    FallbackStrategy, // Stub generation and debt emission
}

/// Strongly-typed kernel events routed through the plugin bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KernelEvent {
    WorkspaceDiscovered {
        workspace: WorkspaceDescriptor,
    },
    PackageTargetScheduled {
        package_name: String,
        target_language: String,
        domain_archetype: DomainArchetype,
        relative_path: PathBuf,
    },
    CodeModeExecute {
        package_name: String,
        source_path: PathBuf,
        target_path: PathBuf,
        context_prompt: String,
    },
    VerificationRequested {
        package_name: String,
        workspace_dir: PathBuf,
        target_language: String,
    },
    DebtRecorded {
        debt: MigrationDebt,
    },
    WaveFinished {
        wave_index: usize,
        packages: Vec<String>,
        success: bool,
    },
    Custom {
        event_name: String,
        payload: Value,
    },
}

/// Shared execution context passed to all kernel plugins and SDK runners.
#[derive(Debug, Clone)]
pub struct KernelContext {
    pub workspace_root: PathBuf,
    pub output_dir: PathBuf,
    pub target_language: String,
    pub variables: Arc<RwLock<HashMap<String, Value>>>,
    pub graph: Option<Arc<SemanticGraph>>,
    pub session_id: String,
    pub debts: Arc<RwLock<Vec<MigrationDebt>>>,
}

impl KernelContext {
    pub fn new(workspace_root: PathBuf, output_dir: PathBuf, target_language: String) -> Self {
        Self {
            workspace_root,
            output_dir,
            target_language,
            variables: Arc::new(RwLock::new(HashMap::new())),
            graph: None,
            session_id: uuid::Uuid::new_v4().to_string(),
            debts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn set_var(&self, key: impl Into<String>, value: Value) {
        let mut vars = self.variables.write().await;
        vars.insert(key.into(), value);
    }

    pub async fn get_var(&self, key: &str) -> Option<Value> {
        let vars = self.variables.read().await;
        vars.get(key).cloned()
    }

    pub async fn record_debt(&self, debt: MigrationDebt) {
        let mut debts = self.debts.write().await;
        debts.push(debt);
    }

    pub async fn get_debts(&self) -> Vec<MigrationDebt> {
        let debts = self.debts.read().await;
        debts.clone()
    }
}

/// Unified micro-kernel plugin interface (Cordis-inspired).
#[async_trait]
pub trait ExodusPlugin: Send + Sync {
    /// Unique identifier for this plugin.
    fn id(&self) -> &'static str;

    /// Plugin category.
    fn category(&self) -> PluginCategory;

    /// Execution priority (higher priority plugins handle events first).
    fn priority(&self) -> i32 {
        0
    }

    /// Called when the plugin is mounted into the kernel.
    async fn on_mount(&mut self, ctx: &mut KernelContext) -> Result<(), KernelError> {
        let _ = ctx;
        Ok(())
    }

    /// Handles a broadcast or targeted kernel event.
    async fn handle_event(
        &self,
        event: &KernelEvent,
        ctx: &mut KernelContext,
    ) -> Result<Option<Value>, KernelError> {
        let _ = (event, ctx);
        Ok(None)
    }

    /// Called when the plugin is unmounted.
    async fn on_unmount(&mut self, ctx: &mut KernelContext) -> Result<(), KernelError> {
        let _ = ctx;
        Ok(())
    }
}

/// The Core Exodus Micro-Kernel.
pub struct ExodusKernel {
    plugins: Vec<Box<dyn ExodusPlugin>>,
    context: KernelContext,
}

impl ExodusKernel {
    pub fn new(context: KernelContext) -> Self {
        Self {
            plugins: Vec::new(),
            context,
        }
    }

    pub fn context(&self) -> &KernelContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut KernelContext {
        &mut self.context
    }

    /// Mounts a plugin into the micro-kernel.
    pub async fn mount(&mut self, mut plugin: Box<dyn ExodusPlugin>) -> Result<(), KernelError> {
        let id = plugin.id();
        if self.plugins.iter().any(|p| p.id() == id) {
            return Err(KernelError::PluginAlreadyMounted(id.to_string()));
        }

        info!(
            "🔌 Mounting Kernel Plugin: '{}' [{:?}]",
            id,
            plugin.category()
        );
        plugin.on_mount(&mut self.context).await?;
        self.plugins.push(plugin);

        // Sort plugins by priority descending
        self.plugins
            .sort_by_key(|a| std::cmp::Reverse(a.priority()));
        Ok(())
    }

    /// Unmounts a plugin by ID.
    pub async fn unmount(&mut self, plugin_id: &str) -> Result<(), KernelError> {
        if let Some(pos) = self.plugins.iter().position(|p| p.id() == plugin_id) {
            let mut plugin = self.plugins.remove(pos);
            info!("🔌 Unmounting Kernel Plugin: '{}'", plugin_id);
            plugin.on_unmount(&mut self.context).await?;
            Ok(())
        } else {
            Err(KernelError::PluginNotFound(plugin_id.to_string()))
        }
    }

    /// Dispatches an event across all active plugins in priority order.
    pub async fn dispatch_event(&mut self, event: &KernelEvent) -> Result<Vec<Value>, KernelError> {
        debug!("⚡ Dispatching Kernel Event: {:?}", event);
        let mut results = Vec::new();

        for plugin in &self.plugins {
            match plugin.handle_event(event, &mut self.context).await {
                Ok(Some(val)) => results.push(val),
                Ok(None) => {}
                Err(err) => {
                    warn!(
                        "Plugin '{}' error during event dispatch: {}",
                        plugin.id(),
                        err
                    );
                    return Err(err);
                }
            }
        }

        Ok(results)
    }

    /// Lists all mounted plugins.
    pub fn list_plugins(&self) -> Vec<(&'static str, PluginCategory, i32)> {
        self.plugins
            .iter()
            .map(|p| (p.id(), p.category(), p.priority()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockParserPlugin;

    #[async_trait]
    impl ExodusPlugin for MockParserPlugin {
        fn id(&self) -> &'static str {
            "mock-parser"
        }

        fn category(&self) -> PluginCategory {
            PluginCategory::SourceParser
        }

        fn priority(&self) -> i32 {
            10
        }

        async fn handle_event(
            &self,
            event: &KernelEvent,
            _ctx: &mut KernelContext,
        ) -> Result<Option<Value>, KernelError> {
            if let KernelEvent::WorkspaceDiscovered { workspace } = event {
                return Ok(Some(serde_json::json!({
                    "handled_by": self.id(),
                    "pkg_count": workspace.packages.len()
                })));
            }
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_kernel_lifecycle_and_dispatch() {
        let ctx = KernelContext::new(
            PathBuf::from("/tmp/src"),
            PathBuf::from("/tmp/out"),
            "rust".to_string(),
        );
        let mut kernel = ExodusKernel::new(ctx);

        kernel.mount(Box::new(MockParserPlugin)).await.unwrap();

        assert_eq!(kernel.list_plugins().len(), 1);
        assert_eq!(kernel.list_plugins()[0].0, "mock-parser");

        let event = KernelEvent::WorkspaceDiscovered {
            workspace: WorkspaceDescriptor {
                toolchain: exodus_toolchain::WorkspaceToolchain::Standalone,
                root_dir: PathBuf::from("/tmp/src"),
                packages: vec![],
                dependency_graph: HashMap::new(),
                total_lines_of_code: 100,
            },
        };

        let results = kernel.dispatch_event(&event).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["handled_by"], "mock-parser");

        kernel.unmount("mock-parser").await.unwrap();
        assert_eq!(kernel.list_plugins().len(), 0);
    }
}
