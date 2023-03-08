macro_rules! display {
    ($($arg:tt)*) => {{
        use owo_colors::OwoColorize;
        println!(
            "{}",
            format!($($arg)*)
                .if_supports_color(owo_colors::Stream::Stdout, |text| text.bold())
        );
    }};
}

macro_rules! critical {
    ($($arg:tt)*) => {{
        use owo_colors::OwoColorize;
        eprintln!(
            "{}",
            format!($($arg)*)
                .if_supports_color(owo_colors::Stream::Stderr, |text| text.bright_red())
        );
    }};
}

macro_rules! define_display_macro {
    ($name:ident, $level:ident, $style:ident, $d:tt) => (
        macro_rules! $name {
            ($d($d arg:tt)*) => {{
                use owo_colors::OwoColorize;
                if log::Level::$level <= *$crate::app::verbosity() {
                    eprintln!(
                        "{}",
                        format!($d($d arg)*)
                            .if_supports_color(owo_colors::Stream::Stderr, |text| text.$style())
                    );
                }
            }};
        }
    );
}

define_display_macro!(trace, Trace, underline, $);
define_display_macro!(debug, Debug, italic, $);
define_display_macro!(info, Info, bold, $);
define_display_macro!(success, Info, bright_cyan, $);
define_display_macro!(waiting, Info, bright_magenta, $);
define_display_macro!(warn, Warn, bright_yellow, $);
define_display_macro!(error, Error, bright_red, $);
