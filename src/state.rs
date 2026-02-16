#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Listening,
    Transcribing,
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    StartListening,
    StopListening,
    BeginTranscription,
    TranscriptionFinished,
    Fail(String),
    Reset,
}

impl AppState {
    pub fn on_event(&self, event: AppEvent) -> Self {
        match (self, event) {
            (Self::Idle, AppEvent::StartListening) => Self::Listening,
            (Self::Listening, AppEvent::StopListening) => Self::Idle,
            (Self::Idle, AppEvent::BeginTranscription) => Self::Transcribing,
            (Self::Listening, AppEvent::BeginTranscription) => Self::Transcribing,
            (Self::Transcribing, AppEvent::TranscriptionFinished) => Self::Idle,
            (_, AppEvent::Fail(message)) => Self::Error(message),
            (Self::Error(_), AppEvent::Reset) => Self::Idle,
            (_, _) => self.clone(),
        }
    }

    pub fn status_text(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Listening => "Listening",
            Self::Transcribing => "Transcribing",
            Self::Error(_) => "Error",
        }
    }

    pub fn can_start_listening(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub fn can_stop_listening(&self) -> bool {
        matches!(self, Self::Listening)
    }

    pub fn can_reset(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}
