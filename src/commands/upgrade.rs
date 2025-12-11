use crate::commands::install::add_application;
use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::docker_compose::compose_down;
use crate::utils::storage::read_from::get_application_by_id;
use crate::utils::walk::get_files_with_names;
use anyhow::anyhow;
use clap::Args;
use std::fs::remove_dir_all;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct Upgrade {
    #[clap(index = 1)]
    pub directory: PathBuf,
    #[clap(short, long)]
    pub id: Option<String>,
    #[clap(short, long)]
    pub value_files: Vec<String>,
}

impl Upgrade {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("Command: {:?}", self);

        let install_id = match &self.id {
            Some(id) => id,
            None => {
                return Err(anyhow!("Could not get ID to upgrade."));
            }
        };

        // Ensure the .composer directory exists
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(install_id);
        trace!(
            "Checking existence of directory: '{}'",
            composer_id_directory.display()
        );
        if !composer_id_directory.exists() {
            return Err(anyhow!(format!(
                "An application with the id '{}' does not exist. Did you mean to `composer install {}` instead?",
                install_id, install_id
            )));
        }

        // Determine the value files to use
        let value_files = if self.value_files.is_empty() {
            // Retrieve the persisted application
            let application = get_application_by_id(install_id)?;
            // Use the previously stored value files
            if application.value_files.is_empty() {
                return Err(anyhow!(
                    "Cannot upgrade application '{}' because no value files were provided and none were found from the previous installation. Use -v <values path> to specify value files.",
                    install_id
                ));
            }
            application.value_files.clone()
        } else {
            self.value_files.clone()
        };

        // Stop containers/networks before removing directory
        let compose_files = get_files_with_names(
            composer_id_directory.to_str().unwrap(),
            &["docker-compose.jinja2", "docker-compose.j2"],
        );
        for compose_file in compose_files {
            compose_down(&compose_file, install_id);
        }

        // Remove the existing directory
        remove_dir_all(&composer_id_directory)?;
        info!("Upgrading application with ID: {}", install_id);

        add_application(
            install_id,
            &composer_id_directory,
            true,
            &value_files,
            &self.directory,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::storage::models::{ApplicationState, PersistedApplication};
    use crate::utils::storage::read_from::get_application_by_id;
    use crate::utils::storage::write_to_storage::append_to_storage;
    use crate::utils::test_utils::clean_up_test_folder;
    use relative_path::RelativePath;
    use serial_test::serial;
    use std::env::current_dir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    #[serial]
    fn test_upgrade_without_id() -> anyhow::Result<()> {
        // Test that trying to upgrade without an ID results in an error
        trace!("Running test_upgrade_without_id.");
        let upgrade_cmd = Upgrade {
            directory: PathBuf::from("some/directory"),
            id: None,
            value_files: vec![],
        };
        let err = upgrade_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "Could not get ID to upgrade.";
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_upgrade_nonexistent_application() -> anyhow::Result<()> {
        // Test that trying to upgrade a nonexistent application results in an error
        trace!("Running test_upgrade_nonexistent_application.");
        let id = "nonexistent_app";
        let current_dir = current_dir()?;
        let upgrade_dir = RelativePath::new("resources/test/simple/")
            .to_logical_path(&current_dir);
        let upgrade_cmd = Upgrade {
            directory: upgrade_dir,
            id: Some(id.to_string()),
            value_files: vec![],
        };
        let err = upgrade_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = format!(
            "An application with the id '{}' does not exist. Did you mean to `composer install {}` instead?",
            id, id
        );
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_upgrade_no_value_files_provided_and_none_stored() -> anyhow::Result<()> {
        // Test that upgrading without value files when none were previously stored results in an error
        trace!("Running test_upgrade_no_value_files_provided_and_none_stored.");
        let id = "test_upgrade_no_values";
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);

        // Simulate that the application exists without stored value files
        // Create the application directory
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(id);
        if !composer_id_directory.exists() {
            fs::create_dir_all(&composer_id_directory)?;
        }

        // Create a persisted application with empty value_files
        let app = PersistedApplication {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            timestamp: 0,
            state: ApplicationState::RUNNING,
            app_name: "Test App".to_string(),
            compose_path: install_dir.to_string_lossy().to_string(),
            value_files: vec![], // Empty value_files
        };
        append_to_storage(&app)?;

        // Now, try to upgrade
        let upgrade_cmd = Upgrade {
            directory: install_dir.clone(),
            id: Some(id.to_string()),
            value_files: vec![],
        };

        let err = upgrade_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = format!(
            "Cannot upgrade application '{}' because no value files were provided and none were found from the previous installation. Use -v <values path> to specify value files.",
            id
        );
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_upgrade_with_provided_value_files() -> anyhow::Result<()> {
        // Test that upgrading with provided value files succeeds
        trace!("Running test_upgrade_with_provided_value_files.");
        let id = "test_upgrade_with_provided_values";
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();

        // Simulate that the application exists with initial value_files
        // Create the application directory
        let composer_directory = get_composer_directory()?;
        let composer_id_directory = composer_directory.join(id);
        if !composer_id_directory.exists() {
            fs::create_dir_all(&composer_id_directory)?;
        }

        // Create a persisted application with initial value_files
        let app = PersistedApplication {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            timestamp: 0,
            state: ApplicationState::RUNNING,
            app_name: "Test App".to_string(),
            compose_path: install_dir.to_string_lossy().to_string(),
            value_files: vec![values_str.clone()],
        };
        append_to_storage(&app)?;

        // Now, upgrade with new values
        let new_values_dir = RelativePath::new("resources/test/test_values/override.yaml")
            .to_logical_path(&current_dir);
        let new_values_str = new_values_dir.to_string_lossy().to_string();

        let upgrade_cmd = Upgrade {
            directory: install_dir.clone(),
            id: Some(id.to_string()),
            value_files: vec![new_values_str.clone()],
        };

        upgrade_cmd.exec()?;

        // Retrieve the application and check that its value_files have been updated
        let app = get_application_by_id(id)?;
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert_eq!(app.value_files, vec![new_values_str]);
        assert_eq!(app.state, ApplicationState::RUNNING);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_upgrade_with_no_value_files_but_stored_values_exist() -> anyhow::Result<()> {
        // Test that upgrading without providing value files uses the stored value files
        trace!("Running test_upgrade_with_no_value_files_but_stored_values_exist.");
        let id = "test_upgrade_with_stored_values";
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();

        // Simulate that the application exists with stored value files
        // Create the application directory
        let composer_directory = get_composer_directory()?;
        let composer_id_directory = composer_directory.join(id);
        if !composer_id_directory.exists() {
            fs::create_dir_all(&composer_id_directory)?;
        }

        // Create a persisted application with initial value_files
        let app = PersistedApplication {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            timestamp: 0,
            state: ApplicationState::RUNNING,
            app_name: "Test App".to_string(),
            compose_path: install_dir.to_string_lossy().to_string(),
            value_files: vec![values_str.clone()],
        };
        append_to_storage(&app)?;

        // Now, upgrade without providing value files
        let upgrade_cmd = Upgrade {
            directory: install_dir.clone(),
            id: Some(id.to_string()),
            value_files: vec![],
        };

        upgrade_cmd.exec()?;

        // Retrieve the application and check that its value_files have not changed
        let app = get_application_by_id(id)?;
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert_eq!(app.value_files, vec![values_str]);
        assert_eq!(app.state, ApplicationState::RUNNING);
        Ok(())
    }
}