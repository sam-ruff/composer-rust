use crate::commands::install::Install;
use crate::commands::list::List;
use crate::commands::test::Test;
use clap::{Parser, Subcommand};
use clap_verbosity_flag::{InfoLevel, Verbosity};

#[derive(Parser, Debug)]
#[command(version, bin_name = "composer")]
pub struct Cli {
    #[clap(flatten)]
    pub verbose: Verbosity<InfoLevel>,

    #[clap(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    // Triple slashes are used for help text in the CLI
    /// Install a docker-compose application using a given jinja2 template
    Install(Install),
    /// List installed composer applications
    List(List),
    Test(Test),
}

impl Cli {
    pub fn run(&self) -> anyhow::Result<()> {
        match &self.cmd {
            Cmd::Install(install) => {
                install.exec()?;
            }
            Cmd::List(list) => {
                list.exec()?;
            }
            Cmd::Test(test) => {
                test.exec()?;
            }
        }
        Ok(())
    }
}
