//! The recompiled library: the game's code compiled once for this device from the player's own
//! executable, instead of translated while playing. bloodborne-recomp's `build_library.py` (in the
//! app's kit, `game/recomp/`) does the work with an LLVM toolchain the launcher downloads, checks
//! against a pinned SHA-256 and deletes after the build. The library, its record
//! (`library.json`) and, while building, the generated C stay in the data folder's `recomp/`: they
//! are the game's code and never leave the device.
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

type Result<T> = std::result::Result<T, String>;

/// LLVM 23.1.4's clang, ld64.lld and clang's headers, trimmed from LLVM's official release by
/// bloodborne-recomp's `tools/recomp/toolchain.sh` (Apache-2.0 with LLVM exception).
const TOOLCHAIN_URL: &str =
    "https://github.com/bloodborne-native/bloodborne-recomp/releases/download/toolchain-llvm-23.1.4/llvm-23.1.4-macos-arm64.tar.xz";
const TOOLCHAIN_SHA256: &str = "7e148b8692fa33ded7e6cba48dc879b8a224a935d05232b0999331d85ed8417c";
const TOOLCHAIN_DIR: &str = "LLVM-23.1.4-macOS-ARM64";
/// What a build needs on disk at its peak: the toolchain (342 MB), the generated C (1.4 GB), the
/// objects (0.4 GB) and the library (0.33 GB), with room to spare.
pub const DISK_NEEDED: u64 = 3_000_000_000;

/// A build in progress: cancelled through `cancel`, its script process killed.
#[derive(Default)]
pub struct Job {
    cancel: Arc<AtomicBool>,
    child: Option<Child>,
    running: bool,
}
pub type Jobs = Arc<Mutex<Job>>;

#[derive(Serialize)]
pub struct Status {
    /// "none", "ready", "stale" (built for another executable or recompiler: `reason` says which)
    /// or "building".
    pub state: &'static str,
    pub reason: Option<&'static str>,
    /// The library's record (library.json): built, seconds, functions, size, compiler.
    pub library: Option<Value>,
    /// The kit the app ships (none: this build has no recompiler, e.g. not on Apple Silicon).
    pub available: bool,
}

pub fn dir(data: &Path) -> PathBuf {
    data.join("recomp")
}

/// The kit's build script: the app's `game/recomp/tools/`, or a checkout's `recomp/tools/recomp/`.
fn script(game_root: &Path) -> Option<PathBuf> {
    [game_root.join("recomp/tools/build_library.py"), game_root.join("recomp/tools/recomp/build_library.py")]
        .into_iter()
        .find(|path| path.is_file())
}

/// The recompiler version the kit builds with (`build_library.py --version`), asked once.
fn kit_version(python: &Path, game_root: &Path) -> Option<String> {
    static VERSION: OnceLock<Option<String>> = OnceLock::new();
    VERSION
        .get_or_init(|| {
            let out = Command::new(python).arg(script(game_root)?).arg("--version").env("PYTHONDONTWRITEBYTECODE", "1").output().ok()?;
            out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
        .clone()
}

/// The executable's SHA-256, remembered while its size and time stay the same.
fn eboot_sha256(game_dir: &Path) -> Option<String> {
    static CACHE: Mutex<Option<(PathBuf, u64, std::time::SystemTime, String)>> = Mutex::new(None);
    let path = game_dir.join("eboot.bin");
    let meta = fs::metadata(&path).ok()?;
    let (len, modified) = (meta.len(), meta.modified().ok()?);
    let mut cache = CACHE.lock().ok()?;
    if let Some((p, l, m, hash)) = cache.as_ref() {
        if *p == path && *l == len && *m == modified {
            return Some(hash.clone());
        }
    }
    let mut digest = Sha256::new();
    let mut file = fs::File::open(&path).ok()?;
    std::io::copy(&mut file, &mut digest).ok()?;
    let hash = format!("{:x}", digest.finalize());
    *cache = Some((path, len, modified, hash.clone()));
    Some(hash)
}

fn record(data: &Path) -> Option<Value> {
    let text = fs::read_to_string(dir(data).join("library.json")).ok()?;
    serde_json::from_str(&text).ok().filter(|_| dir(data).join("librecomp.dylib").is_file())
}

pub fn status(data: &Path, python: &Path, game_root: &Path, game_dir: &Path, jobs: &Jobs) -> Status {
    let available = script(game_root).is_some();
    let library = record(data);
    if jobs.lock().map(|job| job.running).unwrap_or(false) {
        return Status { state: "building", reason: None, library, available };
    }
    let Some(found) = library.clone() else {
        return Status { state: "none", reason: None, library, available };
    };
    let reason = if found["eboot_sha256"].as_str() != eboot_sha256(game_dir).as_deref() {
        Some("game")
    } else if found["recomp"].as_str() != kit_version(python, game_root).as_deref() {
        Some("recompiler")
    } else {
        None
    };
    Status { state: if reason.is_some() { "stale" } else { "ready" }, reason, library, available }
}

/// The library to start the game with: only one built from this executable by this recompiler.
pub fn ready_library(data: &Path, python: &Path, game_root: &Path, game_dir: &Path, jobs: &Jobs) -> Result<PathBuf> {
    match status(data, python, game_root, game_dir, jobs) {
        Status { state: "ready", .. } => Ok(dir(data).join("librecomp.dylib")),
        Status { state: "stale", reason: Some("game"), .. } => Err("it was built from another copy of the game".into()),
        Status { state: "stale", .. } => Err("it was built by another version of the recompiler".into()),
        Status { state: "building", .. } => Err("it is being built".into()),
        _ => Err("it has not been built".into()),
    }
}

fn progress(app: &AppHandle, value: Value) {
    let _ = app.emit("recomp-progress", value);
}

/// The toolchain, downloaded and checked, in `recomp/toolchain/` (kept if a build is retried).
async fn toolchain(app: &AppHandle, root: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
    let unpacked = root.join("toolchain").join(TOOLCHAIN_DIR);
    if unpacked.join("bin/clang").is_file() {
        return Ok(unpacked);
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let archive = root.join("toolchain.tar.xz");
    let client = reqwest::Client::builder().user_agent("bloodborne-launcher").build().map_err(|e| e.to_string())?;
    let mut response = client.get(TOOLCHAIN_URL).send().await.map_err(|e| format!("Downloading the compiler: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Downloading the compiler: {}", response.status()));
    }
    let total = response.content_length().unwrap_or(0);
    let mut file = fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut done = 0u64;
    let mut last = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Downloading the compiler: {e}"))? {
        if cancel.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&archive);
            return Err("cancelled".into());
        }
        digest.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if done - last >= 1 << 20 || done == total {
            last = done;
            progress(app, json!({"step": "download", "done": done, "total": total}));
        }
    }
    drop(file);
    if format!("{:x}", digest.finalize()) != TOOLCHAIN_SHA256 {
        let _ = fs::remove_file(&archive);
        return Err("The downloaded compiler is not the expected file (its checksum differs).".into());
    }
    progress(app, json!({"step": "unpack"}));
    let staging = root.join("toolchain-partial");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    let status = Command::new("tar").arg("-xJf").arg(&archive).arg("-C").arg(&staging).status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Unpacking the compiler failed.".into());
    }
    let _ = fs::remove_dir_all(root.join("toolchain"));
    fs::rename(&staging, root.join("toolchain")).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&archive);
    Ok(unpacked)
}

/// Builds the library: the toolchain, then the kit's script, its progress sent as
/// "recomp-progress" events. The toolchain is deleted once the library is built.
pub async fn build(app: AppHandle, data: PathBuf, python: PathBuf, game_root: PathBuf, game_dir: PathBuf, jobs: Jobs) -> Result<()> {
    let script = script(&game_root).ok_or("This build of the app has no recompiler.")?;
    if !game_dir.join("eboot.bin").is_file() {
        return Err("Select the game folder on Home first.".into());
    }
    let cancel = {
        let mut job = jobs.lock().map_err(|e| e.to_string())?;
        if job.running {
            return Err("The library is already being built.".into());
        }
        job.running = true;
        job.cancel = Arc::new(AtomicBool::new(false));
        job.cancel.clone()
    };
    let result = run(&app, &data, &python, &script, &game_dir, &jobs, &cancel).await;
    if let Ok(mut job) = jobs.lock() {
        job.running = false;
        job.child = None;
    }
    match &result {
        Ok(()) => progress(&app, json!({"step": "done"})),
        Err(e) if cancel.load(Ordering::Relaxed) => progress(&app, json!({"step": "cancelled", "message": e})),
        Err(e) => progress(&app, json!({"step": "error", "message": e})),
    }
    result
}

async fn run(app: &AppHandle, data: &Path, python: &Path, script: &Path, game_dir: &Path, jobs: &Jobs, cancel: &Arc<AtomicBool>) -> Result<()> {
    let root = dir(data);
    let toolchain = toolchain(app, &root, cancel).await?;
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    // Two cores stay free, so the device stays responsive while it builds.
    let jobs_count = std::thread::available_parallelism().map(|n| n.get().saturating_sub(2).max(1)).unwrap_or(4);
    let mut child = Command::new(python)
        .arg(script)
        .args(["--game".as_ref(), game_dir.as_os_str(), "--work".as_ref(), root.as_os_str(), "--toolchain".as_ref(), toolchain.as_os_str()])
        .arg("--jobs")
        .arg(jobs_count.to_string())
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start the build: {e}"))?;
    let stdout = child.stdout.take().ok_or("No output from the build.")?;
    if let Ok(mut job) = jobs.lock() {
        job.child = Some(child);
    }
    let app = app.clone();
    let failure = tauri::async_runtime::spawn_blocking(move || {
        let mut failure = None;
        for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
            let Ok(event) = serde_json::from_str::<Value>(&line) else { continue };
            match event["event"].as_str() {
                Some("step") => progress(&app, json!({"step": event["step"], "functions": event["functions"]})),
                Some("compile") => progress(&app, json!({"step": "compile", "done": event["done"], "total": event["total"]})),
                Some("error") => failure = event["message"].as_str().map(str::to_string),
                _ => {}
            }
        }
        failure
    })
    .await
    .map_err(|e| e.to_string())?;
    let status = {
        let child = jobs.lock().map_err(|e| e.to_string())?.child.take();
        match child {
            Some(mut child) => child.wait().map_err(|e| e.to_string())?,
            None => return Err("cancelled".into()),
        }
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    if !status.success() {
        return Err(failure.unwrap_or_else(|| "The build stopped.".into()));
    }
    // Built: the toolchain is no longer needed (a later build downloads it again).
    let _ = fs::remove_dir_all(root.join("toolchain"));
    Ok(())
}

/// Stops a build: the download stops at its next chunk, the script is killed (what it compiled
/// stays, so the next build continues from there).
pub fn cancel(jobs: &Jobs) -> Result<()> {
    let mut job = jobs.lock().map_err(|e| e.to_string())?;
    job.cancel.store(true, Ordering::Relaxed);
    if let Some(child) = job.child.as_mut() {
        let _ = child.kill();
    }
    Ok(())
}

/// Removes the library, its record, any unfinished build and the toolchain.
pub fn delete(data: &Path, jobs: &Jobs) -> Result<()> {
    if jobs.lock().map(|job| job.running).unwrap_or(false) {
        return Err("Cancel the build first.".into());
    }
    let root = dir(data);
    if root.exists() {
        fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Bytes the library and any unfinished build take now.
pub fn disk_use(data: &Path) -> u64 {
    fn size(path: &Path) -> u64 {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_dir() => fs::read_dir(path).map(|entries| entries.flatten().map(|e| size(&e.path())).sum()).unwrap_or(0),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        }
    }
    size(&dir(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A data folder, a game folder with an eboot.bin and a kit whose script prints a version.
    fn setup(version: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("bb-recomp-test-{}-{version}", std::process::id()));
        let (data, game, kit) = (root.join("data"), root.join("game"), root.join("runtime"));
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(kit.join("recomp/tools")).unwrap();
        fs::create_dir_all(dir(&data)).unwrap();
        fs::write(game.join("eboot.bin"), b"synthetic executable").unwrap();
        fs::write(kit.join("recomp/tools/build_library.py"), format!("print('{version}')\n")).unwrap();
        (data, game, kit)
    }

    fn record(data: &Path, eboot: &str, recomp: &str) {
        fs::write(dir(data).join("librecomp.dylib"), b"library").unwrap();
        fs::write(dir(data).join("library.json"), json!({"eboot_sha256": eboot, "recomp": recomp}).to_string()).unwrap();
    }

    #[test]
    fn the_library_is_used_only_for_its_game_and_recompiler() {
        let (data, game, kit) = setup("v1");
        let python = PathBuf::from("python3");
        let jobs = Jobs::default();
        assert_eq!(status(&data, &python, &kit, &game, &jobs).state, "none");
        assert!(ready_library(&data, &python, &kit, &game, &jobs).is_err());
        let eboot = format!("{:x}", Sha256::digest(b"synthetic executable"));
        record(&data, "another game", "v1");
        let found = status(&data, &python, &kit, &game, &jobs);
        assert_eq!((found.state, found.reason), ("stale", Some("game")));
        record(&data, &eboot, "v0");
        let found = status(&data, &python, &kit, &game, &jobs);
        assert_eq!((found.state, found.reason), ("stale", Some("recompiler")));
        record(&data, &eboot, "v1");
        assert_eq!(status(&data, &python, &kit, &game, &jobs).state, "ready");
        assert_eq!(ready_library(&data, &python, &kit, &game, &jobs).unwrap(), dir(&data).join("librecomp.dylib"));
        // A build in progress: the library is not used until it ends.
        jobs.lock().unwrap().running = true;
        assert_eq!(status(&data, &python, &kit, &game, &jobs).state, "building");
        assert!(delete(&data, &jobs).is_err());
        jobs.lock().unwrap().running = false;
        delete(&data, &jobs).unwrap();
        assert_eq!(status(&data, &python, &kit, &game, &jobs).state, "none");
        fs::remove_dir_all(data.parent().unwrap()).unwrap();
    }
}
