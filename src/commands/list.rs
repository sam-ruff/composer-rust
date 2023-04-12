use crate::utils::storage::models::PersistedApplication;
use crate::utils::storage::read_from::get_all_from_storage;
use clap::Args;

#[derive(Debug, Args)]
pub struct List {
    /// Prints only the ids of the installed applications
    #[clap(short, long)]
    quiet: bool,
}

impl List {
    pub fn exec(&self) -> anyhow::Result<()> {
        let all_applications: Vec<PersistedApplication> = get_all_from_storage()?;
        if !self.quiet && !all_applications.is_empty() {
            info!("Installed Apps:")
        }
        if !self.quiet && all_applications.is_empty() {
            info!("No applications installed.")
        }
        for app in all_applications.iter() {
            if self.quiet {
                println!("{}", app.id)
            } else {
                info!("App: {:#?}", app)
            }
        }
        Ok(())
    }
}
