use anyhow::anyhow;

use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::template::render_template;

use clap::Args;

use std::path::PathBuf;
use std::fs::write;

#[derive(Debug, Args)]
pub struct Template {
    #[arg(short, long)]
    pub template: PathBuf,
    #[arg(short, long)]
    pub value_files: Vec<String>,
    #[arg(short, long, default_value_t = String::new())]
    pub output_file: String,
}


impl Template {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("Command: {:?}", self);
    
        if !&self.template.exists() {
            return Err(anyhow!(
                "You have not provided a template file. Use -t <template path> to specify a template file."
            ));
        }

        if self.value_files.is_empty() {
            return Err(anyhow!(
                "You cannot create a template with no values file. Use -v <values path> to specify values file."
            ));
        }

        let values: Vec<&str> = get_value_files_as_refs(&self.value_files);
        let consolidated_values: serde_yaml::Value = load_yaml_files(&values)?;

        trace!(
            "Consolidated values: \n```\n{}\n```\n",
            serde_yaml::to_string(&consolidated_values).unwrap()
        );
        

        let rendered_template: Result<String, anyhow::Error> = render_template(&self.template.to_str().unwrap(), consolidated_values);

        if self.output_file.is_empty() {
            // Print output to console
            println!("{}", &rendered_template.unwrap());
        } else {
            // Output to file
            let result: Result<(), std::io::Error> = write(&self.output_file, &rendered_template.unwrap());
            match result {
                Ok(_) => {},
                Err(err) => return Err(anyhow!(err))
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};
    use relative_path::RelativePath;

    use crate::commands::template::Template;
    use serial_test::serial;
    use std::env::current_dir;
    use std::path::PathBuf;

    #[test]
    #[serial]
    fn test_template_without_output_file() -> anyhow::Result<()> {
        trace!("Running test_template_without_output_file.");

        let current_dir: PathBuf = current_dir()?;
        let values_path: PathBuf = RelativePath::new("resources/test/test_values/values.yaml").to_logical_path(&current_dir);
        let template_path: PathBuf = RelativePath::new("resources/test/simple/docker-compose.jinja2").to_logical_path(&current_dir);

        let test_template_cmd: Template = Template {
            template: template_path.to_owned(),
            value_files: vec![values_path.to_str().unwrap().to_owned()],
            output_file: String::new()
        };

        test_template_cmd.exec()?;

        Ok(())
    }

    #[test]
    #[serial]
    fn test_template_with_output_file() -> anyhow::Result<()> {
        trace!("Running test_template_with_output_file.");

        let temp_dir: TempDir = tempdir()?;
        let current_dir: PathBuf = current_dir()?;
        let values_path: PathBuf = RelativePath::new("resources/test/test_values/values.yaml").to_logical_path(&current_dir);
        let template_path: PathBuf = RelativePath::new("resources/test/simple/docker-compose.jinja2").to_logical_path(&current_dir);
        let output_path: PathBuf = temp_dir.path().join("output.txt");

        let test_template_cmd: Template = Template {
            template: template_path.to_owned(),
            value_files: vec![values_path.to_str().unwrap().to_owned()],
            output_file: output_path.to_str().unwrap().to_owned()
        };

        test_template_cmd.exec()?;

        assert!(output_path.exists());

        Ok(())
    }

    #[test]
    #[serial]
    fn test_template_no_template_file() -> anyhow::Result<()> {
        trace!("Running test_template_no_template_file.");

        let test_template_cmd: Template = Template {
            template: PathBuf::new(),
            value_files: vec![],
            output_file: String::new()
        };

        let err = test_template_cmd.exec().err().unwrap();
        let actual_err: String = err.to_string();
        let expected_error: String = "You have not provided a template file. Use -t <template path> to specify a template file.".to_string();

        assert_eq!(actual_err, expected_error);

        Ok(())
    }

    #[test]
    #[serial]
    fn test_template_no_value_files() -> anyhow::Result<()> {
        trace!("Running test_template_pass.");

        let current_dir: PathBuf = current_dir()?;
        let template_path: PathBuf = RelativePath::new("resources/test/simple/docker-compose.jinja2").to_logical_path(&current_dir);

        let test_template_cmd: Template = Template {
            template: template_path.to_owned(),
            value_files: vec![],
            output_file: String::new()
        };

        let err = test_template_cmd.exec().err().unwrap();
        let actual_err: String = err.to_string();
        let expected_error: String = "You cannot create a template with no values file. Use -v <values path> to specify values file.".to_string();

        assert_eq!(actual_err, expected_error);

        Ok(())
    }
}