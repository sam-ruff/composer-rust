use crate::utils::storage::read_from::{get_all_from_storage, if_application_exists};
use crate::utils::storage::write_to_storage::delete_application_by_id;
use anyhow::anyhow;
use clap::Args;

#[derive(Debug, Args)]
pub struct Delete {
    /// The application ids to delete, space seperated to delete multiple applications at once
    #[clap(index = 1, required_unless_present = "all", conflicts_with("all"))]
    pub ids: Vec<String>,
    /// If the all flag is set all composer applications will be deleted
    #[clap(long)]
    pub all: bool,
}

impl Delete {
    pub fn exec(&self) -> anyhow::Result<()> {
        // If the all flag is set, delete all applications
        if self.all {
            for app in get_all_from_storage()? {
                delete_application_by_id(&app.id)?;
            }
            return Ok(());
        }
        // Otherwise only delete the applications that have been asked
        for id in self.ids.clone() {
            if !if_application_exists(&id) {
                return Err(anyhow!("Could not find application '{}' to delete it.", id));
            }
            delete_application_by_id(&id)?;
        }
        Ok(())
    }
}
