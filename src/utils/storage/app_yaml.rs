use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AppYaml {
    pub(crate) name: String,
    pub(crate) version: String,
}

pub fn load_app_yaml<P: AsRef<Path>>(path: P) -> Result<AppYaml> {
    let mut file = File::open(&path).context("Failed to open YAML file")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .context("Failed to read YAML file contents")?;

    let yaml_data: AppYaml =
        serde_yaml::from_str(&contents).context("Failed to deserialize YAML data")?;

    Ok(yaml_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_yaml_file_success() {
        let yaml_data = r#"
name: "simple-test"
version: "1.0.0"
unknown_field: "unknown"
"#;

        let mut temp_file = NamedTempFile::new().expect("Failed to create temporary file");
        temp_file
            .write_all(yaml_data.as_bytes())
            .expect("Failed to write to temporary file");
        let temp_path = temp_file.path();

        let result = load_app_yaml(temp_path);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.name, "simple-test");
        assert_eq!(data.version, "1.0.0");
    }

    #[test]
    fn test_load_yaml_file_nonexistent_file() {
        let result = load_app_yaml("nonexistent.yaml");
        assert!(result.is_err());
    }
}
