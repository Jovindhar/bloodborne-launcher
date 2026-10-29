// SPDX-License-Identifier: GPL-2.0-or-later
//! iOS and Android: no tray and no launcher window to hide while the game runs (the game takes
//! over the app). The same functions as tray.rs, doing nothing.
use tauri::AppHandle;

#[derive(serde::Deserialize)]
pub struct Labels {
    #[allow(dead_code)]
    pub show: String,
    #[allow(dead_code)]
    pub stop: String,
}

pub fn setup(_app: &AppHandle, _on_stop: impl Fn(&AppHandle) + Send + Sync + 'static) -> tauri::Result<()> {
    Ok(())
}
pub fn available(_app: &AppHandle) -> bool {
    false
}
pub fn set_labels(_app: &AppHandle, _labels: Labels) -> tauri::Result<()> {
    Ok(())
}
pub fn game_changed(_app: &AppHandle, _running: bool) {}
pub fn set_auto_hide(_app: &AppHandle, _enabled: bool) {}
pub fn show_launcher(_app: &AppHandle) {}
pub fn hide_while_running(_app: &AppHandle) {}
