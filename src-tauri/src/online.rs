// SPDX-License-Identifier: GPL-2.0-or-later
//! Online play (the Online page): the Seamless Hunt account and co-op options, passed to run.sh
//! as the environment the runtime's online module reads (BB_ONLINE, BB_SHADNET_*, BB_BBSC; see
//! the runtime's run.sh and scripts/online.py). The account's password is kept in the macOS
//! Keychain, never in the database or a file, and reaches only the game's environment.
use serde_json::{Map, Value};

/// The Keychain item's service; its account is the Online ID.
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "bloodborne_mac online";

/// The settings' defaults (merged into default_settings).
pub fn defaults() -> Value {
    serde_json::json!({
        "online": false,
        // Seamless Hunt's server when empty (run.sh's default).
        "online_server": "",
        "online_npid": "",
        // BBSC, seamless co-op: the bells and summons of Seamless Hunt's players.
        "online_coop": true,
        "online_upnp": true,
        // BBSC's game server on this computer: "auto" when the server's own does not answer.
        "online_local_server": "auto"
    })
}

/// Why a changed online setting is rejected, or None.
pub fn check(settings: &Map<String, Value>) -> Option<String> {
    for key in ["online", "online_coop", "online_upnp"] {
        if settings.get(key).is_some_and(|v| !v.is_boolean()) {
            return Some(format!("{key} must be a boolean."));
        }
    }
    if settings.get("online_local_server").is_some_and(|v| !matches!(v.as_str(), Some("auto" | "on" | "off"))) {
        return Some("online_local_server must be auto, on or off.".into());
    }
    if let Some(npid) = settings.get("online_npid") {
        match npid.as_str() {
            Some(text) if valid_npid(text) => {}
            _ => return Some("The Online ID has 3 to 16 letters, digits, - or _.".into()),
        }
    }
    if settings.get("online_server").is_some_and(|v| v.as_str().is_none_or(|s| s.chars().any(char::is_whitespace))) {
        return Some("The server address has no spaces.".into());
    }
    None
}

/// An Online ID as PSN and shadNet take it ("" while none is set).
pub fn valid_npid(text: &str) -> bool {
    text.is_empty()
        || ((3..=16).contains(&text.len()) && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

fn text<'a>(s: &'a Map<String, Value>, key: &str) -> &'a str {
    s.get(key).and_then(Value::as_str).map(str::trim).unwrap_or("")
}
fn flag(s: &Map<String, Value>, key: &str, default: bool) -> bool {
    s.get(key).and_then(Value::as_bool).unwrap_or(default)
}

/// The game's online environment: empty while online play is off.
pub fn environment(s: &Map<String, Value>, password: Option<String>) -> Vec<(String, String)> {
    if !flag(s, "online", false) {
        return Vec::new();
    }
    let mut env = vec![("BB_ONLINE".to_string(), "1".to_string())];
    let mut set = |k: &str, v: &str| env.push((k.to_string(), v.to_string()));
    set("BB_BBSC", if flag(s, "online_coop", true) { "1" } else { "0" });
    set("BB_UPNP", if flag(s, "online_upnp", true) { "1" } else { "0" });
    set("BB_BBSC_LOCAL_SERVER", match text(s, "online_local_server") { "on" => "1", "off" => "0", _ => "auto" });
    let server = text(s, "online_server");
    if !server.is_empty() {
        set("BB_SHADNET_SERVER", server);
    }
    let npid = text(s, "online_npid");
    if !npid.is_empty() {
        set("BB_SHADNET_NPID", npid);
        if let Some(password) = password {
            set("BB_SHADNET_PASSWORD", &password);
        }
    }
    env
}

/// The saved password of an Online ID (None when none is saved or there is no Keychain).
pub fn password(npid: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if npid.is_empty() {
            return None;
        }
        security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, npid)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = npid;
        None
    }
}

/// Saves (Some) or forgets (None) the password of an Online ID.
pub fn set_password(npid: &str, password: Option<&str>) -> Result<(), String> {
    if npid.is_empty() || !valid_npid(npid) {
        return Err("Set the Online ID first.".into());
    }
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords;
        match password {
            Some(password) if !password.is_empty() => {
                passwords::set_generic_password(KEYCHAIN_SERVICE, npid, password.as_bytes()).map_err(|e| e.to_string())
            }
            // Nothing saved is already forgotten.
            _ => match passwords::delete_generic_password(KEYCHAIN_SERVICE, npid) {
                Ok(()) => Ok(()),
                Err(e) if e.code() == -25300 => Ok(()), // errSecItemNotFound
                Err(e) => Err(e.to_string()),
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = password;
        Err("Saving the password needs the macOS Keychain.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn settings(extra: Value) -> Map<String, Value> {
        let mut s = defaults().as_object().cloned().unwrap();
        s.extend(extra.as_object().cloned().unwrap());
        s
    }

    #[test]
    fn off_sets_nothing() {
        assert!(environment(&settings(json!({"online_npid": "Hunter"})), Some("secret".into())).is_empty());
    }

    #[test]
    fn on_passes_the_account_and_options() {
        let s = settings(json!({"online": true, "online_npid": " Hunter_1 ", "online_coop": false,
                                "online_local_server": "off", "online_server": "example.org:31313"}));
        let env: BTreeMap<_, _> = environment(&s, Some("secret".into())).into_iter().collect();
        assert_eq!(env["BB_ONLINE"], "1");
        assert_eq!(env["BB_BBSC"], "0");
        assert_eq!(env["BB_UPNP"], "1");
        assert_eq!(env["BB_BBSC_LOCAL_SERVER"], "0");
        assert_eq!(env["BB_SHADNET_SERVER"], "example.org:31313");
        assert_eq!(env["BB_SHADNET_NPID"], "Hunter_1");
        assert_eq!(env["BB_SHADNET_PASSWORD"], "secret");
        // Without an Online ID there is no account; an empty server is run.sh's default.
        let env: BTreeMap<_, _> = environment(&settings(json!({"online": true})), Some("secret".into())).into_iter().collect();
        assert_eq!(env["BB_BBSC"], "1");
        assert_eq!(env["BB_BBSC_LOCAL_SERVER"], "auto");
        assert!(!env.contains_key("BB_SHADNET_NPID") && !env.contains_key("BB_SHADNET_PASSWORD"));
        assert!(!env.contains_key("BB_SHADNET_SERVER"));
    }

    #[test]
    fn checks_values() {
        let one = |v: Value| check(v.as_object().unwrap());
        assert_eq!(one(json!({"online": true, "online_npid": "J08ro", "online_local_server": "auto"})), None);
        assert!(one(json!({"online": "1"})).is_some());
        assert!(one(json!({"online_local_server": "maybe"})).is_some());
        assert!(one(json!({"online_npid": "ab"})).is_some());
        assert!(one(json!({"online_npid": "has space"})).is_some());
        assert!(one(json!({"online_npid": "seventeen-chars-x"})).is_some());
        assert_eq!(one(json!({"online_npid": ""})), None);
        assert!(one(json!({"online_server": "a b"})).is_some());
        assert!(set_password("", Some("x")).is_err());
    }
}
