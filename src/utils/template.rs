use minijinja::Environment;
use serde_yaml::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn render_template(path: &str, values_yaml: Value) -> anyhow::Result<String> {
    // Load the template file into a string
    let mut template_file = File::open(path)?;
    let mut template_string = String::new();
    template_file.read_to_string(&mut template_string)?;

    // Create a Jinja environment and parse the template string
    let mut env = Environment::new();

    // Get the directory of the template file
    let template_dir = Path::new(path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Add 'composer.cwd' to the globals
    let mut values_with_globals = values_yaml.clone();

    // Retrieve the current working directory as a string
    let cwd = template_dir
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert current directory to string"))?
        .to_owned();

    // Insert 'composer.cwd' into the YAML mapping
    match &mut values_with_globals {
        Value::Mapping(mapping) => {
            // Get or create the 'composer' mapping
            let composer_key = Value::String("composer".to_string());
            let composer_value = mapping.entry(composer_key.clone())
                .or_insert(Value::Mapping(serde_yaml::Mapping::new()));

            match composer_value {
                Value::Mapping(composer_mapping) => {
                    composer_mapping.insert(
                        Value::String("cwd".to_string()),
                        Value::String(cwd),
                    );
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Expected 'composer' to be a mapping, but found {:?}",
                        composer_value
                    ));
                }
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Expected YAML mapping for template variables, but found {:?}",
                values_with_globals
            ));
        }
    }

    // Add the template to the environment
    let template_key = "template";
    env.add_template(template_key, &template_string)?;
    let template = env.get_template(template_key)?;

    // Convert the data to minijinja values
    let ctx = minijinja::value::Value::from_serializable(&values_with_globals);

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

#[cfg(test)]
mod tests {
    use crate::utils::template::render_template;
    use relative_path::RelativePath;
    use serde_yaml::Value;
    use std::env::current_dir;

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
        let yaml = "{}";
        let value_file: Value = serde_yaml::from_str(yaml)?;

        // Render the template
        let output_string = render_template(
            template.to_str().ok_or_else(|| {
                anyhow::anyhow!("Failed to convert template path to string")
            })?,
            value_file,
        )?;

        // Determine the expected `composer.cwd` value (the directory containing the template)
        let expected_cwd = template
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory of the template"))?
            .canonicalize()?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert expected cwd to string"))?
            .to_owned();

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



}
