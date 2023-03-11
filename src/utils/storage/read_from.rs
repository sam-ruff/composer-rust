use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::PersistedApplication;
use anyhow::Context;
use std::fs::OpenOptions;
use std::io::BufReader;
use std::path::PathBuf;

pub fn get_all_from_storage() -> anyhow::Result<Vec<PersistedApplication>> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");

    let file = match OpenOptions::new()
        .read(true)
        .open(&composer_json_config_dir)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(e)
                .with_context(|| format!("Could not open file '{:?}'", &composer_json_config_dir))
        }
    };

    let reader = BufReader::new(file);
    let applications: Vec<PersistedApplication> =
        serde_json::from_reader(reader).with_context(|| "Could not parse JSON in config.json")?;

    Ok(applications)
}
