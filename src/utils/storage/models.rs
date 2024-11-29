use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PersistedApplication {
    pub id: String,
    pub version: String,
    pub timestamp: i64,
    pub state: ApplicationState,
    pub app_name: String,
    pub compose_path: String,
    #[serde(default)]
    pub value_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ApplicationState {
    STARTING,
    RUNNING,
    ERROR,
}

use std::fmt;

impl fmt::Display for ApplicationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self {
            ApplicationState::STARTING => "STARTING",
            ApplicationState::RUNNING => "RUNNING",
            ApplicationState::ERROR => "ERROR",
        };
        write!(f, "{:<15}", state_str)
    }
}
