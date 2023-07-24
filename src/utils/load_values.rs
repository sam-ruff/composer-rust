use serde_yaml::{Mapping, Value};

use crate::utils::yaml_string_parser::parse_yaml_string;
use anyhow::Context;
use serde_yaml::mapping::Entry;
use std::fs::File;

fn merge_maps(existing_map: &mut Mapping, new_map: Mapping) {
    for (new_key, new_value) in new_map {
        let new_value_clone = new_value.clone();
        match existing_map.entry(new_key) {
            Entry::Occupied(mut entry) => match (entry.get_mut(), &new_value) {
                (Value::Mapping(existing_inner), Value::Mapping(new_inner)) => {
                    merge_maps(existing_inner, new_inner.clone());
                }
                (Value::Sequence(existing_list), Value::Sequence(new_list)) => {
                    existing_list.extend(new_list.clone());
                }
                _ => {
                    entry.insert(new_value_clone);
                }
            },
            Entry::Vacant(entry) => {
                entry.insert(new_value);
            }
        }
    }
}

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
    let mut yaml_values = Mapping::new();

    for yaml_file in yaml_files {
        let yaml = if yaml_file.contains("=") {
            parse_yaml_string(yaml_file)?
        } else {
            read_yaml_file(yaml_file)
                .with_context(|| format!("Failed to read values YAML file: {}", yaml_file))?
        };

        // Start merging here, whether it's a map or not
        match &yaml {
            Value::Mapping(map) => {
                for (key, value) in map {
                    match yaml_values.entry(key.clone()) {
                        Entry::Occupied(mut entry) => match (entry.get_mut(), value) {
                            (Value::Mapping(existing_inner), Value::Mapping(new_inner)) => {
                                merge_maps(existing_inner, new_inner.clone());
                            }
                            (Value::Sequence(existing_list), Value::Sequence(new_list)) => {
                                existing_list.extend(new_list.clone());
                            }
                            _ => {
                                entry.insert(value.clone());
                            }
                        },
                        Entry::Vacant(entry) => {
                            entry.insert(value.clone());
                        }
                    }
                }
            }
            // In case top-level structure is not a map
            _ => {
                return Err(anyhow::anyhow!(
                    "Expected top-level YAML structure to be a mapping."
                ));
            }
        }
    }

    Ok(Value::Mapping(yaml_values))
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
    use std::collections::HashMap;
    use std::env::current_dir;

    #[derive(Debug, Serialize, Deserialize)]
    struct ExpectedFullValues {
        hello: bool,
        world: String,
        foo: HashMap<Value, Value>,
    }

    use serde_yaml::Mapping;

    #[test]
    fn test_merge_maps() {
        // Define existing map
        let mut existing_map: Mapping = Mapping::new();
        existing_map.insert(
            Value::String("key1".to_string()),
            Value::String("value1".to_string()),
        );

        // Define new map
        let mut new_map: Mapping = Mapping::new();
        new_map.insert(
            Value::String("key2".to_string()),
            Value::String("value2".to_string()),
        );

        // Merge maps
        merge_maps(&mut existing_map, new_map);

        // Check merged map
        assert_eq!(
            existing_map.get(&"key1".to_string()).unwrap(),
            &Value::String("value1".to_string())
        );
        assert_eq!(
            existing_map.get(&"key2".to_string()).unwrap(),
            &Value::String("value2".to_string())
        );
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
            new: "value"
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
            new: "value"
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
            new: "value"
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

    #[derive(Debug, Serialize, Deserialize)]
    struct ExpectedYamlOverride {
        world: String,
    }

    #[test]
    fn test_read_yaml_file() -> anyhow::Result<()> {
        trace!("Running test_read_yaml_file.");

        // Get the current directory and the path to the test YAML file
        let current_dir = current_dir()?;
        let yaml_path = RelativePath::new("resources/test/test_values/override.yaml")
            .to_logical_path(&current_dir);

        // Read the YAML file into a `serde_yaml::Value` object
        let loaded_yaml = read_yaml_file(yaml_path.to_str().unwrap())?;

        // Deserialize the expected YAML contents into a struct
        let expected_yaml: ExpectedYamlOverride = from_str(
            r#"---
        world: "notString""#,
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

    #[test]
    fn test_merge_yaml_lists() -> anyhow::Result<()> {
        // Your inline YAML strings for the first and second YAML contents
        let yaml1_str = r#"
        items:
          - apple
          - banana
        world: "hello""#;

        let yaml2_str = r#"
        items:
          - orange
          - cherry
        world: "goodbye""#;

        // Deserialize the inline YAML strings to serde_yaml::Value objects
        let mut yaml1: Value = from_str(yaml1_str)?;
        let yaml2: Value = from_str(yaml2_str)?;

        if let (Value::Mapping(ref mut map1), Value::Mapping(map2)) = (&mut yaml1, &yaml2) {
            merge_maps(map1, map2.clone());
        }

        // Now, let's define the expected merged YAML result
        let expected_str = r#"
        items:
          - apple
          - banana
          - orange
          - cherry
        world: "goodbye""#;
        let expected: Value = from_str(expected_str)?;

        // Test that the merged YAML content matches the expected result
        assert_eq!(expected, yaml1);

        Ok(())
    }

    #[test]
    fn test_load_yaml_from_strings() -> anyhow::Result<()> {
        // Inline YAML strings for the first and second YAML contents
        trace!("Running test_copy_files_complex_manual_override_multiple.");
        let current_dir = current_dir()?;
        let values_path = RelativePath::new("resources/test/merge_lists/first.yaml")
            .to_logical_path(&current_dir);
        let override_path = RelativePath::new("resources/test/merge_lists/second.yaml")
            .to_logical_path(&current_dir);
        let files = vec![
            values_path.to_str().unwrap(),
            override_path.to_str().unwrap(),
            "fruit.color=red",
        ];

        // Load and merge YAML contents from strings using `load_yaml_files` function
        let merged_yaml = load_yaml_files(&files)?;

        // Now, let's define the expected merged YAML result
        let expected_str = r#"
        items:
          - apple
          - banana
          - orange
          - cherry
        world: "goodbye"
        fruit:
          color: "red"
    "#;
        let expected: Value = from_str(expected_str)?;

        // Test that the merged YAML content matches the expected result
        assert_eq!(expected, merged_yaml);

        Ok(())
    }
}
