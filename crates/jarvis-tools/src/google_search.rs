use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct GoogleSearchClient {
    client: Client,
    api_key: String,
    engine_id: String,
}

impl GoogleSearchClient {
    pub fn new(api_key: impl Into<String>, engine_id: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            engine_id: engine_id.into(),
        }
    }

    pub async fn search(&self, query: &str, limit: u8) -> Result<Vec<SearchResult>> {
        let num = limit.clamp(1, 10).to_string();
        let response: GoogleResponse = self
            .client
            .get("https://www.googleapis.com/customsearch/v1")
            .query(&[
                ("key", self.api_key.as_str()),
                ("cx", self.engine_id.as_str()),
                ("q", query),
                ("num", num.as_str()),
            ])
            .send()
            .await
            .context("Google search request failed")?
            .error_for_status()
            .context("Google search returned an error status")?
            .json()
            .await
            .context("Google search returned invalid JSON")?;

        Ok(response
            .items
            .unwrap_or_default()
            .into_iter()
            .map(|item| SearchResult {
                title: item.title,
                url: item.link,
                snippet: item.snippet.unwrap_or_default(),
            })
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Deserialize)]
struct GoogleResponse {
    items: Option<Vec<GoogleItem>>,
}

#[derive(Debug, Deserialize)]
struct GoogleItem {
    title: String,
    link: String,
    snippet: Option<String>,
}
