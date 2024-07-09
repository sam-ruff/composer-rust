#[macro_use]
mod macros;

mod app;
mod commands;

mod utils;

use crate::commands::cli::Cli;
use log::LevelFilter;
use std::str::FromStr;

use crate::utils::docker_compose::is_compose_installed;
use clap::Parser;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

fn main() -> anyhow::Result<()> {
    // This needs to be a proper command line app, see https://ofek.dev/words/guides/2022-11-19-writing-a-cli-in-rust/
    // Check that docker is installed
    // Ensure .composer exists
    // Copy files into .composer dir with ignore
    // Load all values files specified into memory as consolidated values
    // Replace all .jinja2 files with templated versions
    // Add it the config json file at the top level to keep track
    // If its a template command print it
    // If its an upgrade command delete existing version
    // If its an install command install it
    let cli = Cli::parse();
    // Set the global verbosity
    let log_level = LevelFilter::from_str(&cli.log_level)?;
    app::set_global_verbosity(log_level);
    app::set_global_always_pull(cli.always_pull);
    app::set_global_no_run(cli.no_run);
    if !is_compose_installed() {
        error!("Docker-compose is not installed. Please install it before using composer.");
        std::process::exit(1);
    }
    let result = cli.run();
    match result {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
    Ok(())
    // TODO implement shell completion https://docs.rs/clap_complete/4.1.4/clap_complete/
}
