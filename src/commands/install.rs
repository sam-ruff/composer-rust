use anyhow::anyhow;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct Install {
    #[clap(index = 1)]
    pub directory: PathBuf,
    #[clap(short, long)]
    pub id: Option<String>,
    #[clap(short, long)]
    pub value_files: Vec<String>,
}

impl Install {
    pub fn exec(&self) -> anyhow::Result<()> {
        trace!("Command: {:?}", self);
        let readable_id = &Self::get_readable_id();
        let install_id: &String = self.id.as_ref().unwrap_or(readable_id);
        if self.value_files.is_empty() {
            return Err(anyhow!(
                "You cannot install an application with no values file."
            ));
        }
        trace!("Installing application with ID: {}", install_id);
        Ok(())
    }

    fn get_readable_id() -> String {
        petname::petname(3, "-").to_string()
    }
}
