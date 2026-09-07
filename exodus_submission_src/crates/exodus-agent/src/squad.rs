//! Dynamic Role-Based Squad Orchestrator.
//!
//! Spawns, bounds, and coordinates specialized subagents (Lead Architect, Backend,
//! Database, DevOps, Core Algorithms, CLI UX) tailored to the codebase DomainArchetype.

use crate::policy_guard::PolicyGuard;
use chrono::Utc;
use exodus_core::Result;
use exodus_store::{
    DynamicLivingMemory, DynamicRoleDefinitionRecord, PolicyAction, RoleParameters,
};
use exodus_toolchain::DomainArchetype;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A member of a role-based engineering squad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadMember {
    pub role_id: String,
    pub name: String,
    pub domain_focus: String,
    pub model_tier: String,
    pub allowed_globs: Vec<String>,
    pub max_tokens: usize,
    #[serde(default)]
    pub attached_skills: Vec<exodus_store::RepoSetupSkillRecord>,
}

/// Dynamic squad assembled for an archetype.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSquad {
    pub archetype: DomainArchetype,
    pub lead_architect: SquadMember,
    pub members: Vec<SquadMember>,
    pub total_concurrency: usize,
}

impl AgentSquad {
    /// Format human-readable team roster for terminal display.
    pub fn format_roster(&self) -> String {
        let mut out = format!(
            "👥 Dynamic Agent Squad Provisioned for [{:?}]:\n",
            self.archetype
        );
        out.push_str(&format!(
            "   👑 Lead: {:<28} (Tier: {:<9} Focus: {})\n",
            self.lead_architect.name,
            self.lead_architect.model_tier,
            self.lead_architect.domain_focus
        ));
        for (i, m) in self.members.iter().enumerate() {
            out.push_str(&format!(
                "   {}. {:<30} (Tier: {:<9} Focus: {})\n      Allowed Paths: {}\n",
                i + 1,
                m.name,
                m.model_tier,
                m.domain_focus,
                m.allowed_globs.join(", ")
            ));
        }
        out
    }
}

/// Orchestrator for querying living memory, assembling squads, and executing bounded tasks.
pub struct SquadOrchestrator {
    memory: Arc<dyn DynamicLivingMemory>,
    policy_guard: PolicyGuard,
}

impl SquadOrchestrator {
    pub fn new(memory: Arc<dyn DynamicLivingMemory>) -> Self {
        Self {
            memory,
            policy_guard: PolicyGuard::new(),
        }
    }

    /// Dynamically assemble an engineering squad for a given DomainArchetype from DB records.
    pub async fn assemble_squad(&self, archetype: DomainArchetype) -> Result<AgentSquad> {
        self.memory.ensure_seeded().await?;
        let roles = self.memory.list_roles_for_archetype(archetype).await?;
        let all_skills = self
            .memory
            .get_skills_for_target("any", &archetype)
            .await
            .unwrap_or_default();
        self.policy_guard.load_roles(&roles).await;

        let lead_def = self
            .memory
            .get_role("lead_architect")
            .await?
            .unwrap_or_else(|| DynamicRoleDefinitionRecord {
                id: "lead_architect".to_string(),
                name: "Lead System Architect".to_string(),
                target_archetype: DomainArchetype::Unknown,
                parameters: RoleParameters::default(),
                domain_focus: "System flow and thesis".to_string(),
                system_prompt_template: String::new(),
                abac_policy: Default::default(),
                version: "1.0.0".to_string(),
                updated_at: Utc::now(),
            });

        let lead_skills: Vec<_> = all_skills
            .iter()
            .filter(|s| {
                matches!(
                    s.category,
                    exodus_store::SkillCategory::Architecture
                        | exodus_store::SkillCategory::Documentation
                )
            })
            .cloned()
            .collect();

        let lead_member = SquadMember {
            role_id: lead_def.id,
            name: lead_def.name,
            domain_focus: lead_def.domain_focus,
            model_tier: lead_def.parameters.model_tier,
            allowed_globs: lead_def.abac_policy.allowed_path_globs,
            max_tokens: lead_def.parameters.max_tokens,
            attached_skills: lead_skills,
        };

        let mut members = Vec::new();
        for r in roles {
            if r.id == "lead_architect" {
                continue;
            }

            let member_skills: Vec<_> = all_skills
                .iter()
                .filter(|s| match r.id.as_str() {
                    "devops_sre_engineer" => matches!(
                        s.category,
                        exodus_store::SkillCategory::DevOps
                            | exodus_store::SkillCategory::Observability
                    ),
                    "backend_engineer" => matches!(
                        s.category,
                        exodus_store::SkillCategory::Toolchain
                            | exodus_store::SkillCategory::Observability
                            | exodus_store::SkillCategory::Architecture
                    ),
                    "database_engineer" => matches!(
                        s.category,
                        exodus_store::SkillCategory::Architecture
                            | exodus_store::SkillCategory::Toolchain
                    ),
                    _ => matches!(s.category, exodus_store::SkillCategory::Toolchain),
                })
                .cloned()
                .collect();

            members.push(SquadMember {
                role_id: r.id,
                name: r.name,
                domain_focus: r.domain_focus,
                model_tier: r.parameters.model_tier,
                allowed_globs: r.abac_policy.allowed_path_globs,
                max_tokens: r.parameters.max_tokens,
                attached_skills: member_skills,
            });
        }

        let total_concurrency = 1 + members.len();

        Ok(AgentSquad {
            archetype,
            lead_architect: lead_member,
            members,
            total_concurrency,
        })
    }

    /// Enriches the base system prompt for a squad member with actionable setup skill guidelines.
    pub fn build_enriched_system_prompt(&self, member: &SquadMember, base_prompt: &str) -> String {
        let mut prompt = base_prompt.to_string();
        if !member.attached_skills.is_empty() {
            prompt.push_str("\n\n## Referenced Repository Setup Skills & Blueprints\n");
            for skill in &member.attached_skills {
                prompt.push_str(&format!(
                    "### {}\n{}\n\n",
                    skill.name, skill.setup_guidelines
                ));
            }
        }
        prompt
    }

    /// Check authorization for a squad member before dispatch.
    pub async fn check_permission(
        &self,
        role_id: &str,
        resource: &Path,
        action: PolicyAction,
    ) -> Result<()> {
        self.policy_guard.authorize(role_id, resource, action).await
    }

    pub fn policy_guard(&self) -> &PolicyGuard {
        &self.policy_guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exodus_store::InMemoryLivingMemory;

    #[tokio::test]
    async fn test_squad_orchestrator_assembling_backend_squad() {
        let mem = Arc::new(InMemoryLivingMemory::new());
        let orchestrator = SquadOrchestrator::new(mem);

        let squad = orchestrator
            .assemble_squad(DomainArchetype::BackendService)
            .await
            .unwrap();
        assert_eq!(squad.archetype, DomainArchetype::BackendService);
        assert_eq!(squad.lead_architect.role_id, "lead_architect");

        let member_roles: Vec<_> = squad.members.iter().map(|m| m.role_id.as_str()).collect();
        assert!(member_roles.contains(&"backend_engineer"));
        assert!(member_roles.contains(&"database_engineer"));
        assert!(member_roles.contains(&"devops_sre_engineer"));

        // Verify policy guard checks
        assert!(orchestrator
            .check_permission(
                "devops_sre_engineer",
                Path::new("Dockerfile"),
                PolicyAction::WriteFile
            )
            .await
            .is_ok());
        assert!(orchestrator
            .check_permission(
                "devops_sre_engineer",
                Path::new("src/routes.rs"),
                PolicyAction::WriteFile
            )
            .await
            .is_err());

        // Verify attached setup skills
        assert!(!squad.lead_architect.attached_skills.is_empty());
        let devops = squad
            .members
            .iter()
            .find(|m| m.role_id == "devops_sre_engineer")
            .unwrap();
        assert!(!devops.attached_skills.is_empty());

        let enriched = orchestrator.build_enriched_system_prompt(devops, "Base prompt.");
        assert!(enriched.contains("Referenced Repository Setup Skills"));
    }
}
