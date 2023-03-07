mod copy_file_utils;
mod load_values;
mod log_utils;

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
}
