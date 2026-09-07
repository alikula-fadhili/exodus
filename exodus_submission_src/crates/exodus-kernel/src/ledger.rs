//! Append-Only Trajectory Ledger & Prompt Cache Optimization Engine
//!
//! Standardizes append-only SessionEvents and deterministic prompt structure
//! to maximize LLM prompt KV-cache reuse (targeting 90%+ cache hits).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// An immutable event recorded in the trajectory ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub step: usize,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Append-only trajectory ledger preserving complete session history.
#[derive(Debug, Clone)]
pub struct SessionLedger {
    session_id: String,
    events: Arc<RwLock<Vec<SessionEvent>>>,
}

impl SessionLedger {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Appends an event to the ledger.
    pub async fn append(&self, event_type: impl Into<String>, payload: serde_json::Value) -> usize {
        let mut events = self.events.write().await;
        let step = events.len();
        let event = SessionEvent {
            session_id: self.session_id.clone(),
            timestamp: Utc::now(),
            step,
            event_type: event_type.into(),
            payload,
        };
        events.push(event);
        step
    }

    /// Returns a snapshot of all recorded events.
    pub async fn snapshot(&self) -> Vec<SessionEvent> {
        let events = self.events.read().await;
        events.clone()
    }
}

/// Optimizes system prompts and message prefixes to ensure deterministic KV cache hits.
pub struct PromptCacheOptimizer {
    static_system_prefix: String,
}

impl PromptCacheOptimizer {
    pub fn new(target_language: &str) -> Self {
        let prefix = format!(
            r#"### SYSTEM SPECIFICATION: PROJECT EXODUS AGENT HARNESS
You are an autonomous code transformation agent running inside Project Exodus.
Target Language: {}
Execution Mode: jcode Code Mode (Single-Pass Bundle Synthesis)

Rules:
1. Preserve business logic, mathematical formulas, and data boundaries faithfully.
2. Emit verified, compilable code matching the package domain archetype.
3. Emit companion unit tests covering normal, boundary, and error conditions.
4. Output strict JSON matching the CodeModeBundle schema.
"#,
            target_language
        );
        Self {
            static_system_prefix: prefix,
        }
    }

    /// Returns the static prefix which stays invariant across calls to hit LLM prompt cache.
    pub fn system_prefix(&self) -> &str {
        &self.static_system_prefix
    }

    /// Formats a user turn with deterministic cacheable structure.
    pub fn format_prompt(
        &self,
        package_name: &str,
        archetype: &str,
        source_snippets: &[(&str, &str)],
    ) -> String {
        let mut out = format!(
            "Package: {}\nArchetype: {}\nFiles:\n",
            package_name, archetype
        );
        for (filename, content) in source_snippets {
            out.push_str(&format!(
                "\n--- BEGIN FILE: {} ---\n{}\n--- END FILE: {} ---\n",
                filename, content, filename
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_only_session_ledger() {
        let ledger = SessionLedger::new("sess_123");
        let step0 = ledger
            .append("INIT", serde_json::json!({"repo": "demo"}))
            .await;
        let step1 = ledger
            .append("WAVE_START", serde_json::json!({"wave": 0}))
            .await;

        assert_eq!(step0, 0);
        assert_eq!(step1, 1);

        let snap = ledger.snapshot().await;
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].event_type, "INIT");
        assert_eq!(snap[1].event_type, "WAVE_START");
    }
}
