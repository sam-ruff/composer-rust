use clap::Args;

#[derive(Debug, Args)]
pub struct Install {
    pub test: String,
    pub value_files: Vec<String>,
}

impl Install {
    pub fn exec(&self) -> anyhow::Result<()> {
        error!("Unimplemented {:?}", self);
        Ok(())
    }
}
