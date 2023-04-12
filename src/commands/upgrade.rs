use crate::commands::install::add_application;
use crate::utils::copy_file_utils::get_composer_directory;
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
        let install_id: &String = &self.id.as_ref().expect("Could not get ID to upgrade.");
        // Ensure the .composer directory exists
        let composer_directory = get_composer_directory()?;
        let composer_id_directory: PathBuf = composer_directory.join(install_id);
        trace!("Creating directory: '{}'", composer_id_directory.display());
        if !composer_id_directory.exists() {
            return Err(anyhow!(format!("An application with the id '{}' does not exist. Did you mean to `composer install {}` instead?", install_id, install_id)));
        }
        // First remove the existing directory
        remove_dir_all(&composer_id_directory)?;
        info!("Upgrading application with ID: {}", install_id);

        add_application(
            install_id,
            &composer_id_directory,
            true,
            &self.value_files,
            &self.directory,
        )?;

        Ok(())
    }
}
