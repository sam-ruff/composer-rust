use serde_yaml::{Mapping, Value};

use crate::utils::yaml_string_parser::parse_yaml_string;
use anyhow::Context;
use std::{collections::HashMap, fs::File};

/// Loads one or more YAML files or key-value string(s) into a single `serde_yaml::Value` object.
///
/// This function takes a vector of YAML file paths or key-value strings in the format of "x.y.z=foo", and
/// loads each one into a `serde_yaml::Value` object. If a key-value string is provided, it is parsed into
/// a YAML mapping using the `parse_yaml_string` function. If a file path is provided, the file is read and
/// deserialized into a YAML mapping using the `read_yaml_file` function. The resulting mappings are then merged
/// into a single mapping, with any conflicting values being overwritten by the last value encountered.
///
/// # Errors
///
/// This function returns an `anyhow::Error` if any of the input files or strings cannot be loaded or parsed.
///
/// # Examples
///
/// ```
/// use serde_yaml::Value;
/// use anyhow::Result;
///
/// fn main() -> Result<()> {
///     let yaml_files = vec![
///         "examples/values1.yaml",
///         "examples/values2.yaml",
///         "abc.def.ghi=jkl",
///     ];
///
///     let yaml_value = load_yaml_files(&yaml_files)?;
///
///     assert_eq!(yaml_value["foo"]["bar"], Value::String("baz".to_owned()));
///     assert_eq!(yaml_value["abc"]["def"]["ghi"], Value::String("jkl".to_owned()));
///
///     Ok(())
/// }
/// ```
///
/// # Arguments
///
/// * `yaml_files` - A vector of YAML file paths or key-value strings in the format of "x.y.z=foo".
///
/// # Returns
///
/// A `serde_yaml::Value` object representing the merged YAML mappings loaded from the input files or strings.
pub fn load_yaml_files(yaml_files: &Vec<&str>) -> anyhow::Result<Value> {
    // Create an empty HashMap to store the YAML values
    let mut yaml_values = HashMap::new();
    // Iterate over each YAML file path
    let mut yaml: Value;
    for yaml_file in yaml_files {
        // If the yaml_file is not a path but a x.y.z=value format string load it as a value
        if yaml_file.contains("=") {
            yaml = parse_yaml_string(yaml_file)?;
        } else {
            // Open the YAML file and
            // Deserialize the YAML file into a serde_yaml::Value object
            yaml = read_yaml_file(yaml_file)
                .with_context(|| format!("Failed to read values YAML file: {}", yaml_file))?;
        }

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

pub fn get_value_files_as_refs(strings: &Vec<String>) -> Vec<&str> {
    strings.iter().map(|s| s.as_ref()).collect()
}

pub fn read_yaml_file(path: &str) -> anyhow::Result<Value> {
    trace!("Loading file: {}", path);
    let file = File::open(path)?;
    let yaml: Value = serde_yaml::from_reader(file)?;
    Ok(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use relative_path::RelativePath;
    use serde::{Deserialize, Serialize};
    use serde_yaml::from_str;
    use std::env::current_dir;

    #[derive(Debug, Serialize, Deserialize)]
    struct ExpectedFullValues {
        hello: bool,
        world: String,
        foo: HashMap<Value, Value>,
    }

    #[test]
    fn test_copy_files_simple() -> anyhow::Result<()> {
        trace!("Running test_copy_files_simple.");
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
        let expected_yaml: ExpectedFullValues = from_str(
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
    fn test_copy_files_complex() -> anyhow::Result<()> {
        trace!("Running test_copy_files_complex.");
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
        let expected_yaml: ExpectedFullValues = from_str(
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

    #[test]
    fn test_copy_files_complex_manual_override() -> anyhow::Result<()> {
        trace!("Running test_copy_files_complex_manual_override.");
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
            "foo.bar=manual",
        ];
        let output = load_yaml_files(&files)?;
        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedFullValues = from_str(
            r#"---
        hello: True
        world: "overwritten"
        foo:
          bar: "manual"
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
    fn test_copy_files_complex_manual_override_multiple() -> anyhow::Result<()> {
        trace!("Running test_copy_files_complex_manual_override_multiple.");
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
            "foo.bar=manual",
            "world=world",
            "foo.nested.map=wow",
        ];
        let output = load_yaml_files(&files)?;
        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedFullValues = from_str(
            r#"---
        hello: True
        world: "world"
        foo:
          bar: "manual"
          nested:
            map: "wow""#,
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
    fn test_read_yaml_file() -> anyhow::Result<()> {
        trace!("Running test_read_yaml_file.");

        // Get the current directory and the path to the test YAML file
        let current_dir = current_dir()?;
        let yaml_path = RelativePath::new("resources/test/test_values/override_complex.yaml")
            .to_logical_path(&current_dir);

        // Read the YAML file into a `serde_yaml::Value` object
        let loaded_yaml = read_yaml_file(yaml_path.to_str().unwrap())?;

        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedYamlSimple = from_str(
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
    fn test_read_invalid_yaml_file() -> anyhow::Result<()> {
        // Test that `read_yaml_file()` returns an error when given an invalid path
        assert_matches!(read_yaml_file("invalid/path.yaml"), Err(_));
        Ok(())
    }
}
