//! SDLC Remote Repository and CI/CD Pipeline Integration models.
//!
//! Provides data models and workflow integration configurations for:
//! - Remote Repository Connections (GitHub, GitLab, Bitbucket, generic Git)
//! - Repository Pipeline Plugins (GitHub Actions, GitLab CI, local Git hooks)
//! - CI failure event ingestion converting build crashes into #prod-bug operational items
//! - Automated Pull Request / Merge Request proposals upon human approval

use crate::operational_domain::{DomainPayload, ProdBugPayload};
use crate::operational_item::OperationalItem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Supported remote repository hosting providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteRepoProvider {
    #[default]
    GitHub,
    GitLab,
    Bitbucket,
    GenericGit,
}

impl std::fmt::Display for RemoteRepoProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub => write!(f, "github"),
            Self::GitLab => write!(f, "gitlab"),
            Self::Bitbucket => write!(f, "bitbucket"),
            Self::GenericGit => write!(f, "generic-git"),
        }
    }
}

/// Remote Git repository connection settings and credentials metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRepoConfig {
    /// Remote clone/push URL (e.g. `https://github.com/org/repo.git` or `git@github.com:org/repo.git`).
    pub repo_url: String,
    /// Hosting provider.
    pub provider: RemoteRepoProvider,
    /// Primary default branch (e.g. `main` or `master`).
    pub default_branch: String,
    /// Name of environment variable holding personal access token (e.g. `GITHUB_TOKEN`).
    pub auth_token_env: Option<String>,
    /// Optional path to SSH private key file.
    pub ssh_key_path: Option<PathBuf>,
    /// Webhook shared secret for validating inbound CI trigger events.
    pub webhook_secret: Option<String>,
}

impl Default for RemoteRepoConfig {
    fn default() -> Self {
        Self {
            repo_url: "https://github.com/exodus-migration/exodus.git".to_string(),
            provider: RemoteRepoProvider::GitHub,
            default_branch: "master".to_string(),
            auth_token_env: Some("GITHUB_TOKEN".to_string()),
            ssh_key_path: None,
            webhook_secret: None,
        }
    }
}

impl RemoteRepoConfig {
    pub fn new(repo_url: impl Into<String>, provider: RemoteRepoProvider) -> Self {
        Self {
            repo_url: repo_url.into(),
            provider,
            default_branch: "main".to_string(),
            auth_token_env: Some("GITHUB_TOKEN".to_string()),
            ssh_key_path: None,
            webhook_secret: None,
        }
    }
}

/// Supported CI/CD pipeline automation frameworks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PipelineCiType {
    #[default]
    GitHubActions,
    GitLabCi,
    LocalPreCommit,
}

impl std::fmt::Display for PipelineCiType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHubActions => write!(f, "github-actions"),
            Self::GitLabCi => write!(f, "gitlab-ci"),
            Self::LocalPreCommit => write!(f, "local-pre-commit"),
        }
    }
}

/// Settings governing CI/CD pipeline plugins and automated SDLC actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoPipelinePluginConfig {
    /// Type of CI/CD integration.
    pub ci_type: PipelineCiType,
    /// Automatically capture CI pipeline failures as #prod-bug operational items.
    pub auto_trigger_on_ci_failure: bool,
    /// Automatically propose Pull Request on remote repository when item is HumanApproved.
    pub auto_create_pr_on_approval: bool,
    /// Target branch for Pull Requests.
    pub pr_target_branch: String,
    /// Git branch naming prefix for sandboxed fixes (e.g. `exodus/fix/`).
    pub branch_prefix: String,
    /// Run deterministic unit gate verification directly in CI workflow.
    pub run_unit_gate_in_ci: bool,
}

impl Default for RepoPipelinePluginConfig {
    fn default() -> Self {
        Self {
            ci_type: PipelineCiType::GitHubActions,
            auto_trigger_on_ci_failure: true,
            auto_create_pr_on_approval: true,
            pr_target_branch: "master".to_string(),
            branch_prefix: "exodus/fix/".to_string(),
            run_unit_gate_in_ci: true,
        }
    }
}

/// Comprehensive SDLC Integration Settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SdlcIntegrationSettings {
    pub remote_repo: RemoteRepoConfig,
    pub pipeline_plugin: RepoPipelinePluginConfig,
}

impl SdlcIntegrationSettings {
    pub fn new(remote_repo: RemoteRepoConfig, pipeline_plugin: RepoPipelinePluginConfig) -> Self {
        Self {
            remote_repo,
            pipeline_plugin,
        }
    }

    /// Generate scaffolded CI workflow file for the configured CI/CD engine.
    pub fn generate_ci_scaffold(&self) -> HashMap<String, String> {
        let mut files = HashMap::new();
        match self.pipeline_plugin.ci_type {
            PipelineCiType::GitHubActions => {
                let workflow_content = format!(
                    r#"# Generated by Project Exodus SDLC Pipeline Integration
name: Exodus Governed Verification & CI Gate

on:
  push:
    branches: [ "{default_branch}" ]
  pull_request:
    branches: [ "{default_branch}" ]
  workflow_dispatch:

jobs:
  verify:
    name: Exodus Unit Contract Verification
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Source
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust Toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Run Exodus Contract Gate
        run: |
          cargo test --workspace
          cargo check --workspace

      - name: Trigger Exodus Incident On Failure
        if: failure()
        env:
          EXODUS_AUTH_TOKEN: ${{{{ secrets.EXODUS_API_TOKEN }}}}
        run: |
          echo "CI Failure detected. Ingesting #prod-bug event to Exodus Control Plane..."
          # Ingest failure payload to local/remote Exodus endpoint
"#,
                    default_branch = self.remote_repo.default_branch
                );
                files.insert(
                    ".github/workflows/exodus-verify.yml".to_string(),
                    workflow_content,
                );
            }
            PipelineCiType::GitLabCi => {
                let gitlab_content = format!(
                    r#"# Generated by Project Exodus SDLC Pipeline Integration
stages:
  - verify
  - incident

exodus-verify:
  stage: verify
  image: rust:latest
  script:
    - cargo check --workspace
    - cargo test --workspace
  rules:
    - if: $CI_COMMIT_BRANCH == "{default_branch}"
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"

exodus-on-failure:
  stage: incident
  image: curlimages/curl:latest
  when: on_failure
  script:
    - echo "CI Job failed on commit $CI_COMMIT_SHA. Registering #prod-bug with Exodus..."
"#,
                    default_branch = self.remote_repo.default_branch
                );
                files.insert(".gitlab-ci.yml".to_string(), gitlab_content);
            }
            PipelineCiType::LocalPreCommit => {
                let hook_content = r#"#!/bin/sh
# Generated by Project Exodus Local SDLC Hook
echo "Running Exodus Pre-Push Verification..."
cargo test --workspace || {
    echo "Exodus verification failed! Please resolve or capture as #prod-bug before pushing."
    exit 1
}
"#
                .to_string();
                files.insert(".git/hooks/pre-push".to_string(), hook_content);
            }
        }
        files
    }
}

/// Incoming CI failure event payload parsed from CI webhook or runner failure logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiFailureEventPayload {
    pub provider: RemoteRepoProvider,
    pub repository: String,
    pub commit_sha: String,
    pub branch: String,
    pub workflow_name: String,
    pub job_name: String,
    pub run_id: String,
    pub failing_step: String,
    pub error_logs: String,
}

impl CiFailureEventPayload {
    /// Converts a CI pipeline failure directly into an `OperationalItem` tagged `#prod-bug`.
    pub fn into_operational_item(self, requester: &str) -> OperationalItem {
        let title = format!(
            "CI Failure in {}: {} ({})",
            self.repository, self.job_name, self.failing_step
        );
        let description = format!(
            "Automated CI failure recorded by {} for branch '{}' at commit {}.\nWorkflow: {}\nRun ID: {}",
            self.provider, self.branch, self.commit_sha, self.workflow_name, self.run_id
        );

        let mut payload = ProdBugPayload::new(self.commit_sha, self.error_logs, None);
        payload.reproduction_command = Some(format!("ci run {}", self.job_name));

        OperationalItem::new(
            title,
            description,
            requester,
            DomainPayload::ProdBug(payload),
        )
    }
}
