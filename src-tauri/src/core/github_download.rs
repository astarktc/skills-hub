//! Download a GitHub directory via the Contents API, bypassing git clone entirely.
//! This is much faster than cloning large repos when only a subdirectory is needed.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

use super::cancel_token::CancelToken;
use super::errors::SignalError;
use super::repo_subpath::LinkChain;

const GITHUB_API_BASE: &str = "https://api.github.com";

/// A non-success HTTP status from the GitHub API, classified at the origin.
///
/// Raised through `anyhow` chains by every GitHub request this module makes;
/// callers discriminate by downcast (`err.downcast_ref::<GithubApiError>()`)
/// and map `status` to the typed condition they own (404 → skill not found,
/// 403 → rate limited, anything else → fall back to a git clone). The status
/// never travels as prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubApiError {
    /// HTTP status code of the failed response.
    pub status: u16,
    /// Rounded-up minutes until the rate limit resets, when the response
    /// carried a parseable `x-ratelimit-reset` header.
    pub reset_minutes: Option<i64>,
    /// Request URL or repo-relative path (diagnostic context, not user copy).
    pub url: String,
}

impl fmt::Display for GithubApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GitHub API error {} for: {}", self.status, self.url)?;
        if let Some(minutes) = self.reset_minutes {
            write!(f, " (rate limit resets in ~{minutes} min)")?;
        }
        Ok(())
    }
}

impl std::error::Error for GithubApiError {}

#[derive(Debug, Deserialize)]
struct GithubContent {
    name: String,
    #[serde(rename = "type")]
    content_type: String,
    download_url: Option<String>,
    path: String,
    /// The raw link target; present only on the object the API answers for
    /// a path that *is* a symlink (listing entries of that type carry none).
    #[serde(default)]
    target: Option<String>,
}

/// What `GET /contents/<path>` answers: a listing for a directory, one
/// object for anything else (a file, a symlink, a submodule).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ContentsResponse {
    Listing(Vec<GithubContent>),
    Entry(GithubContent),
}

/// Download a directory from a GitHub repo using the Contents API.
///
/// `owner`/`repo`: repository coordinates
/// `branch`: branch or ref (e.g. "main")
/// `path`: directory path within the repo (e.g. "skills/user/foo")
/// `dest`: local directory to write files into (will be created)
/// `cancel`: optional cancellation token
pub fn download_github_directory(
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    dest: &Path,
    cancel: Option<&CancelToken>,
    token: Option<&str>,
) -> Result<()> {
    download_github_directory_from(
        GITHUB_API_BASE,
        owner,
        repo,
        branch,
        path,
        dest,
        cancel,
        token,
    )
}

/// [`download_github_directory`] against an explicit API origin (tests point
/// it at a local server).
#[allow(clippy::too_many_arguments)]
fn download_github_directory_from(
    api_base: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    dest: &Path,
    cancel: Option<&CancelToken>,
    token: Option<&str>,
) -> Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("build HTTP client")?;

    std::fs::create_dir_all(dest).with_context(|| format!("create directory {:?}", dest))?;

    let mut chain = LinkChain::new();
    download_dir_recursive(
        &client, api_base, owner, repo, branch, path, dest, cancel, token, &mut chain,
    )
}

#[allow(clippy::too_many_arguments)]
fn download_dir_recursive(
    client: &Client,
    api_base: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    dest: &Path,
    cancel: Option<&CancelToken>,
    token: Option<&str>,
    chain: &mut LinkChain,
) -> Result<()> {
    if cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }

    let url = format!(
        "{}/repos/{}/{}/contents/{}?ref={}",
        api_base, owner, repo, path, branch
    );

    let mut req = client
        .get(&url)
        .header("User-Agent", "skills-hub")
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req
        .send()
        .with_context(|| format!("request GitHub contents: {}", url))?;
    let resp = check_github_response(resp, &url)?;

    let items = match resp
        .json::<ContentsResponse>()
        .with_context(|| format!("parse GitHub contents response: {}", url))?
    {
        ContentsResponse::Listing(items) => items,
        // The path is an upstream symlink: follow it, relative to the
        // entry's own directory and within the repository (the shared rule
        // refuses an escaping target before anything at it is requested).
        ContentsResponse::Entry(GithubContent {
            content_type,
            target: Some(target),
            ..
        }) if content_type == "symlink" => {
            let resolved = chain.follow(path, &target)?;
            return download_dir_recursive(
                client, api_base, owner, repo, branch, &resolved, dest, cancel, token, chain,
            );
        }
        ContentsResponse::Entry(entry) => {
            anyhow::bail!(
                "GitHub contents path is not a directory (type {}): {}",
                entry.content_type,
                entry.path
            );
        }
    };

    for item in items {
        if cancel.is_some_and(|c| c.is_cancelled()) {
            anyhow::bail!(SignalError::Cancelled);
        }

        let local_path = dest.join(&item.name);

        match item.content_type.as_str() {
            "file" => {
                if let Some(download_url) = &item.download_url {
                    if let Some(parent) = local_path.parent() {
                        std::fs::create_dir_all(parent)
                            .with_context(|| format!("create parent dir {:?}", parent))?;
                    }
                    let mut file_req = client.get(download_url).header("User-Agent", "skills-hub");
                    if let Some(t) = token {
                        file_req = file_req.header("Authorization", format!("Bearer {}", t));
                    }
                    let file_resp = file_req
                        .send()
                        .with_context(|| format!("download file: {}", item.path))?;
                    let file_resp = check_github_response(file_resp, &item.path)?;
                    let bytes = file_resp
                        .bytes()
                        .with_context(|| format!("read file bytes: {}", item.path))?;

                    std::fs::write(&local_path, &bytes)
                        .with_context(|| format!("write file {:?}", local_path))?;
                }
            }
            "dir" => {
                download_dir_recursive(
                    client,
                    api_base,
                    owner,
                    repo,
                    branch,
                    &item.path,
                    &local_path,
                    cancel,
                    token,
                    chain,
                )?;
            }
            _ => {
                // Skip symlinks nested inside the directory (a listing entry
                // carries no target), submodules, etc.
            }
        }
    }

    Ok(())
}

/// Pass a successful GitHub response through; classify any other status as a
/// typed [`GithubApiError`] carrying the code and the rate-limit reset ETA.
fn check_github_response(
    resp: reqwest::blocking::Response,
    context: &str,
) -> Result<reqwest::blocking::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let reset_minutes = if status.as_u16() == 403 {
        resp.headers()
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok())
            .map(|ts| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                ((ts - now).max(0) + 59) / 60 // round up
            })
    } else {
        None
    };
    Err(anyhow::Error::new(GithubApiError {
        status: status.as_u16(),
        reset_minutes,
        url: context.to_string(),
    }))
}

/// The `(owner, repo)` a GitHub clone URL points at, or `None` when the URL
/// is not a github.com repository — the fast path's precondition, decided in
/// one place (`core::git_acquisition` owns the rest of the policy).
pub fn parse_github_repo(clone_url: &str) -> Option<(String, String)> {
    // Extract owner/repo from clone_url like https://github.com/owner/repo.git
    let url = clone_url.trim_end_matches('/').trim_end_matches(".git");
    let prefix = "https://github.com/";
    if !url.starts_with(prefix) {
        return None;
    }
    let rest = &url[prefix.len()..];
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return None;
    }

    Some((parts[0].to_string(), parts[1].to_string()))
}

/// Fetch the HEAD commit SHA for a branch without cloning.
/// Uses GitHub API: GET /repos/{owner}/{repo}/commits/{branch}
/// Returns the 40-char hex SHA string.
pub fn fetch_branch_sha(
    owner: &str,
    repo: &str,
    branch: &str,
    token: Option<&str>,
) -> Result<String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("build HTTP client")?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        owner, repo, branch
    );

    let mut req = client
        .get(&url)
        .header("User-Agent", "skills-hub")
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req
        .send()
        .with_context(|| format!("request branch SHA: {}", url))?;
    let resp = check_github_response(resp, &url)?;

    let json: serde_json::Value = resp
        .json()
        .with_context(|| format!("parse commits response: {}", url))?;
    let sha = json
        .get("sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing sha field in commits response"))?;

    Ok(sha.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_repo_extracts_owner_and_repo() {
        let result = parse_github_repo("https://github.com/openclaw/skills.git");
        assert_eq!(result, Some(("openclaw".to_string(), "skills".to_string())));
        assert_eq!(
            parse_github_repo("https://github.com/openclaw/skills"),
            Some(("openclaw".to_string(), "skills".to_string()))
        );
    }

    #[test]
    fn parse_github_repo_returns_none_for_non_github() {
        assert_eq!(parse_github_repo("https://gitlab.com/user/repo.git"), None);
        assert_eq!(parse_github_repo("/local/path/to/repo"), None);
    }

    #[test]
    fn parse_github_repo_returns_none_without_a_repo_segment() {
        assert_eq!(parse_github_repo("https://github.com/openclaw"), None);
    }

    #[test]
    fn check_github_response_passes_success() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/ok")
            .with_status(200)
            .with_body("ok")
            .create();
        let client = Client::new();
        let resp = client.get(format!("{}/ok", server.url())).send().unwrap();
        assert!(check_github_response(resp, "test").is_ok());
    }

    #[test]
    fn check_github_response_extracts_rate_limit_reset() {
        let mut server = mockito::Server::new();
        let reset_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 600; // 10 minutes from now
        let _m = server
            .mock("GET", "/limited")
            .with_status(403)
            .with_header("x-ratelimit-reset", &reset_ts.to_string())
            .with_body("rate limited")
            .create();
        let client = Client::new();
        let resp = client
            .get(format!("{}/limited", server.url()))
            .send()
            .unwrap();
        let err = check_github_response(resp, "test").unwrap_err();
        // The status is classified at the origin as a typed error carrying
        // both the code and the parsed reset ETA, recoverable by downcast.
        let Some(api) = err.downcast_ref::<GithubApiError>() else {
            panic!("expected GithubApiError, got: {:#}", err);
        };
        assert_eq!(api.status, 403);
        assert_eq!(api.url, "test");
        let reset_minutes = api.reset_minutes.expect("reset ETA parsed from header");
        assert!(
            (9..=11).contains(&reset_minutes),
            "expected ~10 mins, got {}",
            reset_minutes
        );
    }

    #[test]
    fn check_github_response_handles_403_without_reset_header() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/forbidden")
            .with_status(403)
            .with_body("forbidden")
            .create();
        let client = Client::new();
        let resp = client
            .get(format!("{}/forbidden", server.url()))
            .send()
            .unwrap();
        let err = check_github_response(resp, "test").unwrap_err();
        let api = err
            .downcast_ref::<GithubApiError>()
            .unwrap_or_else(|| panic!("expected GithubApiError, got: {:#}", err));
        assert_eq!(api.status, 403);
        assert_eq!(api.reset_minutes, None, "no header means no ETA");
    }

    #[test]
    fn check_github_response_classifies_404_as_typed_status() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/notfound")
            .with_status(404)
            .with_body("not found")
            .create();
        let client = Client::new();
        let resp = client
            .get(format!("{}/notfound", server.url()))
            .send()
            .unwrap();
        let err = check_github_response(resp, "test").unwrap_err();
        let api = err
            .downcast_ref::<GithubApiError>()
            .unwrap_or_else(|| panic!("expected GithubApiError, got: {:#}", err));
        assert_eq!(
            *api,
            GithubApiError {
                status: 404,
                reset_minutes: None,
                url: "test".to_string(),
            }
        );
        // The typed value survives `.context(...)` layering (anyhow chains).
        let wrapped = err.context("download skill");
        assert!(wrapped.downcast_ref::<GithubApiError>().is_some());
    }

    #[test]
    fn check_github_response_classifies_server_errors_as_typed_status() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/boom")
            .with_status(502)
            .with_body("bad gateway")
            .create();
        let client = Client::new();
        let resp = client.get(format!("{}/boom", server.url())).send().unwrap();
        let err = check_github_response(resp, "test").unwrap_err();
        let api = err
            .downcast_ref::<GithubApiError>()
            .unwrap_or_else(|| panic!("expected GithubApiError, got: {:#}", err));
        assert_eq!(api.status, 502);
        assert_eq!(api.reset_minutes, None);
    }

    // -----------------------------------------------------------------------
    // Upstream in-repo symlinks (API path)
    // -----------------------------------------------------------------------

    /// A directory listing entry as the Contents API spells it.
    fn listing_file(server: &str, dir: &str, name: &str) -> String {
        format!(
            r#"{{"name":"{name}","path":"{dir}/{name}","type":"file","download_url":"{server}/raw/{dir}/{name}"}}"#
        )
    }

    /// The Contents API's answer for a path that is a symlink: one object
    /// (not a listing) with `type: symlink` and the raw `target`.
    fn symlink_object(path: &str, target: &str) -> String {
        let name = path.rsplit('/').next().unwrap_or(path);
        format!(
            r#"{{"name":"{name}","path":"{path}","type":"symlink","target":"{target}","download_url":null}}"#
        )
    }

    /// The upstream case: the requested path answers `type: symlink`, whose
    /// target is followed relative to the entry's own directory, and the
    /// real directory's files land in dest.
    #[test]
    fn a_symlink_path_is_followed_to_its_target_directory() {
        let mut server = mockito::Server::new();
        let base = server.url();
        let alias = "plugins/tanstack-all/skills/tanstack-table";
        let real = "plugins/tanstack-table/skills/tanstack-table";
        let _alias = server
            .mock(
                "GET",
                format!("/repos/o/r/contents/{alias}?ref=main").as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(symlink_object(
                alias,
                "../../tanstack-table/skills/tanstack-table",
            ))
            .create();
        let _real = server
            .mock(
                "GET",
                format!("/repos/o/r/contents/{real}?ref=main").as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!("[{}]", listing_file(&base, real, "SKILL.md")))
            .create();
        let _raw = server
            .mock("GET", format!("/raw/{real}/SKILL.md").as_str())
            .with_status(200)
            .with_body("---\nname: tanstack-table\n---\nthe real skill\n")
            .create();
        let dest = tempfile::tempdir().unwrap();

        download_github_directory_from(&base, "o", "r", "main", alias, dest.path(), None, None)
            .expect("the link is followed");

        assert_eq!(
            std::fs::read_to_string(dest.path().join("SKILL.md")).expect("SKILL.md landed"),
            "---\nname: tanstack-table\n---\nthe real skill\n"
        );
    }

    /// An absolute target and a target that climbs out of the repository are
    /// each refused with the typed condition, and the API is never asked for
    /// anything beyond the link itself.
    #[test]
    fn an_escaping_symlink_target_is_refused_typed_and_never_requested() {
        for target in ["/etc/skills", "../../../../outside"] {
            let mut server = mockito::Server::new();
            let base = server.url();
            let _alias = server
                .mock("GET", "/repos/o/r/contents/skills/x?ref=main")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(symlink_object("skills/x", target))
                .create();
            // Mocks match in definition order: anything the alias mock did
            // not answer is a read past the refusal.
            let nothing_else = server.mock("GET", mockito::Matcher::Any).expect(0).create();
            let dest = tempfile::tempdir().unwrap();

            let err = download_github_directory_from(
                &base,
                "o",
                "r",
                "main",
                "skills/x",
                dest.path(),
                None,
                None,
            )
            .expect_err("the target is refused");

            assert_eq!(
                err.downcast_ref::<SignalError>(),
                Some(&SignalError::SymlinkEscapesRepo {
                    subpath: "skills/x".to_string(),
                    target: target.to_string(),
                }),
                "{target}"
            );
            nothing_else.assert();
            assert!(
                std::fs::read_dir(dest.path()).unwrap().next().is_none(),
                "nothing landed for {target}"
            );
        }
    }

    #[test]
    fn fetch_branch_sha_extracts_sha() {
        let mut server = mockito::Server::new();
        let sha = "abc123def456789012345678901234567890abcd";
        let _m = server
            .mock("GET", "/repos/owner/repo/commits/main")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(r#"{{"sha":"{}","node_id":"xyz"}}"#, sha))
            .create();

        // Override the URL by calling the function with a mock client approach
        // Since fetch_branch_sha uses a hardcoded github.com URL, we test via
        // the internal logic by calling check_github_response + JSON parsing directly
        let client = Client::new();
        let resp = client
            .get(format!("{}/repos/owner/repo/commits/main", server.url()))
            .header("User-Agent", "skills-hub")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .unwrap();
        let resp = check_github_response(resp, "test").unwrap();
        let json: serde_json::Value = resp.json().unwrap();
        let extracted_sha = json.get("sha").and_then(|v| v.as_str()).unwrap();
        assert_eq!(extracted_sha, sha);
    }

    #[test]
    fn fetch_branch_sha_error_on_404() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/repos/owner/repo/commits/nonexistent")
            .with_status(404)
            .with_body("not found")
            .create();

        let client = Client::new();
        let resp = client
            .get(format!(
                "{}/repos/owner/repo/commits/nonexistent",
                server.url()
            ))
            .header("User-Agent", "skills-hub")
            .send()
            .unwrap();
        let err = check_github_response(resp, "test").unwrap_err();
        assert_eq!(
            err.downcast_ref::<GithubApiError>().map(|e| e.status),
            Some(404)
        );
    }
}
