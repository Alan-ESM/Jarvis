use anyhow::{Context, Result};
use jarvis_audit::{AuditEvent, AuditLogger};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::PathBuf, time::Duration};
use tempfile::Builder;
use tokio::{process::Command, time::timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub base_dir: PathBuf,
    pub timeout_secs: u64,
    pub cleanup_after_run: bool,
    pub max_output_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxReport {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: String,
    pub stderr: String,
    pub working_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SandboxRunner {
    policy: SandboxPolicy,
    audit: AuditLogger,
}

impl SandboxRunner {
    pub fn new(policy: SandboxPolicy, audit: AuditLogger) -> Self {
        Self { policy, audit }
    }

    pub async fn run(&self, command: SandboxCommand) -> Result<SandboxReport> {
        std::fs::create_dir_all(&self.policy.base_dir).with_context(|| {
            format!(
                "failed to create sandbox base dir {}",
                self.policy.base_dir.display()
            )
        })?;

        let temp_dir = Builder::new()
            .prefix("jarvis-run-")
            .tempdir_in(&self.policy.base_dir)
            .context("failed to create sandbox temp dir")?;
        let working_dir = temp_dir.path().to_path_buf();

        self.audit.log(
            AuditEvent::new("sandbox.run", &command.program).with_metadata(json!({
                "args": &command.args,
                "reason": &command.reason,
                "working_dir": working_dir.display().to_string(),
            })),
        )?;

        let mut child = Command::new(&command.program);
        child
            .args(&command.args)
            .current_dir(&working_dir)
            .kill_on_drop(true);

        let output = match timeout(
            Duration::from_secs(self.policy.timeout_secs),
            child.output(),
        )
        .await
        {
            Ok(result) => result.context("sandbox command failed to start")?,
            Err(_) => {
                return Ok(SandboxReport {
                    exit_code: None,
                    timed_out: true,
                    stdout: String::new(),
                    stderr: format!("timeout after {} seconds", self.policy.timeout_secs),
                    working_dir,
                });
            }
        };

        Ok(SandboxReport {
            exit_code: output.status.code(),
            timed_out: false,
            stdout: truncate(
                String::from_utf8_lossy(&output.stdout).to_string(),
                self.policy.max_output_bytes,
            ),
            stderr: truncate(
                String::from_utf8_lossy(&output.stderr).to_string(),
                self.policy.max_output_bytes,
            ),
            working_dir,
        })
    }
}

fn truncate(mut value: String, max_bytes: usize) -> String {
    if value.len() > max_bytes {
        value.truncate(max_bytes);
        value.push_str("\n[truncated]");
    }
    value
}
