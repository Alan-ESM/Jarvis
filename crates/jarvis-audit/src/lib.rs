use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditLogger {
    path: Arc<PathBuf>,
}

impl AuditLogger {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create audit dir {}", parent.display()))?;
        }
        Ok(Self {
            path: Arc::new(path),
        })
    }

    pub fn log(&self, event: AuditEvent) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path.as_ref())
            .with_context(|| format!("failed to open audit log {}", self.path.display()))?;

        serde_json::to_writer(&mut file, &event).context("failed to serialize audit event")?;
        file.write_all(b"\n")
            .context("failed to flush audit line")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub decision: AuditDecision,
    pub metadata: Value,
}

impl AuditEvent {
    pub fn new(action: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: "jarvis".to_string(),
            action: action.into(),
            target: target.into(),
            decision: AuditDecision::Allowed,
            metadata: json!({}),
        }
    }

    pub fn denied(mut self, reason: impl Into<String>) -> Self {
        self.decision = AuditDecision::Denied;
        self.metadata = json!({ "reason": reason.into() });
        self
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditDecision {
    Allowed,
    Denied,
    NeedsUserApproval,
    Failed,
}
