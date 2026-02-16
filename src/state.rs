#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppState {
    Idle,
    Listening,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    StartListening,
    StopListening,
}

impl AppState {
    pub fn on_event(&self, event: AppEvent) -> Self {
        match (self, event) {
            (Self::Idle, AppEvent::StartListening) => Self::Listening,
            (Self::Listening, AppEvent::StopListening) => Self::Idle,
            (_, _) => self.clone(),
        }
    }

    pub fn status_text(&self) -> &str {
        match self {
            Self::Idle => "Idle",
            Self::Listening => "Listening",
        }
    }
}
