use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::PersistedApplication;
use anyhow::Context;

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
