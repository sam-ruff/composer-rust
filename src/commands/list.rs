use crate::utils::storage::models::PersistedApplication;
use crate::utils::storage::read_from::get_all_from_storage;
use clap::Args;

use chrono_humanize::HumanTime;
use std::time::{SystemTime, UNIX_EPOCH};

fn print_applications(apps: &[PersistedApplication], quiet: bool, wide: bool) {
    if quiet {
        for app in apps {
            println!("{}", app.id);
        }
    } else {
        for app in apps {
            let time_delta = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as i64
                - app.timestamp;
            let duration = chrono::Duration::seconds(time_delta);

            let time_formatted = HumanTime::from(duration).to_text_en(
                chrono_humanize::Accuracy::Rough,
                chrono_humanize::Tense::Present,
            );
            if !wide {
                // If we aren't printing lots of info
                info_no_bold!(
                    "{app_id:<20} {version:<15} {time:<15} {status:<15} {app_name:<25}",
                    app_id = app.id,
                    version = app.version,
                    time = time_formatted,
                    status = app.state,
                    app_name = app.app_name
                );
            } else {
                // If we are printing more info
                info_no_bold!(
                    "{app_id:<20} {version:<15} {time:<15} {status:<15} {app_name:<25} {compose_name:<20}",
                    app_id = app.id,
                    version = app.version,
                    time = time_formatted,
                    status = app.state,
                    app_name = app.app_name,
                    compose_name = app.compose_path
                );
            }

        }
    }
}

#[derive(Debug, Args)]
pub struct List {
    /// Prints only the ids of the installed applications
    #[clap(short, long)]
    quiet: bool,
    /// A more detailed output for each installed application
    #[clap(short, long)]
    wide: bool,
}

impl List {
    pub fn exec(&self) -> anyhow::Result<()> {
        let all_applications: Vec<PersistedApplication> = get_all_from_storage()?;
        if !self.quiet && !self.wide {
            info!(
                "{app_id:<20} {version:<15} {time:<15} {status:<15} {app_name:<25}",
                app_id = "APP ID",
                version = "VERSION",
                time = "UPTIME",
                status = "STATUS",
                app_name = "APP NAME"
            );
        }
        else if !self.quiet {
            info!(
                "{app_id:<20} {version:<15} {time:<15} {status:<15} {app_name:<25} {compose_name:<20}",
                app_id = "APP ID",
                version = "VERSION",
                time = "UPTIME",
                status = "STATUS",
                app_name = "APP NAME",
                compose_name = "COMPOSE"
            );
        }
        print_applications(&all_applications, self.quiet, self.wide);
        Ok(())
    }
}
