use anyhow::anyhow;

use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::template::render_template;

use clap::Args;

use std::path::PathBuf;
use std::fs::write;

#[derive(Debug, Args)]
pub struct Template {
    #[arg(short, long, default_value_t = String::from("./docker-compose.jijna2"))]
    pub template: String,
    #[arg(short, long)]
    pub value_files: Vec<String>,
    #[arg(short, long, default_value_t = String::new())]
    pub output_file: String,
}


impl Template {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("Command: {:?}", self);

        let template_path: PathBuf = PathBuf::from(&self.template);
    
        if !template_path.exists() {
            return Err(anyhow!(
                "You have not provided a template file, and we could not find the default file. Use -t <template path> to specify a template file."
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
        

        let rendered_template: Result<String, anyhow::Error> = render_template(template_path.to_str().unwrap(), consolidated_values);

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
