//! Stable app releases, independent of the launcher's own repository.
use reqwest::StatusCode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, time::Duration};
use tauri_plugin_opener::OpenerExt;

// The online build (BloodOnline) is released from Jovindhar/bloodborne-mac (branch online-bbnet20):
// an upstream release has no online module, so it is not offered as an update.
const RELEASES: &str =
    "https://api.github.com/repos/Jovindhar/bloodborne-mac/releases/latest";
const RELEASE_PAGE: &str = "https://github.com/Jovindhar/bloodborne-mac/releases/tag/";
const MAX_RESPONSE: usize = 512 * 1024;

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReleaseInfo {
    version: String,
    tag: String,
    newer: bool,
    notes: String,
}

fn stable_version(tag: &str) -> Result<Version, String> {
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag))
        .map_err(|_| "Release tag is not a semantic version.".to_string())?;
    if !version.pre.is_empty() {
        return Err("Release tag is a prerelease.".into());
    }
    Ok(version)
}

fn decode_release(
    status: StatusCode,
    body: &[u8],
    current: &str,
) -> Result<Option<ReleaseInfo>, String> {
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("GitHub release check returned HTTP {status}."));
    }
    let release: GithubRelease = serde_json::from_slice(body)
        .map_err(|_| "GitHub returned an invalid release response.".to_string())?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let version = stable_version(&release.tag_name)?;
    let installed =
        Version::parse(current).map_err(|_| "Installed version is invalid.".to_string())?;
    Ok(Some(ReleaseInfo {
        newer: version.cmp_precedence(&installed) == Ordering::Greater,
        version: version.to_string(),
        tag: release.tag_name,
        notes: release.body.unwrap_or_default(),
    }))
}

#[tauri::command]
pub async fn check_release() -> Result<Option<ReleaseInfo>, String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("bloodborne-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return decode_release(status, &[], env!("CARGO_PKG_VERSION"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > MAX_RESPONSE - body.len() {
            return Err("GitHub release response is too large.".into());
        }
        body.extend_from_slice(&chunk);
    }
    decode_release(status, &body, env!("CARGO_PKG_VERSION"))
}

fn release_url(tag: &str) -> Result<String, String> {
    stable_version(tag)?;
    Ok(format!("{RELEASE_PAGE}{tag}"))
}

#[tauri::command]
pub fn open_release(app: tauri::AppHandle, tag: String) -> Result<(), String> {
    app.opener()
        .open_url(release_url(&tag)?, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"tag_name": tag, "draft": false, "prerelease": false}),
        )
        .unwrap()
    }
    #[test]
    fn compares_numeric_precedence_and_ignores_build_metadata() {
        for (tag, current, newer) in [
            ("v0.5.2", "0.5.1", true),
            ("v0.10.0", "0.9.0", true),
            ("v0.5.1", "0.5.2", false),
            ("v0.5.1+other", "0.5.1+local", false),
            ("v0.5.2", "0.5.2-dev", true),
        ] {
            assert_eq!(
                decode_release(StatusCode::OK, &release(tag), current)
                    .unwrap()
                    .unwrap()
                    .newer,
                newer
            );
        }
    }
    #[test]
    fn excludes_unpublished_releases() {
        for flag in ["draft", "prerelease"] {
            let mut value: serde_json::Value = serde_json::from_slice(&release("v0.6.0")).unwrap();
            value[flag] = true.into();
            assert!(decode_release(
                StatusCode::OK,
                &serde_json::to_vec(&value).unwrap(),
                "0.5.1"
            )
            .unwrap()
            .is_none());
        }
    }
    #[test]
    fn failures_are_not_up_to_date() {
        for status in [
            StatusCode::FORBIDDEN,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            assert!(decode_release(status, &[], "0.5.1").is_err());
        }
        for body in [b"{}".to_vec(), release("invalid"), release("v0.6.0-beta")] {
            assert!(decode_release(StatusCode::OK, &body, "0.5.1").is_err());
        }
        assert!(decode_release(StatusCode::NOT_FOUND, &[], "0.5.1")
            .unwrap()
            .is_none());
    }
    #[test]
    fn release_notes_preserve_markdown_and_allow_missing_content() {
        let mut value: serde_json::Value = serde_json::from_slice(&release("v0.5.2")).unwrap();
        for notes in [
            serde_json::Value::Null,
            "## Changes\n- Fixed playback\n".into(),
        ] {
            value["body"] = notes.clone();
            let result = decode_release(
                StatusCode::OK,
                &serde_json::to_vec(&value).unwrap(),
                "0.5.1",
            )
            .unwrap()
            .unwrap();
            assert_eq!(result.notes, notes.as_str().unwrap_or_default());
        }
        value.as_object_mut().unwrap().remove("body");
        assert!(decode_release(
            StatusCode::OK,
            &serde_json::to_vec(&value).unwrap(),
            "0.5.1"
        )
        .unwrap()
        .unwrap()
        .notes
        .is_empty());
    }
    #[test]
    fn browser_links_cannot_escape_the_app_release_page() {
        assert_eq!(
            release_url("v0.5.2").unwrap(),
            format!("{RELEASE_PAGE}v0.5.2")
        );
        for tag in [
            "https://example.com",
            "../../evil",
            "v0.5.2?x=1",
            "v0.5.2/evil",
            "v0.5.2-rc.1",
        ] {
            assert!(release_url(tag).is_err());
        }
    }
}
