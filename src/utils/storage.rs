pub struct PersistedApplication {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) state: ApplicationState,
}

pub enum ApplicationState {
    STARTING,
    RUNNING,
    ERROR,
}

pub fn append_to_storage(application: &PersistedApplication) -> anyhow::Result<()> {
    // Creation ~/.composer/config.json if it doesn't exist
    // Append application to it, overwrite existing by ID
    // TODO
    Ok(())
}
