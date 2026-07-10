use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::{ApplicationState, PersistedApplication};
use crate::utils::storage::read_from::get_all_from_storage;
use anyhow::Context;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub fn update_application_state(id: &str, new_state: ApplicationState) -> anyhow::Result<()> {
    update_persisted_application_by_id(id, |mut application| {
        application.state = new_state.clone();
        application
    })
}

pub fn update_persisted_application_by_id<F>(
    id: &str,
    mut modify_application: F,
) -> anyhow::Result<()>
where
    F: FnMut(PersistedApplication) -> PersistedApplication,
{
    let applications = get_all_from_storage()?;
    let new_applications: Vec<PersistedApplication> = applications
        .into_iter()
        .map(|application| {
            if application.id == id {
                modify_application(application)
            } else {
                application
            }
        })
        .collect();
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&composer_json_config_dir)
        .with_context(|| format!("Could not open file '{:?}'", composer_json_config_dir))?;
    let mut writer = BufWriter::new(file);
    let json_data = serde_json::to_vec(&new_applications)
        .with_context(|| "Could not serialize JSON to config.json")?;
    writer
        .write_all(&json_data)
        .with_context(|| "Could not write JSON to config.json")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::storage::read_from::get_application_by_id;
    use crate::utils::storage::write_to_storage::append_to_storage;
    use crate::utils::test_utils::ComposerHomeGuard;
    use serial_test::serial;

    fn test_app(id: &str) -> PersistedApplication {
        PersistedApplication {
            id: id.to_string(),
            version: "1".to_string(),
            timestamp: 0,
            state: ApplicationState::Starting,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec![],
        }
    }

    #[test]
    #[serial]
    fn test_update_application_state_round_trip() -> anyhow::Result<()> {
        let _home = ComposerHomeGuard::new()?;
        let id = "update_state_round_trip";
        append_to_storage(&test_app(id))?;
        update_application_state(id, ApplicationState::Error)?;
        let updated = get_application_by_id(id)?;
        assert_eq!(ApplicationState::Error, updated.state);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_update_application_state_leaves_other_apps_untouched() -> anyhow::Result<()> {
        let _home = ComposerHomeGuard::new()?;
        let target = "update_state_target";
        let other = "update_state_other";
        append_to_storage(&test_app(target))?;
        append_to_storage(&test_app(other))?;
        update_application_state(target, ApplicationState::Running)?;
        assert_eq!(
            ApplicationState::Running,
            get_application_by_id(target)?.state
        );
        assert_eq!(
            ApplicationState::Starting,
            get_application_by_id(other)?.state
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_update_persisted_application_modifies_fields() -> anyhow::Result<()> {
        let _home = ComposerHomeGuard::new()?;
        let id = "update_persisted_fields";
        append_to_storage(&test_app(id))?;
        update_persisted_application_by_id(id, |mut application| {
            application.version = "2".to_string();
            application
        })?;
        assert_eq!("2", get_application_by_id(id)?.version);
        Ok(())
    }
}
