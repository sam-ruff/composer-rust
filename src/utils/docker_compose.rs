use crate::utils::storage::models::ApplicationState::ERROR;
use crate::utils::storage::update_storage::update_application_state;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
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

pub fn compose_up(path: &str, application_id: &str) -> anyhow::Result<()> {
    // A compose file is invalid if its empty or invalid yaml
    check_compose_is_valid(path)?;
    if compose_has_no_services(path) {
        // This is a valid use-case for sub-compose files
        // they should be skipped if no services are created
        trace!(
            "Compose file {} has been skipped due to having no services defined.",
            path
        );
        return Ok(());
    }
    trace!("[EXEC] docker-compose up {}", path);
    let exit_code = unbuffered_command(&["docker-compose", "-f", path, "up", "-d"]);

    if exit_code != 0 {
        update_application_state(application_id, ERROR)
            .expect("Could not update application state.");
        error!("docker-compose up has failed for app {}", application_id);
        std::process::exit(exit_code);
    }
    Ok(())
}

// Compose files are invalid if they are empty, invalid yaml
fn check_compose_is_valid(compose_path: &str) -> anyhow::Result<()> {
    // Check if the path exists
    if !Path::new(compose_path).exists() {
        return Err(anyhow::anyhow!(
            "The provided compose file path '{}' does not exist.",
            compose_path
        ));
    }

    // Read the contents of the file
    let contents = fs::read_to_string(compose_path)?;

    // Check if the file is empty
    if contents.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "The provided compose file '{}' is empty.",
            compose_path
        ));
    }

    // Check if the file is valid YAML
    serde_yaml::from_str::<Value>(&contents).expect(&*format!(
        "The provided compose file '{}' is not a valid YAML file",
        compose_path
    ));
    Ok(())
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Compose {
    services: Option<Vec<String>>,
}

fn compose_has_no_services(compose_path: &str) -> bool {
    let compose_content = match fs::read_to_string(compose_path) {
        Ok(content) => content,
        Err(_) => return false, // Error reading file, return false
    };

    let compose: Result<Compose, serde_yaml::Error> = serde_yaml::from_str(&compose_content);

    match compose {
        Ok(compose_obj) => match compose_obj.services {
            Some(services) => services.is_empty(),
            None => true,
        },
        Err(_) => false, // Error parsing YAML, return false
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
