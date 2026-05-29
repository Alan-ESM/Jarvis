use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use jarvis_audit::{AuditDecision, AuditEvent, AuditLogger};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileAccessLevel {
    Intermediate,
    Unlimited,
    Disabled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileOperation {
    Read,
    Write,
    Delete,
    Execute,
    Inspect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccessRequest {
    pub operation: FileOperation,
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionDecision {
    Allow,
    Deny { reason: String },
}

#[async_trait]
pub trait PermissionBroker: Send + Sync {
    async fn request_file_access(&self, request: &FileAccessRequest) -> Result<PermissionDecision>;
}

#[derive(Debug, Default)]
pub struct DenyAllPermissionBroker;

#[async_trait]
impl PermissionBroker for DenyAllPermissionBroker {
    async fn request_file_access(
        &self,
        _request: &FileAccessRequest,
    ) -> Result<PermissionDecision> {
        Ok(PermissionDecision::Deny {
            reason: "no UI permission broker is connected".to_string(),
        })
    }
}

#[derive(Debug, Default)]
pub struct AllowAllPermissionBroker;

#[async_trait]
impl PermissionBroker for AllowAllPermissionBroker {
    async fn request_file_access(
        &self,
        _request: &FileAccessRequest,
    ) -> Result<PermissionDecision> {
        Ok(PermissionDecision::Allow)
    }
}

pub struct FileAccessController {
    level: FileAccessLevel,
    audit: AuditLogger,
    broker: Arc<dyn PermissionBroker>,
    authorized_roots: Vec<PathBuf>,
}

impl FileAccessController {
    pub fn new(
        level: FileAccessLevel,
        audit: AuditLogger,
        broker: Arc<dyn PermissionBroker>,
        authorized_roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            level,
            audit,
            broker,
            authorized_roots,
        }
    }

    pub async fn read_to_string(
        &self,
        path: impl AsRef<Path>,
        reason: impl Into<String>,
    ) -> Result<String> {
        let path = path.as_ref().to_path_buf();
        self.authorize(FileOperation::Read, &path, reason.into())
            .await?;
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
    }

    pub async fn write_string(
        &self,
        path: impl AsRef<Path>,
        content: &str,
        reason: impl Into<String>,
    ) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        self.authorize(FileOperation::Write, &path, reason.into())
            .await?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create parent {}", parent.display()))?;
        }
        fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
    }

    async fn authorize(&self, operation: FileOperation, path: &Path, reason: String) -> Result<()> {
        let request = FileAccessRequest {
            operation,
            path: path.to_path_buf(),
            reason,
        };

        if !self.path_is_in_authorized_root(path)
            && !matches!(self.level, FileAccessLevel::Unlimited)
        {
            self.audit.log(
                AuditEvent::new("file.access", path.display().to_string())
                    .denied("path is outside authorized roots"),
            )?;
            return Err(anyhow!(
                "path is outside authorized roots: {}",
                path.display()
            ));
        }

        match self.level {
            FileAccessLevel::Unlimited => {
                self.audit.log(
                    AuditEvent::new("file.access", path.display().to_string())
                        .with_metadata(json!({ "operation": operation, "mode": "unlimited" })),
                )?;
                Ok(())
            }
            FileAccessLevel::Disabled => match operation {
                FileOperation::Read | FileOperation::Inspect => {
                    self.ask_user(path, request, "disabled-read").await
                }
                FileOperation::Write | FileOperation::Delete | FileOperation::Execute => {
                    self.audit.log(
                        AuditEvent::new("file.access", path.display().to_string())
                            .denied("disabled mode blocks write/delete/execute"),
                    )?;
                    Err(anyhow!("disabled mode blocks {:?}", operation))
                }
            },
            FileAccessLevel::Intermediate => self.ask_user(path, request, "intermediate").await,
        }
    }

    async fn ask_user(&self, path: &Path, request: FileAccessRequest, mode: &str) -> Result<()> {
        self.audit.log(AuditEvent {
            decision: AuditDecision::NeedsUserApproval,
            ..AuditEvent::new("file.access.request", path.display().to_string())
                .with_metadata(json!({ "operation": request.operation, "mode": mode }))
        })?;

        match self.broker.request_file_access(&request).await? {
            PermissionDecision::Allow => {
                self.audit.log(
                    AuditEvent::new("file.access", path.display().to_string())
                        .with_metadata(json!({ "operation": request.operation, "mode": mode })),
                )?;
                Ok(())
            }
            PermissionDecision::Deny { reason } => {
                self.audit.log(
                    AuditEvent::new("file.access", path.display().to_string())
                        .denied(reason.clone()),
                )?;
                Err(anyhow!(
                    "permission denied for {}: {reason}",
                    path.display()
                ))
            }
        }
    }

    fn path_is_in_authorized_root(&self, path: &Path) -> bool {
        let absolute_path = path
            .canonicalize()
            .or_else(|_| path.parent().unwrap_or(path).canonicalize());

        let Ok(absolute_path) = absolute_path else {
            return false;
        };

        self.authorized_roots.iter().any(|root| {
            root.canonicalize()
                .map(|absolute_root| absolute_path.starts_with(absolute_root))
                .unwrap_or(false)
        })
    }
}
