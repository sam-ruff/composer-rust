use clap::Args;

#[derive(Debug, Args)]
pub struct Upgrade {
    pub test: String,
    pub value_files: Vec<String>,
}

impl Upgrade {
    pub fn exec(&self) -> anyhow::Result<()> {
        error!("Unimplemented Command: {:?}", self);
        Ok(())
    }
}
