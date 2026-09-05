//! Repo-relative subpath rules shared by everything that names a path inside
//! a git checkout.
//!
//! One normaliser serves the git cache's coverage record and the sparse
//! fetcher's pattern set, so the two agree by construction on what "the same
//! subpath" is.

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
}
