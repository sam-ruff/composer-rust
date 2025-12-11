use crate::app;
use crate::utils::copy_file_utils::{copy_files_with_ignorefile, get_composer_directory};
use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::walk::{get_files_with_extensions, get_files_with_names};
use anyhow::anyhow;

use crate::utils::docker_compose::{compose_pull, compose_up};
use crate::utils::storage::app_yaml::load_app_yaml;
use crate::utils::storage::models::{ApplicationState, PersistedApplication};
use crate::utils::storage::write_to_storage::append_to_storage;
use crate::utils::template::render_template;
use clap::Args;

use std::fs;
use std::fs::{remove_file, write};
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
        // Ensure the .composer directory exists
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(install_id);
        trace!("Creating directory: '{}'", composer_id_directory.display());
        if composer_id_directory.exists() {
            return Err(anyhow!(format!("An application with the id '{}' already exists. Did you mean to `composer upgrade {}` instead?", install_id, install_id)));
        }
        info!("Installing application with ID: {}", install_id);

        add_application(
            install_id,
            &composer_id_directory,
            false,
            &self.value_files,
            &self.directory,
        )?;

        Ok(())
    }

    fn get_readable_id() -> String {
        petname::petname(3, "-").to_string()
    }
}

fn get_current_timestamp() -> i64 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

    duration_since_epoch.as_secs() as i64
}

fn verify_required_files(directory: &PathBuf) -> anyhow::Result<()> {
    verify_file_exists("app.yaml", directory)?;
    verify_either_file_exists(&["docker-compose.jinja2", "docker-compose.j2"], directory)?;
    Ok(())
}

fn verify_file_exists(file_name: &str, directory: &PathBuf) -> anyhow::Result<()> {
    let file_path = directory.join(file_name);
    if !file_path.exists() {
        return Err(anyhow!(format!(
            "Could not find {} at {}",
            file_name,
            file_path.display()
        )));
    }
    Ok(())
}

fn verify_either_file_exists(file_names: &[&str], directory: &PathBuf) -> anyhow::Result<()> {
    for file_name in file_names {
        let file_path = directory.join(file_name);
        if file_path.exists() {
            return Ok(());
        }
    }
    Err(anyhow!(format!(
        "Could not find any of [{}] in {}",
        file_names.join(", "),
        directory.display()
    )))
}

pub fn add_application(
    install_id: &String,
    composer_id_directory: &PathBuf,
    is_upgrade: bool,
    values_files: &Vec<String>,
    directory: &PathBuf,
) -> anyhow::Result<()> {
    if values_files.is_empty() {
        let mut correct_word = "install";
        if is_upgrade {
            correct_word = "upgrade";
        }
        return Err(anyhow!(
                "You cannot {} an application with no values file. Use -v <values path> to specify values file.", correct_word
            ));
    }

    let values = get_value_files_as_refs(values_files);
    let consolidated_values = load_yaml_files(&values)?;
    trace!(
        "Consolidated values: \n```\n{}\n```\n",
        serde_yaml::to_string(&consolidated_values).unwrap()
    );

    if !directory.exists() {
        return Err(anyhow!(format!(
            "Template directory {} does not exist.",
            &directory.display()
        )));
    }
    // Check for app.yaml and docker-compose.jinja2
    verify_required_files(directory)?;
    // Check if there is an ignore file
    let mut ignore_file_optional: Option<&Path> = None;
    let composer_ignore_path = directory.join(".composerignore");
    if composer_id_directory.exists() {
        ignore_file_optional = Some(composer_ignore_path.as_path());
    }
    // Create the directory to copy the files to
    fs::create_dir_all(&composer_id_directory)?;

    // Copy the files to the .composer directory  using the ID as the folder name
    copy_files_with_ignorefile(directory, &composer_id_directory, ignore_file_optional)?;

    // Read App.yaml to get some of the needed values
    let app_yaml_path = directory.join("app.yaml");
    let app_yaml = load_app_yaml(app_yaml_path)?;
    // Create the persisted application struct
    let mut application = PersistedApplication {
        id: install_id.to_string(),
        version: app_yaml.version,
        timestamp: get_current_timestamp(),
        state: ApplicationState::STARTING,
        app_name: app_yaml.name,
        compose_path: fs::canonicalize(&directory)
            .unwrap_or_else(|_| directory.clone())
            .to_string_lossy()
            .to_string(),
        value_files: values_files.clone(),
    };
    // Change status of app to starting
    append_to_storage(&application)?;
    // For each template render them, then replace them with the actual file
    // Replace the jinja files with templated ones
    let files_to_replace =
        get_files_with_extensions(composer_id_directory.to_str().unwrap(), &["jinja2", "j2"]);
    trace!("Detected templates: {}", files_to_replace.join(","));

    for file_path in files_to_replace {
        trace!("Replacing {}", file_path);
        // Get the rendered template
        let rendered_content = render_template(&file_path, consolidated_values.clone())?;
        // Replace the existing file
        remove_file(&file_path)?;
        write(file_path, rendered_content.as_bytes())?;
    }

    let no_run = app::no_run();

    // Find all docker-compose template files
    let all_compose_files = get_files_with_names(
        composer_id_directory.to_str().unwrap(),
        &["docker-compose.jinja2", "docker-compose.j2"],
    );
    for compose_file in all_compose_files {
        if *app::always_pull() {
            info!("Always pull is enabled. Pulling latest docker images.");
            let compose_path = composer_id_directory.join(&compose_file);
            let dir_str = compose_path.to_str().unwrap();
            compose_pull(dir_str);
        }
        if !no_run {
            compose_up(&compose_file, install_id)?;
        }
    }

    // Change status of app to running
    if !no_run {
        application.state = ApplicationState::RUNNING;
    }
    append_to_storage(&application)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use relative_path::RelativePath;

    use crate::commands::install::{verify_file_exists, Install};

    use crate::utils::storage::models::ApplicationState;
    use crate::utils::storage::read_from::get_application_by_id;
    use serial_test::serial;
    use std::env::current_dir;
    use std::path::PathBuf;
    use crate::utils::test_utils::clean_up_test_folder;

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
    #[serial]
    fn test_verify_file_exists_happy_path() {
        let install = Install {
            directory: PathBuf::from("resources/test/simple/"),
            id: None,
            value_files: vec![],
        };
        let result = verify_file_exists("app.yaml", &install.directory);
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_verify_file_exists_not_happy_path() {
        let install = Install {
            directory: PathBuf::from("resources/test/simple/"),
            id: None,
            value_files: vec![],
        };

        let result = verify_file_exists("non_existent_file.txt", &install.directory);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Could not find non_existent_file.txt at resources/test/simple/non_existent_file.txt"
        );
    }

    #[test]
    #[serial]
    fn test_install_stores_value_files() -> anyhow::Result<()> {
        let current_dir = current_dir()?;
        let install_dir = RelativePath::new("resources/test/simple/")
            .to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();
        let id = "test_install_stores_value_files";
        let value_files = vec![values_str.clone()];
        let test_install_cmd = Install {
            directory: install_dir.clone(),
            id: Some(id.to_string()),
            value_files: value_files.clone(),
        };
        test_install_cmd.exec()?;
        // Read the created app
        let app = get_application_by_id(id)?;
        // Clean up the app after test
        clean_up_test_folder(id)?;
        assert_eq!(app.value_files, value_files);
        Ok(())
    }
}
