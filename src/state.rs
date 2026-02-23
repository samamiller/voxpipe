use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Listening,
    Transcribing,
    TranscribingFile { path: PathBuf },
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    StartListening,
    StopListening,
    #[allow(dead_code)]
    StartFileTranscription(PathBuf),
    TranscriptionReady,
    #[allow(dead_code)]
    Fail(String),
}

impl AppState {
    pub fn on_event(&self, event: AppEvent) -> Self {
        match (self, event) {
            (Self::Idle, AppEvent::StartListening) => Self::Listening,
            (Self::Error(_), AppEvent::StartListening) => Self::Listening,
            (Self::Listening, AppEvent::StopListening) => Self::Transcribing,
            (Self::Idle, AppEvent::StartFileTranscription(path)) => {
                Self::TranscribingFile { path }
            }
            (Self::Error(_), AppEvent::StartFileTranscription(path)) => {
                Self::TranscribingFile { path }
            }
            (Self::Transcribing, AppEvent::TranscriptionReady) => Self::Idle,
            (Self::TranscribingFile { .. }, AppEvent::TranscriptionReady) => Self::Idle,
            (_, AppEvent::Fail(err)) => Self::Error(err),
            (_, _) => self.clone(),
        }
    }

    pub fn status_text(&self) -> String {
        match self {
            Self::Idle => "Idle".to_string(),
            Self::Listening => "Listening".to_string(),
            Self::Transcribing => "Transcribing".to_string(),
            Self::TranscribingFile { path } => {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file");
                format!("Transcribing file: {name}")
            }
            Self::Error(err) => format!("Error: {err}"),
        }
    }

    pub fn mic_enabled(&self) -> bool {
        matches!(self, Self::Idle | Self::Listening | Self::Error(_))
    }

    #[allow(dead_code)]
    pub fn file_enabled(&self) -> bool {
        matches!(self, Self::Idle | Self::Error(_))
    }

    #[allow(dead_code)]
    pub fn model_enabled(&self) -> bool {
        matches!(self, Self::Idle | Self::Error(_))
    }
}
