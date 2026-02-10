use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use butterpaper_update_core::{detect_arch, detect_platform, check_for_update, Repo, UpdateChannel};
use semver::Version;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Update(#[from] butterpaper_update_core::UpdateError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn usage() -> ! {
    eprintln!(
        "usage:\n  butterpaper-updater check --channel stable|beta --current <version>\n  butterpaper-updater apply --channel stable|beta --current <version> --parent-pid <pid> [--silent]\n"
    );
    std::process::exit(2);
}

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[cfg(windows)]
fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn download(url: &str, dest: &Path) -> Result<(), Error> {
    let agent = ureq::agent();
    let resp = agent
        .get(url)
        .set("User-Agent", "ButterPaper-Updater")
        .call()
        .map_err(|e| Error::Message(format!("download failed: {e}")))?;

    let mut reader = resp.into_reader();
    let mut out = File::create(dest)?;
    std::io::copy(&mut reader, &mut out)?;
    out.flush()?;
    Ok(())
}

#[cfg(unix)]
fn wait_for_pid(pid: u32) {
    let pid = pid as i32;
    for _ in 0..2400 {
        // ~10 minutes max
        let rc = unsafe { libc::kill(pid, 0) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(windows)]
fn wait_for_pid(pid: u32) {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};

    unsafe {
        let h: HANDLE = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if h == 0 {
            return;
        }
        // Wait up to 10 minutes.
        let wait = WaitForSingleObject(h, 10 * 60 * 1000);
        let _ = CloseHandle(h);
        if wait == WAIT_OBJECT_0 {
            return;
        }
    }
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let macos_dir = exe.parent()?;
    if macos_dir.file_name()?.to_str()? != "MacOS" {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name()?.to_str()? != "Contents" {
        return None;
    }
    let app = contents.parent()?;
    if app.extension()?.to_str()? != "app" {
        return None;
    }
    Some(app.to_path_buf())
}

#[cfg(target_os = "macos")]
fn apply_macos_zip(zip_path: &Path, parent_pid: u32) -> Result<(), Error> {
    wait_for_pid(parent_pid);

    let app_path = current_app_bundle().ok_or_else(|| {
        Error::Message("could not determine current .app bundle path".into())
    })?;
    let app_dir = app_path
        .parent()
        .ok_or_else(|| Error::Message("could not determine app parent directory".into()))?;

    let tmp = tempfile::tempdir().map_err(|e| Error::Message(e.to_string()))?;
    let extract_dir = tmp.path().join("extract");
    std::fs::create_dir_all(&extract_dir)?;

    // Use ditto to preserve resource forks and signatures.
    let status = Command::new("ditto")
        .arg("-x")
        .arg("-k")
        .arg(zip_path)
        .arg(&extract_dir)
        .status()?;
    if !status.success() {
        return Err(Error::Message("failed to extract zip with ditto".into()));
    }

    // Find the first .app in the extracted root.
    let mut new_app: Option<PathBuf> = None;
    for entry in std::fs::read_dir(&extract_dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("app") {
            new_app = Some(p);
            break;
        }
    }
    let new_app = new_app.ok_or_else(|| Error::Message("no .app found in extracted zip".into()))?;

    let backup = app_dir.join(format!(
        "{}.old",
        app_path.file_name().and_then(|s| s.to_str()).unwrap_or("ButterPaper.app")
    ));
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::rename(&app_path, &backup).map_err(|e| {
        Error::Message(format!(
            "failed to move existing app (permission issue?): {e}"
        ))
    })?;
    std::fs::rename(&new_app, &app_path).map_err(|e| {
        // Try to restore the old app if the move fails.
        let _ = std::fs::rename(&backup, &app_path);
        Error::Message(format!("failed to install new app: {e}"))
    })?;

    // Relaunch.
    let _ = Command::new("open").arg(&app_path).status();
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_linux_appimage(appimage_path: &Path, parent_pid: u32) -> Result<(), Error> {
    let target = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Message("APPIMAGE env var not set; not an AppImage install".into()))?;

    wait_for_pid(parent_pid);

    let tmp_target = target.with_extension("AppImage.new");
    if let Some(parent) = tmp_target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(appimage_path, &tmp_target)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&tmp_target)?.permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&tmp_target, perm)?;
    }

    std::fs::rename(&tmp_target, &target).map_err(|e| {
        Error::Message(format!(
            "failed to replace AppImage at {}: {e}",
            target.display()
        ))
    })?;

    // Relaunch the updated AppImage.
    let _ = Command::new(&target).spawn();
    Ok(())
}

#[cfg(windows)]
fn apply_windows_installer(installer: &Path, parent_pid: u32, silent: bool) -> Result<(), Error> {
    wait_for_pid(parent_pid);

    let mut cmd = Command::new(installer);
    if silent {
        cmd.arg("/S");
    }
    cmd.spawn()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let cmd = args[1].as_str();

    let channel_str = parse_flag_value(&args, "--channel").unwrap_or_else(|| "stable".into());
    let channel = match channel_str.as_str() {
        "stable" => UpdateChannel::Stable,
        "beta" => UpdateChannel::Beta,
        _ => usage(),
    };

    let current_str = parse_flag_value(&args, "--current").unwrap_or_else(|| usage());
    let current_version = Version::parse(&current_str).map_err(|e| Error::Message(e.to_string()))?;

    let platform = detect_platform().ok_or(Error::Message("unsupported platform".into()))?;
    let arch = detect_arch().ok_or(Error::Message("unsupported arch".into()))?;
    let repo = Repo::new("apotenza92", "ButterPaper");

    match cmd {
        "check" => {
            let update = check_for_update(repo, channel, platform, arch, &current_version)?;
            if let Some(update) = update {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "tag": update.tag_name,
                        "version": update.version.to_string(),
                        "asset": update.asset_name,
                        "url": update.download_url,
                    }))?
                );
            } else {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({"up_to_date": true}))?);
            }
            Ok(())
        }
        "apply" => {
            let parent_pid: u32 = parse_flag_value(&args, "--parent-pid")
                .ok_or_else(|| Error::Message("--parent-pid required".into()))?
                .parse()
                .map_err(|_| Error::Message("invalid --parent-pid".into()))?;

            let update = check_for_update(repo, channel, platform, arch, &current_version)?;
            let Some(update) = update else {
                // Nothing to do.
                return Ok(());
            };

            let tmp = tempfile::tempdir().map_err(|e| Error::Message(e.to_string()))?;
            let download_path = tmp.path().join(&update.asset_name);
            download(&update.download_url, &download_path)?;

            #[cfg(target_os = "macos")]
            {
                apply_macos_zip(&download_path, parent_pid)?;
                return Ok(());
            }

            #[cfg(target_os = "linux")]
            {
                if let Err(err) = apply_linux_appimage(&download_path, parent_pid) {
                    eprintln!("Linux self-update failed ({err}); opening download in browser...");
                    let _ = Command::new("xdg-open").arg(&update.download_url).spawn();
                }
                return Ok(());
            }

            #[cfg(windows)]
            {
                let silent = has_flag(&args, "--silent");
                apply_windows_installer(&download_path, parent_pid, silent)?;
                return Ok(());
            }

            #[allow(unreachable_code)]
            return Err(Box::new(Error::Message(
                "apply is not supported on this platform".into(),
            )));
        }
        _ => usage(),
    }
}
