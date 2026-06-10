use anyhow::{anyhow, Context};
use std::fs::File;
use std::io;
use std::path::Path;

const RELEASES_URL: &str = "https://github.com/sam-ruff/composer-rust/releases";
const LATEST_RELEASE_API_URL: &str =
    "https://api.github.com/repos/sam-ruff/composer-rust/releases/latest";

/// A published composer release.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseInfo {
    pub version: String,
    pub assets: Vec<ReleaseAsset>,
}

/// A downloadable file attached to a release.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_url: String,
}

/// Fetches release metadata and downloads release assets.
#[cfg_attr(test, mockall::automock)]
pub trait ReleaseApi {
    fn latest_release(&self) -> anyhow::Result<ReleaseInfo>;
    fn download(&self, url: &str, dest: &Path) -> anyhow::Result<()>;
}

/// Swaps the currently running executable for a staged replacement.
#[cfg_attr(test, mockall::automock)]
pub trait BinaryInstaller {
    fn install(&self, staged: &Path) -> anyhow::Result<()>;
}

#[derive(Debug, PartialEq)]
pub enum SelfUpdateOutcome {
    UpToDate { current: String },
    UpdateAvailable { current: String, latest: String },
    Updated { from: String, to: String },
}

/// Decides whether an update is needed and drives the download and install.
/// Pure orchestration over the injected API and installer so it can be unit
/// tested with the network mocked.
pub fn run_self_update(
    api: &dyn ReleaseApi,
    installer: &dyn BinaryInstaller,
    current_version: &str,
    asset_target: &str,
    staging_path: &Path,
    check_only: bool,
) -> anyhow::Result<SelfUpdateOutcome> {
    let release = api
        .latest_release()
        .context("Failed to fetch the latest release")?;
    let current = parse_version(current_version)
        .ok_or_else(|| anyhow!("Could not parse the current version '{}'", current_version))?;
    let latest = parse_version(&release.version).ok_or_else(|| {
        anyhow!(
            "Could not parse the latest release version '{}'",
            release.version
        )
    })?;

    if latest <= current {
        return Ok(SelfUpdateOutcome::UpToDate {
            current: current_version.to_string(),
        });
    }
    if check_only {
        return Ok(SelfUpdateOutcome::UpdateAvailable {
            current: current_version.to_string(),
            latest: release.version,
        });
    }
    let asset = select_asset(&release.assets, asset_target).ok_or_else(|| {
        anyhow!(
            "Release {} has no binary for this platform (no asset matching '{}'). Download manually from {}",
            release.version,
            asset_target,
            RELEASES_URL
        )
    })?;
    api.download(&asset.download_url, staging_path)
        .with_context(|| format!("Failed to download '{}'", asset.name))?;
    installer
        .install(staging_path)
        .context("Failed to replace the current composer binary")?;
    Ok(SelfUpdateOutcome::Updated {
        from: current_version.to_string(),
        to: release.version,
    })
}

/// Parses an `x.y.z` version, tolerating a leading 'v'.
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let version = version.trim().trim_start_matches('v');
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Picks the raw binary asset for the given target triple, ignoring packaged
/// artefacts such as the RPM and deb.
fn select_asset<'a>(assets: &'a [ReleaseAsset], target: &str) -> Option<&'a ReleaseAsset> {
    assets.iter().find(|asset| {
        asset.name.contains(target)
            && !asset.name.ends_with(".rpm")
            && !asset.name.ends_with(".deb")
    })
}

/// The asset target triple for the running platform, mirroring the targets
/// produced by the release build matrix.
pub fn current_asset_target() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64", target_env = "musl")) {
        Some("x86_64-unknown-linux-musl")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// Manual-download URL shown when self-update cannot run on this platform.
pub fn releases_page() -> &'static str {
    RELEASES_URL
}

/// Real implementation backed by the GitHub releases API.
pub struct GithubReleaseApi {
    latest_release_url: String,
}

impl GithubReleaseApi {
    pub fn new() -> Self {
        Self {
            latest_release_url: LATEST_RELEASE_API_URL.to_string(),
        }
    }
}

impl Default for GithubReleaseApi {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseApi for GithubReleaseApi {
    fn latest_release(&self) -> anyhow::Result<ReleaseInfo> {
        let mut response = ureq::get(&self.latest_release_url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "composer-self-update")
            .call()
            .context("Failed to contact GitHub for the latest release")?;
        let body = response
            .body_mut()
            .read_to_string()
            .context("Failed to read the GitHub release response")?;
        let json: serde_json::Value =
            serde_json::from_str(&body).context("Failed to parse the GitHub release response")?;
        let version = json
            .get("tag_name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("GitHub release response has no tag_name"))?
            .to_string();
        let assets = json
            .get("assets")
            .and_then(|value| value.as_array())
            .map(|assets| {
                assets
                    .iter()
                    .filter_map(|asset| {
                        Some(ReleaseAsset {
                            name: asset.get("name")?.as_str()?.to_string(),
                            download_url: asset.get("browser_download_url")?.as_str()?.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(ReleaseInfo { version, assets })
    }

    fn download(&self, url: &str, dest: &Path) -> anyhow::Result<()> {
        let response = ureq::get(url)
            .header("User-Agent", "composer-self-update")
            .call()
            .with_context(|| format!("Failed to download {}", url))?;
        let mut reader = response.into_body().into_reader();
        let mut file = File::create(dest)
            .with_context(|| format!("Failed to create staging file '{}'", dest.display()))?;
        io::copy(&mut reader, &mut file)
            .context("Failed to write the downloaded binary to disk")?;
        Ok(())
    }
}

/// Real installer that swaps the running executable in place.
pub struct SelfReplaceInstaller;

impl BinaryInstaller for SelfReplaceInstaller {
    fn install(&self, staged: &Path) -> anyhow::Result<()> {
        make_executable(staged)?;
        self_replace::self_replace(staged)
            .context("Failed to replace the running composer executable")?;
        Ok(())
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .with_context(|| format!("Failed to read staging file '{}'", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)
        .with_context(|| format!("Failed to mark '{}' as executable", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn gnu_asset(version: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: format!("composer-{}-ubuntu-latest-x86_64-unknown-linux-gnu", version),
            download_url: format!("https://example.com/{}/gnu", version),
        }
    }

    fn full_asset_list(version: &str) -> Vec<ReleaseAsset> {
        // Mirrors the asset names of a real release
        let names = [
            format!("composer-{}-1.x86_64.rpm", version),
            format!("composer_{}-1_amd64.deb", version),
            format!("composer-{}-macos-latest-x86_64-apple-darwin", version),
            format!("composer-{}-ubuntu-latest-x86_64-unknown-linux-gnu", version),
            format!("composer-{}-ubuntu-latest-x86_64-unknown-linux-musl", version),
            format!("composer-{}-windows-latest-x86_64-pc-windows-msvc.exe", version),
        ];
        names
            .iter()
            .map(|name| ReleaseAsset {
                name: name.clone(),
                download_url: format!("https://example.com/{}", name),
            })
            .collect()
    }

    fn staging_path() -> PathBuf {
        PathBuf::from("/tmp/composer-staging-test")
    }

    #[test]
    fn up_to_date_release_skips_download_and_install() -> anyhow::Result<()> {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.5.3".to_string(),
                assets: vec![gnu_asset("3.5.3")],
            })
        });
        // No expectations on the installer: any call would fail the test
        let installer = MockBinaryInstaller::new();

        let outcome = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )?;
        assert_eq!(
            outcome,
            SelfUpdateOutcome::UpToDate {
                current: "3.5.3".to_string()
            }
        );
        Ok(())
    }

    #[test]
    fn older_remote_release_is_treated_as_up_to_date() -> anyhow::Result<()> {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.5.2".to_string(),
                assets: vec![gnu_asset("3.5.2")],
            })
        });
        let installer = MockBinaryInstaller::new();

        let outcome = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )?;
        assert_eq!(
            outcome,
            SelfUpdateOutcome::UpToDate {
                current: "3.5.3".to_string()
            }
        );
        Ok(())
    }

    #[test]
    fn newer_release_downloads_matching_asset_and_installs() -> anyhow::Result<()> {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.6.0".to_string(),
                assets: full_asset_list("3.6.0"),
            })
        });
        api.expect_download()
            .withf(|url, dest| {
                url == "https://example.com/composer-3.6.0-ubuntu-latest-x86_64-unknown-linux-gnu"
                    && dest == staging_path()
            })
            .times(1)
            .returning(|_, _| Ok(()));
        let mut installer = MockBinaryInstaller::new();
        installer
            .expect_install()
            .withf(|staged| staged == staging_path())
            .times(1)
            .returning(|_| Ok(()));

        let outcome = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )?;
        assert_eq!(
            outcome,
            SelfUpdateOutcome::Updated {
                from: "3.5.3".to_string(),
                to: "3.6.0".to_string()
            }
        );
        Ok(())
    }

    #[test]
    fn check_only_reports_available_update_without_installing() -> anyhow::Result<()> {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.6.0".to_string(),
                assets: full_asset_list("3.6.0"),
            })
        });
        // No download or install expectations: any call would fail the test
        let installer = MockBinaryInstaller::new();

        let outcome = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            true,
        )?;
        assert_eq!(
            outcome,
            SelfUpdateOutcome::UpdateAvailable {
                current: "3.5.3".to_string(),
                latest: "3.6.0".to_string()
            }
        );
        Ok(())
    }

    #[test]
    fn fetch_error_propagates_with_context() {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release()
            .times(1)
            .returning(|| Err(anyhow!("network down")));
        let installer = MockBinaryInstaller::new();

        let err = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )
        .unwrap_err();
        assert!(
            format!("{:#}", err).contains("Failed to fetch the latest release"),
            "Should add fetch context: {:#}",
            err
        );
    }

    #[test]
    fn download_error_propagates_and_skips_install() {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.6.0".to_string(),
                assets: full_asset_list("3.6.0"),
            })
        });
        api.expect_download()
            .times(1)
            .returning(|_, _| Err(anyhow!("connection reset")));
        // Install must not be attempted after a failed download
        let installer = MockBinaryInstaller::new();

        let err = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )
        .unwrap_err();
        assert!(
            format!("{:#}", err).contains("Failed to download"),
            "Should add download context: {:#}",
            err
        );
    }

    #[test]
    fn missing_platform_asset_names_the_target() {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "3.6.0".to_string(),
                assets: vec![ReleaseAsset {
                    name: "composer-3.6.0-windows-latest-x86_64-pc-windows-msvc.exe".to_string(),
                    download_url: "https://example.com/windows".to_string(),
                }],
            })
        });
        let installer = MockBinaryInstaller::new();

        let err = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )
        .unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("x86_64-unknown-linux-gnu"),
            "Should name the missing target: {}",
            message
        );
    }

    #[test]
    fn unparseable_release_version_errors() {
        let mut api = MockReleaseApi::new();
        api.expect_latest_release().times(1).returning(|| {
            Ok(ReleaseInfo {
                version: "latest".to_string(),
                assets: vec![],
            })
        });
        let installer = MockBinaryInstaller::new();

        let err = run_self_update(
            &api,
            &installer,
            "3.5.3",
            "x86_64-unknown-linux-gnu",
            &staging_path(),
            false,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("latest"),
            "Should quote the bad version: {}",
            err
        );
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("3.5.3"), Some((3, 5, 3)));
        assert_eq!(parse_version("v3.5.3"), Some((3, 5, 3)));
        assert_eq!(parse_version("0.0.0"), Some((0, 0, 0)));
        assert_eq!(parse_version("3.5"), None);
        assert_eq!(parse_version("3.5.3.1"), None);
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version("3.5.x"), None);
    }

    #[test]
    fn test_select_asset_per_target() {
        let assets = full_asset_list("3.6.0");
        let gnu = select_asset(&assets, "x86_64-unknown-linux-gnu").expect("gnu asset");
        assert!(gnu.name.ends_with("x86_64-unknown-linux-gnu"));
        let musl = select_asset(&assets, "x86_64-unknown-linux-musl").expect("musl asset");
        assert!(musl.name.ends_with("x86_64-unknown-linux-musl"));
        let windows = select_asset(&assets, "x86_64-pc-windows-msvc").expect("windows asset");
        assert!(windows.name.ends_with(".exe"));
        let mac = select_asset(&assets, "x86_64-apple-darwin").expect("mac asset");
        assert!(mac.name.contains("apple-darwin"));
        assert_eq!(select_asset(&assets, "aarch64-unknown-linux-gnu"), None);
    }

    #[test]
    fn test_select_asset_skips_packaged_artefacts() {
        let assets = vec![ReleaseAsset {
            name: "composer-x86_64-unknown-linux-gnu.rpm".to_string(),
            download_url: "https://example.com/rpm".to_string(),
        }];
        assert_eq!(select_asset(&assets, "x86_64-unknown-linux-gnu"), None);
    }
}
