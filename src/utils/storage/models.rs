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

// The serde renames preserve the upper-case variant names already
// written to existing config.json files.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ApplicationState {
    #[serde(rename = "STARTING")]
    Starting,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "ERROR")]
    Error,
}

use std::fmt;

impl fmt::Display for ApplicationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self {
            ApplicationState::Starting => "STARTING",
            ApplicationState::Running => "RUNNING",
            ApplicationState::Error => "ERROR",
        };
        write!(f, "{:<15}", state_str)
    }
}
