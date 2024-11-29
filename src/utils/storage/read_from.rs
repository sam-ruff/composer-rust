use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::PersistedApplication;
use anyhow::anyhow;
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::PathBuf;

pub fn get_all_from_storage() -> anyhow::Result<Vec<PersistedApplication>> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");

    let file = match OpenOptions::new()
        .read(true)
        .open(&composer_json_config_dir)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Could not open file '{:?}'", &composer_json_config_dir))
        }
    };

    let reader = BufReader::new(file);
    let applications: Vec<PersistedApplication> =
        serde_json::from_reader(reader).with_context(|| "Could not parse JSON in config.json")?;

    Ok(applications)
}

pub fn get_application_by_id(id: &str) -> anyhow::Result<PersistedApplication> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");
    let mut file = File::open(composer_json_config_dir)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let applications: Vec<PersistedApplication> = serde_json::from_str(&contents)?;
    for application in applications {
        if application.id == id {
            return Ok(application);
        }
    }
    Err(anyhow!("Application with id {} not found", id))
}

pub fn if_application_exists(id: &str) -> bool {
    match get_application_by_id(id) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::storage::models::{ApplicationState, PersistedApplication};
    use crate::utils::storage::read_from::{get_all_from_storage, get_application_by_id};
    use crate::utils::storage::write_to_storage::append_to_storage;
    use crate::utils::test_utils::{
        backup_composer_config, create_file_with_contents, move_file_if_exists,
    };

    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_all_from_storage_sunny_day() -> anyhow::Result<()> {
        // Backup config.json
        let (composer_json_config, composer_json_config_backup) = backup_composer_config()?;
        let id = "test_get_all_from_storage_sunny_day_1";
        let app = PersistedApplication {
            id: id.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec!["abc".to_string()],
        };
        let id2 = "test_get_all_from_storage_sunny_day_2";
        let app2 = PersistedApplication {
            id: id2.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec![],
        };
        // Append both apps to storage
        append_to_storage(&app)?;
        append_to_storage(&app2)?;

        let all_apps = get_all_from_storage()?;

        // Before we assert restore previous config file
        move_file_if_exists(&composer_json_config_backup, &composer_json_config)?;
        // Assert that both app and app2 are in all_apps
        assert!(all_apps.contains(&app));
        assert!(all_apps.contains(&app2));
        Ok(())
    }

    #[test]
    #[serial]
    fn test_read_bad_file() -> anyhow::Result<()> {
        // Backup config.json
        let (composer_json_config, composer_json_config_backup) = backup_composer_config()?;
        // Write invalid config to the config.json
        create_file_with_contents(&composer_json_config, "invalid")?;
        // try to append the app to storage, should fail.
        let err = get_all_from_storage().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "Could not parse JSON in config.json".to_string();
        // Before we assert restore previous config file
        move_file_if_exists(&composer_json_config_backup, &composer_json_config)?;
        // Assert the error string is correct
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_application_by_id() -> anyhow::Result<()> {
        // Backup config.json
        let (composer_json_config, composer_json_config_backup) = backup_composer_config()?;
        let id = "test_get_application_by_id";
        let app = PersistedApplication {
            id: id.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec!["abc".to_string(), "def".to_string()],
        };
        let id2 = "not_looked_for";
        let app2 = PersistedApplication {
            id: id2.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec![],
        };
        // Append both apps to storage
        append_to_storage(&app)?;
        append_to_storage(&app2)?;

        let returned_app = get_application_by_id(&id)?;

        // Before we assert restore previous config file
        move_file_if_exists(&composer_json_config_backup, &composer_json_config)?;
        // Assert what was stored an returned are the same
        assert_eq!(&app, &returned_app);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_application_by_id_not_found() -> anyhow::Result<()> {
        // Backup config.json
        let (composer_json_config, composer_json_config_backup) = backup_composer_config()?;
        let id = "test_get_application_by_id_not_found";
        let app = PersistedApplication {
            id: id.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
            value_files: vec![],
        };
        // Append both apps to storage
        append_to_storage(&app)?;

        let err = get_application_by_id("not_found").unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "Application with id not_found not found".to_string();

        // Before we assert restore previous config file
        move_file_if_exists(&composer_json_config_backup, &composer_json_config)?;
        // Assert what was stored an returned are the same
        assert_eq!(expected_err, actual_err);
        Ok(())
    }
}
