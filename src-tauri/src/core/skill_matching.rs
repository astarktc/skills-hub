//! The one definition of "does skill candidate X match target name Y".
//!
//! Used by every backend site that resolves a skill name against a list of
//! discovered candidates (Explore install, legacy-record update backfill,
//! `fetch_skill_files`) and exposed to the frontend through the git listing
//! command, so no matching policy lives in TypeScript.
//!
//! Rule: case-insensitive, tiered. Exact SKILL.md name, then bidirectional
//! containment on the name, then the same two tests on the candidate's
//! directory name. The first tier with any hit decides: one hit resolves,
//! several are ambiguous, none at all is no match.

use serde::Serialize;
use ts_rs::TS;

/// Anything with a skill name and a discovery-root-relative subpath.
pub trait MatchableSkill {
    /// `SKILL.md` `name` (else the folder name) — see `DiscoveredSkill::name`.
    fn name(&self) -> &str;
    /// Path relative to the discovery root (`/` or `\` separated).
    fn subpath(&self) -> &str;
    /// Last component of the subpath (the directory's own name).
    fn dir_name(&self) -> &str {
        dir_name_of(self.subpath())
    }
}

/// Last path component of a `/`- or `\`-separated subpath.
pub fn dir_name_of(subpath: &str) -> &str {
    subpath.rsplit(['/', '\\']).next().unwrap_or(subpath)
}

/// Outcome of matching one target name against a candidate list.
#[derive(Debug, PartialEq, Eq)]
pub enum SkillMatch<'a, T> {
    /// Exactly one candidate matched in the deciding tier.
    Resolved(&'a T),
    /// Several candidates matched in the deciding tier, in input order.
    Ambiguous(Vec<&'a T>),
    /// No tier produced a hit (also for a blank target).
    None,
}

/// Wire form of [`SkillMatch`]: candidates are referenced by subpath, the
/// identity the frontend already keys its selection by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum CandidateMatch {
    Resolved { subpath: String },
    Ambiguous { subpaths: Vec<String> },
    None,
}

impl<T: MatchableSkill> From<SkillMatch<'_, T>> for CandidateMatch {
    fn from(value: SkillMatch<'_, T>) -> Self {
        match value {
            SkillMatch::Resolved(c) => CandidateMatch::Resolved {
                subpath: c.subpath().to_string(),
            },
            SkillMatch::Ambiguous(list) => CandidateMatch::Ambiguous {
                subpaths: list.into_iter().map(|c| c.subpath().to_string()).collect(),
            },
            SkillMatch::None => CandidateMatch::None,
        }
    }
}

/// Match `target` against `candidates` (see the module doc for the rule).
pub fn match_skill_candidate<'a, T: MatchableSkill>(
    target: &str,
    candidates: &'a [T],
) -> SkillMatch<'a, T> {
    let target = target.trim().to_lowercase();
    if target.is_empty() {
        return SkillMatch::None;
    }
    let exact = |value: &str| value.to_lowercase() == target;
    let contains = |value: &str| {
        let value = value.to_lowercase();
        value.contains(&target) || target.contains(&value)
    };
    let tiers: [&dyn Fn(&T) -> bool; 4] = [
        &|c| exact(c.name()),
        &|c| contains(c.name()),
        &|c| exact(c.dir_name()),
        &|c| contains(c.dir_name()),
    ];
    for tier in tiers {
        let mut hits = candidates.iter().filter(|c| tier(c));
        let Some(first) = hits.next() else {
            continue;
        };
        let rest: Vec<&T> = hits.collect();
        if rest.is_empty() {
            return SkillMatch::Resolved(first);
        }
        let mut all = Vec::with_capacity(rest.len() + 1);
        all.push(first);
        all.extend(rest);
        return SkillMatch::Ambiguous(all);
    }
    SkillMatch::None
}

#[cfg(test)]
#[path = "tests/skill_matching.rs"]
mod tests;
