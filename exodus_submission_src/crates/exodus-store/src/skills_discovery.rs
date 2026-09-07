//! Discovery and ingestion of filesystem-based SKILL.md repository setup blueprints.
//!
//! Scans directories (such as `.agents/skills/**/SKILL.md`) and parses YAML frontmatter
//! and markdown guidelines into `RepoSetupSkillRecord`s for planner and agent consultation.

use crate::{RepoSetupSkillRecord, SkillCategory};
use chrono::Utc;
use exodus_core::Result;
use exodus_toolchain::DomainArchetype;
use std::fs;
use std::path::Path;

/// Engine for scanning, discovering, and parsing `SKILL.md` files into dynamic setup skills.
pub struct SkillDiscoverer;

impl SkillDiscoverer {
    /// Discovers all `SKILL.md` files starting from `root_dir` recursively.
    pub fn discover_in_path(root_dir: &Path) -> Vec<RepoSetupSkillRecord> {
        let mut skills = Vec::new();
        Self::scan_recursive(root_dir, &mut skills);
        skills.sort_by(|a, b| a.id.cmp(&b.id));
        skills
    }

    /// Discovers skills from standard repository locations:
    /// 1. `.agents/skills`
    /// 2. `.skills`
    /// 3. `.exodus/skills`
    pub fn discover_standard_locations(workspace_root: &Path) -> Vec<RepoSetupSkillRecord> {
        let candidate_dirs = [
            workspace_root.join(".agents").join("skills"),
            workspace_root.join(".skills"),
            workspace_root.join(".exodus").join("skills"),
        ];

        let mut discovered = Vec::new();
        for dir in &candidate_dirs {
            if dir.exists() && dir.is_dir() {
                discovered.extend(Self::discover_in_path(dir));
            }
        }
        discovered.sort_by(|a, b| a.id.cmp(&b.id));
        discovered.dedup_by(|a, b| a.id == b.id);
        discovered
    }

    /// Parses a single `SKILL.md` file into a `RepoSetupSkillRecord`.
    pub fn parse_skill_file(file_path: &Path) -> Result<Option<RepoSetupSkillRecord>> {
        let content = fs::read_to_string(file_path)?;
        Self::parse_skill_content(&content, file_path)
    }

    /// Parses the raw string contents of a `SKILL.md` file.
    pub fn parse_skill_content(
        content: &str,
        file_path: &Path,
    ) -> Result<Option<RepoSetupSkillRecord>> {
        let trimmed = content.trim();
        if !trimmed.starts_with("---") {
            // Markdown file without frontmatter - synthesize basic skill
            let name = file_path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed_skill");
            return Ok(Some(RepoSetupSkillRecord {
                id: format!("skill_{name}"),
                name: name.replace(['_', '-'], " "),
                category: SkillCategory::Documentation,
                target_language: "any".to_string(),
                target_archetype: DomainArchetype::Unknown,
                description: format!("Setup guidance for {name}"),
                setup_guidelines: content.to_string(),
                recommended_scaffold_files: vec![file_path.display().to_string()],
                version: "1.0.0".to_string(),
                updated_at: Utc::now(),
            }));
        }

        // Split frontmatter and body
        let rest = &trimmed[3..];
        let Some(end_frontmatter) = rest.find("---") else {
            return Ok(None);
        };

        let frontmatter_str = &rest[..end_frontmatter];
        let body = rest[end_frontmatter + 3..].trim().to_string();

        let mut name = None;
        let mut description = None;
        let mut category_str = None;
        let mut target_lang = "any".to_string();
        let mut target_arch = DomainArchetype::Unknown;

        for line in frontmatter_str.lines() {
            let line = line.trim();
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
                match key.as_str() {
                    "name" => name = Some(val),
                    "description" => description = Some(val),
                    "category" => category_str = Some(val),
                    "language" | "target_language" => target_lang = val,
                    "archetype" | "target_archetype" => {
                        target_arch = match val.to_lowercase().as_str() {
                            "backendservice" | "backend_service" | "backend" => {
                                DomainArchetype::BackendService
                            }
                            "sharedlibrary" | "shared_library" | "library" | "lib" => {
                                DomainArchetype::SharedLibrary
                            }
                            "clitool" | "cli_tool" | "cli" => DomainArchetype::CliTool,
                            "frontendapp" | "frontend_app" | "frontend" => {
                                DomainArchetype::FrontendApp
                            }
                            "workerqueue" | "worker_queue" | "worker" => {
                                DomainArchetype::WorkerQueue
                            }
                            _ => DomainArchetype::Unknown,
                        };
                    }
                    _ => {}
                }
            }
        }

        let skill_id = name
            .as_deref()
            .unwrap_or_else(|| {
                file_path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            })
            .to_lowercase()
            .replace([' ', '-'], "_");

        let skill_name = name.unwrap_or_else(|| skill_id.clone());
        let skill_desc = description.unwrap_or_else(|| format!("Setup blueprint for {skill_name}"));

        let category = match category_str.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("architecture") => SkillCategory::Architecture,
            Some("toolchain") => SkillCategory::Toolchain,
            Some("devops") => SkillCategory::DevOps,
            Some("observability") => SkillCategory::Observability,
            Some("testing") => SkillCategory::Testing,
            Some("documentation") => SkillCategory::Documentation,
            _ => {
                if skill_id.contains("wiki") || skill_id.contains("doc") {
                    SkillCategory::Documentation
                } else if skill_id.contains("ci")
                    || skill_id.contains("docker")
                    || skill_id.contains("container")
                {
                    SkillCategory::DevOps
                } else if skill_id.contains("trace")
                    || skill_id.contains("telemetry")
                    || skill_id.contains("health")
                {
                    SkillCategory::Observability
                } else {
                    SkillCategory::Toolchain
                }
            }
        };

        // Scan guidelines for scaffold files mentioned in backticks or markdown headers
        let mut scaffold_files = Vec::new();
        for word in body.split_whitespace() {
            let clean = word
                .trim_matches('`')
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches(',');
            if (clean.contains('.') || clean.contains('/'))
                && (clean.ends_with(".toml")
                    || clean.ends_with(".json")
                    || clean.ends_with(".yaml")
                    || clean.ends_with(".yml")
                    || clean.ends_with(".rs")
                    || clean.ends_with(".go")
                    || clean.ends_with(".md")
                    || clean == "Dockerfile"
                    || clean == ".dockerignore")
                && !scaffold_files.contains(&clean.to_string())
                && scaffold_files.len() < 5
            {
                scaffold_files.push(clean.to_string());
            }
        }

        Ok(Some(RepoSetupSkillRecord {
            id: format!("skill_{skill_id}"),
            name: skill_name,
            category,
            target_language: target_lang,
            target_archetype: target_arch,
            description: skill_desc,
            setup_guidelines: body,
            recommended_scaffold_files: scaffold_files,
            version: "1.0.0".to_string(),
            updated_at: Utc::now(),
        }))
    }

    fn scan_recursive(dir: &Path, results: &mut Vec<RepoSetupSkillRecord>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_recursive(&path, results);
                } else if path.is_file() {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if file_name.eq_ignore_ascii_case("SKILL.md") {
                        if let Ok(Some(skill)) = Self::parse_skill_file(&path) {
                            results.push(skill);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_openwiki_skill_frontmatter() {
        let content = r#"---
name: openwiki
description: Initialize or update an OpenWiki repository wiki using the OpenWiki resumable page-job lifecycle.
---

# OpenWiki
Setup guidelines and required sequence.
"#;
        let skill = SkillDiscoverer::parse_skill_content(content, Path::new("openwiki/SKILL.md"))
            .unwrap()
            .expect("Skill parsed");

        assert_eq!(skill.id, "skill_openwiki");
        assert_eq!(skill.name, "openwiki");
        assert_eq!(skill.category, SkillCategory::Documentation);
        assert!(skill.description.contains("OpenWiki"));
        assert!(skill.setup_guidelines.contains("Setup guidelines"));
    }

    #[test]
    fn test_discover_skills_in_directory_tree() {
        let tmp = tempdir().unwrap();
        let skills_root = tmp.path().join(".agents").join("skills");
        let openwiki_dir = skills_root.join("openwiki");
        fs::create_dir_all(&openwiki_dir).unwrap();

        fs::write(
            openwiki_dir.join("SKILL.md"),
            r#"---
name: OpenWiki Knowledge Base
description: Architecture documentation and claims validation.
---
# Instructions
Scaffold openwiki/quickstart.md and document components.
"#,
        )
        .unwrap();

        let discovered = SkillDiscoverer::discover_standard_locations(tmp.path());
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].id, "skill_openwiki_knowledge_base");
        assert_eq!(discovered[0].category, SkillCategory::Documentation);
    }
}
