use anyhow::{Context, Result};
use std::{path::PathBuf, process::Command};

#[derive(Debug, Clone)]
pub struct GitClient {
    repo: PathBuf,
}

impl GitClient {
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        Self { repo: repo.into() }
    }

    pub fn status_short(&self) -> Result<String> {
        self.run(["status", "--short"])
    }

    pub fn current_branch(&self) -> Result<String> {
        self.run(["branch", "--show-current"])
    }

    fn run<const N: usize>(&self, args: [&str; N]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .output()
            .with_context(|| format!("failed to invoke git in {}", self.repo.display()))?;

        if !output.status.success() {
            anyhow::bail!("git failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
