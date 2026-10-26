//! Updates from GitHub releases: finds a newer version, downloads the
//! Windows installer or the build for this platform, checks it against the
//! release's SHA-256 sums, and hands over to it once the app has closed.

use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};

use serde::Deserialize;

use crate::{http, platform::hash};

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

fn pending() -> std::sync::MutexGuard<'static, Option<Command>> {
    PENDING.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// An installed copy has Inno Setup's uninstaller beside it; on Linux and
/// macOS every copy is the bare program.
fn kind() -> Kind {
    let installed =
        cfg!(windows) && std::env::current_exe().is_ok_and(|exe| exe.with_file_name("unins000.exe").exists());
    if installed { Kind::Installed } else { Kind::Portable }
}

/// How the name of the build for a copy ends in a release, as in
/// `open-synapse_0.2.0_linux-x86_64`. Windows' names came first, and copies
/// already out there look for them.
fn suffix(kind: Kind) -> String {
    match kind {
        Kind::Installed => "_x64-setup.exe".into(),
        Kind::Portable if cfg!(windows) => "_x64-portable.exe".into(),
        Kind::Portable => format!("_{}-{}", std::env::consts::OS, std::env::consts::ARCH),
    }
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
    let suffix = suffix(kind);
    let assets = &release.assets;
    let asset = assets
        .iter()
        .find(|a| a.name.ends_with(&suffix))
        .ok_or("the release has no build for this copy")?;
    let sums = assets
        .iter()
        .find(|a| a.name == SUMS)
        .ok_or("the release has no checksums")?;
    Ok(Some(Update {
        version: format!("{}.{}.{}", latest.0, latest.1, latest.2),
        kind,
        asset: asset.clone(),
        sums: sums.clone(),
    }))
}

fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then(|| hash.to_ascii_lowercase())
    })
}

/// A digest in lower-case hex, as the sums are written.
fn hex(digest: &[u8]) -> String {
    digest.iter().fold(String::with_capacity(digest.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
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
    let actual = hex(&hash::sha256(&build).map_err(fail)?);
    if actual != expected || build.len() != update.asset.size {
        return Err("the download did not match its checksum".into());
    }
    Ok(build)
}

/// Readies a downloaded update to take over once the app closes: the
/// installer is saved to the temp folder, and a portable build put in place.
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
    *pending() = Some(command);
    Ok(())
}

/// `exe` with `suffix` after its name, as in `open-synapse.exe.old`.
fn beside(exe: &Path, suffix: &str) -> PathBuf {
    let mut name = exe.as_os_str().to_owned();
    name.push(suffix);
    name.into()
}

/// Puts `build` where `exe` is. A running exe can be renamed, though not
/// overwritten, so it moves aside until the next start removes it.
#[cfg(windows)]
fn replace(exe: &Path, build: &[u8]) -> io::Result<()> {
    let (new, old) = (beside(exe, ".new"), beside(exe, ".old"));
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

/// Puts `build` where `exe` is, with its permissions. A program renamed
/// over keeps running from the old file, which goes once it exits.
#[cfg(unix)]
fn replace(exe: &Path, build: &[u8]) -> io::Result<()> {
    use std::{
        io::Write,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };
    let new = beside(exe, ".new");
    let mode = fs::metadata(exe)?.permissions().mode();
    let _ = fs::remove_file(&new);
    let replaced = fs::File::options()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&new)
        // On disk before the rename, so a crash cannot leave half a program.
        .and_then(|mut file| file.write_all(build).and_then(|()| file.sync_all()))
        .and_then(|()| fs::rename(&new, exe));
    if replaced.is_err() {
        let _ = fs::remove_file(&new);
        return replaced;
    }
    // An app's signature covers its program, so the app is signed again,
    // ad hoc, as releases are.
    #[cfg(target_os = "macos")]
    if let Some(app) = exe
        .ancestors()
        .nth(3)
        .filter(|a| a.extension().is_some_and(|e| e == "app"))
    {
        let signed = Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-"])
            .arg(app)
            .status();
        if !signed.is_ok_and(|s| s.success()) {
            log::warn!("could not sign {} again", app.display());
        }
    }
    Ok(())
}

/// Starts a staged update. Called as the app exits, after it has let go of
/// its single-instance lock, which the new copy needs.
pub fn launch_pending() {
    if let Some(mut command) = pending().take()
        && let Err(e) = command.spawn()
    {
        log::warn!("could not start the update: {e}");
    }
}

/// Removes the exe a portable update on Windows replaced, once the copy
/// that ran from it has exited and let go of it, which may take a moment.
pub fn clean_up() {
    let Ok(old) = std::env::current_exe().map(|exe| beside(&exe, ".old")) else {
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

    fn release(tag: &str, names: &[String]) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: names
                .iter()
                .map(|name| Asset {
                    name: name.clone(),
                    browser_download_url: format!("https://github.com/x/{name}"),
                    size: 1,
                })
                .collect(),
        }
    }

    /// This platform's portable build, Windows' installer and the sums.
    fn assets() -> Vec<String> {
        vec![
            format!("open-synapse_0.2.0{}", suffix(Kind::Portable)),
            "open-synapse_0.2.0_x64-setup.exe".into(),
            SUMS.into(),
        ]
    }

    #[test]
    fn reads_versions() {
        assert_eq!(version("v0.1.2"), Some((0, 1, 2)));
        assert_eq!(version("1.10.0"), Some((1, 10, 0)));
        assert_eq!(version("v1.3.0-rc.8"), None);
        assert_eq!(version("v1.2"), None);
        assert_eq!(version("v1.2.3.4"), None);
    }

    #[test]
    fn names_builds_as_releases_do() {
        let portable = suffix(Kind::Portable);
        if cfg!(windows) {
            assert_eq!(portable, "_x64-portable.exe");
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(portable, "_linux-x86_64");
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(portable, "_macos-aarch64");
        }
    }

    #[test]
    fn offers_only_newer_releases() {
        let none = |tag| {
            newer(release(tag, &assets()), "0.1.2", Kind::Portable)
                .unwrap()
                .is_none()
        };
        assert!(none("v0.1.2") && none("v0.1.1"));
        let update = newer(release("v0.2.0", &assets()), "0.1.2", Kind::Portable)
            .unwrap()
            .unwrap();
        assert_eq!(update.version, "0.2.0");
        assert_eq!(update.asset.name, assets()[0]);
        let mut early = release("v0.2.0", &assets());
        early.prerelease = true;
        assert!(newer(early, "0.1.2", Kind::Portable).unwrap().is_none());
    }

    #[test]
    fn picks_the_build_for_this_copy() {
        let update = newer(release("v0.2.0", &assets()), "0.1.2", Kind::Installed)
            .unwrap()
            .unwrap();
        assert_eq!(update.asset.name, "open-synapse_0.2.0_x64-setup.exe");
        assert!(
            newer(release("v0.2.0", &assets()[..2]), "0.1.2", Kind::Installed).is_err(),
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
    fn hashes_the_standard_vectors() {
        let sha256 = |data: &[u8]| hex(&hash::sha256(data).unwrap());
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn replaces_the_exe() {
        let dir = std::env::temp_dir().join(format!("open-synapse-update-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("open-synapse.exe");
        fs::write(&exe, b"old").unwrap();
        replace(&exe, b"new").unwrap();
        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert!(!beside(&exe, ".new").exists());
        // Windows cannot overwrite a running exe, so keeps it aside.
        let old = fs::read(beside(&exe, ".old")).ok();
        assert_eq!(old.as_deref(), cfg!(windows).then_some(&b"old"[..]));
        fs::remove_dir_all(&dir).unwrap();
    }
}
