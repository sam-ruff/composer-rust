use crate::commands::cli::Cli;
use clap::Args;

#[derive(Debug, Args)]
pub struct Install {
    pub test: String,
    pub value_files: Vec<String>,
}

impl Install {
    pub fn exec(&self, cli: &Cli) -> anyhow::Result<()> {
        error!("Unimplemented Command: {:?} cli {:?}", self, cli);
        Ok(())
    }
}
