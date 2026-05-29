use crate::{
    models::{AiRequest, AiResponse},
    traits::AiProvider,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use jarvis_config::AiConfig;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: Client,
}

impl OpenAiCompatibleProvider {
    pub fn from_config(config: &AiConfig) -> Result<Self> {
        let api_key = env::var(&config.api_key_env)
            .with_context(|| format!("missing env var {}", config.api_key_env))?;
        Self::new("openai-compatible", config.base_url.clone(), api_key)
    }

    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: String,
    ) -> Result<Self> {
        if api_key.trim().is_empty() {
            anyhow::bail!("AI API key is empty");
        }
        Ok(Self {
            name: name.into(),
            base_url: base_url.into(),
            api_key,
            client: Client::new(),
        })
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn generate(&self, request: AiRequest) -> Result<AiResponse> {
        let url = format!("{}/responses", self.base_url.trim_end_matches('/'));
        let mut body = json!({
            "model": &request.model,
            "input": [
                { "role": "system", "content": &request.system_prompt },
                { "role": "user", "content": &request.user_prompt }
            ],
            "temperature": request.temperature
        });

        if let Some(max_output_tokens) = request.max_output_tokens {
            body["max_output_tokens"] = json!(max_output_tokens);
        }

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("AI provider request failed")?;

        let provider_request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);

        let value: Value = response
            .error_for_status()
            .context("AI provider returned an error status")?
            .json()
            .await
            .context("AI provider returned invalid JSON")?;

        Ok(AiResponse {
            tier: request.tier,
            model: request.model,
            text: extract_text(&value),
            quality_score: 0.80,
            provider_request_id,
        })
    }
}

fn extract_text(value: &Value) -> String {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return text.to_string();
    }

    let mut chunks = Vec::new();
    if let Some(output) = value.get("output").and_then(Value::as_array) {
        for item in output {
            if let Some(content) = item.get("content").and_then(Value::as_array) {
                for block in content {
                    if let Some(text) = block.get("text").and_then(Value::as_str) {
                        chunks.push(text.to_string());
                    }
                }
            }
        }
    }

    if chunks.is_empty() {
        value.to_string()
    } else {
        chunks.join("\n")
    }
}
