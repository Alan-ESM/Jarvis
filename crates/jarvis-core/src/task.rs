use jarvis_providers::ModelTier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntentKind {
    SimpleAnswer,
    WebResearch,
    CodeGeneration,
    FileOperation,
    SandboxRun,
    LogAnalysis,
    PcInspection,
    VoiceCommand,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    GoogleSearch,
    FileSystem,
    Git,
    Sandbox,
    WindowsLogs,
    PcInspection,
    Microphone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub intent: IntentKind,
    pub model_chain: Vec<ModelTier>,
    pub tools: Vec<ToolKind>,
}
