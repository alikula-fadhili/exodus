//! Native Google Gemini Agent Provider.
//!
//! Connects directly to Google Generative Language API (Gemini models)
//! using JSON RPC over HTTPS with structured prompt framing and token counting.

use crate::{extract_code_snippet, AgentProvider, AgentResponse};
use async_trait::async_trait;
use exodus_core::{ExodusError, Result};
use std::time::Instant;

/// Real LLM Agent Provider connecting to Google Gemini REST API.
pub struct GeminiAgentProvider {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    client: reqwest::Client,
}

impl GeminiAgentProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_base_url(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Automatically constructs Gemini provider from environment variables:
    /// - API key: `GEMINI_API_KEY` or `GOOGLE_API_KEY`
    /// - Model: `GEMINI_MODEL` (default: `gemini-1.5-pro`)
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok()?;
        let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-pro".to_string());
        Some(Self::new(api_key, model))
    }
}

#[async_trait]
impl AgentProvider for GeminiAgentProvider {
    fn provider_name(&self) -> &'static str {
        "GeminiAgentProvider"
    }

    async fn complete(&self, system_prompt: &str, user_prompt: &str) -> Result<AgentResponse> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.model,
            self.api_key
        );
        let start = Instant::now();

        let request_body = serde_json::json!({
            "system_instruction": {
                "parts": [{ "text": system_prompt }]
            },
            "contents": [
                {
                    "role": "user",
                    "parts": [{ "text": user_prompt }]
                }
            ],
            "generationConfig": {
                "temperature": 0.1
            }
        });

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ExodusError::Other(format!("Gemini HTTP request failed: {e}")))?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(ExodusError::Other(format!(
                "Gemini API error ({status}): {text}"
            )));
        }

        let json: serde_json::Value = res.json().await.map_err(|e| {
            ExodusError::Other(format!("Failed to parse Gemini JSON response: {e}"))
        })?;

        let mut content = String::new();
        if let Some(candidates) = json["candidates"].as_array() {
            if let Some(first) = candidates.first() {
                if let Some(parts) = first["content"]["parts"].as_array() {
                    for part in parts {
                        if let Some(t) = part["text"].as_str() {
                            content.push_str(t);
                        }
                    }
                }
            }
        }

        let prompt_tokens = json["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;
        let completion_tokens = json["usageMetadata"]["candidatesTokenCount"]
            .as_u64()
            .unwrap_or(0) as usize;
        let duration_ms = start.elapsed().as_millis() as u64;

        let code = extract_code_snippet(&content);

        Ok(AgentResponse {
            raw_content: content,
            proposed_code: code,
            explanation: "Gemini synthesis completed.".to_string(),
            tokens_prompt: prompt_tokens,
            tokens_completion: completion_tokens,
            duration_ms,
        })
    }
}
