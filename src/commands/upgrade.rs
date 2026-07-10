use crate::commands::install::{add_application, verify_required_files};
use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::docker_compose::{compose_down_with, CommandRunner, RealCommandRunner};
use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::storage::read_from::get_application_by_id;
use crate::utils::walk::get_files_with_names;
use anyhow::anyhow;
use clap::Args;
use std::collections::HashSet;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};

const COMPOSE_FILE_NAMES: [&str; 2] = ["docker-compose.jinja2", "docker-compose.j2"];

/// Upgrades an existing application by re-rendering its templates and running
/// `docker compose up` again. By default only the deltas are applied and
/// unchanged containers keep running. `--always_down` forces a full
/// `docker compose down` of every compose file before the application is
/// brought back up.
#[derive(Debug, Args)]
pub struct Upgrade {
    #[clap(index = 1)]
    pub directory: PathBuf,
    #[clap(short, long)]
    pub id: Option<String>,
    #[clap(short, long)]
    pub value_files: Vec<String>,
    /// Force a full `docker compose down` of every compose file before
    /// re-rendering and bringing the application back up. Without it the
    /// upgrade converges only the deltas and leaves unchanged containers
    /// running.
    #[clap(long = "always_down", alias = "always-down")]
    pub always_down: bool,
}

/// Selects the compose files that need a `docker compose down` before the
/// re-render. With `always_down` that is every existing compose file.
/// Otherwise it is only the files with no counterpart in the new template
/// directory: their services would never be converged away by the subsequent
/// `docker compose up`, so they must be stopped now while the rendered file
/// still exists.
fn compose_files_to_teardown(
    always_down: bool,
    existing_files: &[String],
    existing_root: &Path,
    new_files: &[String],
    new_root: &Path,
) -> Vec<String> {
    if always_down {
        return existing_files.to_vec();
    }
    let new_relative: HashSet<PathBuf> = new_files
        .iter()
        .filter_map(|file| strip_root(file, new_root))
        .collect();
    existing_files
        .iter()
        .filter(|file| {
            strip_root(file, existing_root)
                .map(|relative| !new_relative.contains(&relative))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn strip_root(file: &str, root: &Path) -> Option<PathBuf> {
    Path::new(file)
        .strip_prefix(root)
        .ok()
        .map(Path::to_path_buf)
}

fn teardown_compose_files(runner: &impl CommandRunner, compose_files: &[String], install_id: &str) {
    for compose_file in compose_files {
        compose_down_with(runner, compose_file, install_id);
    }
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

        // Validate the new template and values before anything destructive so
        // a doomed upgrade leaves the previous install intact and retryable.
        if !self.directory.exists() {
            return Err(anyhow!(format!(
                "Template directory {} does not exist.",
                self.directory.display()
            )));
        }
        verify_required_files(&self.directory)?;
        load_yaml_files(&get_value_files_as_refs(&value_files))?;

        // Stop containers/networks before removing the directory. By default
        // only compose files absent from the new template version are downed;
        // everything else is converged by `docker compose up --remove-orphans`.
        // With --always_down every compose file is downed.
        let compose_files =
            get_files_with_names(composer_id_directory.to_str().unwrap(), &COMPOSE_FILE_NAMES);
        let new_compose_files =
            get_files_with_names(&self.directory.to_string_lossy(), &COMPOSE_FILE_NAMES);
        let teardown_files = compose_files_to_teardown(
            self.always_down,
            &compose_files,
            &composer_id_directory,
            &new_compose_files,
            &self.directory,
        );
        teardown_compose_files(&RealCommandRunner, &teardown_files, install_id);

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
    use crate::utils::docker_compose::MockCommandRunner;
    use crate::utils::storage::models::{ApplicationState, PersistedApplication};
    use crate::utils::storage::read_from::get_application_by_id;
    use crate::utils::storage::write_to_storage::append_to_storage;
    use crate::utils::test_utils::clean_up_test_folder;
    use relative_path::RelativePath;
    use serial_test::serial;
    use std::env::current_dir;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn compose_file_fixture() -> anyhow::Result<tempfile::NamedTempFile> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(b"services:\n  web:\n    image: busybox\n")?;
        Ok(file)
    }

    #[test]
    fn test_teardown_selects_nothing_when_files_unchanged() {
        let existing = vec![
            "/old/docker-compose.jinja2".to_string(),
            "/old/sub/docker-compose.j2".to_string(),
        ];
        let new = vec![
            "/new/docker-compose.jinja2".to_string(),
            "/new/sub/docker-compose.j2".to_string(),
        ];
        let selected = compose_files_to_teardown(
            false,
            &existing,
            Path::new("/old"),
            &new,
            Path::new("/new"),
        );
        assert!(selected.is_empty());
    }

    #[test]
    fn test_teardown_selects_removed_files_by_default() {
        let existing = vec![
            "/old/docker-compose.jinja2".to_string(),
            "/old/removed/docker-compose.j2".to_string(),
        ];
        let new = vec!["/new/docker-compose.jinja2".to_string()];
        let selected = compose_files_to_teardown(
            false,
            &existing,
            Path::new("/old"),
            &new,
            Path::new("/new"),
        );
        assert_eq!(selected, vec!["/old/removed/docker-compose.j2".to_string()]);
    }

    #[test]
    fn test_teardown_selects_everything_with_always_down() {
        let existing = vec![
            "/old/docker-compose.jinja2".to_string(),
            "/old/sub/docker-compose.j2".to_string(),
        ];
        let new = vec![
            "/new/docker-compose.jinja2".to_string(),
            "/new/sub/docker-compose.j2".to_string(),
        ];
        let selected = compose_files_to_teardown(
            true,
            &existing,
            Path::new("/old"),
            &new,
            Path::new("/new"),
        );
        assert_eq!(selected, existing);
    }

    #[test]
    fn test_teardown_runs_once_per_file() -> anyhow::Result<()> {
        let file_one = compose_file_fixture()?;
        let file_two = compose_file_fixture()?;
        let paths = vec![
            file_one.path().to_string_lossy().into_owned(),
            file_two.path().to_string_lossy().into_owned(),
        ];
        let mut runner = MockCommandRunner::new();
        runner
            .expect_run_unbuffered()
            .withf(|args| args.iter().any(|a| a == "down"))
            .times(2)
            .returning(|_| 0);
        teardown_compose_files(&runner, &paths, "test_app");
        Ok(())
    }

    #[test]
    fn test_teardown_with_no_files_never_runs() {
        // No expectations set: any call to the runner fails the test
        let runner = MockCommandRunner::new();
        teardown_compose_files(&runner, &[], "test_app");
    }

    #[test]
    #[serial]
    fn test_upgrade_without_id() -> anyhow::Result<()> {
        // Test that trying to upgrade without an ID results in an error
        trace!("Running test_upgrade_without_id.");
        let upgrade_cmd = Upgrade {
            directory: PathBuf::from("some/directory"),
            id: None,
            value_files: vec![],
            always_down: false,
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
            always_down: false,
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
            state: ApplicationState::Running,
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
            always_down: false,
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
            state: ApplicationState::Running,
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
            always_down: false,
        };

        upgrade_cmd.exec()?;

        // Retrieve the application and check that its value_files have been updated
        let app = get_application_by_id(id)?;
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert_eq!(app.value_files, vec![new_values_str]);
        assert_eq!(app.state, ApplicationState::Running);
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
            state: ApplicationState::Running,
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
            always_down: false,
        };

        upgrade_cmd.exec()?;

        // Retrieve the application and check that its value_files have not changed
        let app = get_application_by_id(id)?;
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert_eq!(app.value_files, vec![values_str]);
        assert_eq!(app.state, ApplicationState::Running);
        Ok(())
    }

    /// Creates the rendered app directory with a marker file and persists the
    /// application, simulating a previous successful install.
    fn persist_app_with_marker(
        id: &str,
        install_dir: &Path,
        stored_value_files: Vec<String>,
    ) -> anyhow::Result<(PathBuf, PathBuf)> {
        let composer_directory = get_composer_directory()?;
        let composer_id_directory = composer_directory.join(id);
        fs::create_dir_all(&composer_id_directory)?;
        let marker_path = composer_id_directory.join("previous-render.txt");
        fs::write(&marker_path, "rendered by the previous install")?;
        let app = PersistedApplication {
            id: id.to_string(),
            version: "1.0.0".to_string(),
            timestamp: 0,
            state: ApplicationState::Running,
            app_name: "Test App".to_string(),
            compose_path: install_dir.to_string_lossy().to_string(),
            value_files: stored_value_files,
        };
        append_to_storage(&app)?;
        Ok((composer_id_directory, marker_path))
    }

    #[test]
    #[serial]
    fn test_46_failed_upgrade_invalid_values_keeps_previous_install() -> anyhow::Result<()> {
        // A values file that cannot be read must fail the upgrade before the
        // previous rendered directory is removed
        trace!("Running test_46_failed_upgrade_invalid_values_keeps_previous_install.");
        let id = "test_46_invalid_values";
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();

        let (composer_id_directory, marker_path) =
            persist_app_with_marker(id, &install_dir, vec![values_str])?;

        let upgrade_cmd = Upgrade {
            directory: install_dir,
            id: Some(id.to_string()),
            value_files: vec!["/nonexistent/values.yaml".to_string()],
            always_down: false,
        };

        let result = upgrade_cmd.exec();
        let directory_kept = composer_id_directory.exists();
        let marker_kept = marker_path.exists();
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert!(result.is_err(), "Upgrade with an unreadable values file should fail");
        assert!(directory_kept, "Previous install directory should survive a failed upgrade");
        assert!(marker_kept, "Previously rendered files should survive a failed upgrade");
        Ok(())
    }

    #[test]
    #[serial]
    fn test_46_failed_upgrade_missing_template_dir_keeps_previous_install() -> anyhow::Result<()> {
        // A template directory that does not exist must fail the upgrade
        // before the previous rendered directory is removed
        trace!("Running test_46_failed_upgrade_missing_template_dir_keeps_previous_install.");
        let id = "test_46_missing_template_dir";
        let current_dir = current_dir()?;
        let install_dir =
            RelativePath::new("resources/test/simple/").to_logical_path(&current_dir);
        let values_dir = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let values_str = values_dir.to_string_lossy().to_string();

        let (composer_id_directory, marker_path) =
            persist_app_with_marker(id, &install_dir, vec![values_str])?;

        let upgrade_cmd = Upgrade {
            directory: PathBuf::from("does_not_exist"),
            id: Some(id.to_string()),
            value_files: vec![],
            always_down: false,
        };

        let result = upgrade_cmd.exec();
        let directory_kept = composer_id_directory.exists();
        let marker_kept = marker_path.exists();
        // Clean up before assertions in case they fail
        clean_up_test_folder(id)?;

        assert!(result.is_err(), "Upgrade with a missing template directory should fail");
        assert!(directory_kept, "Previous install directory should survive a failed upgrade");
        assert!(marker_kept, "Previously rendered files should survive a failed upgrade");
        Ok(())
    }
}