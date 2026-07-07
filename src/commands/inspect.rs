use chrono::{TimeZone, Utc};
use chrono_humanize::HumanTime;
use clap::Args;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::load_values::{get_value_files_as_refs, load_yaml_files};
use crate::utils::storage::models::{ApplicationState, PersistedApplication};
use crate::utils::storage::read_from::get_application_by_id;

#[derive(Debug, Args)]
pub struct Inspect {
    /// Id of the installed application to inspect
    #[clap(index = 1)]
    pub id: String,
    /// Emit a single JSON document instead of the human-readable sectioned view
    #[clap(short, long)]
    pub json: bool,
}

#[derive(Debug, Clone)]
struct ValueFileEntry {
    path: String,
    missing: bool,
}

impl Inspect {
    pub fn exec(&self) -> anyhow::Result<()> {
        let app = get_application_by_id(&self.id)?;

        let entries: Vec<ValueFileEntry> = app
            .value_files
            .iter()
            .map(|p| ValueFileEntry {
                path: p.clone(),
                missing: !Path::new(p).exists(),
            })
            .collect();

        let missing_count = entries.iter().filter(|e| e.missing).count();
        let present_paths: Vec<String> = entries
            .iter()
            .filter(|e| !e.missing)
            .map(|e| e.path.clone())
            .collect();

        if missing_count > 0 {
            warn!(
                "{} value file(s) recorded at install time are missing on disk; showing merged values from the {} remaining file(s).",
                missing_count,
                present_paths.len(),
            );
        }

        let merged: Option<serde_yaml::Value> = if present_paths.is_empty() {
            None
        } else {
            let refs = get_value_files_as_refs(&present_paths);
            Some(load_yaml_files(&refs)?)
        };

        let output = if self.json {
            render_inspect_json(&app, &entries, merged.as_ref())?
        } else {
            render_inspect_human(&app, &entries, merged.as_ref())?
        };
        println!("{}", output);
        Ok(())
    }
}

fn state_label(state: &ApplicationState) -> &'static str {
    match state {
        ApplicationState::Starting => "STARTING",
        ApplicationState::Running => "RUNNING",
        ApplicationState::Error => "ERROR",
    }
}

fn humanised_installed(timestamp: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(timestamp);
    let duration = chrono::Duration::seconds(now - timestamp);
    let human = HumanTime::from(duration).to_text_en(
        chrono_humanize::Accuracy::Rough,
        chrono_humanize::Tense::Present,
    );
    let absolute = Utc
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("unix {}", timestamp));
    format!("{} ({})", human, absolute)
}

fn iso_installed(timestamp: i64) -> String {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| format!("unix {}", timestamp))
}

/// Prefix every non-empty line of `block` with `spaces` blanks so a YAML block
/// can sit under a flush-left section header.
fn indent(block: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    block
        .lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{}{}", pad, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_inspect_human(
    app: &PersistedApplication,
    value_files: &[ValueFileEntry],
    merged: Option<&serde_yaml::Value>,
) -> anyhow::Result<String> {
    let mut out = String::new();
    out.push_str("APPLICATION\n");
    out.push_str(&format!("  ID:            {}\n", app.id));
    out.push_str(&format!("  Name:          {}\n", app.app_name));
    out.push_str(&format!("  Version:       {}\n", app.version));
    out.push_str(&format!("  Status:        {}\n", state_label(&app.state)));
    out.push_str(&format!(
        "  Installed:     {}\n",
        humanised_installed(app.timestamp)
    ));
    out.push_str(&format!("  Compose path:  {}\n", app.compose_path));
    out.push('\n');

    out.push_str(&format!("VALUE FILES ({})\n", value_files.len()));
    if value_files.is_empty() {
        out.push_str("  (none recorded)\n");
    } else {
        for (idx, entry) in value_files.iter().enumerate() {
            let marker = if entry.missing { "    [MISSING]" } else { "" };
            out.push_str(&format!("  {}. {}{}\n", idx + 1, entry.path, marker));
        }
    }
    out.push('\n');

    out.push_str("MERGED VALUES\n");
    match merged {
        None => out.push_str("  (none)\n"),
        Some(value) => {
            let yaml = serde_yaml::to_string(value)?;
            out.push_str(&indent(yaml.trim_end(), 2));
            out.push('\n');
        }
    }
    Ok(out)
}

fn render_inspect_json(
    app: &PersistedApplication,
    value_files: &[ValueFileEntry],
    merged: Option<&serde_yaml::Value>,
) -> anyhow::Result<String> {
    let files: Vec<serde_json::Value> = value_files
        .iter()
        .map(|e| {
            serde_json::json!({
                "path": e.path,
                "missing": e.missing,
            })
        })
        .collect();

    let merged_json: serde_json::Value = match merged {
        Some(v) => serde_json::to_value(v)?,
        None => serde_json::Value::Null,
    };

    let doc = serde_json::json!({
        "application": {
            "id": app.id,
            "name": app.app_name,
            "version": app.version,
            "status": state_label(&app.state),
            "timestamp": app.timestamp,
            "installed_at": iso_installed(app.timestamp),
            "compose_path": app.compose_path,
        },
        "value_files": files,
        "merged_values": merged_json,
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::storage::models::{ApplicationState, PersistedApplication};
    use crate::utils::storage::write_to_storage::append_to_storage;
    use crate::utils::test_utils::{
        backup_composer_config, move_file_if_exists,
    };
    use serial_test::serial;

    fn sample_app(id: &str, value_files: Vec<String>) -> PersistedApplication {
        PersistedApplication {
            id: id.to_string(),
            version: "1.2.3".to_string(),
            timestamp: 0,
            state: ApplicationState::Running,
            app_name: id.to_string(),
            compose_path: format!("/tmp/{}/docker-compose.yaml", id),
            value_files,
        }
    }

    #[test]
    #[serial]
    fn test_inspect_unknown_id_errors() -> anyhow::Result<()> {
        let (composer_json_config, composer_json_config_backup) = backup_composer_config()?;
        let existing = sample_app("test_inspect_unknown_id_existing", vec![]);
        append_to_storage(&existing)?;

        let cmd = Inspect {
            id: "test_inspect_unknown_id_missing".to_string(),
            json: false,
        };
        let err = cmd.exec().unwrap_err();
        let message = err.to_string();

        move_file_if_exists(&composer_json_config_backup, &composer_json_config)?;

        assert!(
            message.contains("Application with id test_inspect_unknown_id_missing not found"),
            "expected not-found error, got: {}",
            message,
        );
        Ok(())
    }

    #[test]
    fn test_render_inspect_sectioned_contains_all_sections() -> anyhow::Result<()> {
        let app = sample_app("my-app", vec!["/tmp/base.yaml".to_string()]);
        let entries = vec![ValueFileEntry {
            path: "/tmp/base.yaml".to_string(),
            missing: false,
        }];
        let merged: serde_yaml::Value = serde_yaml::from_str("hello: true\nworld: string\n")?;

        let out = render_inspect_human(&app, &entries, Some(&merged))?;

        assert!(out.contains("APPLICATION"), "missing APPLICATION header:\n{}", out);
        assert!(out.contains("VALUE FILES (1)"), "missing VALUE FILES header:\n{}", out);
        assert!(out.contains("MERGED VALUES"), "missing MERGED VALUES header:\n{}", out);
        assert!(out.contains("ID:            my-app"), "missing id row:\n{}", out);
        assert!(out.contains("Version:       1.2.3"), "missing version row:\n{}", out);
        assert!(out.contains("Status:        RUNNING"), "missing status row:\n{}", out);
        assert!(out.contains("1. /tmp/base.yaml"), "missing numbered value file:\n{}", out);
        assert!(out.contains("  hello: true"), "merged values not indented:\n{}", out);
        Ok(())
    }

    #[test]
    fn test_render_inspect_flags_missing_value_files() -> anyhow::Result<()> {
        let app = sample_app(
            "my-app",
            vec!["/tmp/base.yaml".to_string(), "/tmp/gone.yaml".to_string()],
        );
        let entries = vec![
            ValueFileEntry { path: "/tmp/base.yaml".to_string(), missing: false },
            ValueFileEntry { path: "/tmp/gone.yaml".to_string(), missing: true },
        ];
        let merged: serde_yaml::Value = serde_yaml::from_str("hello: true\n")?;

        let out = render_inspect_human(&app, &entries, Some(&merged))?;

        assert!(out.contains("VALUE FILES (2)"), "count should include missing files:\n{}", out);
        assert!(out.contains("2. /tmp/gone.yaml    [MISSING]"),
            "missing file should be flagged:\n{}", out);
        assert!(!out.contains("1. /tmp/base.yaml    [MISSING]"),
            "present file should not be flagged:\n{}", out);
        Ok(())
    }

    #[test]
    fn test_render_inspect_no_value_files() -> anyhow::Result<()> {
        let app = sample_app("my-app", vec![]);
        let out = render_inspect_human(&app, &[], None)?;

        assert!(out.contains("VALUE FILES (0)"), "count should be zero:\n{}", out);
        assert!(out.contains("(none recorded)"), "expected (none recorded):\n{}", out);
        assert!(out.contains("(none)"), "expected (none) merged marker:\n{}", out);
        Ok(())
    }

    #[test]
    fn test_inspect_json_shape() -> anyhow::Result<()> {
        let app = sample_app("my-app", vec!["/tmp/base.yaml".to_string()]);
        let entries = vec![ValueFileEntry {
            path: "/tmp/base.yaml".to_string(),
            missing: false,
        }];
        let merged: serde_yaml::Value = serde_yaml::from_str("hello: true\nworld: string\n")?;

        let out = render_inspect_json(&app, &entries, Some(&merged))?;
        let parsed: serde_json::Value = serde_json::from_str(&out)?;

        assert_eq!(parsed["application"]["id"], "my-app");
        assert_eq!(parsed["application"]["version"], "1.2.3");
        assert_eq!(parsed["application"]["status"], "RUNNING");
        assert_eq!(parsed["value_files"][0]["path"], "/tmp/base.yaml");
        assert_eq!(parsed["value_files"][0]["missing"], false);
        assert_eq!(parsed["merged_values"]["hello"], true);
        assert_eq!(parsed["merged_values"]["world"], "string");
        Ok(())
    }

    #[test]
    fn test_inspect_json_null_when_no_value_files() -> anyhow::Result<()> {
        let app = sample_app("my-app", vec![]);
        let out = render_inspect_json(&app, &[], None)?;
        let parsed: serde_json::Value = serde_json::from_str(&out)?;
        assert!(parsed["merged_values"].is_null(), "expected null merged_values, got: {}", parsed["merged_values"]);
        assert_eq!(parsed["value_files"].as_array().unwrap().len(), 0);
        Ok(())
    }
}
