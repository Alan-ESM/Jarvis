use anyhow::{Context, Result};
use std::process::Command;

pub fn tasklist_csv() -> Result<String> {
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .context("failed to invoke tasklist")?;

    if !output.status.success() {
        anyhow::bail!(
            "tasklist failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn services_text() -> Result<String> {
    let output = Command::new("sc")
        .args(["query", "type=", "service", "state=", "all"])
        .output()
        .context("failed to invoke sc query")?;

    if !output.status.success() {
        anyhow::bail!(
            "sc query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
