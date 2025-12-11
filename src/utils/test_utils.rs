use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::docker_compose::compose_down;
use crate::utils::walk::get_files_with_names;
use anyhow::anyhow;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use crate::utils::storage::read_from::if_application_exists;
use crate::utils::storage::write_to_storage::delete_application_by_id;

#[allow(dead_code)]
pub fn move_file_if_exists(
    source_path: &PathBuf,
    destination_path: &PathBuf,
) -> anyhow::Result<()> {
    if let Ok(metadata) = fs::metadata(source_path) {
        if metadata.is_file() {
            fs::rename(source_path, destination_path)?;
            trace!("Moved file {:?} to {:?}", source_path, destination_path);
        } else {
            return Err(anyhow!("Source path {:?} is not a file", source_path));
        }
    } else {
        return Err(anyhow!("Source path {:?} does not exist", source_path));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn create_file_with_contents(path: &PathBuf, contents: &str) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

#[allow(dead_code)]
pub fn backup_composer_config() -> anyhow::Result<(PathBuf, PathBuf)> {
    let composer_directory = get_composer_directory()?;
    let composer_json_config: PathBuf = composer_directory.join("config.json");
    if !composer_json_config.exists() {
        // If composer config directory does not exist create it
        create_file_with_contents(&composer_json_config, "[]")?;
    }
    let composer_json_config_backup: PathBuf = composer_directory.join("backup-config.json");
    move_file_if_exists(&composer_json_config, &composer_json_config_backup)?;
    Ok((composer_json_config, composer_json_config_backup))
}

#[allow(dead_code)]
pub fn clean_up_test_folder(id: &str) -> anyhow::Result<()> {
    let composer_directory = get_composer_directory()?;
    let composer_id_directory: PathBuf = composer_directory.join(id);

    // Stop Docker containers/networks BEFORE removing files
    if composer_id_directory.exists() {
        let compose_files = get_files_with_names(
            composer_id_directory.to_str().unwrap(),
            &["docker-compose.jinja2", "docker-compose.j2"],
        );
        for compose_file in compose_files {
            compose_down(&compose_file, id);
        }
        fs::remove_dir_all(composer_id_directory)?;
    }

    // Remove the persisted application from config.json if it exists
    if if_application_exists(id) {
        let _ = delete_application_by_id(id);
    }
    Ok(())
}
