use clap::Args;

#[derive(Debug, Args)]
pub struct Template {}

impl Template {
    pub fn exec(&self) -> anyhow::Result<()> {
        error!("Unimplemented {:?}", self);
        Ok(())
    }
}
