use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::PersistedApplication;
use anyhow::anyhow;
use anyhow::Context;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
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

pub fn get_application_by_id(id: &str) -> anyhow::Result<PersistedApplication> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");
    let mut file = File::open(composer_json_config_dir)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let applications: Vec<PersistedApplication> = serde_json::from_str(&contents)?;
    for application in applications {
        if application.id == id {
            return Ok(application);
        }
    }
    Err(anyhow!("Application with id {} not found", id))
}

pub fn if_application_exists(id: &str) -> bool {
    match get_application_by_id(id) {
        Ok(_) => true,
        Err(_) => false,
    }
}
