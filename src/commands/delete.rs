use clap::Args;

#[derive(Debug, Args)]
pub struct Delete {
    /// The application ids to delete, space seperated to delete multiple applications at once
    #[clap(index = 1, required_unless_present = "all", conflicts_with("all"))]
    pub ids: Vec<String>,
    /// If the all flag is set all composer applications will be deleted
    #[clap(long)]
    pub all: bool,
}

impl Delete {
    pub fn exec(&self) -> anyhow::Result<()> {
        error!("Unimplemented Command: {:?}", self);
        Ok(())
    }
}
