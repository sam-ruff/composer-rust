use crate::commands::delete::Delete;
use crate::commands::inspect::Inspect;
use crate::commands::install::Install;
use crate::commands::list::List;
use crate::commands::self_update::SelfUpdate;
use crate::commands::template::Template;
use crate::commands::test::Test;
use crate::commands::upgrade::Upgrade;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, bin_name = "composer")]
pub struct Cli {
    /// Verbosity level settings, values can be INFO, ERROR, TRACE, WARN
    #[clap(short, long, default_value = "INFO", alias = "log_level")]
    pub log_level: String,
    /// If included as a flag, before installing/upgrading an application, all images will attempt to be pulled that are specified in the template.jinja
    #[clap(short, long)]
    pub always_pull: bool,
    /// If included, docker compose up command is omitted
    #[clap(short, long)]
    pub no_run: bool,
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    // Triple slashes are used for help text in the CLI
    /// Install a docker-compose application using a given jinja2 template
    #[clap(alias = "i", alias = "add")]
    Install(Install),
    /// Upgrades an existing composer application. By default this re-renders the
    ///   templates and runs docker compose up again, so existing services remain
    ///   and only deltas are applied. Pass --always_down to force a full docker
    ///   compose down of every compose file before bringing it back up.
    #[clap(alias = "u", alias = "update")]
    Upgrade(Upgrade),
    /// List installed composer applications
    #[clap(alias = "ls", alias = "ps")]
    List(List),
    /// Show all persisted info for an installed application, including the
    ///   ordered list of value files it was installed with and the fully
    ///   merged, reference-resolved values that would be handed to the
    ///   template.
    #[clap(alias = "describe")]
    Inspect(Inspect),
    /// Prints the output docker_compose.yaml once the values have been applied. Can
    ///   be used to produce a compose for use outside of the composer install
    ///   environment or for debugging purposes.
    #[clap(alias = "t")]
    Template(Template),
    /// Deletes a given application(s) (by id unless using --all), removing it
    ///   completely.
    #[clap(alias = "d", alias = "uninstall")]
    Delete(Delete),
    /// Updates composer itself to the latest released version
    SelfUpdate(SelfUpdate),
    // Hidden test function
    Test(Test),
}

impl Cli {
    /// The update notice is redundant noise when the user is already
    /// running self-update.
    pub fn is_self_update(&self) -> bool {
        matches!(self.cmd, Cmd::SelfUpdate(_))
    }

    pub fn run(&self) -> anyhow::Result<()> {
        match &self.cmd {
            Cmd::Install(install) => install.exec()?,
            Cmd::Upgrade(upgrade) => upgrade.exec()?,
            Cmd::List(list) => list.exec()?,
            Cmd::Inspect(inspect) => inspect.exec()?,
            Cmd::Test(test) => test.exec()?,
            Cmd::Template(template) => template.exec()?,
            Cmd::Delete(delete) => delete.exec()?,
            Cmd::SelfUpdate(self_update) => self_update.exec()?,
        }
        Ok(())
    }
}
