use crate::utils::load_values::MergeOptions;
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
    #[serde(default)]
    pub merge_options: MergeOptions,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialises_legacy_entry_without_merge_options() -> anyhow::Result<()> {
        // Config written before merge_options existed must load with defaults
        let legacy = r#"{
            "id": "legacy-app",
            "version": "1.0.0",
            "timestamp": 1700000000,
            "state": "RUNNING",
            "app_name": "legacy",
            "compose_path": "/tmp/legacy",
            "value_files": ["values.yaml"]
        }"#;
        let application: PersistedApplication = serde_json::from_str(legacy)?;
        assert_eq!(application.merge_options, MergeOptions::default());
        assert!(!application.merge_options.overwrite_lists);
        assert!(!application.merge_options.overwrite_maps);
        Ok(())
    }
}
