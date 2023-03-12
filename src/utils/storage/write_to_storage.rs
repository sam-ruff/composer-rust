use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::PersistedApplication;
use anyhow::anyhow;
use anyhow::Context;

use crate::utils::storage::read_from::get_all_from_storage;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read};
use std::path::PathBuf;

pub fn append_to_storage(application: &PersistedApplication) -> anyhow::Result<()> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");

    // Create ~/.composer/config.json if it doesn't exist
    if !composer_json_config_dir.exists() {
        fs::create_dir_all(&composer_directory)
            .with_context(|| format!("Could not create directory '{:?}'", &composer_directory))?;
        File::create(&composer_json_config_dir)
            .with_context(|| format!("Could not create file '{:?}'", &composer_json_config_dir))?;
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&composer_json_config_dir)
        .with_context(|| format!("Could not open file '{:?}'", &composer_json_config_dir))?;

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let mut applications: Vec<PersistedApplication> = if contents.trim().is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&contents)?
    };

    let id = &application.id;
    applications.retain(|a| a.id != *id);
    applications.push(application.clone());

    let writer = BufWriter::new(
        File::create(&composer_json_config_dir)
            .with_context(|| format!("Could not create file '{:?}'", &composer_json_config_dir))?,
    );
    serde_json::to_writer(writer, &applications)?;

    Ok(())
}

pub fn delete_application_by_id(id: &str) -> anyhow::Result<()> {
    let applications = get_all_from_storage()?;
    let mut found = false;
    let new_applications: Vec<PersistedApplication> = applications
        .into_iter()
        .filter_map(|application| {
            if application.id == id {
                found = true;
                None
            } else {
                Some(application)
            }
        })
        .collect();
    if found {
        let composer_directory = get_composer_directory()?;
        let composer_json_config_dir: std::path::PathBuf = composer_directory.join("config.json");
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&composer_json_config_dir)
            .with_context(|| format!("Could not open file '{:?}'", &composer_json_config_dir))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &new_applications)
            .with_context(|| "Could not serialize JSON to config.json")?;
        Ok(())
    } else {
        Err(anyhow!("Application with id {} not found", id))
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::storage::models::{ApplicationState, PersistedApplication};
    use crate::utils::storage::read_from::{get_application_by_id, if_application_exists};
    use crate::utils::storage::write_to_storage::{append_to_storage, delete_application_by_id};
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_write_to_storage_sunny_day() -> anyhow::Result<()> {
        let id = "sunny_day_storage";
        let app = PersistedApplication {
            id: id.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
        };
        // Append the app to storage
        append_to_storage(&app)?;
        // Check if app exists
        let app_exist = if_application_exists(id);
        // Get the app, but don't fail immediately
        let retrieved_app_result = get_application_by_id(id);
        // Clean up before the assert in case it fails
        // This might fail but we tried
        let _ = delete_application_by_id(id);
        // Assert that the app serialised and de-serialised correctly
        assert_eq!(true, app_exist);
        // Assert that the app retrieved from storage is correct
        let retrieved_app = retrieved_app_result?;
        assert_eq!(app, retrieved_app);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_delete_sunny_day() -> anyhow::Result<()> {
        let id = "delete_sunny_day";
        let app = PersistedApplication {
            id: id.to_string(),
            version: "123".to_string(),
            timestamp: 0,
            state: ApplicationState::STARTING,
            app_name: id.to_string(),
            compose_path: id.to_string(),
        };
        // Append the app to storage
        append_to_storage(&app)?;
        // Delete the app
        delete_application_by_id(id)?;
        // Check if app exists
        let app_exist = if_application_exists(id);
        // Assert that the app serialised and de-serialised correctly
        assert_eq!(false, app_exist);
        Ok(())
    }
}
