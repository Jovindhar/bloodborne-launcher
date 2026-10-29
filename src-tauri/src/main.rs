// SPDX-License-Identifier: GPL-2.0-or-later
//! The desktop launcher's program: the launcher itself is the library (lib.rs), which the iOS and
//! Android projects start too.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    bloodborne_launcher_lib::run();
}
