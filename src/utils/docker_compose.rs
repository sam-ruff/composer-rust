use crate::utils::storage::models::ApplicationState::ERROR;
use crate::utils::storage::update_storage::update_application_state;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn unbuffered_command(command_line_args: &[&str]) -> i32 {
    let mut process = Command::new(command_line_args[0])
        .args(&command_line_args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();

    if let Some(stdout) = process.stdout.take() {
        log_subprocess_output(stdout);
    }

    process.wait().unwrap().code().unwrap_or(-1)
}

fn log_subprocess_output(pipe: impl std::io::Read) {
    let reader = BufReader::new(pipe);

    for line in reader.lines() {
        if let Ok(line) = line {
            info!("{}", line);
        }
    }
}

pub fn compose_up(path: &str, application_id: &str) {
    trace!("[EXEC] docker-compose up {}", path);
    let exit_code = unbuffered_command(&["docker-compose", "-f", path, "up", "-d"]);

    if exit_code != 0 {
        update_application_state(application_id, ERROR)
            .expect("Could not update application state.");
        error!("docker-compose up has failed for app {}", application_id);
        std::process::exit(exit_code);
    }
}

pub fn compose_down(path: &str, application_id: &str) {
    trace!("[EXEC] docker-compose down {}", path);
    let exit_code = unbuffered_command(&["docker-compose", "-f", path, "down"]);

    if exit_code != 0 {
        update_application_state(application_id, ERROR)
            .expect("Could not update application state.");
        error!(
            "docker-compose down has failed for app {}. Some containers may still persist.",
            application_id
        );
    }
}

pub fn is_compose_installed() -> bool {
    silent_run(&["docker-compose", "version"])
        .status()
        .unwrap()
        .success()
}

pub fn silent_run(args: &[&str]) -> Command {
    trace!("Running command: {:?}", args);
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

pub fn compose_pull(path: &str) {
    let command_to_run = [
        "docker-compose",
        "-f",
        path,
        "pull",
        "--ignore-pull-failures",
    ];
    info!("Always pull is enabled. Pulling latest images. Will ignore failures of local images.");
    unbuffered_command(&command_to_run);
}
