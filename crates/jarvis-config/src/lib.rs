use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct JarvisConfig {
    pub app: AppConfig,
    pub network: NetworkConfig,
    pub ai: AiConfig,
    pub google: GoogleConfig,
    pub security: SecurityConfig,
    pub sandbox: SandboxConfig,
    pub paths: PathConfig,
}

impl Default for JarvisConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            network: NetworkConfig::default(),
            ai: AiConfig::default(),
            google: GoogleConfig::default(),
            security: SecurityConfig::default(),
            sandbox: SandboxConfig::default(),
            paths: PathConfig::default(),
        }
    }
}

impl JarvisConfig {
    pub fn load() -> Result<Self> {
        let explicit_path = env::var_os("JARVIS_CONFIG").map(PathBuf::from);
        let config_path = explicit_path.or_else(default_config_path);

        match config_path {
            Some(path) if path.exists() => {
                let raw = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read config {}", path.display()))?;
                toml::from_str(&raw)
                    .with_context(|| format!("failed to parse config {}", path.display()))
            }
            _ => Ok(Self::default()),
        }
    }
}

fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("ai", "Jarvis", "Jarvis").map(|dirs| dirs.config_dir().join("config.toml"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub name: String,
    pub environment: String,
    pub audit_log_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: "Jarvis".to_string(),
            environment: "development".to_string(),
            audit_log_path: PathBuf::from("logs/audit.jsonl"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub internet_required_for_ai: bool,
    pub probe_url: String,
    pub timeout_ms: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            internet_required_for_ai: true,
            probe_url: "https://www.google.com/generate_204".to_string(),
            timeout_ms: 2500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub provider: String,
    pub base_url: String,
    pub api_key_env: String,
    pub flash_model: String,
    pub x_model: String,
    pub ultra_model: String,
    pub quality_threshold: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openai-compatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            flash_model: "gpt-5-mini".to_string(),
            x_model: "gpt-5".to_string(),
            ultra_model: "gpt-5".to_string(),
            quality_threshold: 0.74,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GoogleConfig {
    pub enabled: bool,
    pub api_key_env: String,
    pub engine_id_env: String,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key_env: "GOOGLE_SEARCH_API_KEY".to_string(),
            engine_id_env: "GOOGLE_SEARCH_ENGINE_ID".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    pub file_access_level: FileAccessLevelConfig,
    pub require_audit_log: bool,
    pub allow_unlimited_pc_inspection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            file_access_level: FileAccessLevelConfig::Intermediate,
            require_audit_log: true,
            allow_unlimited_pc_inspection: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileAccessLevelConfig {
    Intermediate,
    Unlimited,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub base_dir: PathBuf,
    pub timeout_secs: u64,
    pub cleanup_after_run: bool,
    pub max_output_bytes: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_dir: PathBuf::from(".sandbox"),
            timeout_secs: 120,
            cleanup_after_run: true,
            max_output_bytes: 200_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PathConfig {
    pub authorized_roots: Vec<PathBuf>,
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            authorized_roots: vec![PathBuf::from(".")],
        }
    }
}
