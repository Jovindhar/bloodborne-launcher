//! BloodOnline: Yharnam Together (Bloodborne seamless co-op, the runtime's gpu/ytcoop module), the
//! Online page's third pool. Everyone types the host's address; the host runs Yharnam Together's
//! server (its Windows release, or a hosted one). The game signs in with a player id the launcher
//! makes once, as Yharnam Together's own launcher does (bbport_coop.py with_player_id): nobody
//! types an account or a password.

use serde_json::{json, Map, Value};

/// The server's ports (Yharnam Together's bbport_coop.py): the game's link and its WebAPI.
const LINK_PORT: u16 = 31314;
const WEBAPI_PORT: u16 = 31315;

/// An IPv4 or IPv6 address or a host name, without a port or a scheme.
pub fn valid_host(host: &str) -> bool {
    let host = host.trim();
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

fn random_hex(bytes: usize) -> String {
    let mut out = String::new();
    while out.len() < bytes * 2 {
        out.push_str(&uuid::Uuid::new_v4().simple().to_string());
    }
    out.truncate(bytes * 2);
    out
}

fn valid_account(value: &str) -> bool {
    (3..=16).contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn valid_secret(value: &str) -> bool {
    (1..=32).contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Makes this player's id ("p" and 14 hex digits, with a 32-digit secret) when there is none yet.
/// True when it made one (the settings are to be written).
pub fn with_player_id(settings: &mut Map<String, Value>) -> bool {
    let account = settings.get("yt_account").and_then(Value::as_str).unwrap_or("");
    let secret = settings.get("yt_secret").and_then(Value::as_str).unwrap_or("");
    if valid_account(account) && valid_secret(secret) {
        return false;
    }
    settings.insert("yt_account".into(), json!(format!("p{}", random_hex(7))));
    settings.insert("yt_secret".into(), json!(random_hex(16)));
    true
}

/// The game's environment for a Yharnam Together session at `host` (run.sh adds the game server's
/// address, the host's backend).
pub fn environment(host: &str, account: &str, secret: &str) -> Vec<(&'static str, String)> {
    let host = host.trim();
    let address = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    vec![
        ("BB_NET_BACKEND", "ytcoop".into()),
        ("BB_SHADNET_SERVER", format!("{address}:{LINK_PORT}")),
        ("BB_SHADNET_WEBAPI", format!("http://{address}:{WEBAPI_PORT}")),
        ("BB_SHADNET_NPID", account.into()),
        ("BB_SHADNET_PASSWORD", secret.into()),
        ("BB_BBSC", "0".into()),
        ("SHADPS4_BLOODBORNE_SEAMLESS_COOP", "1".into()),
        ("SHADPS4_BLOODBORNE_RE_TRACE", "0".into()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        assert!(valid_host("192.168.1.20"));
        assert!(valid_host("26.12.0.4"));
        assert!(valid_host("fd7a:115c:a1e0::1"));
        assert!(valid_host("my-pc.tailnet.ts.net"));
        assert!(!valid_host(""));
        assert!(!valid_host("192.168.1.20:31314"));
        assert!(!valid_host("http://host"));
        assert!(!valid_host("bad host"));
    }

    #[test]
    fn player_id_is_made_once() {
        let mut settings = Map::new();
        assert!(with_player_id(&mut settings));
        let account = settings["yt_account"].as_str().unwrap().to_string();
        assert!(account.starts_with('p') && account.len() == 15);
        assert_eq!(settings["yt_secret"].as_str().unwrap().len(), 32);
        assert!(!with_player_id(&mut settings));
        assert_eq!(settings["yt_account"].as_str().unwrap(), account);
    }

    #[test]
    fn environment_names_the_host() {
        let env: std::collections::BTreeMap<_, _> =
            environment("10.0.0.5", "p0123456789abcd", "s").into_iter().collect();
        assert_eq!(env["BB_SHADNET_SERVER"], "10.0.0.5:31314");
        assert_eq!(env["BB_SHADNET_WEBAPI"], "http://10.0.0.5:31315");
        assert_eq!(env["BB_NET_BACKEND"], "ytcoop");
        let v6: std::collections::BTreeMap<_, _> = environment("fd00::1", "a", "b").into_iter().collect();
        assert_eq!(v6["BB_SHADNET_SERVER"], "[fd00::1]:31314");
    }
}
