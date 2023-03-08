mod copy_file_utils;
mod load_values;
mod log_utils;
mod template;

use log::info;
use log::trace;
use log::LevelFilter;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;

fn main() {
    log_utils::setup_logging(LevelFilter::Info, false);
    trace!("Starting up.");
    let world = "World";
    info!("Hello {}!", world);
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
}
