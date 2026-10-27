//! Download a GitHub directory via the Contents API, bypassing git clone entirely.
//! This is much faster than cloning large repos when only a subdirectory is needed.
//!
//! **Executable bits.** The Contents API carries no file modes, so every file
//! lands with the default mode. After the download, *one* request,
//! `GET /repos/{owner}/{repo}/git/trees/{ref}?recursive=1` (same ref, token
//! and error classification as the other GitHub GETs), supplies them: a
//! written file whose tree entry (looked up by its real repo path, so a
//! followed upstream link maps correctly) has mode `100755` gains `+x`.
//! When that call fails for any reason or answers `truncated: true`, or a
//! written file has no tree entry, the fallback gives `+x` to a file whose
//! first bytes are `#!`. Modes never fail an acquisition (the bytes have
//! landed); the fallback is logged. Cost: one extra API request per
//! nonempty Unix fast-path acquisition, outside the per-file loop (none on
//! Windows, where the step is a no-op). The ref is URL-encoded as one path
//! parameter, including branch names containing '/'.

use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};

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

/// Download a directory from a GitHub repo using the Contents API, then
/// restore executable bits (module doc).
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
    let mut written = Vec::new();
    download_dir_recursive(
        &client,
        api_base,
        owner,
        repo,
        branch,
        path,
        dest,
        cancel,
        token,
        &mut chain,
        &mut written,
    )?;
    if cancel.is_some_and(|c| c.is_cancelled()) {
        anyhow::bail!(SignalError::Cancelled);
    }
    if cfg!(unix) && !written.is_empty() {
        let tree_ref = urlencoding::encode(branch);
        let tree_url = format!("{api_base}/repos/{owner}/{repo}/git/trees/{tree_ref}?recursive=1");
        restore_executable_bits(&client, &tree_url, token, &written);
    }
    Ok(())
}

/// One file the Contents download wrote.
struct WrittenFile {
    /// The repo path its tree entry is filed under (`item.path`: the real
    /// path, a followed upstream link already resolved).
    repo_path: String,
    local_path: PathBuf,
    /// Whether the bytes start with `#!` (the fallback rule).
    shebang: bool,
}

/// Give `+x` to every written file the repository records as executable;
/// fall back to the shebang rule when the tree is unavailable or incomplete.
/// Never fails: a mode problem is logged, the acquisition stands.
fn restore_executable_bits(
    client: &Client,
    tree_url: &str,
    token: Option<&str>,
    written: &[WrittenFile],
) {
    let executable_paths = match executable_tree_paths(client, tree_url, token) {
        Ok(paths) => Some(paths),
        Err(err) => {
            log::warn!(
                "[github_download] file modes unavailable, marking shebang files executable instead: {err:#}"
            );
            None
        }
    };
    for file in written {
        let executable = match &executable_paths {
            Some(paths) if paths.blobs.contains(&file.repo_path) => {
                paths.executable.contains(&file.repo_path)
            }
            Some(_) => {
                log::warn!(
                    "[github_download] no tree entry for {}, using the shebang rule",
                    file.repo_path
                );
                file.shebang
            }
            None => file.shebang,
        };
        if executable {
            if let Err(err) = add_execute_bits(&file.local_path) {
                log::warn!(
                    "[github_download] could not mark {:?} executable: {err:#}",
                    file.local_path
                );
            }
        }
    }
}

/// The blob paths of a complete recursive tree, and which of them are
/// executable (`100755`).
struct TreeModes {
    blobs: HashSet<String>,
    executable: HashSet<String>,
}

/// `GET git/trees/{ref}?recursive=1`; a truncated answer is an error (the
/// caller falls back rather than trusting a partial listing).
fn executable_tree_paths(client: &Client, url: &str, token: Option<&str>) -> Result<TreeModes> {
    #[derive(Deserialize)]
    struct Tree {
        tree: Vec<TreeEntry>,
        #[serde(default)]
        truncated: bool,
    }
    #[derive(Deserialize)]
    struct TreeEntry {
        path: String,
        mode: String,
        #[serde(rename = "type")]
        entry_type: String,
    }

    let mut req = client
        .get(url)
        .header("User-Agent", "skills-hub")
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }
    let resp = req
        .send()
        .with_context(|| format!("request GitHub tree: {url}"))?;
    let tree: Tree = check_github_response(resp, url)?
        .json()
        .with_context(|| format!("parse GitHub tree response: {url}"))?;
    if tree.truncated {
        anyhow::bail!("GitHub tree response truncated: {url}");
    }
    let mut modes = TreeModes {
        blobs: HashSet::new(),
        executable: HashSet::new(),
    };
    for entry in tree.tree.into_iter().filter(|e| e.entry_type == "blob") {
        if entry.mode == "100755" {
            modes.executable.insert(entry.path.clone());
        }
        modes.blobs.insert(entry.path);
    }
    Ok(modes)
}

/// `mode | 0o111` on Unix; a no-op elsewhere.
fn add_execute_bits(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
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
    written: &mut Vec<WrittenFile>,
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
                written,
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
                    written.push(WrittenFile {
                        repo_path: item.path.clone(),
                        shebang: bytes.starts_with(b"#!"),
                        local_path,
                    });
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
                    written,
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

/// Discover branch names for an ambiguous GitHub tree URL. Acquisition owns
/// the best-effort policy; HTTP failures retain the same classification as SHA lookup.
pub fn fetch_matching_refs(
    owner: &str,
    repo: &str,
    prefix: &str,
    token: Option<&str>,
) -> Result<Vec<String>> {
    let url =
        format!("https://api.github.com/repos/{owner}/{repo}/git/matching-refs/heads/{prefix}");
    matching_refs_at(&url, token)
}

/// Shared request policy for the lightweight GitHub metadata GETs.
fn github_get(
    url: &str,
    token: Option<&str>,
    operation: &str,
) -> Result<reqwest::blocking::Response> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("build HTTP client")?;
    let mut req = client
        .get(url)
        .header("User-Agent", "skills-hub")
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let resp = req
        .send()
        .with_context(|| format!("request {operation}: {url}"))?;
    check_github_response(resp, url)
}

fn matching_refs_at(url: &str, token: Option<&str>) -> Result<Vec<String>> {
    let resp = github_get(url, token, "matching refs")?;
    #[derive(serde::Deserialize)]
    struct BranchRef {
        r#ref: String,
    }
    let refs: Vec<BranchRef> = resp
        .json()
        .with_context(|| format!("parse matching refs: {url}"))?;
    Ok(refs
        .into_iter()
        .filter_map(|entry| entry.r#ref.strip_prefix("refs/heads/").map(str::to_string))
        .collect())
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
    let url = format!(
        "https://api.github.com/repos/{}/{}/commits/{}",
        owner, repo, branch
    );

    let resp = github_get(&url, token, "branch SHA")?;

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
    fn matching_refs_uses_auth_and_extracts_only_heads() {
        let mut server = mockito::Server::new();
        let mock = server.mock("GET", "/repos/o/r/git/matching-refs/heads/feature")
            .match_header("authorization", "Bearer test-token")
            .match_header("user-agent", "skills-hub")
            .match_header("accept", "application/vnd.github.v3+json")
            .with_status(200)
            .with_body(r#"[{"ref":"refs/heads/feature"},{"ref":"refs/heads/feature/x"},{"ref":"refs/tags/feature"}]"#)
            .expect(1).create();
        let refs = matching_refs_at(
            &format!("{}/repos/o/r/git/matching-refs/heads/feature", server.url()),
            Some("test-token"),
        )
        .unwrap();
        assert_eq!(refs, ["feature", "feature/x"]);
        mock.assert();
    }

    #[test]
    fn credential_store_failure_sends_no_bearer_to_github() {
        let credentials = crate::core::credentials::MemoryStore::failing();
        assert!(crate::core::settings::github_token(&credentials).is_err());
        let token = crate::core::settings::github_token_or_none(&credentials);
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/refs")
            .match_header("authorization", mockito::Matcher::Missing)
            .with_status(200)
            .with_body(r#"[{"ref":"refs/heads/main"}]"#)
            .expect(1)
            .create();

        assert_eq!(
            matching_refs_at(&format!("{}/refs", server.url()), token.as_deref()).unwrap(),
            ["main"]
        );
        mock.assert();
    }

    #[test]
    fn matching_refs_preserves_http_error_classification() {
        let mut server = mockito::Server::new();
        for status in [403, 404, 500] {
            let mock = server
                .mock("GET", "/refs")
                .with_status(status)
                .expect(1)
                .create();
            let err = matching_refs_at(&format!("{}/refs", server.url()), None).unwrap_err();
            assert_eq!(
                err.downcast_ref::<GithubApiError>().unwrap().status,
                status as u16
            );
            mock.assert();
        }
    }

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

    // -----------------------------------------------------------------------
    // Executable bits (the trees call and its shebang fallback)
    // -----------------------------------------------------------------------

    /// Only execute bits belong to this rule; read/write bits obey the caller's umask.
    #[cfg(unix)]
    fn assert_execute_bits(path: &Path, expected_mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .unwrap_or_else(|err| panic!("stat {path:?}: {err}"))
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, expected_mode & 0o111, "{path:?}");
    }

    /// A directory listing entry for a subdirectory.
    #[cfg(unix)]
    fn listing_dir(dir: &str, name: &str) -> String {
        format!(r#"{{"name":"{name}","path":"{dir}/{name}","type":"dir","download_url":null}}"#)
    }

    /// One `git/trees` entry.
    #[cfg(unix)]
    fn tree_entry(path: &str, mode: &str) -> String {
        let kind = if mode == "040000" { "tree" } else { "blob" };
        format!(r#"{{"path":"{path}","mode":"{mode}","type":"{kind}","sha":"0"}}"#)
    }

    /// The skill at `dir` as the Contents API serves it:
    /// - `run.sh`: a shebang script, `100755` upstream;
    /// - `notes.sh`: a shebang file that is *not* executable upstream;
    /// - `README.md`: a plain file;
    /// - `nested/tool`: executable upstream but without a shebang, so only
    ///   the trees answer can know it.
    #[cfg(unix)]
    fn serve_skill(server: &mut mockito::Server, dir: &str) -> Vec<mockito::Mock> {
        let base = server.url();
        let mut mocks = vec![
            server
                .mock(
                    "GET",
                    format!("/repos/o/r/contents/{dir}?ref=main").as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(format!(
                    "[{},{},{},{}]",
                    listing_file(&base, dir, "run.sh"),
                    listing_file(&base, dir, "notes.sh"),
                    listing_file(&base, dir, "README.md"),
                    listing_dir(dir, "nested"),
                ))
                .create(),
            server
                .mock(
                    "GET",
                    format!("/repos/o/r/contents/{dir}/nested?ref=main").as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(format!(
                    "[{}]",
                    listing_file(&base, &format!("{dir}/nested"), "tool")
                ))
                .create(),
        ];
        for (name, body) in [
            ("run.sh", "#!/bin/sh\necho run\n"),
            ("notes.sh", "#!/bin/sh\necho notes\n"),
            ("README.md", "# readme\n"),
            ("nested/tool", "\x7fELF-ish binary\n"),
        ] {
            mocks.push(
                server
                    .mock("GET", format!("/raw/{dir}/{name}").as_str())
                    .with_status(200)
                    .with_body(body)
                    .create(),
            );
        }
        mocks
    }

    /// The `git/trees` answer for a repository holding the skill at `dir`.
    #[cfg(unix)]
    fn tree_body(dir: &str, truncated: bool) -> String {
        let entries = [
            tree_entry(dir, "040000"),
            tree_entry(&format!("{dir}/run.sh"), "100755"),
            tree_entry(&format!("{dir}/notes.sh"), "100644"),
            tree_entry(&format!("{dir}/README.md"), "100644"),
            tree_entry(&format!("{dir}/nested"), "040000"),
            tree_entry(&format!("{dir}/nested/tool"), "100755"),
        ];
        format!(
            r#"{{"sha":"0","url":"x","tree":[{}],"truncated":{truncated}}}"#,
            entries.join(",")
        )
    }

    #[cfg(unix)]
    fn download_skill(base: &str, path: &str, dest: &Path, token: Option<&str>) {
        download_github_directory_from(base, "o", "r", "main", path, dest, None, token)
            .expect("the acquisition succeeds");
    }

    /// The faithful path: one trees request, with the download's ref and
    /// token, decides every written file's execute bits — including a file
    /// in a nested directory and a file only the tree knows is executable;
    /// a shebang file the tree records as plain stays plain.
    #[cfg(unix)]
    #[test]
    fn the_trees_answer_restores_executable_bits_on_every_written_file() {
        let mut server = mockito::Server::new();
        let _skill = serve_skill(&mut server, "skills/a");
        let trees = server
            .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
            .match_header("authorization", "Bearer test-token")
            .match_header("user-agent", "skills-hub")
            .match_header("accept", "application/vnd.github.v3+json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tree_body("skills/a", false))
            .expect(1)
            .create();
        let dest = tempfile::tempdir().unwrap();

        download_skill(&server.url(), "skills/a", dest.path(), Some("test-token"));

        trees.assert();
        assert_execute_bits(&dest.path().join("run.sh"), 0o755);
        assert_execute_bits(&dest.path().join("README.md"), 0o644);
        assert_execute_bits(&dest.path().join("notes.sh"), 0o644);
        assert_execute_bits(&dest.path().join("nested/tool"), 0o755);
    }

    /// Any failed trees call (a server error, a rate-limit 403) falls back to
    /// the shebang rule, and the acquisition still succeeds: the bytes have
    /// landed.
    #[cfg(unix)]
    #[test]
    fn a_failed_trees_call_falls_back_to_shebang_detection() {
        for status in [500, 403, 404] {
            let mut server = mockito::Server::new();
            let _skill = serve_skill(&mut server, "skills/a");
            let trees = server
                .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
                .with_status(status)
                .expect(1)
                .create();
            let dest = tempfile::tempdir().unwrap();

            download_skill(&server.url(), "skills/a", dest.path(), None);

            trees.assert();
            assert_execute_bits(&dest.path().join("run.sh"), 0o755);
            assert_execute_bits(&dest.path().join("notes.sh"), 0o755);
            assert_execute_bits(&dest.path().join("README.md"), 0o644);
            // No shebang, so the fallback cannot know.
            assert_execute_bits(&dest.path().join("nested/tool"), 0o644);
        }
    }

    /// A truncated tree is not trusted: the shebang rule applies instead.
    #[cfg(unix)]
    #[test]
    fn a_truncated_tree_falls_back_to_shebang_detection() {
        let mut server = mockito::Server::new();
        let _skill = serve_skill(&mut server, "skills/a");
        let _trees = server
            .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tree_body("skills/a", true))
            .create();
        let dest = tempfile::tempdir().unwrap();

        download_skill(&server.url(), "skills/a", dest.path(), None);

        assert_execute_bits(&dest.path().join("run.sh"), 0o755);
        assert_execute_bits(&dest.path().join("notes.sh"), 0o755);
        assert_execute_bits(&dest.path().join("README.md"), 0o644);
        assert_execute_bits(&dest.path().join("nested/tool"), 0o644);
    }

    /// Through a followed upstream link, modes map by the real repo path the
    /// bytes came from, not by the alias that was requested.
    #[cfg(unix)]
    #[test]
    fn modes_map_by_the_real_path_behind_a_followed_upstream_link() {
        let mut server = mockito::Server::new();
        let alias = "plugins/all/a";
        let _alias = server
            .mock(
                "GET",
                format!("/repos/o/r/contents/{alias}?ref=main").as_str(),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(symlink_object(alias, "../../skills/a"))
            .create();
        let _skill = serve_skill(&mut server, "skills/a");
        let _trees = server
            .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tree_body("skills/a", false))
            .create();
        let dest = tempfile::tempdir().unwrap();

        download_skill(&server.url(), alias, dest.path(), None);

        // Only the tree entry at the real path knows this file is executable.
        assert_execute_bits(&dest.path().join("nested/tool"), 0o755);
        assert_execute_bits(&dest.path().join("notes.sh"), 0o644);
        assert_execute_bits(&dest.path().join("run.sh"), 0o755);
    }

    /// Malformed JSON and missing entries use the same best-effort fallback.
    #[cfg(unix)]
    #[test]
    fn unusable_tree_data_falls_back_without_failing_the_download() {
        for body in ["not JSON", r#"{"tree":[],"truncated":false}"#] {
            let mut server = mockito::Server::new();
            let _skill = serve_skill(&mut server, "skills/a");
            let trees = server
                .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
                .with_status(200)
                .with_body(body)
                .expect(1)
                .create();
            let dest = tempfile::tempdir().unwrap();

            download_skill(&server.url(), "skills/a", dest.path(), None);

            trees.assert();
            assert_execute_bits(&dest.path().join("run.sh"), 0o755);
            assert_execute_bits(&dest.path().join("notes.sh"), 0o755);
            assert_execute_bits(&dest.path().join("README.md"), 0o644);
            assert_execute_bits(&dest.path().join("nested/tool"), 0o644);
        }
    }

    #[test]
    fn cancellation_during_the_last_file_skips_the_tree_request() {
        let mut server = mockito::Server::new();
        let base = server.url();
        let token = std::sync::Arc::new(CancelToken::new());
        let during_download = token.clone();
        let contents = server
            .mock("GET", "/repos/o/r/contents/skills/a?ref=main")
            .with_status(200)
            .with_body(format!("[{}]", listing_file(&base, "skills/a", "run.sh")))
            .expect(1)
            .create();
        let raw = server
            .mock("GET", "/raw/skills/a/run.sh")
            .with_status(200)
            .with_body_from_request(move |_| {
                during_download.cancel();
                b"#!/bin/sh\necho run\n".to_vec()
            })
            .expect(1)
            .create();
        let trees = server
            .mock("GET", "/repos/o/r/git/trees/main?recursive=1")
            .expect(0)
            .create();
        let dest = tempfile::tempdir().unwrap();

        let err = download_github_directory_from(
            &base,
            "o",
            "r",
            "main",
            "skills/a",
            dest.path(),
            Some(&token),
            None,
        )
        .expect_err("cancellation is not a best-effort mode failure");

        assert_eq!(
            err.downcast_ref::<SignalError>(),
            Some(&SignalError::Cancelled)
        );
        assert!(dest.path().join("run.sh").exists(), "the last file landed");
        contents.assert();
        raw.assert();
        trees.assert();
    }

    /// A ref is one Trees path parameter, even when the branch contains '/'.
    #[cfg(unix)]
    #[test]
    fn a_slash_branch_uses_an_encoded_tree_ref() {
        let mut server = mockito::Server::new();
        let base = server.url();
        let contents = server
            .mock("GET", "/repos/o/r/contents/skills/a?ref=release/1.x")
            .with_status(200)
            .with_body(format!("[{}]", listing_file(&base, "skills/a", "tool")))
            .expect(1)
            .create();
        let raw = server
            .mock("GET", "/raw/skills/a/tool")
            .with_status(200)
            .with_body("no shebang: only the tree can mark this executable")
            .expect(1)
            .create();
        let trees = server
            .mock("GET", "/repos/o/r/git/trees/release%2F1.x?recursive=1")
            .with_status(200)
            .with_body(format!(
                r#"{{"tree":[{}],"truncated":false}}"#,
                tree_entry("skills/a/tool", "100755")
            ))
            .expect(1)
            .create();
        let dest = tempfile::tempdir().unwrap();

        download_github_directory_from(
            &base,
            "o",
            "r",
            "release/1.x",
            "skills/a",
            dest.path(),
            None,
            None,
        )
        .unwrap();

        assert_execute_bits(&dest.path().join("tool"), 0o755);
        contents.assert();
        raw.assert();
        trees.assert();
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
