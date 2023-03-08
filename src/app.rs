use log::LevelFilter;

static VERBOSITY: once_cell::sync::OnceCell<LevelFilter> = once_cell::sync::OnceCell::new();

pub fn verbosity() -> &'static LevelFilter {
    match VERBOSITY.get() {
        Some(value) => &value,
        None => &LevelFilter::Trace,
    }
}

pub fn set_global_verbosity(verbosity: LevelFilter) {
    VERBOSITY.set(verbosity).expect("could not set verbosity")
}
