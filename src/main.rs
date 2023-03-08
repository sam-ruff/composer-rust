#[macro_use]
mod macros;

mod app;
mod commands;
mod copy_file_utils;
mod load_values;

mod template;

use crate::commands::cli::Cli;

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
    app::set_global_verbosity(cli.verbose.log_level_filter());
    cli.run()
}
