//! App releases, independent of the launcher's own repository: a release build checks the latest
//! stable release; a release-candidate build also the newer candidates.
use reqwest::StatusCode;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, time::Duration};
use tauri_plugin_opener::OpenerExt;

// BloodOnline is released from Jovindhar/bloodborne-mac: an upstream release has no seamless
// co-op, so it is not offered as an update.
const RELEASES: &str =
    "https://api.github.com/repos/Jovindhar/bloodborne-mac/releases/latest";
/// Every recent release, candidates included (newest first; drafts are not listed publicly).
const RELEASE_LIST: &str =
    "https://api.github.com/repos/Jovindhar/bloodborne-mac/releases?per_page=20";
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

/// The installed app's version: the tag the package was built from (`BB_RELEASE_TAG`, set by
/// bloodborne-mac's packaging), which names a candidate (`v0.9.0-rc.2`) where the version files
/// carry only the release's number; otherwise the launcher's own.
pub fn installed_version() -> &'static str {
    option_env!("BB_RELEASE_TAG")
        .map(|tag| tag.strip_prefix('v').unwrap_or(tag))
        .filter(|version| Version::parse(version).is_ok())
        .unwrap_or(env!("CARGO_PKG_VERSION"))
}

fn is_candidate(installed: &str) -> bool {
    Version::parse(installed).is_ok_and(|version| !version.pre.is_empty())
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

/// A candidate's check: the highest published release, candidates included, among the listed ones.
fn decode_releases(
    status: StatusCode,
    body: &[u8],
    current: &str,
) -> Result<Option<ReleaseInfo>, String> {
    if !status.is_success() {
        return Err(format!("GitHub release check returned HTTP {status}."));
    }
    let releases: Vec<GithubRelease> = serde_json::from_slice(body)
        .map_err(|_| "GitHub returned an invalid release response.".to_string())?;
    let installed =
        Version::parse(current).map_err(|_| "Installed version is invalid.".to_string())?;
    let newest = releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            Some((version, release))
        })
        .max_by(|a, b| a.0.cmp_precedence(&b.0));
    Ok(newest.map(|(version, release)| ReleaseInfo {
        newer: version.cmp_precedence(&installed) == Ordering::Greater,
        version: version.to_string(),
        tag: release.tag_name,
        notes: release.body.unwrap_or_default(),
    }))
}

#[tauri::command]
pub async fn check_release() -> Result<Option<ReleaseInfo>, String> {
    let installed = installed_version();
    let candidate = is_candidate(installed);
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("bloodborne-launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(if candidate { RELEASE_LIST } else { RELEASES })
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let decode = if candidate {
        decode_releases
    } else {
        decode_release
    };
    if !status.is_success() {
        return decode(status, &[], installed);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > MAX_RESPONSE - body.len() {
            return Err("GitHub release response is too large.".into());
        }
        body.extend_from_slice(&chunk);
    }
    decode(status, &body, installed)
}

/// A release page to open: a stable release's, or for a candidate build a candidate's too. The tag
/// must be a whole semantic version, so the link cannot leave the app's release pages.
fn release_url(tag: &str, candidate: bool) -> Result<String, String> {
    if candidate {
        Version::parse(
            tag.strip_prefix('v')
                .ok_or("Release tag is not a version.")?,
        )
        .map_err(|_| "Release tag is not a semantic version.".to_string())?;
    } else {
        stable_version(tag)?;
    }
    Ok(format!("{RELEASE_PAGE}{tag}"))
}

#[tauri::command]
pub fn open_release(app: tauri::AppHandle, tag: String) -> Result<(), String> {
    app.opener()
        .open_url(
            release_url(&tag, is_candidate(installed_version()))?,
            None::<&str>,
        )
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
            release_url("v0.5.2", false).unwrap(),
            format!("{RELEASE_PAGE}v0.5.2")
        );
        for tag in [
            "https://example.com",
            "../../evil",
            "v0.5.2?x=1",
            "v0.5.2/evil",
            "v0.5.2-rc.1",
        ] {
            assert!(release_url(tag, false).is_err());
        }
        // A candidate build may open a candidate's page, and still nothing else.
        assert!(release_url("v0.5.2-rc.1", true).is_ok());
        for tag in [
            "https://example.com",
            "../../evil",
            "v0.5.2?x=1",
            "v0.5.2/evil",
            "0.5.2",
        ] {
            assert!(release_url(tag, true).is_err());
        }
    }
    fn listed(releases: &[(&str, bool, bool)]) -> Vec<u8> {
        serde_json::to_vec(
            &releases
                .iter()
                .map(|(tag, draft, prerelease)| {
                    serde_json::json!({"tag_name": tag, "draft": draft, "prerelease": prerelease})
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }
    #[test]
    fn candidates_learn_of_newer_candidates_and_the_final_release() {
        let newest = |releases: &[(&str, bool, bool)], current: &str| {
            decode_releases(StatusCode::OK, &listed(releases), current)
                .unwrap()
                .map(|release| (release.tag, release.newer))
        };
        // The final release is newer than its candidates.
        assert_eq!(
            newest(
                &[("v0.9.0", false, false), ("v0.9.0-rc.2", false, true)],
                "0.9.0-rc.2"
            ),
            Some(("v0.9.0".into(), true))
        );
        // A newer candidate, and the installed one itself.
        assert_eq!(
            newest(
                &[("v0.9.0-rc.3", false, true), ("v0.8.3", false, false)],
                "0.9.0-rc.2"
            ),
            Some(("v0.9.0-rc.3".into(), true))
        );
        assert_eq!(
            newest(&[("v0.9.0-rc.2", false, true)], "0.9.0-rc.2"),
            Some(("v0.9.0-rc.2".into(), false))
        );
        // Drafts and tags that are not versions are skipped; an empty list is no release.
        assert_eq!(
            newest(
                &[("v1.0.0", true, false), ("latest", false, false)],
                "0.9.0-rc.2"
            ),
            None
        );
        assert!(decode_releases(StatusCode::FORBIDDEN, &[], "0.9.0-rc.2").is_err());
        assert!(is_candidate("0.9.0-rc.2") && !is_candidate("0.9.0"));
    }
}
