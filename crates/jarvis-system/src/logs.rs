use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub timestamp: Option<DateTime<Utc>>,
    pub source: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogAnalysis {
    pub source: String,
    pub error_count: usize,
    pub warning_count: usize,
    pub anomalies: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Default, Clone)]
pub struct LogAnalyzer;

impl LogAnalyzer {
    pub fn analyze_text(&self, source: impl Into<String>, raw: &str) -> LogAnalysis {
        let source = source.into();
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut anomalies = Vec::new();

        for line in raw.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("error")
                || lower.contains("failed")
                || lower.contains("exception")
                || lower.contains("panic")
            {
                error_count += 1;
                anomalies.push(line.trim().to_string());
            } else if lower.contains("warning")
                || lower.contains("denied")
                || lower.contains("timeout")
            {
                warning_count += 1;
            }
        }

        let summary = if error_count == 0 && warning_count == 0 {
            "No obvious error pattern detected.".to_string()
        } else {
            format!("{error_count} error patterns and {warning_count} warning patterns detected.")
        };

        LogAnalysis {
            source,
            error_count,
            warning_count,
            anomalies,
            summary,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct WindowsLogReader;

impl WindowsLogReader {
    pub fn read_recent(&self, channel: &str, limit: usize) -> Result<String> {
        let count = format!("/c:{limit}");
        let output = Command::new("wevtutil")
            .args(["qe", channel, &count, "/f:text", "/rd:true"])
            .output()
            .with_context(|| format!("failed to invoke wevtutil for channel {channel}"))?;

        if !output.status.success() {
            anyhow::bail!(
                "wevtutil failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
