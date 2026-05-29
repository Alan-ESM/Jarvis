use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelTier {
    Flash,
    X,
    Ultra,
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelTier::Flash => write!(f, "Flash"),
            ModelTier::X => write!(f, "X"),
            ModelTier::Ultra => write!(f, "Ultra"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoute {
    pub tier: ModelTier,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub session_id: String,
    pub task_id: String,
    pub tier: ModelTier,
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub temperature: f32,
    pub max_output_tokens: Option<u32>,
}

impl AiRequest {
    pub fn new(
        session_id: impl Into<String>,
        task_id: impl Into<String>,
        tier: ModelTier,
        user_prompt: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            task_id: task_id.into(),
            tier,
            model: String::new(),
            system_prompt: default_system_prompt(tier),
            user_prompt: user_prompt.into(),
            temperature: 0.2,
            max_output_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub tier: ModelTier,
    pub model: String,
    pub text: String,
    pub quality_score: f32,
    pub provider_request_id: Option<String>,
}

fn default_system_prompt(tier: ModelTier) -> String {
    match tier {
        ModelTier::Flash => {
            "You are Jarvis Flash. Classify intent fast and answer only simple tasks.".to_string()
        }
        ModelTier::X => {
            "You are Jarvis X. Reason carefully, plan tasks, and decompose work.".to_string()
        }
        ModelTier::Ultra => {
            "You are Jarvis Ultra. Produce final, high-quality, critical outputs.".to_string()
        }
    }
}
