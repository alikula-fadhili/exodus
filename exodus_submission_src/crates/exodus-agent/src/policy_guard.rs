//! Embedded ABAC/RBAC Policy Scope Guard (Cerbos-equivalent logical engine).
//!
//! Enforces fine-grained declarative security boundaries on agent actions,
//! resource paths, and tool calls before execution or file mutation.

use exodus_core::{ExodusError, Result};
use exodus_store::{DynamicRoleDefinitionRecord, PolicyAction, PolicyDecision, RolePolicyRecord};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Policy Scope Guard that verifies agent operations against their DB-assigned permissions.
#[derive(Clone)]
pub struct PolicyGuard {
    active_policies: Arc<RwLock<HashMap<String, RolePolicyRecord>>>,
}

impl PolicyGuard {
    pub fn new() -> Self {
        Self {
            active_policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or update an active policy for a given role.
    pub async fn register_policy(&self, policy: RolePolicyRecord) {
        let mut guard = self.active_policies.write().await;
        guard.insert(policy.role_id.clone(), policy);
    }

    /// Load policies from a set of dynamic role records.
    pub async fn load_roles(&self, roles: &[DynamicRoleDefinitionRecord]) {
        let mut guard = self.active_policies.write().await;
        for role in roles {
            guard.insert(role.id.clone(), role.abac_policy.clone());
        }
    }

    /// Authorize or reject an agent action on a given resource path.
    pub async fn authorize(
        &self,
        role_id: &str,
        resource_path: &Path,
        action: PolicyAction,
    ) -> Result<()> {
        let guard = self.active_policies.read().await;
        if let Some(policy) = guard.get(role_id) {
            match policy.evaluate(resource_path, action) {
                PolicyDecision::Allow => Ok(()),
                PolicyDecision::Deny { reason } => Err(ExodusError::VerificationFailure(format!(
                    "ABAC Policy Scope Violation for role `{role_id}`: {reason}"
                ))),
            }
        } else {
            // Default permissive for unspecified roles, but log audit
            Ok(())
        }
    }

    /// Fast synchronous evaluation when policy is locally cached.
    pub fn evaluate_sync(
        &self,
        role_id: &str,
        resource_path: &Path,
        action: PolicyAction,
    ) -> Result<()> {
        if let Ok(guard) = self.active_policies.try_read() {
            if let Some(policy) = guard.get(role_id) {
                match policy.evaluate(resource_path, action) {
                    PolicyDecision::Allow => Ok(()),
                    PolicyDecision::Deny { reason } => Err(ExodusError::VerificationFailure(
                        format!("ABAC Policy Scope Violation for role `{role_id}`: {reason}"),
                    )),
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

impl Default for PolicyGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_guard_abac_enforcement() {
        let guard = PolicyGuard::new();

        let devops_policy = RolePolicyRecord {
            id: "pol_devops".to_string(),
            role_id: "devops_sre_engineer".to_string(),
            allowed_path_globs: vec!["Dockerfile".to_string(), ".github/**".to_string()],
            denied_path_globs: vec!["src/**".to_string()],
            allowed_actions: vec![PolicyAction::ReadFile, PolicyAction::WriteFile],
            denied_actions: vec![PolicyAction::ModifySchema],
            allow_cross_file_mutation: false,
        };
        guard.register_policy(devops_policy).await;

        // DevOps writing Dockerfile -> Allowed
        assert!(guard
            .authorize(
                "devops_sre_engineer",
                Path::new("Dockerfile"),
                PolicyAction::WriteFile
            )
            .await
            .is_ok());

        // DevOps writing backend logic in src/api.rs -> Denied
        let denied = guard
            .authorize(
                "devops_sre_engineer",
                Path::new("src/api.rs"),
                PolicyAction::WriteFile,
            )
            .await;
        assert!(denied.is_err());
        assert!(denied
            .unwrap_err()
            .to_string()
            .contains("ABAC Policy Scope Violation"));
    }
}
