use crate::app;
use crate::utils::copy_file_utils::{copy_files_with_ignorefile, get_composer_directory};
use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::walk::get_files_with_extension;
use anyhow::anyhow;

use crate::utils::storage::app_yaml::load_app_yaml;
use crate::utils::storage::models::{ApplicationState, PersistedApplication};
use crate::utils::storage::write_to_storage::append_to_storage;
use clap::Args;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Args)]
pub struct Install {
    #[clap(index = 1)]
    pub directory: PathBuf,
    #[clap(short, long)]
    pub id: Option<String>,
    #[clap(short, long)]
    pub value_files: Vec<String>,
}

impl Install {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("Command: {:?}", self);
        let readable_id = &Self::get_readable_id();
        let install_id: &String = self.id.as_ref().unwrap_or(readable_id);
        info!("Installing application with ID: {}", install_id);
        if self.value_files.is_empty() {
            return Err(anyhow!(
                "You cannot install an application with no values file. Use -v <values path> to specify values file."
            ));
        }
        let values = get_value_files_as_refs(&self.value_files);
        let consolidated_values = load_yaml_files(&values)?;
        trace!(
            "Consolidated values: \n```\n{}\n```\n",
            serde_yaml::to_string(&consolidated_values).unwrap()
        );
        // Ensure the .composer directory exists
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(install_id);
        trace!("Creating directory: '{}'", composer_id_directory.display());
        if composer_id_directory.exists() {
            return Err(anyhow!(format!("An application with the id '{}' already exists. Did you mean to `composer upgrade {}` instead?", install_id, install_id)));
        }
        if !self.directory.exists() {
            return Err(anyhow!(format!(
                "Template directory {} does not exist.",
                &self.directory.display()
            )));
        }
        // Check for app.yaml and docker-compose.jinja2
        self.verify_required_files()?;
        // Check if there is an ignore file
        let mut ignore_file_optional: Option<&Path> = None;
        let composer_ignore_path = self.directory.join(".composerignore");
        if composer_id_directory.exists() {
            ignore_file_optional = Some(composer_ignore_path.as_path());
        }
        // Create the directory to copy the files to
        fs::create_dir_all(&composer_id_directory)?;

        // Copy the files to the .composer directory  using the ID as the folder name
        copy_files_with_ignorefile(
            &self.directory,
            &composer_id_directory,
            ignore_file_optional,
        )?;
        // Replace the jinja files with templated ones
        let files_to_replace = get_files_with_extension(self.directory.to_str().unwrap(), "jinja2");
        trace!("Detected templates: {}", files_to_replace.join(","));
        // Read App.yaml to get some of the needed values
        let app_yaml_path = self.directory.join("app.yaml");
        let app_yaml = load_app_yaml(app_yaml_path)?;
        // Create the persisted application struct
        let mut application = PersistedApplication {
            id: install_id.to_string(),
            version: app_yaml.version,
            timestamp: self.get_current_timestamp(),
            state: ApplicationState::STARTING,
            app_name: app_yaml.name,
            compose_path: self.directory.to_string_lossy().to_string(),
        };
        // TODO For each template render then replace them with actual file

        // Change status of app to starting
        append_to_storage(&application)?;

        if *app::always_pull() {
            info!("Always pull is enabled. Pulling latest docker images.");
            // TODO
        }

        // TODO Run docker-compose up -f docker-compose.jinja2, print stdout + stderr
        // TODO Add a global for under test or work out mocking. Probably call out
        // To a function in a mod, check for testing, I know it sucks
        self.docker_compose_up();
        // TODO If it errors change the status to error

        // Change status of app to running
        application.state = ApplicationState::RUNNING;
        append_to_storage(&application)?;

        Ok(())
    }

    fn verify_required_files(&self) -> anyhow::Result<()> {
        self.verify_file_exists("app.yaml")?;
        self.verify_file_exists("docker-compose.jinja2")?;
        Ok(())
    }

    fn get_current_timestamp(&self) -> i64 {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

        duration_since_epoch.as_secs() as i64
    }

    fn verify_file_exists(&self, file_name: &str) -> anyhow::Result<()> {
        let file_path = self.directory.join(file_name);
        if !file_path.exists() {
            return Err(anyhow!(format!(
                "Could not find {} at {}",
                file_name,
                file_path.display()
            )));
        }
        Ok(())
    }

    fn get_readable_id() -> String {
        petname::petname(3, "-").to_string()
    }
    fn docker_compose_up(&self) {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    use relative_path::RelativePath;

    use crate::commands::install::Install;

    use crate::utils::copy_file_utils::get_composer_directory;
    use crate::utils::storage::models::ApplicationState;
    use crate::utils::storage::read_from::{get_application_by_id, if_application_exists};
    use crate::utils::storage::write_to_storage::delete_application_by_id;
    use serial_test::serial;
    use std::env::current_dir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    #[serial]
    fn test_failed_install_no_values() -> anyhow::Result<()> {
        trace!("Running test_failed_install_no_values.");
        let current_dir = current_dir()?;
        let install_dir = RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let id = "install_no_values";
        let test_install_cmd = Install {
            directory: install_dir,
            id: Some(id.to_string()),
            value_files: vec![],
        };
        let err = test_install_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "You cannot install an application with no values file. Use -v <values path> to specify values file.".to_string();
        clean_up_test_folder(id)?;
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_failed_install_invalid_values() -> anyhow::Result<()> {
        trace!("Running test_failed_install_no_values.");
        let current_dir = current_dir()?;
        let install_dir = RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let id = "test_failed_install_invalid_values";
        let test_install_cmd = Install {
            directory: install_dir,
            id: Some(id.to_string()),
            value_files: vec![String::from("doesNotExist.yaml")],
        };
        let err = test_install_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "Failed to read values YAML file: doesNotExist.yaml".to_string();
        clean_up_test_folder(id)?;
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_no_app_yaml() -> anyhow::Result<()> {
        trace!("Running test_no_app_yaml.");
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simpleNoApp/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(current_dir);
        let values_str = values_dir.to_string_lossy().to_string();
        let app_str = install_dir.join("app.yaml").to_string_lossy().to_string();
        let id = "test_no_app_yaml";
        let test_install_cmd = Install {
            directory: install_dir,
            id: Some(id.to_string()),
            value_files: vec![values_str],
        };
        let err = test_install_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = format!("Could not find app.yaml at {}", app_str);
        clean_up_test_folder(id)?;
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_failed_install_no_template_path() -> anyhow::Result<()> {
        trace!("Running test_failed_install_no_template_path.");
        let id = "test_failed_install_no_template_path";
        let current_dir = current_dir()?;
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(current_dir);
        let values_str = values_dir.to_string_lossy().to_string();
        let test_install_cmd = Install {
            directory: PathBuf::from("does_not_exist"),
            id: Some(id.to_string()),
            value_files: vec![values_str],
        };
        let err = test_install_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "Template directory does_not_exist does not exist.".to_string();
        clean_up_test_folder(id)?;
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_failed_install_existing_install() -> anyhow::Result<()> {
        trace!("Running test_failed_install_existing_install.");
        let id = "test_failed_install_existing_install";
        let current_dir = current_dir()?;
        let install_dir = RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();
        let test_install_cmd = Install {
            directory: install_dir,
            id: Some(id.to_string()),
            value_files: vec![values_str],
        };
        // Call exec once, so that the folder is created
        test_install_cmd.exec()?;
        // Call it again, this time it should fail
        let err = test_install_cmd.exec().unwrap_err();
        let actual_err = err.to_string();
        let expected_err = "An application with the id 'test_failed_install_existing_install' already exists. Did you mean to `composer upgrade test_failed_install_existing_install` instead?".to_string();
        clean_up_test_folder(id)?;
        // Then assert, if the test fails the folder is still cleaned up
        assert_eq!(expected_err, actual_err);
        Ok(())
    }

    fn clean_up_test_folder(id: &str) -> anyhow::Result<()> {
        // Clean up folder for test
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(id);
        // Remove the composer directory if it exists
        if composer_id_directory.exists() {
            fs::remove_dir_all(composer_id_directory)?;
        }
        // Remove the persisted application from config.json if it exists
        if if_application_exists(id) {
            // This might fail but we tried
            let _ = delete_application_by_id(id);
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn test_install_with_correct_values_from_app_yaml() -> anyhow::Result<()> {
        trace!("Running test_install_with_correct_values_from_app_yaml.");
        let current_dir = current_dir()?;
        let install_dir = RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();
        let id = "test_install_with_correct_values_from_app_yaml";
        let test_install_cmd = Install {
            directory: install_dir.clone(),
            id: Some(id.to_string()),
            value_files: vec![values_str],
        };
        test_install_cmd.exec()?;

        // Read the created app
        let app = get_application_by_id(id).unwrap();
        // Clean up the app before assertions
        clean_up_test_folder(id)?;
        assert_eq!(app.id, id);
        assert_eq!(app.version, "1.0.0");
        assert_eq!(app.state, ApplicationState::RUNNING);
        assert_eq!(app.app_name, "simple-test");
        assert_eq!(app.compose_path, install_dir.to_string_lossy());
        Ok(())
    }

    #[test]
    fn test_verify_file_exists_happy_path() {
        let install = Install {
            directory: PathBuf::from("resources/test/simple/"),
            id: None,
            value_files: vec![],
        };
        let result = install.verify_file_exists("app.yaml");
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_file_exists_not_happy_path() {
        let install = Install {
            directory: PathBuf::from("resources/test/simple/"),
            id: None,
            value_files: vec![],
        };

        let result = install.verify_file_exists("non_existent_file.txt");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Could not find non_existent_file.txt at resources/test/simple/non_existent_file.txt"
        );
    }
}
