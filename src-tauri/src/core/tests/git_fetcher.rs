use std::fs;

use crate::core::git_fetcher::{clone_or_pull, clone_or_pull_sparse};

fn commit_file(repo: &git2::Repository, path: &str, content: &[u8], msg: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("workdir");
    let file_path = workdir.join(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&file_path, content).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(path)).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let sig = git2::Signature::now("t", "t@example.com").unwrap();
    let parents = match repo.head() {
        Ok(head) => vec![repo.find_commit(head.target().unwrap()).unwrap()],
        Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, parent_refs.as_slice())
        .unwrap()
}

#[test]
fn clone_then_pull_updates_head() {
    let origin_dir = tempfile::tempdir().unwrap();
    let origin = git2::Repository::init(origin_dir.path()).unwrap();
    let _c1 = commit_file(&origin, "a.txt", b"v1", "c1");
    let c2 = commit_file(&origin, "a.txt", b"v2", "c2");

    let dest_dir = tempfile::tempdir().unwrap();
    let dest = dest_dir.path().join("clone");

    let h1 = clone_or_pull(
        origin_dir.path().to_string_lossy().as_ref(),
        &dest,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        h1,
        c2.to_string(),
        "first clone should point at the latest commit"
    );

    let c3 = commit_file(&origin, "b.txt", b"v3", "c3");
    let h2 = clone_or_pull(
        origin_dir.path().to_string_lossy().as_ref(),
        &dest,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        h2,
        c3.to_string(),
        "a second call should update to the latest commit"
    );
}

#[test]
fn sparse_clone_only_materializes_requested_subpath() {
    let origin_dir = tempfile::tempdir().unwrap();
    let origin = git2::Repository::init(origin_dir.path()).unwrap();
    let _ = commit_file(&origin, "skills/a/SKILL.md", b"---\nname: A\n---\n", "c1");
    let _ = commit_file(&origin, "skills/b/SKILL.md", b"---\nname: B\n---\n", "c2");

    let dest_dir = tempfile::tempdir().unwrap();
    let dest = dest_dir.path().join("clone");

    let head = match clone_or_pull_sparse(
        origin_dir.path().to_string_lossy().as_ref(),
        &dest,
        None,
        &["skills/a"],
        None,
    ) {
        Ok(head) => head,
        Err(err) if format!("{:#}", err).contains("system git is required") => return,
        Err(err) => panic!("sparse clone failed: {:#}", err),
    };

    assert!(!head.is_empty());
    assert!(dest.join("skills/a/SKILL.md").exists());
    assert!(
        !dest.join("skills/b/SKILL.md").exists(),
        "unrequested subdirectories should not be checked out into the workspace"
    );
}

// ---------------------------------------------------------------------------
// The checkout policy under an ambient `core.autocrlf=true`
// ---------------------------------------------------------------------------
//
// Git for Windows ships `core.autocrlf=true` in its system config, which
// checks every LF text file out as CRLF. These tests give a whole process
// that ambient config and prove the app's checkouts keep the committed
// bytes anyway. Process-wide config is only safe in a process of its own —
// the tests of one binary share one environment and libgit2 one config
// search path — so each runs in a re-executed child of this test binary.

/// Set only in such a child: the directory holding its global git config.
const AMBIENT_CHILD_ENV: &str = "SKILLS_HUB_TEST_AMBIENT_AUTOCRLF_HOME";

/// The child's global config directory, when this process is such a child.
fn ambient_child_home() -> Option<std::path::PathBuf> {
    std::env::var_os(AMBIENT_CHILD_ENV).map(std::path::PathBuf::from)
}

/// Run the tests `filter` selects in a child of this test binary whose
/// git config says `core.autocrlf=true` everywhere: the CLI through
/// `GIT_CONFIG_COUNT` (command scope, which outranks every config file — the
/// strongest ambient setting the app must beat) and libgit2 through a
/// global config file the child points its search path at. Asserts the
/// child ran at least one test, so a filter that matches nothing cannot
/// pass.
fn run_in_ambient_autocrlf_child(filter: &str, exact: bool) {
    let home = tempfile::tempdir().expect("tempdir");
    fs::write(
        home.path().join(".gitconfig"),
        "[core]\n\tautocrlf = true\n",
    )
    .expect("write the ambient global config");

    let mut cmd = std::process::Command::new(std::env::current_exe().expect("test binary"));
    cmd.arg(filter);
    if exact {
        cmd.arg("--exact");
    }
    let out = cmd
        .env(AMBIENT_CHILD_ENV, home.path())
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.autocrlf")
        .env("GIT_CONFIG_VALUE_0", "true")
        .output()
        .expect("run the child test binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "child `{filter}` failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ran: usize = stdout
        .lines()
        .find_map(|line| {
            line.strip_prefix("running ")?
                .split(' ')
                .next()?
                .parse()
                .ok()
        })
        .expect("the child reports how many tests it runs");
    assert!(ran > 0, "child `{filter}` ran no test:\n{stdout}");
}

fn commit_lf_skills(repo: &git2::Repository) {
    commit_file(repo, "skills/a/SKILL.md", b"---\nname: a\n---\nbody\n", "a");
    commit_file(
        repo,
        "skills/a/scripts/run.sh",
        b"#!/bin/sh\necho a\n",
        "a script",
    );
    commit_file(repo, "skills/b/SKILL.md", b"---\nname: b\n---\n", "b");
}

fn read(path: &std::path::Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

/// Both transports, a nested sparse checkout and an in-place update keep
/// the committed LF bytes under an ambient `core.autocrlf=true`.
#[test]
fn checkouts_keep_committed_line_endings_under_an_ambient_autocrlf_true() {
    if ambient_child_home().is_some() {
        return;
    }
    run_in_ambient_autocrlf_child(
        "core::git_fetcher::tests::ambient_child_checkouts_keep_committed_line_endings",
        true,
    );
}

/// Runs only inside [`run_in_ambient_autocrlf_child`]; a no-op otherwise.
#[test]
fn ambient_child_checkouts_keep_committed_line_endings() {
    let Some(home) = ambient_child_home() else {
        return;
    };
    // SAFETY: this process is a child running this one test alone, so no
    // other thread reads libgit2's config search path.
    unsafe { git2::opts::set_search_path(git2::ConfigLevel::Global, home.as_path()) }
        .expect("point libgit2 at the ambient global config");

    let origin_dir = tempfile::tempdir().unwrap();
    let origin = git2::Repository::init(origin_dir.path()).unwrap();
    commit_lf_skills(&origin);
    let url = origin_dir.path().to_string_lossy().to_string();
    let work = tempfile::tempdir().unwrap();
    let crlf_skill = b"---\r\nname: a\r\n---\r\nbody\r\n".to_vec();
    let lf_skill = b"---\nname: a\n---\nbody\n".to_vec();

    // Controls: the ambient setting is live for both transports.
    let plain_cli = work.path().join("plain-cli");
    let out = std::process::Command::new("git")
        .args(["clone", "-q"])
        .arg(&url)
        .arg(&plain_cli)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(read(&plain_cli.join("skills/a/SKILL.md")), crlf_skill);
    let plain_lib = work.path().join("plain-libgit2");
    git2::Repository::clone(&url, &plain_lib).expect("plain libgit2 clone");
    assert_eq!(read(&plain_lib.join("skills/a/SKILL.md")), crlf_skill);

    // The CLI: a full clone, a nested sparse checkout.
    let full = work.path().join("cli-full");
    clone_or_pull(&url, &full, None, None).expect("cli clone");
    assert_eq!(read(&full.join("skills/a/SKILL.md")), lf_skill);
    assert_eq!(
        read(&full.join("skills/a/scripts/run.sh")),
        b"#!/bin/sh\necho a\n"
    );
    let sparse = work.path().join("cli-sparse");
    clone_or_pull_sparse(&url, &sparse, None, &["skills/a/scripts"], None).expect("sparse clone");
    assert_eq!(
        read(&sparse.join("skills/a/scripts/run.sh")),
        b"#!/bin/sh\necho a\n"
    );
    assert!(!sparse.join("skills/b").exists());

    // libgit2: a fresh clone.
    let lib = work.path().join("libgit2");
    super::clone_or_pull_via_libgit2(&url, &lib, None).expect("libgit2 clone");
    assert_eq!(read(&lib.join("skills/a/SKILL.md")), lf_skill);
    assert_eq!(
        read(&lib.join("skills/a/scripts/run.sh")),
        b"#!/bin/sh\necho a\n"
    );

    // An upstream change, pulled in place by every checkout above.
    let changed = b"---\nname: a\n---\nchanged\n".to_vec();
    commit_file(&origin, "skills/a/SKILL.md", &changed, "change a");
    clone_or_pull(&url, &full, None, None).expect("cli pull");
    assert_eq!(read(&full.join("skills/a/SKILL.md")), changed);
    clone_or_pull_sparse(&url, &sparse, None, &["skills/a"], None).expect("sparse widen");
    assert_eq!(read(&sparse.join("skills/a/SKILL.md")), changed);
    super::clone_or_pull_via_libgit2(&url, &lib, None).expect("libgit2 pull");
    assert_eq!(read(&lib.join("skills/a/SKILL.md")), changed);
}

/// A cache entry a release before the checkout policy checked out under an
/// ambient `core.autocrlf=true` (CRLF bytes, a fresh record) is never served
/// and never touched: the acquisition clones a new entry under the current
/// policy's key, whose bytes and content identity are the commit's, and the
/// old entry stays on disk byte for byte until the cache cleanup ages it out.
#[test]
fn a_pre_policy_cache_entry_is_left_alone_under_an_ambient_autocrlf_true() {
    if ambient_child_home().is_some() {
        return;
    }
    run_in_ambient_autocrlf_child(
        "core::git_fetcher::tests::ambient_child_a_pre_policy_cache_entry_is_left_alone",
        true,
    );
}

/// Every file under `dir`, `.git` included, with its bytes.
fn snapshot(dir: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        into: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read_dir {dir:?}: {err}")) {
            let path = entry.unwrap().path();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_dir() {
                walk(root, &path, into);
            } else {
                into.insert(path.strip_prefix(root).unwrap().to_path_buf(), read(&path));
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    walk(dir, dir, &mut files);
    files
}

/// Runs only inside [`run_in_ambient_autocrlf_child`]; a no-op otherwise.
#[test]
fn ambient_child_a_pre_policy_cache_entry_is_left_alone() {
    use crate::core::content_identity::{read as identity, Source};
    use crate::core::git_cache::{
        fetch_through_cache, repo_cache_key, repo_cache_key_under_policy, CacheKeyInputs,
        FetchRequest,
    };

    if ambient_child_home().is_none() {
        return;
    }
    let origin_dir = tempfile::tempdir().unwrap();
    let origin = git2::Repository::init(origin_dir.path()).unwrap();
    commit_lf_skills(&origin);
    let url = origin_dir.path().to_string_lossy().to_string();
    let head = origin.head().unwrap().target().unwrap().to_string();
    let inputs = CacheKeyInputs {
        clone_url: &url,
        branch: None,
    };

    // The entry as the previous release left it, at the name it computed
    // (policy 0 is the shipped key, pinned in the git-cache tests): an
    // unpinned clone, then a fresh record in that release's format.
    let cache = tempfile::tempdir().unwrap();
    let cache_root = cache.path().join("skills-hub-git-cache");
    let old_entry = cache_root.join(repo_cache_key_under_policy(&inputs, 0));
    let out = std::process::Command::new("git")
        .args(["clone", "-q"])
        .arg(&url)
        .arg(&old_entry)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        read(&old_entry.join("skills/a/SKILL.md")),
        b"---\r\nname: a\r\n---\r\nbody\r\n",
        "control: the old entry holds CRLF"
    );
    fs::write(
        old_entry.join(".skills-hub-cache.json"),
        serde_json::json!({
            "last_fetched_ms": crate::core::clock::now_ms(),
            "head": head,
            "checkout": { "kind": "full" },
        })
        .to_string(),
    )
    .unwrap();
    let committed = identity(Source::Directory(&origin_dir.path().join("skills/a")));
    assert!(committed.is_some());
    let old_identity = identity(Source::Directory(&old_entry.join("skills/a")));
    assert_ne!(
        old_identity, committed,
        "control: CRLF bytes are another content identity"
    );
    let old_bytes = snapshot(&old_entry);

    let (dir, served) = fetch_through_cache(
        cache.path(),
        &crate::core::removal_fence::ProtectedLibrary::unrecorded(),
        &FetchRequest {
            clone_url: &url,
            branch: None,
            subpath: Some("skills/a"),
            ttl_ms: 10 * 60 * 1000,
            cancel: None,
        },
    )
    .expect("fetch through the cache");

    assert_eq!(
        dir,
        cache_root.join(repo_cache_key(&inputs)),
        "served from the current policy's entry"
    );
    assert_ne!(dir, old_entry, "the pre-policy entry is never served");
    assert_eq!(served, head);
    assert_eq!(
        read(&dir.join("skills/a/SKILL.md")),
        b"---\nname: a\n---\nbody\n"
    );
    assert_eq!(
        identity(Source::Directory(&dir.join("skills/a"))),
        committed
    );
    assert_eq!(
        snapshot(&old_entry),
        old_bytes,
        "the pre-policy entry is left on disk byte for byte, record and .git included"
    );
    assert_eq!(
        identity(Source::Directory(&old_entry.join("skills/a"))),
        old_identity
    );
}

/// The whole acquisition suite — sparse and nested acquisitions, symlinked
/// subpaths, clone fallbacks, byte comparisons against the committed files —
/// passes with the CLI under an ambient `core.autocrlf=true`, as it does on
/// Windows.
#[test]
fn the_acquisition_suite_passes_under_an_ambient_autocrlf_true() {
    if ambient_child_home().is_some() {
        return;
    }
    run_in_ambient_autocrlf_child("core::git_acquisition::tests::", false);
}
