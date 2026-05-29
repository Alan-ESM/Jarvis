use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MicrophoneState {
    Disabled,
    Idle,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone)]
pub struct PushToTalkController {
    state: MicrophoneState,
}

impl Default for PushToTalkController {
    fn default() -> Self {
        Self {
            state: MicrophoneState::Idle,
        }
    }
}

impl PushToTalkController {
    pub fn state(&self) -> MicrophoneState {
        self.state
    }

    pub fn enable(&mut self) {
        if self.state == MicrophoneState::Disabled {
            self.state = MicrophoneState::Idle;
        }
    }

    pub fn disable(&mut self) {
        self.state = MicrophoneState::Disabled;
    }

    pub fn start_push_to_talk(&mut self) {
        if self.state == MicrophoneState::Idle {
            self.state = MicrophoneState::Recording;
        }
    }

    pub fn stop_and_transcribe(&mut self) {
        if self.state == MicrophoneState::Recording {
            self.state = MicrophoneState::Transcribing;
        }
    }

    pub fn finish_transcription(&mut self) {
        if self.state == MicrophoneState::Transcribing {
            self.state = MicrophoneState::Idle;
        }
    }
}
