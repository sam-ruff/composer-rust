use crate::utils::storage::models::ApplicationState::ERROR;
use crate::utils::storage::update_storage::update_application_state;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

/// Abstraction over external process execution so command orchestration
/// can be unit tested without spawning real processes.
#[cfg_attr(test, mockall::automock)]
pub trait CommandRunner {
    fn run_unbuffered(&self, args: Vec<String>) -> i32;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run_unbuffered(&self, args: Vec<String>) -> i32 {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        unbuffered_command(&arg_refs)
    }
}

pub fn unbuffered_command(command_line_args: &[&str]) -> i32 {
    let Some((command, args)) = command_line_args.split_first() else {
        error!("Cannot run an empty command.");
        return -1;
    };
    let mut process = match Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            error!("Failed to spawn command {:?}: {}", command_line_args, e);
            return -1;
        }
    };

    if let Some(stdout) = process.stdout.take() {
        log_subprocess_output(stdout);
    }

    match process.wait() {
        Ok(status) => status.code().unwrap_or(-1),
        Err(e) => {
            error!("Failed to wait on command {:?}: {}", command_line_args, e);
            -1
        }
    }
}

fn log_subprocess_output(pipe: impl std::io::Read) {
    let reader = BufReader::new(pipe);

    for line in reader.lines().map_while(Result::ok) {
        info!("{}", line);
    }
}

fn build_compose_up_args(path: &str) -> Vec<String> {
    ["docker", "compose", "-f", path, "up", "-d", "--remove-orphans"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn build_compose_down_args(path: &str) -> Vec<String> {
    ["docker", "compose", "-f", path, "down", "--remove-orphans"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn build_compose_pull_args(path: &str) -> Vec<String> {
    ["docker", "compose", "-f", path, "pull", "--ignore-pull-failures"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn compose_up(path: &str, application_id: &str) -> anyhow::Result<()> {
    compose_up_with(&RealCommandRunner, path, application_id)
}

fn compose_up_with(
    runner: &impl CommandRunner,
    path: &str,
    application_id: &str,
) -> anyhow::Result<()> {
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
    trace!("[EXEC] docker compose up {}", path);
    let exit_code = runner.run_unbuffered(build_compose_up_args(path));

    if exit_code != 0 {
        if let Err(e) = update_application_state(application_id, ERROR) {
            error!(
                "Could not update application state for app {}: {}",
                application_id, e
            );
        }
        error!("docker compose up has failed for app {}", application_id);
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
    serde_yaml::from_str::<Value>(&contents).map_err(|e| {
        anyhow::anyhow!(
            "The provided compose file '{}' is not a valid YAML file: {}",
            compose_path,
            e
        )
    })?;
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
    compose_down_with(&RealCommandRunner, path, application_id)
}

fn compose_down_with(runner: &impl CommandRunner, path: &str, application_id: &str) {
    trace!("[EXEC] docker compose down {}", path);
    if compose_has_no_services(path) {
        // This is a valid use-case for sub-compose files
        // they should be skipped if no services are created
        trace!(
            "Compose down for file {} has been skipped due to having no services defined.",
            path
        );
        return;
    }
    let exit_code = runner.run_unbuffered(build_compose_down_args(path));

    if exit_code != 0 {
        if let Err(e) = update_application_state(application_id, ERROR) {
            error!(
                "Could not update application state for app {}: {}",
                application_id, e
            );
        }
        error!(
            "docker compose down has failed for app {}. Some containers may still persist.",
            application_id
        );
    }
}

pub fn is_compose_installed() -> bool {
    match silent_run(&["docker", "compose", "version"]).status() {
        Ok(status) => {
            if status.success() {
                true
            } else {
                error!("docker compose is installed but returned an error.");
                false
            }
        }
        Err(_) => false,
    }
}

pub fn silent_run(args: &[&str]) -> Command {
    trace!("Running command: {:?}", args);
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

pub fn compose_pull(path: &str) {
    compose_pull_with(&RealCommandRunner, path)
}

fn compose_pull_with(runner: &impl CommandRunner, path: &str) {
    info!("Always pull is enabled. Pulling latest images. Will ignore failures of local images.");
    runner.run_unbuffered(build_compose_pull_args(path));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const COMPOSE_WITH_SERVICES: &str = "services:\n  web:\n    image: busybox\n";

    fn temp_compose_file(content: &str) -> anyhow::Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;
        Ok(file)
    }

    fn path_str(file: &NamedTempFile) -> String {
        file.path().to_string_lossy().into_owned()
    }

    #[test]
    fn test_build_compose_up_args() {
        let expected = vec![
            "docker",
            "compose",
            "-f",
            "compose.yaml",
            "up",
            "-d",
            "--remove-orphans",
        ];
        assert_eq!(expected, build_compose_up_args("compose.yaml"));
    }

    #[test]
    fn test_build_compose_down_args() {
        let expected = vec![
            "docker",
            "compose",
            "-f",
            "compose.yaml",
            "down",
            "--remove-orphans",
        ];
        assert_eq!(expected, build_compose_down_args("compose.yaml"));
    }

    #[test]
    fn test_build_compose_pull_args() {
        let expected = vec![
            "docker",
            "compose",
            "-f",
            "compose.yaml",
            "pull",
            "--ignore-pull-failures",
        ];
        assert_eq!(expected, build_compose_pull_args("compose.yaml"));
    }

    #[test]
    fn test_compose_up_runs_command_for_valid_file() -> anyhow::Result<()> {
        let file = temp_compose_file(COMPOSE_WITH_SERVICES)?;
        let path = path_str(&file);
        let expected_args = build_compose_up_args(&path);
        let mut runner = MockCommandRunner::new();
        runner
            .expect_run_unbuffered()
            .withf(move |args| *args == expected_args)
            .times(1)
            .returning(|_| 0);
        compose_up_with(&runner, &path, "test_app")
    }

    #[test]
    fn test_compose_up_skips_file_with_no_services() -> anyhow::Result<()> {
        let file = temp_compose_file("services: []\n")?;
        // No expectations set: any call to the runner fails the test
        let runner = MockCommandRunner::new();
        compose_up_with(&runner, &path_str(&file), "test_app")
    }

    #[test]
    fn test_compose_up_missing_file_errors() {
        let runner = MockCommandRunner::new();
        let result = compose_up_with(&runner, "/nonexistent/compose.yaml", "test_app");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("does not exist"), "unexpected error: {}", err);
    }

    #[test]
    fn test_compose_up_empty_file_errors() -> anyhow::Result<()> {
        let file = temp_compose_file("   \n")?;
        let runner = MockCommandRunner::new();
        let result = compose_up_with(&runner, &path_str(&file), "test_app");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("is empty"), "unexpected error: {}", err);
        Ok(())
    }

    #[test]
    fn test_compose_up_invalid_yaml_errors_without_panicking() -> anyhow::Result<()> {
        let file = temp_compose_file("services: [unclosed\n")?;
        let runner = MockCommandRunner::new();
        let result = compose_up_with(&runner, &path_str(&file), "test_app");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not a valid YAML file"),
            "unexpected error: {}",
            err
        );
        Ok(())
    }

    #[test]
    fn test_compose_down_runs_command_for_valid_file() -> anyhow::Result<()> {
        let file = temp_compose_file(COMPOSE_WITH_SERVICES)?;
        let path = path_str(&file);
        let expected_args = build_compose_down_args(&path);
        let mut runner = MockCommandRunner::new();
        runner
            .expect_run_unbuffered()
            .withf(move |args| *args == expected_args)
            .times(1)
            .returning(|_| 0);
        compose_down_with(&runner, &path, "test_app");
        Ok(())
    }

    #[test]
    fn test_compose_down_skips_file_with_no_services() -> anyhow::Result<()> {
        let file = temp_compose_file("services: []\n")?;
        let runner = MockCommandRunner::new();
        compose_down_with(&runner, &path_str(&file), "test_app");
        Ok(())
    }

    #[test]
    fn test_compose_pull_runs_command() {
        let expected_args = build_compose_pull_args("compose.yaml");
        let mut runner = MockCommandRunner::new();
        runner
            .expect_run_unbuffered()
            .withf(move |args| *args == expected_args)
            .times(1)
            .returning(|_| 0);
        compose_pull_with(&runner, "compose.yaml");
    }

    #[test]
    fn test_unbuffered_command_empty_args_returns_error_code() {
        assert_eq!(-1, unbuffered_command(&[]));
    }

    #[test]
    fn test_unbuffered_command_missing_binary_returns_error_code() {
        assert_eq!(
            -1,
            unbuffered_command(&["this-binary-does-not-exist-composer-test"])
        );
    }
}
