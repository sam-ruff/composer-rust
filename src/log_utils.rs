use log::LevelFilter;
use std::io::Write;

// This stops panic if the logging library is initialised multiple times. As this happens in the tests
pub fn setup_logging(filter: LevelFilter, is_test: bool) {
    let _ = pretty_env_logger::env_logger::Builder::from_default_env()
        .is_test(is_test)
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .filter(None, filter)
        .try_init();
}
