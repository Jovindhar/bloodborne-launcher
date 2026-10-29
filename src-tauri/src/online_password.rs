//! The online account's password (Seamless Hunt's shadNet login), saved only when the player
//! accepts it: a file of its own, readable by their user account alone (0600 on Unix), kept out of the
//! settings database, its backups and launcher.json. It is not encrypted: shadNet takes the
//! password itself, so the launcher must be able to read it back. Not saved: asked at each launch.
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

fn file(data: &Path) -> PathBuf {
    data.join("online-password")
}

/// The saved password, if there is one.
pub fn saved(data: &Path) -> Option<String> {
    fs::read_to_string(file(data)).ok().filter(|password| !password.is_empty())
}

pub fn save(data: &Path, password: &str) -> Result<(), String> {
    if password.is_empty() {
        return forget(data);
    }
    fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let pending = data.join("online-password.pending");
    let _ = fs::remove_file(&pending);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut out = options
        .open(&pending)
        .map_err(|e| format!("Could not save the password: {e}"))?;
    out.write_all(password.as_bytes()).map_err(|e| format!("Could not save the password: {e}"))?;
    drop(out);
    fs::rename(&pending, file(data)).map_err(|e| format!("Could not save the password: {e}"))
}

pub fn forget(data: &Path) -> Result<(), String> {
    match fs::remove_file(file(data)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Could not remove the saved password: {e}")),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_password_is_private_and_forgotten() {
        let dir = std::env::temp_dir().join(format!("bb-online-password-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(saved(&dir), None);
        save(&dir, "secret").unwrap();
        assert_eq!(saved(&dir).as_deref(), Some("secret"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(file(&dir)).unwrap().permissions().mode() & 0o777, 0o600);
        }
        save(&dir, "").unwrap();
        assert_eq!(saved(&dir), None);
        save(&dir, "again").unwrap();
        forget(&dir).unwrap();
        forget(&dir).unwrap();
        assert_eq!(saved(&dir), None);
        fs::remove_dir_all(&dir).unwrap();
    }
}
