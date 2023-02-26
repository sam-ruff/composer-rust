mod copy_file_utils;
mod log_utils;

use log::info;
use log::trace;
use log::LevelFilter;

fn main() {
    log_utils::setup_logging(LevelFilter::Info, false);
    trace!("Starting up.");
    let world = "World";
    info!("Hello {}!", world);
}
