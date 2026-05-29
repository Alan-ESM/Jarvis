use jarvis_config::NetworkConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct InternetGate {
    client: Client,
    probe_url: String,
    timeout: Duration,
}

impl InternetGate {
    pub fn from_config(config: &NetworkConfig) -> Self {
        Self {
            client: Client::new(),
            probe_url: config.probe_url.clone(),
            timeout: Duration::from_millis(config.timeout_ms),
        }
    }

    pub async fn check(&self) -> ConnectivityState {
        let result = self
            .client
            .get(&self.probe_url)
            .timeout(self.timeout)
            .send()
            .await;

        match result {
            Ok(response)
                if response.status().is_success() || response.status().is_redirection() =>
            {
                ConnectivityState::Online
            }
            Ok(response) => ConnectivityState::Offline {
                reason: format!("probe returned HTTP {}", response.status()),
            },
            Err(error) => ConnectivityState::Offline {
                reason: error.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectivityState {
    Online,
    Offline { reason: String },
}

impl ConnectivityState {
    pub fn is_online(&self) -> bool {
        matches!(self, ConnectivityState::Online)
    }
}
