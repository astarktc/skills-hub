import { describe, expect, it } from "vitest";
import {
  defaultImportVariantPath,
  filterAndSortSkills,
  formatRelativeTime,
  groupSkillsByRepo,
  repoInfo,
  skillSourceLabel,
  sourceKind,
  unlocatableRepairs,
  type ImportVariantFields,
  type SkillPresentationFields,
} from "./skillPresentation";

const skill = (
  over: Partial<SkillPresentationFields> & { name: string },
): SkillPresentationFields & { name: string } => ({
  source_type: "local",
  source_ref: null,
  central_path: "/central/skill",
  created_at: 0,
  updated_at: 0,
  ...over,
});

// A translator that makes the key and its interpolation visible.
const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key}(${JSON.stringify(opts)})` : key;

describe("sourceKind", () => {
  it("reads git from the source type", () => {
    expect(sourceKind(skill({ name: "a", source_type: "GitHub" }))).toBe("git");
    expect(sourceKind(skill({ name: "a", source_type: "git" }))).toBe("git");
  });

  it("reads imported as its own kind — the central copy is the truth", () => {
    expect(sourceKind(skill({ name: "a", source_type: "imported" }))).toBe(
      "imported",
    );
  });

  it("treats everything else as local", () => {
    expect(sourceKind(skill({ name: "a", source_type: "local" }))).toBe(
      "local",
    );
  });
});

describe("repoInfo", () => {
  it("returns null for an empty source", () => {
    expect(repoInfo(null)).toBeNull();
    expect(repoInfo(undefined)).toBeNull();
    expect(repoInfo("")).toBeNull();
  });

  it("strips a git+ prefix and a .git suffix", () => {
    expect(repoInfo("git+https://github.com/owner/repo.git")).toEqual({
      label: "owner/repo",
      href: "https://github.com/owner/repo",
    });
  });

  it("keeps only owner/repo from a deep URL", () => {
    expect(repoInfo("https://github.com/owner/repo/tree/main/skills/x")).toEqual(
      { label: "owner/repo", href: "https://github.com/owner/repo" },
    );
  });

  it("handles ssh and scp-like github forms", () => {
    expect(repoInfo("git@github.com:owner/repo.git")).toEqual({
      label: "owner/repo",
      href: "https://github.com/owner/repo",
    });
  });

  it("returns null for non-GitHub hosts", () => {
    expect(repoInfo("https://gitlab.com/owner/repo.git")).toBeNull();
    expect(repoInfo("/Users/me/skills/local-thing")).toBeNull();
  });
});

describe("skillSourceLabel", () => {
  it("uses the source ref for git skills and the central path otherwise", () => {
    expect(
      skillSourceLabel(
        skill({
          name: "a",
          source_type: "github",
          source_ref: "https://github.com/o/r",
        }),
      ),
    ).toBe("https://github.com/o/r");
    expect(skillSourceLabel(skill({ name: "a" }))).toBe("/central/skill");
  });
});

describe("groupSkillsByRepo", () => {
  it("groups git skills by repo label and puts the local group last", () => {
    const groups = groupSkillsByRepo(
      [
        skill({ name: "local-one" }),
        skill({
          name: "zeta",
          source_type: "git",
          source_ref: "https://github.com/zoo/zeta.git",
        }),
        skill({
          name: "alpha",
          source_type: "git",
          source_ref: "git+https://github.com/acme/alpha",
        }),
        skill({
          name: "alpha-two",
          source_type: "git",
          source_ref: "https://github.com/acme/alpha",
        }),
      ],
      { local: "Local", ungrouped: "Ungrouped" },
    );

    expect(groups.map((g) => g.label)).toEqual([
      "acme/alpha",
      "zoo/zeta",
      "Local",
    ]);
    expect(groups[0].skills.map((s) => s.name)).toEqual(["alpha", "alpha-two"]);
    expect(groups[0].href).toBe("https://github.com/acme/alpha");
    expect(groups[2].href).toBeNull();
  });

  it("falls back to the raw ref for a git URL on an unknown host", () => {
    const groups = groupSkillsByRepo(
      [
        skill({
          name: "x",
          source_type: "git",
          source_ref: "https://example.com/team/x.git",
        }),
      ],
      { local: "Local", ungrouped: "Ungrouped" },
    );
    expect(groups).toHaveLength(1);
    expect(groups[0].label).toBe("https://example.com/team/x.git");
    expect(groups[0].href).toBeNull();
  });
});

describe("filterAndSortSkills", () => {
  const skills = [
    skill({
      name: "beta",
      source_type: "git",
      source_ref: "https://github.com/acme/beta",
      created_at: 30,
      updated_at: 10,
    }),
    skill({
      name: "alpha",
      central_path: "/central/alpha",
      created_at: 10,
      updated_at: 30,
    }),
    skill({
      name: "gamma",
      central_path: "/central/gamma",
      created_at: 20,
      updated_at: 20,
    }),
  ];

  it("sorts by name, added and updated", () => {
    expect(
      filterAndSortSkills(skills, { query: "", sort: "name" }).map(
        (s) => s.name,
      ),
    ).toEqual(["alpha", "beta", "gamma"]);
    expect(
      filterAndSortSkills(skills, { query: "", sort: "added" }).map(
        (s) => s.name,
      ),
    ).toEqual(["beta", "gamma", "alpha"]);
    expect(
      filterAndSortSkills(skills, { query: "", sort: "updated" }).map(
        (s) => s.name,
      ),
    ).toEqual(["alpha", "gamma", "beta"]);
  });

  it("matches a plain query as a case-insensitive substring", () => {
    expect(
      filterAndSortSkills(skills, { query: "  ACME ", sort: "name" }).map(
        (s) => s.name,
      ),
    ).toEqual(["beta"]);
  });

  it("supports wildcard queries", () => {
    expect(
      filterAndSortSkills(skills, { query: "a*a", sort: "name" }).map(
        (s) => s.name,
      ),
    ).toEqual(["alpha", "beta", "gamma"]);
    expect(
      filterAndSortSkills(skills, { query: "gam*", sort: "name" }).map(
        (s) => s.name,
      ),
    ).toEqual(["gamma"]);
  });

  it("escapes regex metacharacters between wildcards", () => {
    const dotted = [
      skill({ name: "a.b", central_path: "/central/a.b" }),
      skill({ name: "axb", central_path: "/central/axb" }),
    ];
    expect(
      filterAndSortSkills(dotted, { query: "a.b*", sort: "name" }).map(
        (s) => s.name,
      ),
    ).toEqual(["a.b"]);
  });

  it("does not throw on an unbalanced bracket query", () => {
    expect(() =>
      filterAndSortSkills(skills, { query: "a[*", sort: "name" }),
    ).not.toThrow();
  });
});

describe("formatRelativeTime", () => {
  const now = 1_000_000_000;

  it("renders empty for a missing or future timestamp", () => {
    expect(formatRelativeTime(null, t, now)).toBe("relative.empty");
    expect(formatRelativeTime(undefined, t, now)).toBe("relative.empty");
    expect(formatRelativeTime(0, t, now)).toBe("relative.empty");
    expect(formatRelativeTime(now + 5000, t, now)).toBe("relative.empty");
  });

  it("crosses the minute, hour and day thresholds", () => {
    expect(formatRelativeTime(now - 59_999, t, now)).toBe("relative.justNow");
    expect(formatRelativeTime(now - 60_000, t, now)).toBe(
      'relative.minutesAgo({"minutes":1})',
    );
    expect(formatRelativeTime(now - 59 * 60_000, t, now)).toBe(
      'relative.minutesAgo({"minutes":59})',
    );
    expect(formatRelativeTime(now - 60 * 60_000, t, now)).toBe(
      'relative.hoursAgo({"hours":1})',
    );
    expect(formatRelativeTime(now - 23 * 3_600_000, t, now)).toBe(
      'relative.hoursAgo({"hours":23})',
    );
    expect(formatRelativeTime(now - 24 * 3_600_000, t, now)).toBe(
      'relative.daysAgo({"days":1})',
    );
    expect(formatRelativeTime(now - 10 * 24 * 3_600_000, t, now)).toBe(
      'relative.daysAgo({"days":10})',
    );
  });
});

describe("defaultImportVariantPath", () => {
  // Registry order lists claude_code before pi, so a link found in Claude
  // comes first even when the real directory it points at lives in Pi.
  const link: ImportVariantFields = {
    path: "/home/.claude/skills/alpha",
    is_link: true,
  };
  const dir: ImportVariantFields = {
    path: "/home/.pi/agent/skills/alpha",
    is_link: false,
  };

  it("prefers a real directory over a link in a consistent group", () => {
    expect(
      defaultImportVariantPath({ has_conflict: false, variants: [link, dir] }),
    ).toBe("/home/.pi/agent/skills/alpha");
  });

  it("keeps the first variant when no variant is a real directory", () => {
    const other: ImportVariantFields = {
      path: "/home/.cursor/skills/alpha",
      is_link: true,
    };
    expect(
      defaultImportVariantPath({
        has_conflict: false,
        variants: [link, other],
      }),
    ).toBe("/home/.claude/skills/alpha");
  });

  it("keeps the first real directory when several tie", () => {
    const second: ImportVariantFields = {
      path: "/home/.cursor/skills/alpha",
      is_link: false,
    };
    expect(
      defaultImportVariantPath({
        has_conflict: false,
        variants: [link, dir, second],
      }),
    ).toBe("/home/.pi/agent/skills/alpha");
  });

  it("leaves a conflicting group at its first variant for the operator to resolve", () => {
    expect(
      defaultImportVariantPath({ has_conflict: true, variants: [link, dir] }),
    ).toBe("/home/.claude/skills/alpha");
  });

  it("is undefined for an empty group", () => {
    expect(
      defaultImportVariantPath({ has_conflict: false, variants: [] }),
    ).toBeUndefined();
  });
});

describe("unlocatableRepairs", () => {
  it("offers nothing for a skill the app can locate", () => {
    expect(
      unlocatableRepairs({
        unlocatable: null,
        refreshable: true,
        detachable: true,
      }),
    ).toEqual([]);
  });

  it("offers Re-point and Detach when the source is gone and the central copy is present", () => {
    expect(
      unlocatableRepairs({
        unlocatable: "source_missing",
        refreshable: true,
        detachable: true,
      }),
    ).toEqual(["repoint", "detach"]);
  });

  it("withholds Detach when the central copy is gone too — Re-point rebuilds both", () => {
    expect(
      unlocatableRepairs({
        unlocatable: "source_missing",
        refreshable: true,
        detachable: false,
      }),
    ).toEqual(["repoint"]);
  });

  it("offers Restore for a missing central copy exactly when the skill is refreshable", () => {
    expect(
      unlocatableRepairs({
        unlocatable: "central_missing",
        refreshable: true,
        detachable: false,
      }),
    ).toEqual(["restore"]);
    expect(
      unlocatableRepairs({
        unlocatable: "central_missing",
        refreshable: false,
        detachable: false,
      }),
    ).toEqual([]);
  });
});
