//! BloodOnline: Yharnam Together (Bloodborne seamless co-op, the runtime's gpu/ytcoop module), the
//! Online page's third pool. Everyone types the host's address; the host runs Yharnam Together's
//! server (its Windows release, or a hosted one). The game signs in with a player id the launcher
//! makes once, as Yharnam Together's own launcher does (bbport_coop.py with_player_id): nobody
//! types an account or a password.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

// --- the Global server (Yharnam Together v1.4.0's bbport/launcher/bbport_auth.py and bbport_coop.py,
// commit efa0ad2): its own accounts, a name and a password sent to it alone over HTTPS. The launcher
// keeps the id and the signed 30-day secret the server answers (a file of their own, 0600), never
// the password.

/// The public server (bbport_coop.py GLOBAL_SERVER).
pub const GLOBAL_SERVER: &str = "https://seamlesshunt.com";
/// The key its link answers with, shipped as Yharnam Together ships it (KNOWN_SERVER_KEYS): the game
/// refuses a server that answers with another.
const GLOBAL_KEY: &str = "e55f4f13d3845aa4eddeb0e12a9d143cb5f68af0a71e0dba111c688a1014c958";
/// A sign-in holds 30 days; a day before, the launcher asks for a new one (SECRET_DAYS).
const SECRET_DAYS: u64 = 29;
/// More sign-up work than a few seconds: a server we do not want (MAX_WORK_BITS).
const MAX_WORK_BITS: u32 = 28;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct GlobalAccount {
    pub id: String,
    pub secret: String,
    pub name: String,
}

fn account_file(data: &Path) -> PathBuf {
    data.join("yt-global-account.json")
}

/// The signed-in Global account, if there is one.
pub fn global_account(data: &Path) -> Option<GlobalAccount> {
    let text = fs::read_to_string(account_file(data)).ok()?;
    serde_json::from_str::<GlobalAccount>(&text).ok().filter(|a| !a.id.is_empty() && !a.secret.is_empty())
}

fn save_account(data: &Path, account: &GlobalAccount) -> Result<(), String> {
    fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let pending = data.join("yt-global-account.pending");
    let _ = fs::remove_file(&pending);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut out = options.open(&pending).map_err(|e| format!("Could not keep the sign-in: {e}"))?;
    out.write_all(serde_json::to_string(account).map_err(|e| e.to_string())?.as_bytes())
        .map_err(|e| format!("Could not keep the sign-in: {e}"))?;
    drop(out);
    fs::rename(&pending, account_file(data)).map_err(|e| format!("Could not keep the sign-in: {e}"))
}

pub fn forget_account(data: &Path) {
    let _ = fs::remove_file(account_file(data));
}

/// Whether the sign-in's secret is still good (it starts with when it was issued: unix seconds, hex).
pub fn secret_fresh(secret: &str, now: u64) -> bool {
    secret.get(..8).and_then(|h| u64::from_str_radix(h, 16).ok())
        .is_some_and(|issued| now.saturating_sub(issued) < SECRET_DAYS * 24 * 3600)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The signed-in account when its sign-in still holds.
pub fn signed_in(data: &Path) -> Option<GlobalAccount> {
    global_account(data).filter(|a| secret_fresh(&a.secret, now()))
}

pub fn name_problem(name: &str) -> Option<&'static str> {
    if !(3..=20).contains(&name.chars().count()) {
        return Some("A name has 3 to 20 characters.");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
        return Some("A name has only letters, digits, _ - and . in it.");
    }
    None
}

pub fn password_problem(password: &str, name: &str) -> Option<&'static str> {
    let length = password.chars().count();
    if length < 10 {
        return Some("A password has at least 10 characters.");
    }
    if length > 128 {
        return Some("A password has at most 128 characters.");
    }
    if name.len() >= 3 && password.to_lowercase().contains(&name.to_lowercase()) {
        return Some("The password must not contain your name.");
    }
    None
}

/// The sign-up's work (the server's password_accounts.h WorkDone): a nonce such that SHA-256 of
/// "bb signup v1|<challenge>|<name in lower case>|<nonce, 8 bytes little-endian>" starts with `bits`
/// zero bits.
pub fn solve(challenge: &str, name: &str, bits: u32) -> Result<u64, String> {
    if bits > MAX_WORK_BITS {
        return Err("The server is busy: try again in a minute.".into());
    }
    let mut prefix = Sha256::new();
    prefix.update(b"bb signup v1|");
    prefix.update(challenge.as_bytes());
    prefix.update(b"|");
    prefix.update(name.to_lowercase().as_bytes());
    prefix.update(b"|");
    for nonce in 0u64.. {
        let mut digest = prefix.clone();
        digest.update(nonce.to_le_bytes());
        let hash = digest.finalize();
        let mut zero = 0u32;
        for byte in hash.iter() {
            if *byte == 0 {
                zero += 8;
                if zero >= bits { break; }
                continue;
            }
            zero += byte.leading_zeros();
            break;
        }
        if zero >= bits {
            return Ok(nonce);
        }
    }
    unreachable!()
}

fn client(timeout: Duration) -> Result<reqwest::Client, String> {
    // HTTPS only, the system's certificates, no redirects: a password never goes anywhere else.
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .user_agent(concat!("bloodborne-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

/// (status, answer) of `path` on the Global server: a GET without a body, else a JSON POST.
async fn call(path: &str, body: Option<Value>) -> Result<(u16, Value), String> {
    call_within(path, body, Duration::from_secs(30)).await
}

/// The same with a shorter wait (the lookup at Play: 10 s, as Yharnam Together's launcher waits).
async fn call_within(path: &str, body: Option<Value>, timeout: Duration) -> Result<(u16, Value), String> {
    let client = client(timeout)?;
    let url = format!("{GLOBAL_SERVER}{path}");
    let request = match body {
        Some(body) => client.post(&url).header("Content-Type", "application/json").body(body.to_string()),
        None => client.get(&url),
    };
    let response = request.send().await.map_err(|e| {
        if e.is_connect() || e.is_timeout() { "The server does not answer: try again in a minute.".to_string() }
        else { format!("The server could not be reached: {e}") }
    })?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    Ok((status, serde_json::from_slice(&bytes).unwrap_or(Value::Null)))
}

fn refusal(status: u16, answer: &Value, wrong: &str) -> String {
    let why = answer.get("error").and_then(Value::as_str).unwrap_or("");
    match (status, why) {
        (401, _) => wrong.into(),
        (429, "wait") => format!("Too many wrong tries: wait {} s and try again.",
                                 answer.get("seconds").and_then(Value::as_u64).unwrap_or(1)),
        (429, "signups") => "Too many new accounts from your address: try again in an hour.".into(),
        (429 | 503, _) => "The server is busy: try again in a minute.".into(),
        (409, _) => "That name is taken: choose another.".into(),
        (404, _) => "This server has no accounts.".into(),
        (400, "name") => "The server refused the name: 3 to 20 letters, digits, _ - or .".into(),
        (400, "password") => match answer.get("problem").and_then(Value::as_str) {
            Some("common") => "That password is too common: choose another.".into(),
            Some("name") => "The password must not contain your name.".into(),
            _ => "The server refused the password: 10 to 128 characters.".into(),
        },
        _ => format!("The server refused it ({status}): try again."),
    }
}

/// The account a sign-in or sign-up answered, kept; with its recovery code (a sign-up's), shown once.
fn accepted(data: &Path, status: u16, answer: &Value, wrong: &str) -> Result<(GlobalAccount, Option<String>), String> {
    let text = |key: &str| answer.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    if status != 200 || text("id").is_empty() || text("secret").is_empty() {
        return Err(refusal(status, answer, wrong));
    }
    let account = GlobalAccount { id: text("id"), secret: text("secret"), name: text("name") };
    save_account(data, &account)?;
    let code = Some(text("recovery_code")).filter(|c| !c.is_empty());
    Ok((account, code))
}

pub async fn sign_in(data: &Path, name: &str, password: &str) -> Result<GlobalAccount, String> {
    if name.is_empty() || password.is_empty() {
        return Err("Wrong name or password.".into());
    }
    let (status, answer) = call("/bb/auth/sign-in", Some(json!({"name": name, "password": password}))).await?;
    accepted(data, status, &answer, "Wrong name or password.").map(|(account, _)| account)
}

/// A new account; the recovery code is shown to the player once and never kept.
pub async fn sign_up(data: &Path, name: &str, password: &str) -> Result<(GlobalAccount, String), String> {
    if let Some(problem) = name_problem(name).or_else(|| password_problem(password, name)) {
        return Err(problem.into());
    }
    let (status, work) = call("/bb/auth/challenge", Some(json!({}))).await?;
    let challenge = work.get("challenge").and_then(Value::as_str).unwrap_or("").to_string();
    if status != 200 || challenge.is_empty() {
        return Err(refusal(status, &work, "Wrong name or password."));
    }
    let bits = work.get("bits").and_then(Value::as_u64).unwrap_or(0) as u32;
    let (c, n) = (challenge.clone(), name.to_string());
    let nonce = tauri::async_runtime::spawn_blocking(move || solve(&c, &n, bits)).await.map_err(|e| e.to_string())??;
    let (status, answer) = call("/bb/auth/sign-up", Some(json!({"name": name, "password": password,
        "challenge": challenge, "nonce": nonce}))).await?;
    let (account, code) = accepted(data, status, &answer, "Wrong name or password.")?;
    Ok((account, code.unwrap_or_default()))
}

/// Ends this account's sign-ins on the server (best effort) and forgets it here.
pub async fn sign_out(data: &Path) {
    if let Some(account) = global_account(data) {
        let _ = call("/bb/auth/sign-out", Some(json!({"id": account.id, "secret": account.secret}))).await;
    }
    forget_account(data);
}

/// Where the game links on the Global server: an address, the link port and the key it pins.
#[derive(Clone, Debug, PartialEq)]
pub struct GlobalTarget {
    pub address: String,
    pub port: u16,
    pub key: String,
}

fn shard_address(shard: &Value) -> Option<(String, u16)> {
    let text = shard.get("address")?.as_str()?;
    let (host, port) = text.rsplit_once(':')?;
    let ip: std::net::Ipv4Addr = host.parse().ok()?;
    let port: u16 = port.parse().ok().filter(|p| *p > 0)?;
    Some((ip.to_string(), port))
}

fn valid_key(key: &str) -> bool {
    key.len() == 64 && key.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

/// The shard the game starts on (pick_shard): the player's own when it is usable, else the least
/// loaded by its share of its capacity.
pub fn pick_shard(shards: &[Value], own: Option<&Value>) -> Option<Value> {
    let number = |s: &Value, k: &str| s.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let usable = |s: &Value| shard_address(s).is_some()
        && s.get("key").and_then(Value::as_str).is_some_and(valid_key)
        && !s.get("draining").and_then(Value::as_bool).unwrap_or(false)
        && number(s, "players") < number(s, "capacity").max(1.0);
    if let Some(own) = own.filter(|o| usable(o)) {
        return Some(own.clone());
    }
    shards.iter().filter(|s| usable(s)).min_by(|a, b| {
        let load = |s: &Value| number(s, "players") / number(s, "capacity").max(1.0);
        load(a).partial_cmp(&load(b)).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.get("name").and_then(Value::as_str).cmp(&b.get("name").and_then(Value::as_str)))
    }).cloned()
}

/// Where the game links for this account (global_game_settings): the shard the region's directory
/// names, or the server itself when it has no directory.
pub async fn resolve(account: &GlobalAccount) -> Result<GlobalTarget, String> {
    let down = || format!("The region's directory {GLOBAL_SERVER} does not answer: try again in a minute.");
    let quick = Duration::from_secs(10);
    let (status, listed) = call_within("/v1/shards", None, quick).await.map_err(|e| format!("{} ({e})", down()))?;
    if status == 200 {
        let (_, where_) = call_within(&format!("/v1/where?player={}", account.id), None, quick).await.map_err(|e| format!("{} ({e})", down()))?;
        let shards = listed.get("shards").and_then(Value::as_array).cloned().unwrap_or_default();
        let shard = pick_shard(&shards, where_.get("shard"))
            .ok_or_else(|| format!("No shard of the region {GLOBAL_SERVER} has room right now: try again in a minute."))?;
        let (address, port) = shard_address(&shard).expect("a usable shard has an address");
        return Ok(GlobalTarget { address, port, key: GLOBAL_KEY.into() });
    }
    if status != 404 {
        return Err(down());
    }
    // A lone server: its own address, the usual link port.
    let host = GLOBAL_SERVER.trim_start_matches("https://");
    let address = tauri::async_runtime::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        (host, 443).to_socket_addrs().ok().and_then(|mut found| found.find(|a| a.is_ipv4()))
    }).await.ok().flatten().ok_or_else(|| format!("The server {GLOBAL_SERVER} could not be found."))?;
    Ok(GlobalTarget { address: address.ip().to_string(), port: LINK_PORT, key: GLOBAL_KEY.into() })
}

/// The game's environment on the Global server.
pub fn global_environment(target: &GlobalTarget, account: &GlobalAccount) -> Vec<(&'static str, String)> {
    let mut env = environment(&target.address, &account.id, &account.secret);
    for (key, value) in env.iter_mut() {
        if *key == "BB_SHADNET_SERVER" {
            *value = format!("{}:{}", target.address, target.port);
        }
    }
    env.push(("BB_SHADNET_SERVER_KEY", target.key.clone()));
    env
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

    #[test]
    fn signup_work_has_its_zero_bits() {
        let nonce = solve("abc", "Hunter", 12).unwrap();
        let mut digest = Sha256::new();
        digest.update(b"bb signup v1|abc|hunter|");
        digest.update(nonce.to_le_bytes());
        let hash = digest.finalize();
        assert_eq!(hash[0], 0);
        assert!(hash[1] >> 4 == 0);
        assert!(solve("abc", "x", 40).is_err());
    }

    #[test]
    fn secrets_last_29_days() {
        let issued = 0x6900_0000u64;
        let secret = format!("{issued:08x}rest");
        assert!(secret_fresh(&secret, issued + 3600));
        assert!(!secret_fresh(&secret, issued + 30 * 24 * 3600));
        assert!(!secret_fresh("zz", issued));
    }

    #[test]
    fn shards_least_loaded_or_own() {
        let key = "a".repeat(64);
        let a = json!({"name": "a", "address": "1.2.3.4:31314", "key": key, "players": 50, "capacity": 100});
        let b = json!({"name": "b", "address": "5.6.7.8:31320", "key": key, "players": 10, "capacity": 100});
        let full = json!({"name": "c", "address": "9.9.9.9:1", "key": key, "players": 5, "capacity": 5});
        let shards = vec![a.clone(), b.clone(), full.clone()];
        assert_eq!(pick_shard(&shards, None), Some(b.clone()));
        assert_eq!(pick_shard(&shards, Some(&a)), Some(a));
        assert_eq!(pick_shard(&shards, Some(&full)), Some(b));
        let target = GlobalTarget { address: "5.6.7.8".into(), port: 31320, key: key.clone() };
        let account = GlobalAccount { id: "g1".into(), secret: "s".into(), name: "n".into() };
        let env: std::collections::BTreeMap<_, _> = global_environment(&target, &account).into_iter().collect();
        assert_eq!(env["BB_SHADNET_SERVER"], "5.6.7.8:31320");
        assert_eq!(env["BB_SHADNET_WEBAPI"], "http://5.6.7.8:31315");
        assert_eq!(env["BB_SHADNET_SERVER_KEY"], key);
        assert_eq!(env["BB_SHADNET_NPID"], "g1");
    }
}
