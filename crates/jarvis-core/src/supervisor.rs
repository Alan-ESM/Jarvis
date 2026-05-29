use crate::task::{IntentKind, TaskPlan, ToolKind};
use anyhow::Result;
use jarvis_audit::{AuditEvent, AuditLogger};
use jarvis_providers::{AiRequest, AiResponse, ModelTier, ProviderRegistry};
use jarvis_system::{ConnectivityState, InternetGate};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorReply {
    pub text: String,
    pub model_chain: Vec<ModelTier>,
    pub final_tier: Option<ModelTier>,
    pub offline_read_only: bool,
}

#[derive(Clone)]
pub struct JarvisSupervisor {
    providers: Arc<ProviderRegistry>,
    internet_gate: InternetGate,
    audit: AuditLogger,
    quality_threshold: f32,
}

impl JarvisSupervisor {
    pub fn new(
        providers: Arc<ProviderRegistry>,
        internet_gate: InternetGate,
        audit: AuditLogger,
        quality_threshold: f32,
    ) -> Self {
        Self {
            providers,
            internet_gate,
            audit,
            quality_threshold,
        }
    }

    pub async fn handle(&self, message: UserMessage) -> Result<SupervisorReply> {
        let connectivity = self.internet_gate.check().await;
        if !connectivity.is_online() {
            let reason = match connectivity {
                ConnectivityState::Online => String::new(),
                ConnectivityState::Offline { reason } => reason,
            };
            self.audit
                .log(AuditEvent::new("ai.blocked.offline", "supervisor").denied(reason.clone()))?;
            return Ok(SupervisorReply {
                text: format!(
                    "Mode hors ligne: les reponses IA sont bloquees. Lecture locale uniquement. Raison: {reason}"
                ),
                model_chain: Vec::new(),
                final_tier: None,
                offline_read_only: true,
            });
        }

        let plan = self.plan(&message.content);
        self.audit.log(
            AuditEvent::new("supervisor.plan", "message").with_metadata(json!({
                "intent": plan.intent,
                "model_chain": plan.model_chain,
                "tools": plan.tools,
            })),
        )?;

        let mut last_response: Option<AiResponse> = None;
        let mut last_error: Option<String> = None;

        for tier in &plan.model_chain {
            let mut request = AiRequest::new(
                &message.session_id,
                Uuid::new_v4().to_string(),
                *tier,
                build_prompt(&plan, &message.content),
            );
            request.temperature = match tier {
                ModelTier::Flash => 0.1,
                ModelTier::X => 0.25,
                ModelTier::Ultra => 0.15,
            };

            match self.providers.generate_for_tier(*tier, request).await {
                Ok(response)
                    if response.quality_score >= self.quality_threshold
                        || *tier == ModelTier::Ultra =>
                {
                    return Ok(SupervisorReply {
                        text: response.text,
                        model_chain: plan.model_chain.clone(),
                        final_tier: Some(*tier),
                        offline_read_only: false,
                    });
                }
                Ok(response) => {
                    last_response = Some(response);
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                    continue;
                }
            }
        }

        if let Some(response) = last_response {
            Ok(SupervisorReply {
                text: response.text,
                model_chain: plan.model_chain.clone(),
                final_tier: Some(response.tier),
                offline_read_only: false,
            })
        } else {
            anyhow::bail!(
                "all AI routes failed: {}",
                last_error.unwrap_or_else(|| "unknown error".to_string())
            )
        }
    }

    pub fn plan(&self, content: &str) -> TaskPlan {
        let intent = classify_intent(content);
        route_intent(intent)
    }
}

fn classify_intent(content: &str) -> IntentKind {
    let lower = content.to_ascii_lowercase();
    if lower.contains("google") || lower.contains("recherche") || lower.contains("search") {
        IntentKind::WebResearch
    } else if lower.contains("compile") || lower.contains("test") || lower.contains("sandbox") {
        IntentKind::SandboxRun
    } else if lower.contains("log") || lower.contains("erreur systeme") {
        IntentKind::LogAnalysis
    } else if lower.contains("fichier") || lower.contains("dossier") || lower.contains("commit") {
        IntentKind::FileOperation
    } else if lower.contains("inspecte mon pc")
        || lower.contains("processus")
        || lower.contains("services")
    {
        IntentKind::PcInspection
    } else if lower.contains("code") || lower.contains("genere") || lower.contains("cree") {
        IntentKind::CodeGeneration
    } else {
        IntentKind::SimpleAnswer
    }
}

fn route_intent(intent: IntentKind) -> TaskPlan {
    match intent {
        IntentKind::SimpleAnswer => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash],
            tools: vec![],
        },
        IntentKind::WebResearch => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X, ModelTier::Ultra],
            tools: vec![ToolKind::GoogleSearch],
        },
        IntentKind::CodeGeneration | IntentKind::FileOperation => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X, ModelTier::Ultra],
            tools: vec![ToolKind::FileSystem, ToolKind::Git],
        },
        IntentKind::SandboxRun => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X],
            tools: vec![ToolKind::Sandbox],
        },
        IntentKind::LogAnalysis => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X],
            tools: vec![ToolKind::WindowsLogs],
        },
        IntentKind::PcInspection => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X],
            tools: vec![ToolKind::PcInspection],
        },
        IntentKind::VoiceCommand => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash],
            tools: vec![ToolKind::Microphone],
        },
        IntentKind::Unknown => TaskPlan {
            intent,
            model_chain: vec![ModelTier::Flash, ModelTier::X],
            tools: vec![],
        },
    }
}

fn build_prompt(plan: &TaskPlan, content: &str) -> String {
    format!(
        "Intent: {:?}\nTools: {:?}\nUser request:\n{}",
        plan.intent, plan.tools, content
    )
}
