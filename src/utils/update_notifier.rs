use crate::utils::copy_file_utils::get_composer_directory;
use crate::utils::self_updater::{parse_version, GithubReleaseApi, ReleaseApi};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Set to any non-empty value to disable the background update check.
pub const DISABLE_ENV_VAR: &str = "COMPOSER_NO_UPDATE_CHECK";

const CACHE_FILE: &str = "update_check.json";
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;
/// Bounded wait for an in-flight refresh after the command has finished.
const REFRESH_GRACE: Duration = Duration::from_millis(300);

/// Last known latest release, persisted so the startup notice never needs
/// a network round trip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateCheckCache {
    pub latest_version: String,
    pub checked_at_unix: u64,
}

/// What this invocation should do about update notices.
#[derive(Debug, PartialEq)]
pub struct CheckPlan {
    /// Newer version known from the cache, to mention after the command.
    pub cached_newer: Option<String>,
    /// Whether the cache is missing or stale and needs a background refresh.
    pub refresh: bool,
}

/// Pure decision logic: what the cache already tells us and whether to
/// refresh it.
pub fn plan_check(
    current_version: &str,
    cache: Option<&UpdateCheckCache>,
    now_unix: u64,
    disabled: bool,
) -> CheckPlan {
    if disabled {
        return CheckPlan {
            cached_newer: None,
            refresh: false,
        };
    }
    let Some(current) = parse_version(current_version) else {
        return CheckPlan {
            cached_newer: None,
            refresh: false,
        };
    };
    let cached_newer = cache.and_then(|cached| {
        let latest = parse_version(&cached.latest_version)?;
        (latest > current).then(|| cached.latest_version.clone())
    });
    let refresh = match cache {
        Some(cached) => now_unix.saturating_sub(cached.checked_at_unix) >= CACHE_TTL_SECS,
        None => true,
    };
    CheckPlan {
        cached_newer,
        refresh,
    }
}

/// The notice to print once the command has run. A fresh fetch wins over
/// the cache so an already-updated binary is never nagged by stale data.
pub fn post_command_notice(
    current_version: &str,
    cached_newer: Option<&str>,
    fetched: Option<&str>,
) -> Option<String> {
    let candidate = fetched.or(cached_newer)?;
    let current = parse_version(current_version)?;
    let latest = parse_version(candidate)?;
    (latest > current).then(|| update_notice(current_version, candidate))
}

/// The notice line shown when a newer release is known.
pub fn update_notice(current_version: &str, latest_version: &str) -> String {
    format!(
        "A newer composer release is available: {} (currently {}). Run 'composer self-update' to install it.",
        latest_version, current_version
    )
}

/// Handle for a check started before the command ran.
pub struct UpdateCheck {
    current_version: &'static str,
    cached_newer: Option<String>,
    pending: Option<mpsc::Receiver<String>>,
}

/// Starts the update check without printing anything or blocking the
/// command. Kicks off a background refresh when the cache is missing or
/// older than a day; all failures are silent.
pub fn start(current_version: &'static str, skip: bool) -> UpdateCheck {
    let disabled = skip
        || std::env::var_os(DISABLE_ENV_VAR).is_some_and(|value| !value.is_empty());
    let cache = cache_path().ok().and_then(|path| read_cache_from(&path));
    let plan = plan_check(current_version, cache.as_ref(), unix_now(), disabled);
    let pending = plan.refresh.then(|| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            if let Ok(release) = GithubReleaseApi::new().latest_release() {
                let _ = tx.send(release.version);
            }
        });
        rx
    });
    UpdateCheck {
        current_version,
        cached_newer: plan.cached_newer,
        pending,
    }
}

/// Prints the update notice, if any, after the command has run. Waits at
/// most a short grace period for an in-flight refresh and persists the
/// result for future runs.
pub fn finish(check: UpdateCheck) {
    let fetched = check
        .pending
        .and_then(|rx| rx.recv_timeout(REFRESH_GRACE).ok());
    if let (Some(fetched), Ok(path)) = (&fetched, cache_path()) {
        let _ = write_cache_to(
            &path,
            &UpdateCheckCache {
                latest_version: fetched.clone(),
                checked_at_unix: unix_now(),
            },
        );
    }
    let notice = post_command_notice(
        check.current_version,
        check.cached_newer.as_deref(),
        fetched.as_deref(),
    );
    if let Some(notice) = notice {
        info!("{}", notice);
    }
}

fn cache_path() -> anyhow::Result<PathBuf> {
    Ok(get_composer_directory()?.join(CACHE_FILE))
}

fn read_cache_from(path: &Path) -> Option<UpdateCheckCache> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Writes via a temp file and rename so a killed process cannot leave a
/// half-written cache behind.
fn write_cache_to(path: &Path, cache: &UpdateCheckCache) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string(cache)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_750_000_000;

    fn fresh_cache(latest: &str) -> UpdateCheckCache {
        UpdateCheckCache {
            latest_version: latest.to_string(),
            checked_at_unix: NOW - 60,
        }
    }

    fn stale_cache(latest: &str) -> UpdateCheckCache {
        UpdateCheckCache {
            latest_version: latest.to_string(),
            checked_at_unix: NOW - CACHE_TTL_SECS - 1,
        }
    }

    #[test]
    fn disabled_check_does_nothing() {
        let plan = plan_check("3.5.3", Some(&fresh_cache("9.9.9")), NOW, true);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: None,
                refresh: false
            }
        );
    }

    #[test]
    fn missing_cache_refreshes_with_nothing_to_report() {
        let plan = plan_check("3.5.3", None, NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: None,
                refresh: true
            }
        );
    }

    #[test]
    fn fresh_cache_with_newer_version_reports_without_refreshing() {
        let plan = plan_check("3.5.3", Some(&fresh_cache("3.6.0")), NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: Some("3.6.0".to_string()),
                refresh: false
            }
        );
    }

    #[test]
    fn fresh_cache_with_same_version_stays_quiet() {
        let plan = plan_check("3.5.3", Some(&fresh_cache("3.5.3")), NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: None,
                refresh: false
            }
        );
    }

    #[test]
    fn stale_cache_with_newer_version_reports_and_refreshes() {
        let plan = plan_check("3.5.3", Some(&stale_cache("3.6.0")), NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: Some("3.6.0".to_string()),
                refresh: true
            }
        );
    }

    #[test]
    fn unparseable_cached_version_still_refreshes() {
        let plan = plan_check("3.5.3", Some(&stale_cache("latest")), NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: None,
                refresh: true
            }
        );
    }

    #[test]
    fn unparseable_current_version_does_nothing() {
        let plan = plan_check("dev", Some(&fresh_cache("3.6.0")), NOW, false);
        assert_eq!(
            plan,
            CheckPlan {
                cached_newer: None,
                refresh: false
            }
        );
    }

    #[test]
    fn fetched_newer_version_produces_a_notice() {
        let notice = post_command_notice("3.5.3", None, Some("3.6.0"));
        assert_eq!(notice, Some(update_notice("3.5.3", "3.6.0")));
    }

    #[test]
    fn fresh_fetch_overrides_stale_cached_claim() {
        // Cache claimed an update but the fetch says we are current
        assert_eq!(post_command_notice("3.6.0", Some("3.6.0"), Some("3.6.0")), None);
        // Fetch found something even newer than the cache knew about
        assert_eq!(
            post_command_notice("3.5.3", Some("3.6.0"), Some("3.7.0")),
            Some(update_notice("3.5.3", "3.7.0"))
        );
    }

    #[test]
    fn cached_newer_version_is_used_when_no_fetch_arrived() {
        let notice = post_command_notice("3.5.3", Some("3.6.0"), None);
        assert_eq!(notice, Some(update_notice("3.5.3", "3.6.0")));
    }

    #[test]
    fn no_information_means_no_notice() {
        assert_eq!(post_command_notice("3.5.3", None, None), None);
        assert_eq!(post_command_notice("3.5.3", None, Some("3.5.3")), None);
        assert_eq!(post_command_notice("3.5.3", None, Some("3.5.2")), None);
        assert_eq!(post_command_notice("3.5.3", None, Some("latest")), None);
    }

    #[test]
    fn notice_names_both_versions_and_the_command() {
        let notice = update_notice("3.5.3", "3.6.0");
        assert!(notice.contains("3.6.0"));
        assert!(notice.contains("3.5.3"));
        assert!(notice.contains("composer self-update"));
    }

    #[test]
    fn cache_round_trips_through_disk() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("nested").join("update_check.json");
        let cache = UpdateCheckCache {
            latest_version: "3.6.0".to_string(),
            checked_at_unix: NOW,
        };
        write_cache_to(&path, &cache)?;
        assert_eq!(read_cache_from(&path), Some(cache));
        Ok(())
    }

    #[test]
    fn missing_or_corrupt_cache_reads_as_none() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("update_check.json");
        assert_eq!(read_cache_from(&path), None);
        std::fs::write(&path, "not json")?;
        assert_eq!(read_cache_from(&path), None);
        Ok(())
    }
}
