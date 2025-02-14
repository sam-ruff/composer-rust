use minijinja::{Environment, ErrorKind};
use serde_yaml::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// A custom filter for required
/// Hello {{ world_variable | required }}
/// an error will be thrown in the example above is world_variable is empty
fn required(value: String) -> Result<String, minijinja::Error> {
    if value.is_empty() {
        return Err(
            minijinja::Error::new(
                ErrorKind::InvalidOperation, 
                "Failed to render template. Value marked as 'required' must be present. Check your values file to ensure it exists.")
        )
    } 
    Ok(value)
}


/// Renders a Jinja2 template with the provided YAML values and additional global variables.
///
/// # Arguments
///
/// * `path` - The file path to the Jinja2 template.
/// * `values_yaml` - A `serde_yaml::Value` containing the variables to inject into the template.
///
/// # Returns
///
/// * `Ok(String)` containing the rendered template if successful.
/// * `Err(anyhow::Error)` if an error occurs during rendering.
pub fn render_template(path: &str, values_yaml: Value) -> anyhow::Result<String> {
    // Load the template file into a string
    let mut template_file = File::open(path)?;
    let mut template_string = String::new();
    template_file.read_to_string(&mut template_string)?;

    // Create a Jinja environment
    let mut env = Environment::new();

    // Get the directory of the template file
    let template_dir = Path::new(path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Retrieve global variables to add
    let global_vars = get_global_variables(&template_dir)?;

    // Remove 'composer' key from the input YAML values
    let cleansed_values = remove_composer_key(values_yaml)?;

    // Merge the cleansed YAML values with the global variables
    let merged_values = merge_yaml(cleansed_values, global_vars)?;
    // Add the custom `required` function
    env.add_filter("required", required);
    // Add the template to the environment
    let template_key = "template";
    env.add_template(template_key, &template_string)?;
    let template = env.get_template(template_key)?;

    // Convert the merged data to minijinja values
    let ctx = minijinja::value::Value::from_serializable(&merged_values);

    // Render the template with the input data
    let rendered = template.render(&ctx).map_err(|e| {
        anyhow::anyhow!(
            "Failed to render template {}: due to an error in the template. Error: {}",
            path,
            e
        )
    })?;

    // Return the rendered string
    Ok(rendered)
}

/// Retrieves global variables to be injected into the template.
///
/// # Arguments
///
/// * `template_dir` - The directory of the template file.
///
/// # Returns
///
/// * `Ok(Value)` containing the global variables.
/// * `Err(anyhow::Error)` if an error occurs while constructing global variables.
fn get_global_variables(template_dir: &Path) -> anyhow::Result<Value> {
    // Initialize an empty YAML mapping
    let mut globals = serde_yaml::Mapping::new();

    // All global variables should have composer key
    let composer_key = Value::String("composer".to_string());
    let cwd = template_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert template directory to string"))?
        .to_owned();
    // Add cwd for current working directory
    let mut composer_mapping = serde_yaml::Mapping::new();
    composer_mapping.insert(Value::String("cwd".to_string()), Value::String(cwd));
    globals.insert(composer_key, Value::Mapping(composer_mapping));
    // Add more global variables here if needed in the future
    Ok(Value::Mapping(globals))
}

/// Removes the 'composer' key from the provided YAML `Value` if it exists.
///
/// # Arguments
///
/// * `values` - The original YAML `Value`.
///
/// # Returns
///
/// * `Ok(Value)` with the 'composer' key removed.
/// * `Err(anyhow::Error)` if the YAML structure is invalid.
fn remove_composer_key(values: Value) -> anyhow::Result<Value> {
    match values {
        Value::Mapping(mut mapping) => {
            // Remove the 'composer' key if it exists
            if let Some(pos) = mapping
                .keys()
                .position(|k| match k {
                    Value::String(s) => s == "composer",
                    _ => false,
                })
            {
                // To remove by index, collect keys into a vector
                let key_to_remove = mapping.keys().nth(pos).cloned().unwrap();
                mapping.remove(&key_to_remove);
            }
            Ok(Value::Mapping(mapping))
        }
        _ => Ok(values), // If it's not a mapping, return as is
    }
}

/// Merges two YAML `Value` objects, giving precedence to the second one.
///
/// # Arguments
///
/// * `base` - The base YAML `Value`.
/// * `override_val` - The YAML `Value` to merge into the base.
///
/// # Returns
///
/// * `Ok(Value)` containing the merged YAML.
/// * `Err(anyhow::Error)` if the YAML structures are incompatible.
fn merge_yaml(base: Value, override_val: Value) -> anyhow::Result<Value> {
    match (base, override_val) {
        (Value::Mapping(mut base_map), Value::Mapping(override_map)) => {
            for (k, v) in override_map {
                base_map.insert(k, v);
            }
            Ok(Value::Mapping(base_map))
        }
        _ => Err(anyhow::anyhow!(
            "Both base and override YAML values must be mappings."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::render_template;
    use relative_path::RelativePath;
    use serde_yaml::Value;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    fn test_render_basic_template() -> anyhow::Result<()> {
        trace!("Running test_render_basic_template.");
        let current_dir = current_dir()?;
        let template = RelativePath::new("resources/test/templates/world.jinja2")
            .to_logical_path(&current_dir);
        let yaml = "
        val: world
        ";
        let value_file: Value = serde_yaml::from_str(yaml)?;
        let output_string = render_template(template.to_str().unwrap(), value_file)?;
        let expected_string = "Hello, world!".to_string();
        assert_eq!(expected_string, output_string);
        Ok(())
    }

    #[test]
    fn test_render_template_required_error() -> anyhow::Result<()> {
        // Get the current directory.
        let current_dir = current_dir()?;
        // Construct the full template path.
        let template = RelativePath::new("resources/test/templates/required.jinja2")
            .to_logical_path(&current_dir);
        // Provide YAML with an empty value for the required variable.
        let yaml = r#"
        not_given: ""
        "#;
        let value_file: Value = serde_yaml::from_str(yaml)?;
        // Attempt to render the template.
        let result = render_template(template.to_str().unwrap(), value_file);
        // Assert that rendering fails.
        assert!(result.is_err(), "Template rendering should have failed for an empty required value.");
        // Check that the error message contains the expected text.
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("Failed to render template. Value marked as 'required' must be present"),
            "Error message did not match expected output. Got: {}",
            error_message
        );
        Ok(())
    }


    #[test]
    fn test_render_nested_template_with_default() -> anyhow::Result<()> {
        trace!("Running test_render_nested_template_with_default.");
        let current_dir = current_dir()?;
        let template = RelativePath::new("resources/test/templates/nested-default.jinja2")
            .to_logical_path(&current_dir);
        let yaml = "
        val: world
        nested:
            second_level:
                bool_val: true
        ";
        let value_file: Value = serde_yaml::from_str(yaml)?;
        let output_string = render_template(template.to_str().unwrap(), value_file)?;
        let expected_string = "test default_str true".to_string();
        assert_eq!(expected_string, output_string);
        Ok(())
    }

    #[test]
    fn test_render_invalid_template() -> anyhow::Result<()> {
        trace!("Running test_render_invalid_template.");
        let current_dir = current_dir()?;
        let template = RelativePath::new("resources/test/templates/nested-default.jinja2")
            .to_logical_path(&current_dir);
        let yaml = "
        val: world
        nested:
            some: 'other_value'
        ";
        let value_file: Value = serde_yaml::from_str(yaml)?;
        assert_matches!(
            render_template(template.to_str().unwrap(), value_file),
            Err(_)
        );
        Ok(())
    }

    fn get_current_directory(template: PathBuf) -> anyhow::Result<String> {
        Ok(template
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory of the template"))?
            .canonicalize()?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert expected cwd to string"))?
            .to_owned())
    }

    #[test]
    fn test_render_cwd() -> anyhow::Result<()> {
        trace!("Running test_render_cwd.");

        // Get the current working directory
        let current_dir = current_dir()?;

        // Define the relative path to the cwd template
        let template = RelativePath::new("resources/test/templates/cwd_template.jinja2")
            .to_logical_path(&current_dir);

        // Ensure the template file exists
        assert!(
            template.exists(),
            "Template file does not exist at path: {}",
            template.display()
        );

        // Define minimal YAML input (can be empty as `composer.cwd` is automatically added)
        let yaml = "
        composer:
            some_key: some_value
        ";
        let value_file: Value = serde_yaml::from_str(yaml)?;

        // Render the template
        let output_string = render_template(
            template.to_str().ok_or_else(|| {
                anyhow::anyhow!("Failed to convert template path to string")
            })?,
            value_file,
        )?;

        // Determine the expected `composer.cwd` value (the directory containing the template)
        let expected_cwd = get_current_directory(template)?;

        // The rendered output should be "hi <expected_cwd>"
        let expected_string = format!("hi {}", expected_cwd);
        let rendered_output = output_string.trim(); // Trim to remove any accidental whitespace

        // Assert that the rendered output matches the expected string
        assert_eq!(
            expected_string, rendered_output,
            "Rendered output '{}' does not match expected '{}'",
            rendered_output, expected_string
        );

        // Log the successful test completion
        trace!("test_render_cwd completed successfully.");

        Ok(())
    }

    /// Test to ensure that the 'composer' key in the input YAML is overwritten by the global 'composer' variables.
    #[test]
    fn test_composer_key_overwritten() -> anyhow::Result<()> {
        trace!("Running test_composer_key_overwritten.");
        let current_dir = current_dir()?;
        let template = RelativePath::new("resources/test/templates/composer-overwrite.jinja2")
            .to_logical_path(&current_dir);

        // Ensure the template file exists
        assert!(
            template.exists(),
            "Template file does not exist at path: {}",
            template.display()
        );

        // Define YAML input with 'composer' key that should be overwritten
        let yaml = "
        composer:
            cwd: '/path/from/input'
            other_key: 'should_be_removed_or_ignored'
        ";
        let value_file: Value = serde_yaml::from_str(yaml)?;

        // Render the template
        let output_string = render_template(
            template.to_str().ok_or_else(|| {
                anyhow::anyhow!("Failed to convert template path to string")
            })?,
            value_file,
        )?;

        // Determine the expected 'composer.cwd' value (the directory containing the template)
        let expected_cwd = get_current_directory(template)?;

        // The rendered output should show the global 'composer.cwd', not the one from input
        // The composer.other_key should use the default of 'removed' in the template
        let expected_string = format!("composer cwd: {} and removed", expected_cwd);
        let rendered_output = output_string.trim(); // Trim to remove any accidental whitespace

        // Assert that the rendered output matches the expected string
        assert_eq!(
            expected_string, rendered_output,
            "Rendered output '{}' does not match expected '{}'",
            rendered_output, expected_string
        );

        // Additionally, ensure that other keys under 'composer' are not present
        // For example, 'other_key' should not appear in the rendered output
        assert!(
            !rendered_output.contains("other_key"),
            "Rendered output should not contain keys from input 'composer', but found."
        );

        // Log the successful test completion
        trace!("test_composer_key_overwritten completed successfully.");

        Ok(())
    }


}
