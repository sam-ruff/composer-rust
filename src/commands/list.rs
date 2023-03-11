use crate::utils::storage::models::PersistedApplication;
use crate::utils::storage::read_from::get_all_from_storage;
use clap::Args;

#[derive(Debug, Args)]
pub struct List {}

impl List {
    pub fn exec(&self) -> anyhow::Result<()> {
        let all_applications: Vec<PersistedApplication> = get_all_from_storage()?;
        for app in all_applications.iter() {
            info!("App: {:#?}", app)
        }
        Ok(())
    }
}
