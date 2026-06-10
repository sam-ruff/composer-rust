use crate::utils::self_updater::{
    current_asset_target, releases_page, run_self_update, GithubReleaseApi, SelfReplaceInstaller,
    SelfUpdateOutcome,
};
use anyhow::{anyhow, Context};
use clap::Args;

#[derive(Debug, Args)]
pub struct SelfUpdate {
    /// Only check whether a newer release exists, without installing it
    #[clap(long)]
    pub check: bool,
}

impl SelfUpdate {
    pub fn exec(&self) -> anyhow::Result<()> {
        let current_version = env!("CARGO_PKG_VERSION");
        let target = current_asset_target().ok_or_else(|| {
            anyhow!(
                "Self-update is not supported on this platform. Download a release manually from {}",
                releases_page()
            )
        })?;
        let staging_dir = tempfile::tempdir().context("Failed to create a staging directory")?;
        let staging_path = staging_dir.path().join("composer-update");

        let api = GithubReleaseApi::new();
        let installer = SelfReplaceInstaller;
        let outcome = run_self_update(
            &api,
            &installer,
            current_version,
            target,
            &staging_path,
            self.check,
        )?;
        match outcome {
            SelfUpdateOutcome::UpToDate { current } => {
                info!("composer {} is already the latest version.", current);
            }
            SelfUpdateOutcome::UpdateAvailable { current, latest } => {
                info!(
                    "A newer composer release is available: {} (currently {}). Run 'composer self-update' to install it.",
                    latest, current
                );
            }
            SelfUpdateOutcome::Updated { from, to } => {
                info!("Updated composer {} -> {}.", from, to);
            }
        }
        Ok(())
    }
}
