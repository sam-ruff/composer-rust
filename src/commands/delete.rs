use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::docker_compose::{
    compose_down_with, CommandRunner, RealCommandRunner, COMPOSE_FILE_NAMES,
};
use crate::utils::storage::read_from::{get_all_from_storage, if_application_exists};
use crate::utils::storage::write_to_storage::delete_application_by_id;
use crate::utils::walk::get_files_with_names;
use anyhow::anyhow;
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub struct Delete {
    /// The application ids to delete, space seperated to delete multiple applications at once
    #[clap(index = 1, required_unless_present = "all", conflicts_with("all"))]
    pub ids: Vec<String>,
    /// If the all flag is set all composer applications will be deleted
    #[clap(long)]
    pub all: bool,
}

// Call docker compose down on all compose files for this application
fn compose_down_by_id(runner: &impl CommandRunner, id: &str) -> anyhow::Result<()> {
    // Ensure the .composer directory exists
    let composer_directory = get_composer_directory()?;
    let composer_id_directory: PathBuf = composer_directory.join(id);
    compose_down_in_directory(runner, &composer_id_directory, id);
    Ok(())
}

// Run docker compose down on every compose file found under the directory
fn compose_down_in_directory(runner: &impl CommandRunner, directory: &Path, id: &str) {
    let all_compose_files = get_files_with_names(&directory.to_string_lossy(), &COMPOSE_FILE_NAMES);
    for compose_file in all_compose_files {
        compose_down_with(runner, &compose_file, id);
    }
}

impl Delete {
    pub fn exec(&self) -> anyhow::Result<()> {
        // If the all flag is set, delete all applications
        if self.all {
            for app in get_all_from_storage()? {
                compose_down_by_id(&RealCommandRunner, &app.id)?;
                delete_application_by_id(&app.id)?;
                info!("Deleted application {}", app.id);
            }
            return Ok(());
        }
        // Otherwise only delete the applications that have been asked
        for id in self.ids.clone() {
            if !if_application_exists(&id) {
                return Err(anyhow!("Could not find application '{}' to delete it.", id));
            }
            compose_down_by_id(&RealCommandRunner, &id)?;
            delete_application_by_id(&id)?;
            info!("Deleted application {}", id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::docker_compose::MockCommandRunner;
    use std::fs;
    use tempfile::TempDir;

    const COMPOSE_WITH_SERVICES: &str = "services:\n  web:\n    image: busybox\n";

    // Builds <temp>/<id>/<file> for each file name, mirroring the layout of a
    // rendered application under the composer directory.
    fn app_dir_with_compose_files(
        id: &str,
        file_names: &[&str],
    ) -> anyhow::Result<(TempDir, PathBuf)> {
        let composer_dir = TempDir::new()?;
        let app_dir = composer_dir.path().join(id);
        fs::create_dir_all(&app_dir)?;
        for file_name in file_names {
            fs::write(app_dir.join(file_name), COMPOSE_WITH_SERVICES)?;
        }
        Ok((composer_dir, app_dir))
    }

    fn expect_down_for_file(runner: &mut MockCommandRunner, compose_path: &Path) {
        let expected_path = compose_path.to_string_lossy().into_owned();
        runner
            .expect_run_unbuffered()
            .withf(move |args| {
                args.iter().any(|arg| arg == "down") && args.iter().any(|arg| arg == &expected_path)
            })
            .times(1)
            .returning(|_| 0);
    }

    #[test]
    fn test_delete_downs_j2_compose_file() -> anyhow::Result<()> {
        let id = "test_delete_j2";
        let (_composer_dir, app_dir) = app_dir_with_compose_files(id, &["docker-compose.j2"])?;
        let mut runner = MockCommandRunner::new();
        expect_down_for_file(&mut runner, &app_dir.join("docker-compose.j2"));
        compose_down_in_directory(&runner, &app_dir, id);
        Ok(())
    }

    #[test]
    fn test_delete_downs_jinja2_compose_file() -> anyhow::Result<()> {
        let id = "test_delete_jinja2";
        let (_composer_dir, app_dir) = app_dir_with_compose_files(id, &["docker-compose.jinja2"])?;
        let mut runner = MockCommandRunner::new();
        expect_down_for_file(&mut runner, &app_dir.join("docker-compose.jinja2"));
        compose_down_in_directory(&runner, &app_dir, id);
        Ok(())
    }

    #[test]
    fn test_delete_downs_mixed_compose_files() -> anyhow::Result<()> {
        let id = "test_delete_mixed";
        let (_composer_dir, app_dir) =
            app_dir_with_compose_files(id, &["docker-compose.jinja2", "docker-compose.j2"])?;
        let mut runner = MockCommandRunner::new();
        expect_down_for_file(&mut runner, &app_dir.join("docker-compose.jinja2"));
        expect_down_for_file(&mut runner, &app_dir.join("docker-compose.j2"));
        compose_down_in_directory(&runner, &app_dir, id);
        Ok(())
    }

    #[test]
    fn test_delete_with_no_compose_files_never_runs() -> anyhow::Result<()> {
        let id = "test_delete_empty";
        let (_composer_dir, app_dir) = app_dir_with_compose_files(id, &[])?;
        // No expectations set: any call to the runner fails the test
        let runner = MockCommandRunner::new();
        compose_down_in_directory(&runner, &app_dir, id);
        Ok(())
    }
}
