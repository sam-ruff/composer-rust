use clap::Args;

#[derive(Debug, Args)]
pub struct List {}

impl List {
    pub fn exec(&self) -> anyhow::Result<()> {
        error!("Unimplemented {:?}", self);
        Ok(())
    }
}
