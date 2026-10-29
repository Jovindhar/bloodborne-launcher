# The launcher while the game runs

[← Documentation](README.md)

**Advanced → Launcher → Hide launcher during gameplay** (`hide_launcher`, on by default; applies
at once, also during a game).

- When the game starts, the launcher shows its menu-bar (tray) icon and, with hiding on, hides
  its window and its Dock entry (macOS's Accessory activation policy; the taskbar API on
  Windows and Linux).
- The menu has **Show launcher** (the window comes back; the game keeps running; closing it again
  returns it to the tray) and **Stop game**.
- When the game exits, however it exits, the launcher brings back its window and Dock entry and
  hides the tray icon. A launch that fails early does not hide it. A failed process wait keeps the
  Stop action and retries; it is not taken as an exit.
- Output and game state keep flowing while hidden. If hiding fails the launcher stays visible; if
  showing fails the tray stays for another try.

The tray icon is the project's own hunter's sigil (`src-tauri/icons/tray-icon.svg`, a template
image tinted by macOS). Window restoration follows [Leari](https://github.com/leari-mail/leari/tree/5876482/src-tauri/src) (MIT).

## Checks

`cargo test` covers process polling (normal, nonzero and killed exits, a missing handle, a wait
error). `cargo run --manifest-path src-tauri/Cargo.toml --example tray_lifecycle` is a native
smoke test with its own app id and short synthetic processes (it opens a small window and a tray
icon: say so before running it).
