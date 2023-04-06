use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::docker_compose::compose_down;
use crate::utils::storage::read_from::{get_all_from_storage, if_application_exists};
use crate::utils::storage::write_to_storage::delete_application_by_id;
use crate::utils::walk::get_files_with_name;
use anyhow::anyhow;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct Delete {
    /// The application ids to delete, space seperated to delete multiple applications at once
    #[clap(index = 1, required_unless_present = "all", conflicts_with("all"))]
    pub ids: Vec<String>,
    /// If the all flag is set all composer applications will be deleted
    #[clap(long)]
    pub all: bool,
}

// Call docker-compose down on all docker-compose.jinja2 files for this application
fn compose_down_by_id(id: &str) -> anyhow::Result<()> {
    // Ensure the .composer directory exists
    let composer_directory = get_composer_directory()?;
    let composer_id_directory: PathBuf = composer_directory.join(id);
    // Find all docker-compose.jinja2 files
    let all_compose_files = get_files_with_name(
        composer_id_directory.to_str().unwrap(),
        "docker-compose.jinja2",
    );
    for compose_file in all_compose_files {
        compose_down(&compose_file, id);
    }
    Ok(())
}

impl Delete {
    pub fn exec(&self) -> anyhow::Result<()> {
        // If the all flag is set, delete all applications
        if self.all {
            for app in get_all_from_storage()? {
                compose_down_by_id(&app.id)?;
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
            compose_down_by_id(&id)?;
            delete_application_by_id(&id)?;
            info!("Deleted application {}", &id);
        }
        Ok(())
    }
}
