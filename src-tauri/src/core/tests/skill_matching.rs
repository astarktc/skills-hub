use super::{match_skill_candidate, CandidateMatch, MatchableSkill, SkillMatch};

#[derive(Debug, PartialEq, Eq)]
struct Cand {
    name: &'static str,
    subpath: &'static str,
}

impl MatchableSkill for Cand {
    fn name(&self) -> &str {
        self.name
    }
    fn subpath(&self) -> &str {
        self.subpath
    }
}

const fn cand(name: &'static str, subpath: &'static str) -> Cand {
    Cand { name, subpath }
}

fn resolved_subpath(target: &str, candidates: &[Cand]) -> Option<&'static str> {
    match match_skill_candidate(target, candidates) {
        SkillMatch::Resolved(c) => Some(c.subpath),
        _ => None,
    }
}

fn ambiguous_subpaths(target: &str, candidates: &[Cand]) -> Option<Vec<&'static str>> {
    match match_skill_candidate(target, candidates) {
        SkillMatch::Ambiguous(list) => Some(list.into_iter().map(|c| c.subpath).collect()),
        _ => None,
    }
}

/// One row per matching situation: (target, candidates, expected).
#[test]
fn match_skill_candidate_table() {
    let react_vue = [cand("react", "skills/react"), cand("vue", "skills/vue")];
    let react_native = [
        cand("react", "skills/react"),
        cand("react-native", "skills/react-native"),
    ];
    let alpha_beta = [cand("alpha", "skills/alpha"), cand("beta", "skills/beta")];

    // Exact, case-insensitive.
    assert_eq!(resolved_subpath("React", &react_vue), Some("skills/react"));
    // Exact beats containment even when several candidates contain the target.
    assert_eq!(
        resolved_subpath("react", &react_native),
        Some("skills/react")
    );
    // skills.sh name vs SKILL.md name: bidirectional containment.
    assert_eq!(
        resolved_subpath("json-render-react", &react_vue),
        Some("skills/react")
    );
    assert_eq!(
        resolved_subpath("re", &[cand("react", "skills/react")]),
        Some("skills/react")
    );
    // Nothing matches.
    assert_eq!(
        match_skill_candidate("gamma", &alpha_beta),
        SkillMatch::None
    );
    // Empty input.
    assert_eq!(
        match_skill_candidate("react", &[] as &[Cand]),
        SkillMatch::None
    );
    // A blank target never matches (it would "contain" into everything).
    assert_eq!(match_skill_candidate("  ", &react_vue), SkillMatch::None);
}

#[test]
fn containment_with_several_hits_is_ambiguous() {
    let candidates = [
        cand("react", "skills/react"),
        cand("render", "skills/render"),
        cand("vue", "skills/vue"),
    ];
    assert_eq!(
        ambiguous_subpaths("json-render-react", &candidates),
        Some(vec!["skills/react", "skills/render"])
    );
}

#[test]
fn duplicate_exact_names_are_ambiguous() {
    let candidates = [cand("alpha", "a/alpha"), cand("Alpha", "b/alpha")];
    assert_eq!(
        ambiguous_subpaths("alpha", &candidates),
        Some(vec!["a/alpha", "b/alpha"])
    );
}

#[test]
fn directory_name_is_the_fallback_tier() {
    // SKILL.md names do not match; the folder name does.
    let candidates = [
        cand("Frontend Best Practices", "skills/react"),
        cand("Backend Best Practices", "skills/node"),
    ];
    assert_eq!(resolved_subpath("react", &candidates), Some("skills/react"));
    // A name-tier hit wins over a dir-tier hit.
    let mixed = [
        cand("react", "skills/one"),
        cand("Frontend", "skills/react"),
    ];
    assert_eq!(resolved_subpath("react", &mixed), Some("skills/one"));
    // Windows separators in the subpath still yield the last component.
    let win = [cand("Frontend", "skills\\react")];
    assert_eq!(resolved_subpath("react", &win), Some("skills\\react"));
}

#[test]
fn wire_shape_carries_subpaths() {
    let react_vue = [cand("react", "skills/react"), cand("vue", "skills/vue")];
    assert_eq!(
        CandidateMatch::from(match_skill_candidate("react", &react_vue)),
        CandidateMatch::Resolved {
            subpath: "skills/react".to_string()
        }
    );
    let dup = [cand("alpha", "a/alpha"), cand("alpha", "b/alpha")];
    assert_eq!(
        CandidateMatch::from(match_skill_candidate("alpha", &dup)),
        CandidateMatch::Ambiguous {
            subpaths: vec!["a/alpha".to_string(), "b/alpha".to_string()]
        }
    );
    assert_eq!(
        CandidateMatch::from(match_skill_candidate("zzz", &react_vue)),
        CandidateMatch::None
    );
    // serde: internally tagged on `kind`.
    let json = serde_json::to_value(CandidateMatch::Resolved {
        subpath: "skills/react".to_string(),
    })
    .unwrap();
    assert_eq!(json["kind"], "resolved");
    assert_eq!(json["subpath"], "skills/react");
    assert_eq!(
        serde_json::to_value(CandidateMatch::None).unwrap()["kind"],
        "none"
    );
}
