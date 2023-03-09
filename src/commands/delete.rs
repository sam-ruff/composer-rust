use clap::Args;

#[derive(Debug, Args)]
pub struct Delete {
    /// The application ids to delete
    #[clap(short, long, conflicts_with("all"))]
    pub ids: Option<Vec<String>>,
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
