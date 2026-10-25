//! Updates from GitHub releases: finds a newer version, downloads the
//! installer or the portable exe, checks it against the release's SHA-256
//! sums, and hands over to it once the app has closed.

use std::{fs, io, path::Path, process::Command, sync::Mutex, time::Duration};

use serde::Deserialize;

use crate::win::{hash, http};

const LATEST: &str = "https://api.github.com/repos/Visxoz/open-synapse/releases/latest";
/// GitHub's API and the hosts its release downloads redirect to.
const HOSTS: &[&str] = &["api.github.com", "github.com", "githubusercontent.com"];
const SUMS: &str = "SHA256SUMS";
/// The release data and its sums are a few kilobytes; a build about 20 MB.
const MAX_INFO: usize = 1 << 20;
const MAX_BUILD: usize = 64 << 20;
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// How this copy was installed, which decides what an update replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Installed,
    Portable,
}

#[derive(Debug, Clone, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: usize,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Debug, Clone)]
pub struct Update {
    pub version: String,
    kind: Kind,
    asset: Asset,
    sums: Asset,
}

/// The command that takes over once the app has closed.
static PENDING: Mutex<Option<Command>> = Mutex::new(None);

/// An installed copy has Inno Setup's uninstaller beside it.
fn kind() -> Kind {
    let installed = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("unins000.exe")))
        .is_some_and(|uninstaller| uninstaller.exists());
    if installed { Kind::Installed } else { Kind::Portable }
}

/// `x.y.z`, with or without a leading `v`; anything else, such as a
/// pre-release suffix, is no version to update to.
fn version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
    let mut next = || parts.next()?.parse().ok();
    let version = (next()?, next()?, next()?);
    parts.next().is_none().then_some(version)
}

/// The latest release, if newer than this build. Fails while the repo is
/// private, as GitHub then answers anonymous requests with not found.
pub fn check() -> Result<Option<Update>, String> {
    let body = http::get_from(LATEST, MAX_INFO, HOSTS, &mut |_| {})
        .map_err(|e| format!("could not check for updates: {e}"))?;
    let release = serde_json::from_slice(&body).map_err(|e| format!("unexpected release data: {e}"))?;
    newer(release, CURRENT, kind())
}

fn newer(release: Release, current: &str, kind: Kind) -> Result<Option<Update>, String> {
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let (Some(latest), Some(current)) = (version(&release.tag_name), version(current)) else {
        return Ok(None);
    };
    if latest <= current {
        return Ok(None);
    }
    let suffix = match kind {
        Kind::Installed => "_x64-setup.exe",
        Kind::Portable => "_x64-portable.exe",
    };
    let find = |matches: &dyn Fn(&str) -> bool| release.assets.iter().find(|a| matches(&a.name)).cloned();
    let asset = find(&|name| name.ends_with(suffix)).ok_or("the release has no build for this copy")?;
    let sums = find(&|name| name == SUMS).ok_or("the release has no checksums")?;
    Ok(Some(Update {
        version: format!("{}.{}.{}", latest.0, latest.1, latest.2),
        kind,
        asset,
        sums,
    }))
}

fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then(|| hash.to_ascii_lowercase())
    })
}

/// Downloads the update and checks it against the release's checksum and
/// size before anything is written. `progress` hears the fraction done.
pub fn download(update: &Update, mut progress: impl FnMut(f32)) -> Result<Vec<u8>, String> {
    let fail = |e: io::Error| format!("the download failed: {e}");
    let sums = http::get_from(&update.sums.browser_download_url, MAX_INFO, HOSTS, &mut |_| {}).map_err(fail)?;
    let expected = expected_hash(&String::from_utf8_lossy(&sums), &update.asset.name)
        .ok_or("the release has no checksum for it")?;
    let total = update.asset.size.max(1) as f32;
    let build = http::get_from(&update.asset.browser_download_url, MAX_BUILD, HOSTS, &mut |done| {
        progress((done as f32 / total).min(1.))
    })
    .map_err(fail)?;
    let actual = hash::sha256(&build).map_err(fail)?;
    if actual != expected || build.len() != update.asset.size {
        return Err("the download did not match its checksum".into());
    }
    Ok(build)
}

/// Readies a downloaded update to take over once the app closes: the
/// installer is saved to the temp folder, and a portable exe put in place.
pub fn stage(update: &Update, build: &[u8]) -> Result<(), String> {
    let fail = |e: io::Error| format!("could not install the update: {e}");
    let command = match update.kind {
        Kind::Installed => {
            let installer = std::env::temp_dir().join(&update.asset.name);
            fs::write(&installer, build).map_err(fail)?;
            let mut command = Command::new(installer);
            command.args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/relaunch=1"]);
            command
        }
        Kind::Portable => {
            let exe = std::env::current_exe().map_err(fail)?;
            replace(&exe, build).map_err(fail)?;
            Command::new(exe)
        }
    };
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(command);
    Ok(())
}

/// Puts `build` where `exe` is. A running exe can be renamed, though not
/// overwritten, so it moves aside until the next start removes it.
fn replace(exe: &Path, build: &[u8]) -> io::Result<()> {
    let (new, old) = (exe.with_extension("exe.new"), exe.with_extension("exe.old"));
    fs::write(&new, build)?;
    let _ = fs::remove_file(&old);
    if let Err(e) = fs::rename(exe, &old) {
        let _ = fs::remove_file(&new);
        return Err(e);
    }
    fs::rename(&new, exe).inspect_err(|_| {
        let _ = fs::rename(&old, exe);
    })
}

/// Starts a staged update. Called as the app exits, after it has let go of
/// its single-instance lock, which the new copy needs.
pub fn launch_pending() {
    if let Some(mut command) = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take()
        && let Err(e) = command.spawn()
    {
        log::warn!("could not start the update: {e}");
    }
}

/// Removes the exe a portable update replaced, once the copy that ran from
/// it has exited and let go of it, which may take a moment.
pub fn clean_up() {
    let Ok(old) = std::env::current_exe().map(|exe| exe.with_extension("exe.old")) else {
        return;
    };
    if !old.exists() {
        return;
    }
    let _ = std::thread::Builder::new().name("clean-up".into()).spawn(move || {
        for _ in 0..20 {
            if fs::remove_file(&old).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        log::warn!("could not remove {}", old.display());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, names: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: names
                .iter()
                .map(|name| Asset {
                    name: (*name).into(),
                    browser_download_url: format!("https://github.com/x/{name}"),
                    size: 1,
                })
                .collect(),
        }
    }

    const ASSETS: &[&str] = &[
        "open-synapse_0.2.0_x64-portable.exe",
        "open-synapse_0.2.0_x64-setup.exe",
        "SHA256SUMS",
    ];

    #[test]
    fn reads_versions() {
        assert_eq!(version("v0.1.2"), Some((0, 1, 2)));
        assert_eq!(version("1.10.0"), Some((1, 10, 0)));
        assert_eq!(version("v1.3.0-rc.8"), None);
        assert_eq!(version("v1.2"), None);
        assert_eq!(version("v1.2.3.4"), None);
    }

    #[test]
    fn offers_only_newer_releases() {
        let none = |tag| newer(release(tag, ASSETS), "0.1.2", Kind::Portable).unwrap().is_none();
        assert!(none("v0.1.2") && none("v0.1.1"));
        let update = newer(release("v0.2.0", ASSETS), "0.1.2", Kind::Portable)
            .unwrap()
            .unwrap();
        assert_eq!(update.version, "0.2.0");
        assert_eq!(update.asset.name, "open-synapse_0.2.0_x64-portable.exe");
        let mut early = release("v0.2.0", ASSETS);
        early.prerelease = true;
        assert!(newer(early, "0.1.2", Kind::Portable).unwrap().is_none());
    }

    #[test]
    fn picks_the_build_for_this_copy() {
        let update = newer(release("v0.2.0", ASSETS), "0.1.2", Kind::Installed)
            .unwrap()
            .unwrap();
        assert_eq!(update.asset.name, "open-synapse_0.2.0_x64-setup.exe");
        assert!(
            newer(release("v0.2.0", &ASSETS[..2]), "0.1.2", Kind::Installed).is_err(),
            "no checksums, no update"
        );
    }

    #[test]
    fn finds_the_checksum() {
        let sums = "ABC123  open-synapse_0.2.0_x64-setup.exe\ndef456 *open-synapse_0.2.0_x64-portable.exe\n";
        assert_eq!(
            expected_hash(sums, "open-synapse_0.2.0_x64-setup.exe").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            expected_hash(sums, "open-synapse_0.2.0_x64-portable.exe").as_deref(),
            Some("def456")
        );
        assert_eq!(expected_hash(sums, "other.exe"), None);
    }

    #[test]
    fn replaces_the_exe_and_keeps_the_old_one_aside() {
        let dir = std::env::temp_dir().join(format!("open-synapse-update-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("open-synapse.exe");
        fs::write(&exe, b"old").unwrap();
        replace(&exe, b"new").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert_eq!(fs::read(exe.with_extension("exe.old")).unwrap(), b"old");
        assert!(!exe.with_extension("exe.new").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
