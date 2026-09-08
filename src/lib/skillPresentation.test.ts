import { describe, expect, it } from "vitest";
import type { SkillTargetDto } from "../components/skills/types";
import {
  defaultImportVariantPath,
  filterAndSortSkills,
  formatRelativeTime,
  groupSkillsByRepo,
  importedSourceLine,
  repoInfo,
  skillSourceLabel,
  sourceKind,
  toolLabel,
  skillToolChips,
  unlocatableRepairs,
  visibleToolChoices,
  UNLOCATABLE_STATE_KEY,
  UNLOCATABLE_TOOLTIP_KEY,
  UNLOCATABLE_REPAIR_KEY,
  SKIPPED_REASON_KEY,
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

describe("unlocatable translation maps", () => {
  it("maps wire states and repairs to presentation keys", () => {
    expect(UNLOCATABLE_STATE_KEY).toEqual({
      source_missing: "unlocatable.sourceMissing",
      central_missing: "unlocatable.centralMissing",
    });
    expect(UNLOCATABLE_TOOLTIP_KEY).toEqual({
      source_missing: "unlocatable.sourceMissingTooltip",
      central_missing: "unlocatable.centralMissingTooltip",
    });
    expect(UNLOCATABLE_REPAIR_KEY).toEqual({
      repoint: "unlocatable.repoint",
      detach: "unlocatable.detach",
      restore: "unlocatable.restore",
    });
    expect(SKIPPED_REASON_KEY).toEqual({
      source_missing: "errors.refreshSkippedSourceMissing",
      central_missing: "errors.refreshSkippedCentralMissing",
    });
  });
});

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

describe("toolLabel", () => {
  it("reads the Tool's label from the tools.* catalog", () => {
    expect(toolLabel(t, "claude_code")).toBe(
      'tools.claude_code({"defaultValue":"claude_code"})',
    );
  });

  it("falls back to the registry key for a Tool the catalog does not name", () => {
    const withDefault = (key: string, opts?: Record<string, unknown>) =>
      String(opts?.defaultValue ?? key);
    expect(toolLabel(withDefault, "not-a-tool")).toBe("not-a-tool");
  });
});

describe("importedSourceLine", () => {
  it("composes 'Managed here' with the found-in Tool as display-only history", () => {
    const line = importedSourceLine({ imported_from_tool: "pi" }, t);
    expect(line.tool).toBe('tools.pi({"defaultValue":"pi"})');
    expect(line.managedHere).toBe("provenance.managedHere");
    expect(line.importedFrom).toBe(
      'provenance.importedFrom({"tool":"tools.pi({\\"defaultValue\\":\\"pi\\"})"})',
    );
    expect(line.text).toBe(`${line.managedHere} · ${line.importedFrom}`);
  });

  it("names the Tool 'unknown' when the record kept none", () => {
    const line = importedSourceLine({ imported_from_tool: null }, t);
    expect(line.tool).toBe("unknown");
    expect(line.importedFrom).toBe('provenance.importedFrom({"tool":"unknown"})');
  });
});

describe("skillToolChips", () => {
  const target = (tool: string): SkillTargetDto => ({
    tool,
    mode: "symlink",
    status: "synced",
    target_path: `/tools/${tool}/skill`,
    synced_at: 1,
  });
  const allTools = [
    { id: "claude_code", label: "Claude Code" },
    { id: "cursor", label: "Cursor" },
    { id: "pi", label: "Pi" },
  ];
  const installed = [
    { id: "claude_code", label: "Claude Code" },
    { id: "pi", label: "Pi" },
  ];

  it("renders a row-only tool as an undetected chip keeping its target", () => {
    const chips = skillToolChips([target("cursor")], [], allTools);
    expect(chips).toEqual([
      {
        id: "cursor",
        label: "Cursor",
        target: target("cursor"),
        detected: false,
      },
    ]);
  });

  it("renders an installed-only tool as a detected chip with no target", () => {
    const chips = skillToolChips([], installed, allTools);
    expect(chips.map((c) => [c.id, c.detected, c.target])).toEqual([
      ["claude_code", true, null],
      ["pi", true, null],
    ]);
  });

  it("renders a tool that is both installed and synced exactly once", () => {
    const chips = skillToolChips([target("pi")], installed, allTools);
    expect(chips).toHaveLength(2);
    expect(chips[1]).toEqual({
      id: "pi",
      label: "Pi",
      target: target("pi"),
      detected: true,
    });
  });

  it("renders nothing when the skill has no row and nothing is installed", () => {
    expect(skillToolChips([], [], allTools)).toEqual([]);
  });

  it("labels a row whose tool the registry no longer knows by its key", () => {
    const chips = skillToolChips([target("gone_tool")], [], allTools);
    expect(chips[0].label).toBe("gone_tool");
    expect(chips[0].detected).toBe(false);
  });

  it("keeps installed tools first, undetected rows after", () => {
    const chips = skillToolChips(
      [target("cursor"), target("claude_code")],
      installed,
      allTools,
    );
    expect(chips.map((c) => c.id)).toEqual(["claude_code", "pi", "cursor"]);
  });
});

describe("visibleToolChoices", () => {
  const allTools = [{ key: "claude_code" }, { key: "cursor" }, { key: "pi" }];

  it("shows every tool when the filter is off", () => {
    expect(
      visibleToolChoices(allTools, ["pi"], new Set(), false),
    ).toHaveLength(3);
  });

  it("shows detected tools when the filter is on", () => {
    expect(
      visibleToolChoices(allTools, ["pi"], new Set(), true).map((t) => t.key),
    ).toEqual(["pi"]);
  });

  it("keeps a selected-but-undetected tool visible so it can be unticked", () => {
    expect(
      visibleToolChoices(allTools, ["pi"], new Set(["cursor"]), true).map(
        (t) => t.key,
      ),
    ).toEqual(["cursor", "pi"]);
  });

  it("hides an undetected tool that is not selected", () => {
    expect(
      visibleToolChoices(allTools, ["pi"], new Set(["pi"]), true).map(
        (t) => t.key,
      ),
    ).toEqual(["pi"]);
  });
});
