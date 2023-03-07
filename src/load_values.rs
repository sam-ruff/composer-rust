use log::trace;

use serde_yaml::{Mapping, Value};
use std::io::Read;

use std::{collections::HashMap, fs::File};

// This function loads a list of yaml files and returns a serde_yaml::Value object, that
// object is a flatten list of the values in the yaml file with the correct values being
// overwritten based on loading order. See the tests for an example where the world: "string"
// value is overriden by "notString"
fn load_yaml_files(yaml_files: &Vec<&str>) -> Result<Value, Box<dyn std::error::Error>> {
    // Create an empty HashMap to store the YAML values
    let mut yaml_values = HashMap::new();

    // Iterate over each YAML file path
    for yaml_file in yaml_files {
        // Open the YAML file and
        // Deserialize the YAML file into a serde_yaml::Value object
        let yaml: Value = read_yaml_file(yaml_file)?;

        // Merge the YAML values with the existing values
        if let Value::Mapping(map) = yaml {
            for (key, value) in map {
                if let Some(existing_value) = yaml_values.get_mut(&key) {
                    // If the existing value is a Mapping, merge the two mappings
                    if let Value::Mapping(existing_map) = existing_value {
                        if let Value::Mapping(new_map) = value {
                            for (new_key, new_value) in new_map {
                                existing_map.insert(new_key.clone(), new_value.clone());
                            }
                            continue;
                        }
                    }
                }
                // Otherwise, insert the new value
                yaml_values.insert(key.clone(), value.clone());
            }
        }
    }

    // Create a new Mapping object from the HashMap entries
    let mut mapping = Mapping::new();
    for (key, value) in yaml_values.iter() {
        mapping.insert(key.clone(), value.clone());
    }

    // Convert the Mapping to a serde_yaml::Value object and return it
    Ok(Value::Mapping(mapping))
}

pub fn read_yaml_file(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    trace!("Loading file: {}", path);
    // Open the file for reading
    let mut file = File::open(path)?;

    // Read the file contents into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    // Parse the YAML into a serde_yaml::Value
    let yaml: Value = serde_yaml::from_str(&contents)?;
    // Return the yaml file
    Ok(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_utils;
    use log::{trace, LevelFilter};
    use relative_path::RelativePath;
    use serde::{Deserialize, Serialize};
    use std::env::current_dir;

    #[derive(Debug, Serialize, Deserialize)]
    struct ExpectedFullValues {
        hello: bool,
        world: String,
        foo: HashMap<Value, Value>,
    }

    #[test]
    fn test_copy_files_simple() -> Result<(), Box<dyn std::error::Error>> {
        log_utils::setup_logging(LevelFilter::Trace, true);
        println!();
        trace!("Running simple load values test.");
        let current_dir = current_dir()?;
        let values_path = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let override_path = RelativePath::new("resources/test/test_values/override.yaml")
            .to_logical_path(&current_dir);
        let files = vec![
            values_path.to_str().unwrap(),
            override_path.to_str().unwrap(),
        ];
        let output = load_yaml_files(&files)?;
        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedFullValues = serde_yaml::from_str(
            r#"---
        hello: True
        world: "notString"
        foo:
          bar: "hi"
          nested:
            map: "here""#,
        )?;
        // Convert the expected YAML contents into a `serde_yaml::Value` object
        let expected_value = serde_yaml::to_value(expected_yaml)?;
        // Test that the loaded YAML contents match the expected YAML contents
        assert_eq!(expected_value, output);
        Ok(())
    }

    #[test]
    fn test_copy_files_complex() -> Result<(), Box<dyn std::error::Error>> {
        log_utils::setup_logging(LevelFilter::Trace, true);
        println!();
        trace!("Running simple load values test.");
        let current_dir = current_dir()?;
        let values_path = RelativePath::new("resources/test/test_values/values.yaml")
            .to_logical_path(&current_dir);
        let override_path = RelativePath::new("resources/test/test_values/override.yaml")
            .to_logical_path(&current_dir);
        let override_complex_path =
            RelativePath::new("resources/test/test_values/override_complex.yaml")
                .to_logical_path(&current_dir);
        let files = vec![
            values_path.to_str().unwrap(),
            override_path.to_str().unwrap(),
            override_complex_path.to_str().unwrap(),
        ];
        let output = load_yaml_files(&files)?;
        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedFullValues = serde_yaml::from_str(
            r#"---
        hello: True
        world: "overwritten"
        foo:
          bar: "hi2"
          nested:
            map: "here""#,
        )?;
        // Convert the expected YAML contents into a `serde_yaml::Value` object
        let expected_value = serde_yaml::to_value(expected_yaml)?;
        // Test that the loaded YAML contents match the expected YAML contents
        assert_eq!(expected_value, output);
        Ok(())
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ExpectedYamlSimple {
        world: String,
        foo: HashMap<String, String>,
    }

    #[test]
    fn test_read_yaml_file() -> Result<(), Box<dyn std::error::Error>> {
        // Set up logging
        log_utils::setup_logging(LevelFilter::Trace, true);

        // Print a log message to show which test is running
        println!();
        trace!("Running read_yaml_file test.");

        // Get the current directory and the path to the test YAML file
        let current_dir = current_dir()?;
        let yaml_path = RelativePath::new("resources/test/test_values/override_complex.yaml")
            .to_logical_path(&current_dir);

        // Read the YAML file into a `serde_yaml::Value` object
        let loaded_yaml = read_yaml_file(yaml_path.to_str().unwrap())?;

        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedYamlSimple = serde_yaml::from_str(
            r#"---
        world: "overwritten"
        foo:
          bar: "hi2""#,
        )?;

        // Convert the expected YAML contents into a `serde_yaml::Value` object
        let expected_value = serde_yaml::to_value(expected_yaml)?;

        // Test that the loaded YAML contents match the expected YAML contents
        assert_eq!(expected_value, loaded_yaml);

        Ok(())
    }

    #[test]
    fn test_read_invalid_yaml_file() -> Result<(), Box<dyn std::error::Error>> {
        // Test that `read_yaml_file()` returns an error when given an invalid path
        assert_matches!(read_yaml_file("invalid/path.yaml"), Err(_));
        Ok(())
    }
}
