//! Repo-relative subpath rules shared by everything that names a path inside
//! a git checkout.
//!
//! One normaliser serves the git cache's coverage record and the sparse
//! fetcher's pattern set, so the two agree by construction on what "the same
//! subpath" is. One link resolver ([`LinkChain`]) serves both acquisition
//! adapters (the sparse clone and the GitHub Contents API), so an upstream
//! that publishes a skill as an in-repo symlink is followed by one bounded
//! rule with one typed refusal, wherever the bytes come from.

use std::path::Path;

use anyhow::Result;

use super::errors::SignalError;

/// How many symlink hops acquisition follows before refusing: enough for any
/// aggregation bundle (one hop in the wild), small enough that a link cycle
/// is an error, not a hang.
pub(crate) const MAX_LINK_DEPTH: usize = 8;

/// The canonical spelling of a repo-relative subpath: `/`-separated, no
/// leading or trailing separator, no empty or `.` segments. `..` is not
/// interpreted here — a subpath is a name, not a traversal.
pub(crate) fn normalize_subpath(subpath: &str) -> String {
    subpath
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// One acquisition's walk along upstream symlinks. Each [`follow`] resolves a
/// link's target against the link's own directory, *lexically and within the
/// repository root* — the checkout's real filesystem is never consulted, so a
/// target is judged before anything at it is read — and counts the hop
/// against [`MAX_LINK_DEPTH`].
///
/// [`follow`]: LinkChain::follow
#[derive(Debug, Default)]
pub(crate) struct LinkChain {
    hops: usize,
}

impl LinkChain {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The repo-relative path the link at `link_subpath` points to, given its
    /// raw `target`.
    ///
    /// Refuses, typed ([`SignalError::SymlinkEscapesRepo`]), a target that is
    /// absolute or resolves to or above the repository root; refuses, plain,
    /// the hop past the bound. The resolved path is logged as diagnostics — a
    /// caller records the alias it was asked for, never this.
    pub(crate) fn follow(&mut self, link_subpath: &str, target: &str) -> Result<String> {
        self.hops += 1;
        if self.hops > MAX_LINK_DEPTH {
            anyhow::bail!(
                "symlink chain at {link_subpath} exceeds {MAX_LINK_DEPTH} links (target: {target})"
            );
        }
        let refused = || SignalError::SymlinkEscapesRepo {
            subpath: link_subpath.to_string(),
            target: target.to_string(),
        };
        if target.is_empty()
            || target.starts_with('/')
            || target.starts_with('\\')
            || Path::new(target).is_absolute()
        {
            anyhow::bail!(refused());
        }

        // The link's directory, then the target's segments applied to it.
        let link = normalize_subpath(link_subpath);
        let mut segments: Vec<&str> = link.split('/').filter(|s| !s.is_empty()).collect();
        segments.pop();
        for segment in target.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        anyhow::bail!(refused());
                    }
                }
                name => segments.push(name),
            }
        }
        if segments.is_empty() {
            anyhow::bail!(refused());
        }
        let resolved = segments.join("/");
        log::info!(
            "[acquire] followed in-repo symlink (hop {}/{}) {} -> {} = {}",
            self.hops,
            MAX_LINK_DEPTH,
            link_subpath,
            target,
            resolved
        );
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spelling of one subpath normalises to the same string: this is
    /// the property the cache's coverage check and the fetcher's sparse
    /// pattern both rely on.
    #[test]
    fn normalize_subpath_gives_one_spelling_per_path() {
        for spelling in [
            "skills/a",
            "/skills/a",
            "skills/a/",
            "//skills//a//",
            "./skills/./a",
        ] {
            assert_eq!(normalize_subpath(spelling), "skills/a", "{spelling:?}");
        }
    }

    /// The root and its aliases normalise to the empty string; `..` is left
    /// alone because normalising is not resolving.
    #[test]
    fn normalize_subpath_keeps_root_empty_and_parent_segments_intact() {
        assert_eq!(normalize_subpath(""), "");
        assert_eq!(normalize_subpath("."), "");
        assert_eq!(normalize_subpath("/"), "");
        assert_eq!(normalize_subpath("a/../b"), "a/../b");
    }

    /// The upstream case this exists for: `tanstack-skills/tanstack-skills`
    /// publishes `plugins/tanstack-all/skills/tanstack-table` as a link to
    /// `../../tanstack-table/skills/tanstack-table`, which — relative to the
    /// link's own directory — is `plugins/tanstack-table/skills/tanstack-table`.
    #[test]
    fn a_relative_target_resolves_against_the_links_own_directory() {
        let mut chain = LinkChain::new();
        let resolved = chain
            .follow(
                "plugins/tanstack-all/skills/tanstack-table",
                "../../tanstack-table/skills/tanstack-table",
            )
            .expect("an in-repo target resolves");
        assert_eq!(resolved, "plugins/tanstack-table/skills/tanstack-table");
    }

    /// `.` segments and a trailing separator in the target are spelling, not
    /// structure; the answer is canonical.
    #[test]
    fn a_resolved_target_is_normalised() {
        let mut chain = LinkChain::new();
        assert_eq!(chain.follow("a/b/link", "./c/").expect("resolves"), "a/b/c");
    }

    /// An absolute target is refused before anything is read, with the
    /// typed condition naming the link and its target.
    #[test]
    fn an_absolute_target_is_refused_typed() {
        let mut chain = LinkChain::new();
        let err = chain
            .follow("skills/x", "/etc/passwd")
            .expect_err("absolute targets are refused");
        assert_eq!(
            err.downcast_ref::<SignalError>(),
            Some(&SignalError::SymlinkEscapesRepo {
                subpath: "skills/x".to_string(),
                target: "/etc/passwd".to_string(),
            })
        );
    }

    /// A target that climbs above the repository root — or lands exactly on
    /// it — is refused with the same typed condition.
    #[test]
    fn a_target_that_leaves_the_repository_root_is_refused_typed() {
        for (link, target) in [
            ("skills/x", "../../outside"),
            ("skills/x", "../.."),
            ("x", ".."),
            ("skills/x", ".."),
        ] {
            let mut chain = LinkChain::new();
            let err = chain
                .follow(link, target)
                .expect_err("{link} -> {target} must be refused");
            assert!(
                matches!(
                    err.downcast_ref::<SignalError>(),
                    Some(SignalError::SymlinkEscapesRepo { subpath, target: t })
                        if subpath == link && t == target
                ),
                "{link} -> {target}: expected SymlinkEscapesRepo, got {err:#}"
            );
        }
    }

    /// A chain is followed up to [`MAX_LINK_DEPTH`] hops; the hop past the
    /// bound is refused instead of looping forever on a link cycle.
    #[test]
    fn a_chain_past_the_bound_is_refused() {
        let mut chain = LinkChain::new();
        for hop in 0..MAX_LINK_DEPTH {
            chain
                .follow(&format!("l{hop}"), &format!("l{}", hop + 1))
                .unwrap_or_else(|err| panic!("hop {hop} is within the bound: {err:#}"));
        }
        let err = chain
            .follow("l8", "l9")
            .expect_err("the hop past the bound is refused");
        assert!(
            err.downcast_ref::<SignalError>().is_none(),
            "a too-deep chain is not the escape condition: {err:#}"
        );
        assert!(format!("{err:#}").contains("l8"), "names the link: {err:#}");
    }
}
