use crate::commands::cli::Cli;
use clap::Args;

#[derive(Debug, Args)]
pub struct Upgrade {
    pub test: String,
    pub value_files: Vec<String>,
}

impl Upgrade {
    pub fn exec(&self, cli: &Cli) -> anyhow::Result<()> {
        error!("Unimplemented Command: {:?} cli {:?}", self, cli);
        Ok(())
    }
}
