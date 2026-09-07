//! Host toolchain inspection, discovery, and capability pre-flight checks for Project Exodus.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

pub mod workspace;
pub use workspace::*;
pub mod target_strategy;
pub use target_strategy::*;

/// Specific host tool classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolKind {
    Git,
    Cargo,
    Rustc,
    Python,
    Node,
    Java,
    Kotlinc,
    Go,
    Zig,
    Bun,
    Deno,
    Docker,
    Agy,
    ClaudeCode,
    Codex,
    Ollama,
    Custom(String),
}

impl ToolKind {
    pub fn binary_name(&self) -> String {
        match self {
            Self::Git => "git".to_string(),
            Self::Cargo => "cargo".to_string(),
            Self::Rustc => "rustc".to_string(),
            Self::Python => "python3".to_string(),
            Self::Node => "node".to_string(),
            Self::Java => "java".to_string(),
            Self::Kotlinc => "kotlinc".to_string(),
            Self::Go => "go".to_string(),
            Self::Zig => "zig".to_string(),
            Self::Bun => "bun".to_string(),
            Self::Deno => "deno".to_string(),
            Self::Docker => "docker".to_string(),
            Self::Agy => "agy".to_string(),
            Self::ClaudeCode => "claude".to_string(),
            Self::Codex => "codex".to_string(),
            Self::Ollama => "ollama".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Git => "Git CLI".to_string(),
            Self::Cargo => "Cargo Package Manager".to_string(),
            Self::Rustc => "Rust Compiler (rustc)".to_string(),
            Self::Python => "Python 3 Runtime".to_string(),
            Self::Node => "Node.js Runtime".to_string(),
            Self::Java => "Java Development Kit".to_string(),
            Self::Kotlinc => "Kotlin Compiler".to_string(),
            Self::Go => "Go Toolchain (go)".to_string(),
            Self::Zig => "Zig Compiler (zig)".to_string(),
            Self::Bun => "Bun Runtime & Bundler".to_string(),
            Self::Deno => "Deno Runtime".to_string(),
            Self::Docker => "Docker Container Engine".to_string(),
            Self::Agy => "Antigravity CLI (agy)".to_string(),
            Self::ClaudeCode => "Claude Code CLI".to_string(),
            Self::Codex => "OpenAI Codex CLI".to_string(),
            Self::Ollama => "Ollama Local LLM".to_string(),
            Self::Custom(name) => format!("Custom Tool ({name})"),
        }
    }

    pub fn install_guidance(&self) -> String {
        match self {
            Self::Git => "Install Git via your package manager: `sudo apt install git` or `brew install git`".to_string(),
            Self::Cargo | Self::Rustc => "Install Rust toolchain via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`".to_string(),
            Self::Python => "Install Python 3.11+: `sudo apt install python3 python3-pip` or `brew install python`".to_string(),
            Self::Node => "Install Node.js 20+: `nvm install 22` or `sudo apt install nodejs`".to_string(),
            Self::Java => "Install OpenJDK 17+: `sudo apt install openjdk-17-jdk` or `brew install openjdk@17`".to_string(),
            Self::Kotlinc => "Install Kotlin compiler: `sudo apt install kotlin` or `sdk install kotlin`".to_string(),
            Self::Go => "Install Go toolchain: `sudo apt install golang` or `brew install go`".to_string(),
            Self::Zig => "Install Zig compiler: `brew install zig` or download from https://ziglang.org/download/".to_string(),
            Self::Bun => "Install Bun: `curl -fsSL https://bun.sh/install | bash`".to_string(),
            Self::Deno => "Install Deno: `curl -fsSL https://deno.land/install.sh | sh`".to_string(),
            Self::Docker => "Install Docker Engine: https://docs.docker.com/engine/install/".to_string(),
            Self::Agy => "Install Antigravity CLI or set ANTIGRAVITY_API_KEY / GEMINI_API_KEY".to_string(),
            Self::ClaudeCode => "Install Claude Code CLI (`npm i -g @anthropic-ai/claude-code`) or set ANTHROPIC_API_KEY".to_string(),
            Self::Codex => "Install OpenAI CLI (`npm i -g @openai/codex`) or set OPENAI_API_KEY".to_string(),
            Self::Ollama => "Install Ollama from https://ollama.ai and start with `ollama serve`".to_string(),
            Self::Custom(name) => format!("Ensure `{name}` is installed and available on system PATH"),
        }
    }
}

/// Status of host tool availability and compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    Available,
    Missing,
    Incompatible(String),
}

/// Execution execution mode for verification tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Native,
    ContainerFallback,
    Unavailable,
}

/// Inspected tool details and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub kind: ToolKind,
    pub name: String,
    pub executable_path: Option<PathBuf>,
    pub version: Option<String>,
    pub required_version: Option<String>,
    pub status: ToolStatus,
    pub execution_mode: ExecutionMode,
    pub install_guidance: Option<String>,
}

/// Comprehensive toolchain inspection report for trajectory logs and CLI diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainReport {
    pub timestamp: DateTime<Utc>,
    pub tools: Vec<ToolInfo>,
    pub all_required_present: bool,
}

/// Toolchain inspector probing the host environment without shell interpolation.
pub struct ToolchainInspector;

impl ToolchainInspector {
    /// Probes multiple binary candidates dynamically without shell interpolation.
    pub fn probe_binary_dynamic(
        binary_names: &[String],
        version_flag: &str,
    ) -> (ToolStatus, Option<String>, Option<String>, ExecutionMode) {
        let mut resolved_path = None;
        for bin in binary_names {
            if let Some(path) = which_binary(bin) {
                resolved_path = Some(path);
                break;
            }
        }

        if let Some(path) = resolved_path {
            let ver_output = Command::new(&path).arg(version_flag).output().ok();
            let raw_ver = ver_output.and_then(|out| {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let combined = if stdout.trim().is_empty() {
                    stderr
                } else {
                    stdout
                };
                combined.lines().next().map(|s| s.trim().to_string())
            });
            (
                ToolStatus::Available,
                Some(path.to_string_lossy().to_string()),
                raw_ver,
                ExecutionMode::Native,
            )
        } else {
            let docker_present = which_binary("docker").is_some();
            let mode = if docker_present {
                ExecutionMode::ContainerFallback
            } else {
                ExecutionMode::Unavailable
            };
            (ToolStatus::Missing, None, None, mode)
        }
    }

    /// Probes a single host tool for presence and version string.
    pub fn probe_tool(kind: ToolKind) -> ToolInfo {
        let bin = kind.binary_name();
        let ver_arg = match &kind {
            ToolKind::Java | ToolKind::Kotlinc | ToolKind::Docker => "-version",
            ToolKind::Go | ToolKind::Zig => "version",
            _ => "--version",
        };

        let (status, path_str, version, mode) = Self::probe_binary_dynamic(&[bin], ver_arg);

        ToolInfo {
            name: kind.display_name(),
            executable_path: path_str.map(PathBuf::from),
            version,
            required_version: None,
            status: status.clone(),
            execution_mode: mode,
            install_guidance: if status == ToolStatus::Missing {
                Some(kind.install_guidance())
            } else {
                None
            },
            kind,
        }
    }

    /// Audits all standard host tools.
    pub fn audit_all() -> ToolchainReport {
        let kinds = [
            ToolKind::Git,
            ToolKind::Cargo,
            ToolKind::Rustc,
            ToolKind::Python,
            ToolKind::Node,
            ToolKind::Go,
            ToolKind::Zig,
            ToolKind::Bun,
            ToolKind::Deno,
            ToolKind::Java,
            ToolKind::Kotlinc,
            ToolKind::Docker,
        ];

        let mut tools = Vec::new();
        let mut all_required = true;

        for kind in kinds {
            let is_req = matches!(
                kind,
                ToolKind::Git | ToolKind::Cargo | ToolKind::Rustc | ToolKind::Python
            );
            let info = Self::probe_tool(kind);
            if is_req && info.status != ToolStatus::Available {
                all_required = false;
            }
            tools.push(info);
        }

        ToolchainReport {
            timestamp: Utc::now(),
            tools,
            all_required_present: all_required,
        }
    }

    /// Audits specific source -> target migration toolchain requirements.
    pub fn audit_lane(source: &str, target: &str) -> ToolchainReport {
        let mut required_kinds = vec![ToolKind::Git];

        match source.to_lowercase().as_str() {
            "python" | "py" => required_kinds.push(ToolKind::Python),
            "javascript" | "js" | "typescript" | "ts" => required_kinds.push(ToolKind::Node),
            "java" => required_kinds.push(ToolKind::Java),
            "kotlin" | "kt" => required_kinds.push(ToolKind::Kotlinc),
            "go" | "golang" => required_kinds.push(ToolKind::Go),
            "zig" => required_kinds.push(ToolKind::Zig),
            _ => {}
        }

        match target.to_lowercase().as_str() {
            "rust" | "rs" => {
                required_kinds.push(ToolKind::Rustc);
                required_kinds.push(ToolKind::Cargo);
            }
            "typescript" | "ts" | "javascript" | "js" => required_kinds.push(ToolKind::Node),
            "kotlin" | "kt" => required_kinds.push(ToolKind::Kotlinc),
            "go" | "golang" => required_kinds.push(ToolKind::Go),
            "zig" => required_kinds.push(ToolKind::Zig),
            "python" | "py" => required_kinds.push(ToolKind::Python),
            _ => {}
        }

        let mut tools = Vec::new();
        let mut all_required = true;

        for kind in required_kinds {
            let info = Self::probe_tool(kind);
            if info.status != ToolStatus::Available {
                all_required = false;
            }
            tools.push(info);
        }

        ToolchainReport {
            timestamp: Utc::now(),
            tools,
            all_required_present: all_required,
        }
    }

    /// Audits available host AI agent CLIs (Antigravity agy, Claude Code, OpenAI Codex, Ollama).
    pub fn detect_installed_agents() -> Vec<ToolInfo> {
        let agent_kinds = [
            ToolKind::Agy,
            ToolKind::ClaudeCode,
            ToolKind::Codex,
            ToolKind::Ollama,
        ];

        agent_kinds.into_iter().map(Self::probe_tool).collect()
    }
}

/// Finds the executable path for a given binary name on PATH.
fn which_binary(name: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_all_probes_cleanly() {
        let report = ToolchainInspector::audit_all();
        assert!(!report.tools.is_empty());
        let git_info = report.tools.iter().find(|t| t.kind == ToolKind::Git);
        assert!(git_info.is_some());
    }

    #[test]
    fn test_audit_lane_python_to_rust() {
        let report = ToolchainInspector::audit_lane("python", "rust");
        let kinds: Vec<ToolKind> = report.tools.iter().map(|t| t.kind.clone()).collect();
        assert!(kinds.contains(&ToolKind::Git));
        assert!(kinds.contains(&ToolKind::Python));
        assert!(kinds.contains(&ToolKind::Rustc));
        assert!(kinds.contains(&ToolKind::Cargo));
    }

    #[test]
    fn test_detect_installed_agents() {
        let agents = ToolchainInspector::detect_installed_agents();
        assert_eq!(agents.len(), 4);
        let names: Vec<String> = agents.iter().map(|a| a.kind.binary_name()).collect();
        assert!(names.contains(&"agy".to_string()));
        assert!(names.contains(&"claude".to_string()));
        assert!(names.contains(&"codex".to_string()));
        assert!(names.contains(&"ollama".to_string()));
    }
}
