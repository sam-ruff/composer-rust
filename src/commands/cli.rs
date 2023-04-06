use crate::commands::delete::Delete;
use crate::commands::install::Install;
use crate::commands::list::List;
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
    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    // Triple slashes are used for help text in the CLI
    /// Install a docker-compose application using a given jinja2 template
    #[clap(alias = "i", alias = "add")]
    Install(Install),
    /// Upgrades an existing composer application, this is equivalent to doing docker-compose up again, so existing services will remain and only deltas will be applied.
    #[clap(alias = "u", alias = "update")]
    Upgrade(Upgrade),
    /// List installed composer applications
    #[clap(alias = "ls", alias = "ps")]
    List(List),
    /// Prints the output docker_compose.yaml once the values have been applied. Can
    ///   be used to produce a compose for use outside of the composer install
    ///   environment or for debugging purposes.
    #[clap(alias = "t")]
    Template(Template),
    /// Deletes a given application(s) (by id unless using --all), removing it
    ///   completely.
    #[clap(alias = "d", alias = "uninstall")]
    Delete(Delete),
    // Hidden test function
    Test(Test),
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match &self.cmd {
            Cmd::Install(install) => install.exec()?,
            Cmd::Upgrade(upgrade) => upgrade.exec()?,
            Cmd::List(list) => list.exec()?,
            Cmd::Test(test) => test.exec()?,
            Cmd::Template(template) => template.exec()?,
            Cmd::Delete(delete) => delete.exec()?,
        }
        Ok(())
    }
}
