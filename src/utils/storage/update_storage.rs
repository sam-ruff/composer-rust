use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::storage::models::{ApplicationState, PersistedApplication};
use crate::utils::storage::read_from::get_all_from_storage;
use anyhow::Context;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub fn update_application_state(id: &str, new_state: ApplicationState) -> anyhow::Result<()> {
    update_persisted_application_by_id(id, |mut application| {
        application.state = new_state.clone();
        application
    })
}

pub fn update_persisted_application_by_id<F>(
    id: &str,
    mut modify_application: F,
) -> anyhow::Result<()>
where
    F: FnMut(PersistedApplication) -> PersistedApplication,
{
    let applications = get_all_from_storage()?;
    let new_applications: Vec<PersistedApplication> = applications
        .into_iter()
        .filter_map(|application| {
            if application.id == id {
                Some(modify_application(application))
            } else {
                Some(application)
            }
        })
        .collect();
    let composer_directory = get_composer_directory()?;
    let composer_json_config_dir: PathBuf = composer_directory.join("config.json");
    let file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&composer_json_config_dir)
        .with_context(|| format!("Could not open file '{:?}'", &composer_json_config_dir))?;
    let mut writer = BufWriter::new(file);
    let json_data = serde_json::to_vec(&new_applications)
        .with_context(|| "Could not serialize JSON to config.json")?;
    writer
        .write_all(&json_data)
        .with_context(|| "Could not write JSON to config.json")?;
    Ok(())
}
