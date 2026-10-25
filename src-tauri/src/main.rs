// SPDX-License-Identifier: GPL-2.0-or-later
//! bloodborne_mac launcher: the Tauri backend behind the React interface (src/).
//!
//! It keeps two files in the data directory (~/Library/Application Support/bloodborne_mac):
//! launcher.json (start-up options passed to run.sh as environment variables) and bbport.ini
//! (the port's settings, shared with the in-game menu; lines it does not change are kept).
//! Mods, third-party patches and the game check come from scripts/launcher_api.py (the same code
//! run.sh uses), controllers from bb-gpu-capabilities (--gamepads, --read-input). The game runs
//! as `bash run.sh` with the app's own bash and Python; its output is sent to the window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod saves;
mod character_growth;
mod save_editor;
mod profiles;
mod trophies;
mod trophy_catalog;
mod database;
mod tray;
mod game_lifecycle;
mod updates;
mod settings_store;
mod online;

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Where the launcher finds the game's programs and scripts.
#[derive(Clone, Serialize)]
struct Paths {
    /// run.sh, scripts/, patches/ (Contents/Resources/game, or a source checkout).
    game_root: PathBuf,
    bash: PathBuf,
    python: PathBuf,
    probe: PathBuf,
    capabilities: PathBuf,
    data: PathBuf,
    /// Packaged in an app bundle (else a development run from a checkout).
    bundled: bool,
}

impl Paths {
    fn find(data: PathBuf) -> Paths {
        let exe = std::env::current_exe().unwrap_or_default();
        let macos = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        let resources = macos.parent().map(|c| c.join("Resources")).unwrap_or_default();
        if resources.join("game/run.sh").is_file() {
            return Paths {
                game_root: resources.join("game"),
                bash: macos.join("bash"),
                python: resources.join("python/bin/python3"),
                probe: macos.join("bb-probe"),
                capabilities: macos.join("bb-gpu-capabilities"),
                data,
                bundled: true,
            };
        }
        // Development (npm run tauri dev): the checkout's build (the runtime/ submodule,
        // bloodborne-runtime, or BB_REPO), Homebrew's bash, any Python 3.
        let root = std::env::var_os("BB_REPO").map(PathBuf::from).unwrap_or_else(|| {
            let mut dir = macos.clone();
            loop {
                if dir.join("runtime/run.sh").is_file() {
                    return dir.join("runtime");
                }
                if dir.join("run.sh").is_file() || !dir.pop() {
                    return dir;
                }
            }
        });
        let bash = ["/opt/homebrew/bin/bash", "/usr/local/bin/bash"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file())
            .unwrap_or_else(|| PathBuf::from("/bin/bash"));
        Paths {
            probe: root.join("out/bb-probe"),
            capabilities: root.join("out/bb-gpu-capabilities"),
            game_root: root,
            bash,
            python: PathBuf::from("python3"),
            data,
            bundled: false,
        }
    }
}

fn dirs_data() -> PathBuf {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// QA can select synthetic data without changing HOME or touching player files.
fn launcher_data_dir(override_path: Option<PathBuf>) -> Result<PathBuf, String> {
    match override_path {
        Some(path) if path.is_absolute() => Ok(path),
        Some(_) => Err("BB_LAUNCHER_DATA_DIR must be an absolute path.".into()),
        None => Ok(dirs_data().join("bloodborne_mac")),
    }
}

/// Start-up options (launcher.json) and their defaults; unknown keys are kept.
fn default_settings() -> Map<String, Value> {
    let v = json!({
        "game_dir": "",
        "user_dir": "",
        "save_profile": "original",
        "mods_dir": "",
        "mods_enabled": true,
        "patches_dir": "",
        "language": "1",
        "fullscreen": false,
        "display": "",
        "hdr": false,
        "present_mode": "Fifo",
        "gamepad": "",
        "gamepad_name": "",
        "fps_mode": "uncap",
        "fps_limit": 0,
        "draw_pipe": "",
        "readbacks": "",
        "preupload": "",
        "frame_stats": true,
        "save_log": false,
        "hide_launcher": true,
        "trophy_notifications": true,
        "metal_hud": false,
        "renderer": "metal",
        "check_updates": true,
        "crash_diag": false,
        "gpu_profile": false,
        "extra_env": "",
        "ui_language": ""
    });
    let mut settings = v.as_object().cloned().unwrap_or_default();
    settings.extend(online::defaults().as_object().cloned().unwrap_or_default());
    settings
}

/// bbport.ini values the launcher shows before the file has them (macOS defaults).
fn default_ini() -> BTreeMap<String, String> {
    [
        ("upscaler", "off"),
        ("preset", "2"),
        ("sharpen", "1"),
        ("sharpness", "0.50"),
        ("object_motion", "0"),
        ("show_fps", "1"),
        ("fps_size", "small"),
        ("fps_position", "top_center"),
        ("output_res", "1920x1080"),
        ("aspect_ratio", "16:9"),
        ("model_lod", "0"),
        ("live_resolution", "0"),
        ("effect_chromatic_aberration", "1"),
        ("effect_dof", "1"),
        ("effect_motion_blur", "1"),
        ("effect_ssao", "1"),
        ("effect_game_aa", "1"),
        ("effect_dynamic_shadows", "1"),
        ("effect_ssr", "0"),
        ("effect_bloom", "1"),
        ("skip_intro", "0"),
        ("debug_camera", "0"),
        ("debug_menu", "0"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

struct Game {
    child: Option<Child>,
    /// The game's input: settings changed in the launcher while it runs ("BBSET key=value").
    input: Option<std::process::ChildStdin>,
}

#[derive(Clone)]
struct AppState {
    paths: Paths,
    game: Arc<Mutex<Game>>,
}

fn settings_file(p: &Paths) -> PathBuf {
    p.data.join("launcher.json")
}
fn ini_file(p: &Paths) -> PathBuf {
    p.data.join("bbport.ini")
}

/// The launcher's options: defaults, then the database (launcher.json if it cannot be read).
fn load_settings(p: &Paths) -> Map<String, Value> {
    let mut settings = default_settings();
    match settings_store::load(&p.data, settings_store::LAUNCHER) {
        Ok(stored) => {
            for (key, value) in stored {
                settings.insert(key, serde_json::from_str(&value).unwrap_or(Value::String(value)));
            }
        }
        Err(error) => {
            eprintln!("Settings: database unavailable ({error}); reading launcher.json");
            if let Ok(text) = fs::read_to_string(settings_file(p)) {
                if let Ok(Value::Object(saved)) = serde_json::from_str::<Value>(&text) {
                    settings.extend(saved);
                }
            }
        }
    }
    settings
}

fn read_ini_lines(p: &Paths) -> Vec<String> {
    fs::read_to_string(ini_file(p))
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn ini_key(line: &str) -> Option<&str> {
    if line.trim_start().starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(k, _)| k.trim())
}

/// The game's settings: defaults, then the database (bbport.ini if it cannot be read).
fn load_ini(p: &Paths) -> BTreeMap<String, String> {
    let mut values = default_ini();
    match settings_store::load(&p.data, settings_store::GAME) {
        Ok(stored) => {
            values.extend(stored);
            return values;
        }
        Err(error) => eprintln!("Settings: database unavailable ({error}); reading bbport.ini"),
    }
    for line in read_ini_lines(p) {
        if let Some((k, v)) = line.split_once('=') {
            if !k.trim_start().starts_with('#') {
                values.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    values
}

fn str_setting(s: &Map<String, Value>, key: &str) -> String {
    match s.get(key) {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => (if *b { "1" } else { "0" }).to_string(),
        _ => String::new(),
    }
}

fn bool_setting(s: &Map<String, Value>, key: &str) -> bool {
    matches!(s.get(key), Some(Value::Bool(true)))
}

fn expand(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn mods_dir(p: &Paths, s: &Map<String, Value>) -> PathBuf {
    let dir = str_setting(s, "mods_dir");
    if dir.is_empty() { p.data.join("mods") } else { expand(&dir) }
}

fn patches_dir(p: &Paths, s: &Map<String, Value>) -> PathBuf {
    let dir = str_setting(s, "patches_dir");
    if dir.is_empty() { p.data.join("patches") } else { expand(&dir) }
}

/// scripts/launcher_api.py with the app's Python.
fn api(p: &Paths, args: &[&str]) -> Result<Value, String> {
    let out = Command::new(&p.python)
        .arg(p.game_root.join("scripts/launcher_api.py"))
        .args(args)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .map_err(|e| format!("Python: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

async fn blocking<T: Send + 'static>(operation: impl FnOnce() -> Result<T,String> + Send + 'static) -> Result<T,String> {
    tauri::async_runtime::spawn_blocking(operation).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn database_path(state: State<'_, AppState>) -> Result<String,String> {
    let data=state.paths.data.clone();
    blocking(move || { database::block(async { database::connect(&data).await?; Ok(()) })?; Ok(database::path(&data).to_string_lossy().into()) }).await
}

#[tauri::command]
fn set_tray_labels(app: AppHandle, labels: tray::Labels) -> Result<(), String> {
    tray::set_labels(&app, labels).map_err(|error| error.to_string())
}

// --- commands --------------------------------------------------------------------------------

#[tauri::command]
async fn load_config(state: State<'_ , AppState>) -> Result<Value, String> {
    let state=state.inner().clone();
    blocking(move || {
    let p = &state.paths;
    let s = load_settings(p);
    Ok(json!({
        "settings": s,
        "ini": load_ini(p),
        "paths": {
            "data": p.data,
            "mods": mods_dir(p, &s),
            "patches": patches_dir(p, &s),
            "logs": p.data.join("logs"),
            "saves": selected_user_dir(p).unwrap_or_default(),
        },
        "bundled": p.bundled,
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "memory_mb": system_memory_mb(),
        "running": state.game.lock().map(|g| g.child.is_some()).unwrap_or(false),
    }))
    }).await
}

/// The computer's memory in MiB (0 when unknown), for the Graphics page's memory estimate.
fn system_memory_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        Command::new("/usr/sbin/sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|t| t.trim().parse::<u64>().ok())
            .map_or(0, |bytes| bytes >> 20)
    }
    #[cfg(target_os = "linux")]
    {
        fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|t| t.lines().find_map(|l| l.strip_prefix("MemTotal:").map(str::to_string)))
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .map_or(0, |kib| kib >> 10)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        0
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<AppState>, settings: Map<String, Value>) -> Result<(), String> {
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if settings.contains_key("save_profile") || settings.contains_key("user_dir") { return Err("Manage profile ownership in the Profiles tab.".into()); }
    // The lock serializes settings writes with profile changes and launch.
    let mut merged = load_settings(&state.paths);
    if settings.get("check_updates").is_some_and(|v| !v.is_boolean()) {
        return Err("check_updates must be a boolean.".into());
    }
    if settings.get("renderer").is_some_and(|v| !matches!(v.as_str(), Some("moltenvk" | "metal"))) {
        return Err("renderer must be moltenvk or metal.".into());
    }
    if settings.get("metal_hud").is_some_and(|v| !v.is_boolean()) {
        return Err("metal_hud must be a boolean.".into());
    }
    if settings.get("trophy_notifications").is_some_and(|v| !v.is_boolean()) {
        return Err("trophy_notifications must be a boolean.".into());
    }
    if settings.get("hide_launcher").is_some_and(|v| !v.is_boolean()) {
        return Err("hide_launcher must be a boolean.".into());
    }
    if let Some(problem) = online::check(&settings) {
        return Err(problem);
    }
    let sent: String = settings
        .iter()
        .filter(|(key, value)| settings_store::GAME_SYNCED.contains(&key.as_str()) && merged.get(*key) != Some(*value))
        .map(|(key, value)| format!("BBSET launcher:{key}={}\n", settings_store::launcher_text(value)))
        .collect();
    merged.extend(settings);
    let auto_hide = bool_setting(&merged, "hide_launcher");
    write_settings(&state.paths, merged)?;
    send_to_game(&mut game, &sent);
    tray::set_auto_hide(&app, auto_hide);
    Ok(())
}

fn write_settings(p: &Paths, settings: Map<String, Value>) -> Result<(), String> {
    let values: Vec<(String, Option<String>)> =
        settings.iter().map(|(k, v)| (k.clone(), Some(v.to_string()))).collect();
    settings_store::put(&p.data, settings_store::LAUNCHER, &values)?;
    write_launcher_json(p, settings)
}

/// A setting the in-game menu changed: a game option (bbport.ini's key) or one of the launcher's
/// that the menu shows ("launcher:key", typed like its default). Other launcher keys are ignored.
fn store_game_setting(p: &Paths, key: &str, value: &str) -> Result<(), String> {
    let Some(name) = key.strip_prefix("launcher:") else {
        return settings_store::put(&p.data, settings_store::GAME, &[(key.to_string(), Some(value.to_string()))]);
    };
    let defaults = default_settings();
    let Some(typed) = defaults.get(name).and_then(|d| settings_store::launcher_value(name, value, d)) else {
        return Err(format!("{key}={value} is not a launcher option the game may change"));
    };
    settings_store::put(&p.data, settings_store::LAUNCHER, &[(name.to_string(), Some(typed.to_string()))])?;
    write_launcher_json(p, load_settings(p))
}

/// launcher.json follows the database, for older launchers and as a readable copy.
fn write_launcher_json(p: &Paths, settings: Map<String, Value>) -> Result<(), String> {
    fs::create_dir_all(&p.data).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&Value::Object(settings)).map_err(|e| e.to_string())?;
    let pending = p.data.join("launcher.json.pending");
    fs::write(&pending, text + "\n").map_err(|e| e.to_string())?;
    fs::rename(pending, settings_file(p)).map_err(|e| e.to_string())
}
fn original_user_dir(p: &Paths, s: &Map<String, Value>) -> PathBuf {
    let custom = str_setting(s, "user_dir");
    if custom.is_empty() { p.data.join("user") } else { expand(&custom) }
}
fn selected_user_dir(p: &Paths) -> Result<PathBuf, String> {
    Ok(profiles::selected(&p.data)?.path)
}
fn saves_idle(game: &Game) -> Result<(), String> {
    if game.child.is_some() { return Err("Stop the game before changing or copying saves.".into()); }
    // Also exclude games started outside this launcher, including isolated benchmark runs.
    let status = Command::new("pgrep").args(["-x", "bb-probe"]).stdout(Stdio::null()).status().map_err(|e| e.to_string())?;
    if status.success() { return Err("Stop the running game before changing or copying saves.".into()); }
    if status.code() != Some(1) { return Err("Could not check whether the game is running.".into()); }
    Ok(())
}
#[tauri::command]
async fn list_profiles(state: State<'_, AppState>) -> Result<profiles::ProfileList, String> {
    let state=state.inner().clone();
    blocking(move || {
    let _game = state.game.lock().map_err(|e| e.to_string())?;
    let s = load_settings(&state.paths);
    profiles::list(&state.paths.data, &original_user_dir(&state.paths, &s))
    }).await
}
#[tauri::command]
async fn change_profile(state: State<'_, AppState>, action: String, id: String, name: Option<String>, source: Option<String>) -> Result<(), String> {
    let state=state.inner().clone();
    blocking(move || {
    let game = state.game.lock().map_err(|e| e.to_string())?;
    saves_idle(&game)?;
    let p = &state.paths;
    if action == "initialize" {
        let s = load_settings(p);
        profiles::initialize(&p.data, &original_user_dir(p, &s), &str_setting(&s, "save_profile"), name.as_deref().unwrap_or_default())
    } else { profiles::change(&p.data, &action, &id, name.as_deref(), source.as_deref()) }
    }).await
}
#[tauri::command]
async fn list_saves(state: State<'_, AppState>, id: Option<String>) -> Result<Value, String> {
    let state=state.inner().clone();
    blocking(move || {
    let _game = state.game.lock().map_err(|e| e.to_string())?;
    profile_saves(&state.paths, id.as_deref())
    }).await
}
#[tauri::command]
async fn list_characters(state: State<'_, AppState>, id: String) -> Result<Vec<save_editor::Character>, String> {
    let state = state.inner().clone();
    blocking(move || {
        let game = state.game.lock().map_err(|e| e.to_string())?;
        saves_idle(&game)?;
        profile_saves(&state.paths, Some(&id))?;
        let registry = profiles::require(&state.paths.data)?;
        let user = saves::user_dir(&state.paths.data, &registry.original_user_dir, &id)?;
        save_editor::list(&user)
    }).await
}
#[tauri::command]
async fn edit_character(state: State<'_, AppState>, id: String, edit: save_editor::Edit) -> Result<(), String> {
    let state = state.inner().clone();
    blocking(move || {
        let game = state.game.lock().map_err(|e| e.to_string())?;
        saves_idle(&game)?;
        profile_saves(&state.paths, Some(&id))?;
        let p = &state.paths;
        let registry = profiles::require(&p.data)?;
        let operation = profiles::operation(&p.data, Some(&id), "edit")?;
        let result = (|| {
            saves::edit(&p.data, &registry.original_user_dir, &id, |stage| save_editor::rewrite(stage, &edit))?;
            profiles::list(&p.data, &registry.original_user_dir)?;
            Ok(())
        })();
        profiles::finish(&p.data, &operation, &result)?;
        result
    }).await
}
#[tauri::command]
async fn change_save(state: State<'_, AppState>, action: String, id: String, backup: Option<String>) -> Result<(), String> {
    let state=state.inner().clone();
    blocking(move || {
    let game = state.game.lock().map_err(|e| e.to_string())?;
    saves_idle(&game)?;
    let p = &state.paths;
    save_operation(p, &id, &action, backup.as_deref())
    }).await
}

fn profile_saves(p: &Paths, id: Option<&str>) -> Result<Value, String> {
    let registry = profiles::require(&p.data)?;
    let inventory = profiles::list(&p.data, &registry.original_user_dir)?;
    let target = id.unwrap_or(&inventory.active);
    let profile = inventory.profiles.iter().find(|profile| profile.id == target)
        .ok_or("Unknown profile.")?;
    Ok(json!({"active": inventory.active, "profiles": [profile]}))
}

fn save_operation(p: &Paths, id: &str, action: &str, backup: Option<&str>) -> Result<(), String> {
    if action != "backup" && action != "restore" { return Err("Manage identities in the Profiles tab.".into()); }
    let registry=profiles::require(&p.data)?;
    // Browsing an identity does not select it for Play. Only registered live owners may mutate.
    profile_saves(p, Some(id))?;
    let operation=profiles::operation(&p.data, Some(id), action)?;
    let result=(|| {
        if action == "backup" { saves::backup(&p.data, &registry.original_user_dir, id)?; }
        else { saves::restore(&p.data, &registry.original_user_dir, id, backup.unwrap_or_default())?; }
        profiles::list(&p.data, &registry.original_user_dir)?;
        Ok(())
    })();
    profiles::finish(&p.data, &operation, &result)?;
    result
}

/// Sets game settings (null removes a key: the default applies): in the database, in bbport.ini
/// (the game's own restart reads it), and in the running game ("BBSET key=value" on its input).
/// Settings changed in the launcher go to the running game's menu ("BBSET ..." lines).
fn send_to_game(game: &mut Game, lines: &str) {
    if lines.is_empty() {
        return;
    }
    if let Some(input) = game.input.as_mut() {
        use std::io::Write;
        if input.write_all(lines.as_bytes()).and_then(|_| input.flush()).is_err() {
            game.input = None; // the game closed its input: changes apply at the next start
        }
    }
}

#[tauri::command]
fn save_ini(state: State<AppState>, values: BTreeMap<String, Option<String>>) -> Result<(), String> {
    let p = &state.paths;
    let entries: Vec<(String, Option<String>)> = values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    settings_store::put(&p.data, settings_store::GAME, &entries)?;
    if let Ok(mut game) = state.game.lock() {
        if game.child.is_some() {
            let defaults = default_ini();
            let mut sent = String::new();
            for (key, value) in &values {
                if let Some(value) = value.as_ref().or_else(|| defaults.get(key)) {
                    sent.push_str(&format!("BBSET {key}={value}\n"));
                }
            }
            send_to_game(&mut game, &sent);
        }
    }
    fs::create_dir_all(&p.data).map_err(|e| e.to_string())?;
    let mut lines = read_ini_lines(p);
    for (key, value) in &values {
        let pos = lines.iter().position(|l| ini_key(l) == Some(key.as_str()));
        match (pos, value) {
            (Some(i), Some(v)) => lines[i] = format!("{key}={v}"),
            (Some(i), None) => {
                lines.remove(i);
            }
            (None, Some(v)) => lines.push(format!("{key}={v}")),
            (None, None) => {}
        }
    }
    fs::write(ini_file(p), lines.join("\n") + "\n").map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_game(state: State<'_, AppState>, dir: String) -> Result<Value, String> {
    if dir.is_empty() {
        return Ok(json!({"problem": "Choose the game folder.", "game": null}));
    }
    api(&state.paths, &["check", &expand(&dir).to_string_lossy()])
}

#[tauri::command]
async fn list_mods(state: State<'_, AppState>) -> Result<Value, String> {
    let state = state.inner().clone();
    blocking(move || {
        let p = &state.paths;
        let s = load_settings(p);
        let dir = mods_dir(p, &s);
        let _ = fs::create_dir_all(&dir);
        let mut v = api(p, &["mods", &dir.to_string_lossy(), &p.data.join("mods.json").to_string_lossy()])?;
        v["dir"] = json!(dir);
        Ok(v)
    }).await
}

#[tauri::command]
fn save_mods(state: State<AppState>, order: Vec<String>, disabled: Vec<String>) -> Result<(), String> {
    let p = &state.paths;
    fs::create_dir_all(&p.data).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&json!({"order": order, "disabled": disabled}))
        .map_err(|e| e.to_string())?;
    fs::write(p.data.join("mods.json"), text + "\n").map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_patches(state: State<'_, AppState>) -> Result<Value, String> {
    let state = state.inner().clone();
    blocking(move || {
        let p = &state.paths;
        let s = load_settings(p);
        let dir = patches_dir(p, &s);
        let _ = fs::create_dir_all(&dir);
        let mut v = api(p, &["patches", &dir.to_string_lossy(), &p.data.join("patches.json").to_string_lossy()])?;
        v["dir"] = json!(dir);
        Ok(v)
    }).await
}

/// patches.json: the switches that differ from each file's default; choices for patches not
/// listed now (another folder) are kept.
#[tauri::command]
fn save_patches(state: State<AppState>, shown: Vec<String>, enabled: Vec<String>, disabled: Vec<String>) -> Result<(), String> {
    let p = &state.paths;
    let path = p.data.join("patches.json");
    let old: Value = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
    let keep = |key: &str| -> Vec<String> {
        old.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).filter(|k| !shown.iter().any(|s| s == k)).map(str::to_string).collect())
            .unwrap_or_default()
    };
    let mut en = keep("enabled");
    en.extend(enabled);
    en.sort();
    en.dedup();
    let mut dis = keep("disabled");
    dis.extend(disabled);
    dis.sort();
    dis.dedup();
    fs::create_dir_all(&p.data).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&json!({"enabled": en, "disabled": dis})).map_err(|e| e.to_string())?;
    fs::write(path, text + "\n").map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Gamepad {
    guid: String,
    name: String,
}

#[tauri::command]
async fn list_gamepads(state: State<'_, AppState>) -> Result<Vec<Gamepad>, String> {
    let out = Command::new(&state.paths.capabilities)
        .arg("--gamepads")
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(g, n)| Gamepad { guid: g.to_string(), name: n.to_string() })
        .collect())
}

#[derive(Serialize)]
struct Display {
    name: String,
    size: String,
    primary: bool,
}

/// The monitors in SDL's order, as the game sees them: "name<TAB>WxH<TAB>primary" lines.
#[tauri::command]
async fn list_displays(state: State<'_, AppState>) -> Result<Vec<Display>, String> {
    let out = Command::new(&state.paths.capabilities)
        .arg("--displays")
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some(Display { name: f.next()?.to_string(), size: f.next()?.to_string(), primary: f.next()? == "1" })
        })
        .collect())
}

#[derive(Serialize)]
struct Fsr411Support {
    supported: bool,
    /// Why not, as bb-gpu-capabilities says it ("" when supported).
    reason: String,
}

/// The FSR 4.1.1 Metal files the runtime's build made: the app's game/fsr4_metal, or out/ in a
/// checkout.
fn fsr4_metal_dir(p: &Paths) -> PathBuf {
    if p.bundled { p.game_root.join("fsr4_metal") } else { p.game_root.join("out/fsr4_metal") }
}

/// FSR 4.1.1 on Metal can run on this Mac: bb-gpu-capabilities --fsr411 checks the GPU, the
/// built files and AMD's two files in the data folder's fsr4_411 (run.sh gives that folder to
/// the game as BB_FSR411_DIR). "1", or "0<TAB>reason".
#[tauri::command]
async fn fsr411_support(state: State<'_, AppState>) -> Result<Fsr411Support, String> {
    let out = Command::new(&state.paths.capabilities)
        .arg("--fsr411")
        .arg(fsr4_metal_dir(&state.paths))
        .env("BB_DATA_DIR", &state.paths.data)
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or_default();
    Ok(match line.split_once('\t') {
        Some((_, reason)) => Fsr411Support { supported: false, reason: reason.to_string() },
        None => Fsr411Support { supported: line == "1", reason: String::new() },
    })
}

/// Takes AMD's two FSR 4.1.1 files from the player's own upscaler DLL into the data folder's
/// fsr4_411, with the runtime's tools/fsr4_metal_initializers.sh (it checks the DLL and the files
/// by SHA-256). Nothing of AMD's ships with the app.
#[tauri::command]
async fn add_fsr411_files(state: State<'_, AppState>, dll: String) -> Result<(), String> {
    let p = state.paths.clone();
    let out = tauri::async_runtime::spawn_blocking(move || {
        Command::new(&p.bash)
            .arg(p.game_root.join("tools/fsr4_metal_initializers.sh"))
            .arg(&dll)
            .arg(p.data.join("fsr4_411"))
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// One key or button for a control: bb-gpu-capabilities opens a small window and prints
/// "key <name>" or "pad <name>"; None when cancelled.
#[tauri::command]
async fn read_input(state: State<'_, AppState>, kind: String) -> Result<Option<String>, String> {
    let caps = state.paths.capabilities.clone();
    let out = tauri::async_runtime::spawn_blocking(move || Command::new(caps).args(["--read-input", &kind]).output())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(text.split_once(' ').map(|(_, name)| name.to_string()))
}

#[tauri::command]
fn open_launcher_data(state: State<'_, AppState>) -> Result<(), String> {
    // This remains available when database initialization fails.
    open_path(state.paths.data.to_string_lossy().into())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let path = expand(&path);
    let _ = fs::create_dir_all(&path);
    Command::new("open").arg(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

// GUI names may be long; keep the guest value within a 16-byte caller buffer including NUL.
fn guest_profile_name(name: &str) -> String {
    let end = name.char_indices().map(|(start,c)|start+c.len_utf8()).take_while(|end|*end<=15).last().unwrap_or(0);
    name[..end].to_string()
}

fn game_environment(p: &Paths, s: &Map<String, Value>) -> Result<Vec<(String, String)>, String> {
    let mut env: Vec<(String, String)> = Vec::new();
    let mut set = |k: &str, v: String| env.push((k.to_string(), v));
    let path_dirs = [p.bash.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default(),
                     "/usr/bin:/bin:/usr/sbin:/sbin".to_string()];
    set("PATH", path_dirs.join(":"));
    set("BB_PREBUILT", "1".into());
    set("BB_PROBE", p.probe.to_string_lossy().into());
    set("BB_DATA_DIR", p.data.to_string_lossy().into());
    set("PYTHON", p.python.to_string_lossy().into());
    set("PYTHONDONTWRITEBYTECODE", "1".into());
    set("BB_GAME_DIR", expand(&str_setting(s, "game_dir")).to_string_lossy().into());
    set("BB_MODS_DIR", mods_dir(p, s).to_string_lossy().into());
    set("BB_MODS_CONFIG", p.data.join("mods.json").to_string_lossy().into());
    set("BB_MODS_ENABLED", (if bool_setting(s, "mods_enabled") { "1" } else { "0" }).into());
    set("BB_PATCHES_DIR", patches_dir(p, s).to_string_lossy().into());
    set("BB_PATCHES_CONFIG", p.data.join("patches.json").to_string_lossy().into());
    set("BB_LANGUAGE", str_setting(s, "language"));
    // The in-game menu follows the launcher's language; the game matches regional codes the
    // way the launcher does (gpu/shim/bbport_text.cpp).
    let ui = str_setting(s, "ui_language");
    set("BB_UI_LANGUAGE", if ui.is_empty() { system_language() } else { ui });
    set("BB_FULLSCREEN", (if bool_setting(s, "fullscreen") { "1" } else { "0" }).into());
    set("BB_PRESENT_MODE", str_setting(s, "present_mode"));
    // display: the monitor's name (BB_DISPLAY matches part of it), so it survives a new order.
    for (key, var) in [("gamepad", "BB_GAMEPAD"), ("display", "BB_DISPLAY"), ("draw_pipe", "BB_DRAW_PIPE"),
                       ("readbacks", "BB_READBACKS"), ("preupload", "BB_PREUPLOAD")] {
        let v = str_setting(s, key);
        if !v.is_empty() {
            set(var, v);
        }
    }
    if bool_setting(s, "hdr") {
        set("BB_HDR", "1".into());
    }
    set("BB_FPS", str_setting(s, "fps_mode"));
    let limit = s.get("fps_limit").and_then(Value::as_u64).unwrap_or(0);
    if limit > 0 {
        set("BB_FPS_LIMIT", limit.to_string());
    }
    if bool_setting(s, "frame_stats") {
        set("BB_FRAME_STATS", "1".into());
    }
    if bool_setting(s, "save_log") {
        set("BB_SAVE_LOG", "1".into());
    }
    if bool_setting(s, "crash_diag") {
        set("BB_FREE_CHECK", "1".into());
        set("BB_WRITE_LOG", "1".into());
    }
    if bool_setting(s, "gpu_profile") {
        set("BB_GPU_PROFILE", "1".into());
    }
    // Online play: the account's password comes from the Keychain (extra_env can still override).
    for (k, v) in online::environment(s, online::password(str_setting(s, "online_npid").trim())) {
        set(&k, v);
    }
    for item in str_setting(s, "extra_env").split_whitespace() {
        if let Some((k, v)) = item.split_once('=') {
            set(k, v.to_string());
        }
    }
    set("BB_TROPHY_OVERLAY", if bool_setting(s,"trophy_notifications") {"1"} else {"0"}.into());
    // Apple's Metal Performance HUD (frame time, GPU time, memory), drawn by macOS over the game.
    // The renderer: the Metal backend's frame path (runtime renderer_metal/, the default), or
    // MoltenVK. Set either way: run.sh defaults to Metal.
    #[cfg(target_os = "macos")]
    set("BB_METAL_FRAME", if str_setting(s, "renderer") == "moltenvk" { "0" } else { "1" }.into());
    #[cfg(target_os = "macos")]
    if bool_setting(s, "metal_hud") {
        set("MTL_HUD_ENABLED", "1".into());
    }
    // The selected profile wins over inherited and extra environment save paths. GPU caches
    // stay in the original user directory, shared by all profiles.
    let user=selected_user_dir(p)?;
    set("BB_TROPHIES_ELIGIBLE", if trophies::eligible(&user) { "1" } else { "0" }.into());
    set("BB_USER_DIR", user.to_string_lossy().into());

    set("BB_GPU_USER_DIR", profiles::require(&p.data)?.original_user_dir.to_string_lossy().into());
    set("BB_USER_NAME", guest_profile_name(&profiles::selected(&p.data)?.name));
    Ok(env)
}

/// The system's preferred language ("pt-BR"): an app started from Finder has no LANG.
fn system_language() -> String {
    Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .ok()
        .and_then(|o| {
            // "(\n    \"pt-BR\",\n    en\n)": the first item, quoted or not
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().trim_end_matches(',').trim_matches('"'))
                .find(|l| !l.is_empty() && *l != "(" && *l != ")")
                .map(str::to_string)
        })
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default()
}

#[tauri::command]
async fn launch(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let state=state.inner().clone();
    blocking(move || start_game(&app, &state)).await
}

fn start_game(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let app = app.clone();
    let p = state.paths.clone();
    let mut game = state.game.lock().map_err(|e| e.to_string())?;
    if game.child.is_some() {
        return Err("The game is already running.".into());
    }
    saves_idle(&game)?;
    let s = load_settings(&p);
    selected_user_dir(&p)?;
    let registry = profiles::require(&p.data)?;
    fs::create_dir_all(&registry.original_user_dir).map_err(|e| e.to_string())?;
    // Back up the complete set before launch. Failure prevents starting without a recovery copy.
    save_operation(&p, &registry.active, "backup", None)?;
    fs::create_dir_all(p.data.join("logs")).map_err(|e| e.to_string())?;
    let log_path = p.data.join("logs/last.log");
    // bbport.ini from the database: run.sh, the patch scripts and the game read it at start.
    settings_store::export_ini(&p.data, &load_ini(&p))?;
    let mut cmd = Command::new(&p.bash);
    cmd.arg("run.sh")
        .current_dir(&p.game_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in game_environment(&p, &s)? {
        cmd.env(k, v);
    }
    // The in-game menu and the launcher keep each other's settings (BBSET lines both ways).
    cmd.env("BB_SETTINGS_CHANNEL", "1");
    let mut child = cmd.spawn().map_err(|e| format!("Could not start the game: {e}"))?;
    game.input = child.stdin.take();
    let log = Arc::new(Mutex::new(fs::File::create(&log_path).ok()));
    for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
                   child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)]
        .into_iter()
        .flatten()
    {
        let app = app.clone();
        let log = log.clone();
        let paths = p.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                // The in-game menu changed a setting: stored before the UI hears of it (the line
                // below makes it re-read the settings).
                if let Some((key, value)) = settings_store::parse_game_line(&line) {
                    if let Err(error) = store_game_setting(&paths, &key, &value) {
                        eprintln!("Settings: the game's change was not stored: {error}");
                    }
                }
                if let Ok(mut f) = log.lock() {
                    if let Some(f) = f.as_mut() {
                        use std::io::Write;
                        let _ = writeln!(f, "{line}");
                    }
                }
                let _ = app.emit("game-output", line);
            }
        });
    }
    game.child = Some(child);
    tray::set_auto_hide(&app, bool_setting(&s, "hide_launcher"));
    tray::game_changed(&app, true);
    let _ = app.emit("game-state", json!({"running": true}));
    // Waits for the game to end without holding the lock.
    let shared = state.game.clone();
    std::thread::spawn(move || {
        let mut wait_error_reported = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let mut g = match shared.lock() {
                Ok(g) => g,
                Err(error) => { eprintln!("Game monitor: {error}"); tray::show_launcher(&app); return; }
            };
            let code = match game_lifecycle::poll(g.child.as_mut()) {
                game_lifecycle::Poll::Running => { wait_error_reported = false; continue; }
                game_lifecycle::Poll::Failed(error) => {
                    // Preserve the child handle and Stop action on a wait error. Restore the
                    // launcher once for this error episode, then continue monitoring.
                    if !wait_error_reported { eprintln!("Game monitor: {error}"); tray::show_launcher(&app); }
                    wait_error_reported = true;
                    continue;
                }
                game_lifecycle::Poll::Finished(code) => code,
            };
            g.child = None;
            g.input = None;
            tray::game_changed(&app, false);
            let _ = app.emit("game-state", json!({"running": false, "code": code}));
            return;
        }
    });
    Ok(())
}

/// Whether the Online ID has a saved password (the password itself never goes to the window).
#[tauri::command]
async fn online_password_saved(npid: String) -> Result<bool, String> {
    blocking(move || Ok(online::password(npid.trim()).is_some())).await
}

/// Saves the Online ID's password in the Keychain; an empty one forgets it.
#[tauri::command]
async fn set_online_password(npid: String, password: String) -> Result<(), String> {
    blocking(move || online::set_password(npid.trim(), Some(&password))).await
}

#[tauri::command]
fn stop(state: State<AppState>) -> Result<(), String> {
    let game = state.game.lock().map_err(|e| e.to_string())?;
    if let Some(child) = game.child.as_ref() {
        // SIGTERM: run.sh execs the game, which quits cleanly on it.
        let _ = Command::new("kill").arg(child.id().to_string()).status();
    }
    Ok(())
}

fn main() {
    let data = match launcher_data_dir(std::env::var_os("BB_LAUNCHER_DATA_DIR").map(PathBuf::from)) {
        Ok(data) => data,
        Err(error) => { eprintln!("Launcher data: {error}"); std::process::exit(1); }
    };
    let paths = Paths::find(data);
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _arguments, _cwd| {
            tray::show_launcher(app);
        }))
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { paths, game: Arc::new(Mutex::new(Game { child: None, input: None })) })
        // `open Bloodborne.app --args --play`: start the game at once (shortcuts, scripted tests).
        .setup(|app| {
            if let Err(error) = tray::setup(app.handle(), |app| {
                if let Err(error) = stop(app.state::<AppState>()) { eprintln!("Stop game: {error}"); }
            }) { eprintln!("Tray unavailable; launcher remains visible: {error}"); }
            let state = app.state::<AppState>();
            let game = state.game.lock().map_err(|e| std::io::Error::other(e.to_string()))?;
            if saves_idle(&game).is_ok() {
                if let Err(error) = profiles::recover_operations(&state.paths.data) { eprintln!("Profile recovery: {error}"); }
            }
            drop(game);
            if std::env::args().any(|a| a == "--play") {
                let handle = app.handle().clone();
                let state = app.state::<AppState>();
                if let Err(e) = start_game(&handle, &state) {
                    eprintln!("--play: {e}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            updates::check_release,
            updates::open_release,
            load_config, save_settings, save_ini, check_game, list_mods, save_mods, list_patches,
            save_patches, list_gamepads, list_displays, fsr411_support, add_fsr411_files, read_input, open_path, open_launcher_data, launch, stop, online_password_saved, set_online_password, list_saves, list_characters, edit_character, change_save, list_profiles, change_profile, database_path, set_tray_labels
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let running = window.app_handle().try_state::<AppState>()
                        .and_then(|state| state.game.lock().ok().map(|game| game.child.is_some())).unwrap_or(false);
                    if running && tray::available(window.app_handle()) {
                        api.prevent_close();
                        tray::hide_while_running(window.app_handle());
                    }
                }
            }
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if let Ok(game) = state.game.lock() {
                        if let Some(child) = game.child.as_ref() {
                            let _ = Command::new("kill").arg(child.id().to_string()).status();
                        }
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building the launcher")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = event { tray::show_launcher(app); }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(test)]
mod save_tests {
    #[test]
    fn launcher_data_defaults_and_absolute_override_are_explicit() {
        assert_eq!(super::launcher_data_dir(None).unwrap(), super::dirs_data().join("bloodborne_mac"));
        let synthetic = std::env::temp_dir().join("bloodborne-launcher-data-test");
        assert_eq!(super::launcher_data_dir(Some(synthetic.clone())).unwrap(), synthetic);
    }
    #[test]
    fn invalid_data_override_never_falls_back_to_player_data() {
        for path in [std::path::PathBuf::new(), std::path::PathBuf::from("relative/qa")] {
            assert!(super::launcher_data_dir(Some(path)).is_err());
        }
    }

    use super::*;

    #[test]
    fn guest_profile_name_fits_small_buffers_without_splitting_utf8() {
        assert_eq!(guest_profile_name("Pablo"), "Pablo");
        let long=guest_profile_name("日本語の長いプロフィール名");
        assert!(long.len() < 16);
        assert!("日本語の長いプロフィール名".starts_with(&long));
    }
    #[test]
    fn save_changes_are_rejected_while_launcher_child_runs() {
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let mut game = Game { child: Some(child), input: None };
        assert!(saves_idle(&game).unwrap_err().contains("Stop the game"));
        game.child.as_mut().unwrap().kill().unwrap();
        game.child.as_mut().unwrap().wait().unwrap();
    }

    #[test]
    fn selected_profile_controls_launch_and_shares_only_gpu_cache_path() {
        let root = fs::canonicalize(std::env::temp_dir()).unwrap().join(format!(
            "bb-save-launch-test-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&root).unwrap();
        let p = Paths { game_root: root.clone(), bash: PathBuf::from("bash"), python: PathBuf::from("python3"),
            probe: root.join("bb-probe"), capabilities: root.join("capabilities"), data: root.clone(), bundled: false };
        let mut settings = default_settings();
        assert!(selected_user_dir(&p).is_err());
        profiles::initialize(&root, &root.join("user"), "original", "First").unwrap();
        assert_eq!(selected_user_dir(&p).unwrap(), root.join("user"));
        let id = saves::create(&root, &root.join("user"), "Synthetic", None).unwrap();
        let inactive_user = saves::user_dir(&root, &root.join("user"), &id).unwrap();
        fs::create_dir_all(inactive_user.join("savedata")).unwrap();
        fs::write(inactive_user.join("savedata/synthetic"), b"before").unwrap();
        save_operation(&p, &id, "backup", None).unwrap();
        let backup = profile_saves(&p, Some(&id)).unwrap()["profiles"][0]["backups"][0]["id"]
            .as_str().unwrap().to_owned();
        fs::write(inactive_user.join("savedata/synthetic"), b"after").unwrap();
        save_operation(&p, &id, "restore", Some(&backup)).unwrap();
        assert_eq!(fs::read(inactive_user.join("savedata/synthetic")).unwrap(), b"before");
        assert_eq!(profiles::selected(&root).unwrap().id, "original");
        assert_eq!(profile_saves(&p, Some(&id)).unwrap()["profiles"][0]["id"], id);
        assert!(save_operation(&p, "missing-profile", "backup", None).is_err());
        assert!(profile_saves(&p, Some("missing-profile")).is_err());
        profiles::change(&root, "select", &id, None, None).unwrap();
        save_operation(&p, &id, "backup", None).unwrap();
        assert_eq!(profiles::selected(&root).unwrap().backups.len(), 3);
        settings.insert("extra_env".into(), json!("BB_USER_DIR=/wrong BB_GPU_USER_DIR=/wrong BB_USER_NAME=wrong BB_TROPHIES_ELIGIBLE=1"));
        let env: BTreeMap<_,_> = game_environment(&p, &settings).unwrap().into_iter().collect();
        assert_eq!(env["BB_USER_DIR"], root.join("save-profiles").join(&id).join("user").to_string_lossy());
        assert_eq!(env["BB_GPU_USER_DIR"], root.join("user").to_string_lossy());
        assert_eq!(env["BB_USER_NAME"], "Synthetic");
        assert_eq!(env["BB_TROPHIES_ELIGIBLE"], "1");
        let user=selected_user_dir(&p).unwrap();
        fs::create_dir_all(user.join("savedata")).unwrap();
        fs::write(user.join("savedata/.bb-edited"),"edited-v1\n").unwrap();
        let env: BTreeMap<_,_> = game_environment(&p, &settings).unwrap().into_iter().collect();
        assert_eq!(env["BB_TROPHIES_ELIGIBLE"], "0");
        // Configuration and original save files are never moved by profile creation/selection.
        assert!(!root.join("user").exists());
        fs::remove_dir_all(root.join("save-profiles").join(&id)).unwrap();
        assert!(game_environment(&p, &settings).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
