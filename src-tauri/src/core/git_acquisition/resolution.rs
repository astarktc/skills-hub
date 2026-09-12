//! Source decisions are private to acquisition: supplied selections are final,
//! stored suffixes are hints, and only a missing hinted branch permits repair.
use super::{
    FastPathFailure, FastPathStage, GitSource, GitSourceResolution, GithubApi, GithubApiError,
    GithubCoords, Result, SignalError, SkillIntent,
};

#[derive(Clone, Debug)]
pub(super) enum Intent {
    Subpath(String),
    ByName(Option<String>),
    Legacy(String),
    Listing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TreeSplit {
    Parser,
    StoredHint,
    MatchingRefs,
}

pub(super) struct Resolved {
    pub source: GitSource,
    pub intent: Intent,
    split: TreeSplit,
}

impl Resolved {
    pub fn new(source: &GitSource, intent: SkillIntent, api: &dyn GithubApi) -> Self {
        let hint = match intent {
            SkillIntent::StoredRecord { subpath, .. } => subpath,
            _ => None,
        };
        let (source, split) = match intent {
            SkillIntent::Selection(super::GitSelection {
                resolution: Some(resolution),
                ..
            }) => {
                let mut source = source.clone();
                // None is deliberate: do not re-assume the parser's branch.
                source.branch = resolution.branch.clone();
                source.subpath = resolution.subpath.clone();
                (source, TreeSplit::Parser)
            }
            _ => resolve_tree_source(source, hint, api),
        };
        let intent = match intent {
            SkillIntent::StoredRecord {
                subpath: Some(path),
                ..
            }
            | SkillIntent::Selection(super::GitSelection {
                subpath: Some(path),
                ..
            }) => Intent::Subpath(path.into()),
            SkillIntent::StoredRecord {
                name,
                subpath: None,
            } => Intent::Legacy(name.into()),
            SkillIntent::ByName(name) => Intent::ByName(name.map(str::to_string)),
            SkillIntent::Selection(_) => Intent::Listing,
        };
        // A blob root is explicit, unlike a branch consuming the whole URL.
        let intent = if source.subpath.as_deref() == Some(".")
            && matches!(intent, Intent::ByName(_) | Intent::Legacy(_))
        {
            Intent::Subpath(".".into())
        } else {
            intent
        };
        Self {
            source,
            intent,
            split,
        }
    }

    pub fn coordinates(&self) -> GitSourceResolution {
        GitSourceResolution {
            branch: self.source.branch.clone(),
            subpath: self.source.subpath.clone(),
        }
    }

    pub fn known_subpath(&self) -> Option<&str> {
        match &self.intent {
            Intent::Subpath(path) => Some(path.as_str()),
            _ => self.source.subpath.as_deref(),
        }
        .filter(|path| !path.is_empty() && *path != ".")
    }

    /// A typed refusal is final. None means clone fallback; Some means the
    /// one allowed stored-hint repair, already resolved (never re-resolve it).
    pub fn after_failure(
        &self,
        original: &GitSource,
        failure: FastPathFailure,
        coords: &GithubCoords,
        api: &dyn GithubApi,
    ) -> Result<Option<Self>> {
        if self.split == TreeSplit::StoredHint
            && failure.stage == FastPathStage::Sha
            && matches!(
                failure.error.downcast_ref::<GithubApiError>(),
                Some(GithubApiError { status: 404, .. })
            )
        {
            let (source, split) = resolve_tree_source(original, None, api);
            if split == TreeSplit::MatchingRefs {
                let intent = Intent::Subpath(source.subpath.clone().unwrap_or_else(|| ".".into()));
                return Ok(Some(Self {
                    source,
                    split,
                    intent,
                }));
            }
        }
        classify_fast_path_failure(failure, coords, self.source.branch.is_none())?;
        Ok(None)
    }
}

/// Discovery is best-effort; only byte-acquisition stages raise GitHub signals.
fn resolve_tree_source(
    source: &GitSource,
    stored_subpath: Option<&str>,
    api: &dyn GithubApi,
) -> (GitSource, TreeSplit) {
    let mut resolved = source.clone();
    let (Some(repo), Some(first), Some(rest)) = (&source.api, &source.branch, &source.subpath)
    else {
        return (resolved, TreeSplit::Parser);
    };
    if rest.is_empty() || rest == "." {
        return (resolved, TreeSplit::Parser);
    }
    let tree_path = format!("{first}/{rest}");
    if let Some(subpath) = stored_subpath.filter(|path| !path.is_empty() && *path != ".") {
        let branch = tree_path.strip_suffix(&format!("/{subpath}"));
        if let Some(branch) = branch.filter(|branch| !branch.is_empty()) {
            resolved.branch = Some(branch.to_string());
            resolved.subpath = Some(subpath.to_string());
            return (resolved, TreeSplit::StoredHint);
        }
    }
    let refs = match api.matching_refs(repo, first) {
        Ok(refs) => refs,
        Err(err) => {
            log::debug!("[acquire] branch discovery failed; keeping first-segment split: {err:#}");
            return (resolved, TreeSplit::Parser);
        }
    };
    let branch = refs
        .iter()
        .filter(|branch| {
            !branch.is_empty()
                && (tree_path == **branch || tree_path.starts_with(&format!("{branch}/")))
        })
        .max_by_key(|branch| branch.len());
    if let Some(branch) = branch {
        resolved.branch = Some(branch.clone());
        resolved.subpath = tree_path
            .strip_prefix(&format!("{branch}/"))
            .map(str::to_string);
        (resolved, TreeSplit::MatchingRefs)
    } else {
        log::debug!("[acquire] no matching branch for {tree_path}; keeping first-segment split");
        (resolved, TreeSplit::Parser)
    }
}

fn classify_fast_path_failure(
    failure: FastPathFailure,
    coords: &GithubCoords,
    branch_assumed: bool,
) -> Result<()> {
    let FastPathFailure { stage, error: err } = failure;
    if err.downcast_ref::<SignalError>().is_some() {
        return Err(err);
    }
    match err.downcast_ref::<GithubApiError>() {
        Some(GithubApiError { status: 404, .. })
            if stage == FastPathStage::Sha && branch_assumed =>
        {
            log::warn!(
                "[acquire] assumed branch {:?} does not exist in {}/{}, falling back to git clone",
                coords.branch,
                coords.owner,
                coords.repo
            );
            Ok(())
        }
        Some(GithubApiError { status: 404, .. }) => {
            anyhow::bail!(SignalError::GithubSkillNotFound {
                url: coords.tree_url()
            })
        }
        Some(GithubApiError {
            status: 403,
            reset_minutes,
            ..
        }) => anyhow::bail!(SignalError::RateLimited {
            reset_minutes: reset_minutes.unwrap_or(0)
        }),
        _ => {
            log::warn!("[acquire] GitHub API download failed, falling back to git clone: {err:#}");
            Ok(())
        }
    }
}
