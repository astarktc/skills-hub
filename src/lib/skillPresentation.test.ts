import { describe, expect, it } from "vitest";
import type { SkillTargetDto } from "../components/skills/types";
import {
  defaultImportVariantPath,
  deployAction,
  HEALTH_ROW_ORDER,
  skillHealth,
  skillSourceLine,
  filterAndSortSkills,
  formatRelativeTime,
  groupSkillsByRepo,
  importedSourceLine,
  INDIVIDUAL_GROUP_KEY,
  repoInfo,
  repointCurrentSource,
  repointKind,
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

describe("repointKind", () => {
  it.each([
    ["git", "git"],
    ["GitHub", "git"],
    ["local", "local"],
    // An imported skill has no source: Change source starts on a folder.
    ["imported", "local"],
  ] as const)("%s preselects %s", (source_type, expected) => {
    expect(repointKind(skill({ name: "a", source_type }))).toBe(expected);
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
  it("uses the source ref for git skills", () => {
    expect(
      skillSourceLabel(
        skill({
          name: "a",
          source_type: "github",
          source_ref: "https://github.com/o/r",
        }),
      ),
    ).toBe("https://github.com/o/r");
  });

  it("shows a local skill's recorded folder, not its central copy", () => {
    expect(
      skillSourceLabel(
        skill({ name: "a", source_type: "local", source_ref: "/Users/me/Downloads/scratchpad" }),
      ),
    ).toBe("/Users/me/Downloads/scratchpad");
  });

  it("falls back to the central path only when no source is recorded", () => {
    expect(skillSourceLabel(skill({ name: "a" }))).toBe("/central/skill");
    expect(skillSourceLabel(skill({ name: "a", source_ref: "  " }))).toBe("/central/skill");
    expect(
      skillSourceLabel(skill({ name: "a", source_type: "imported", source_ref: "/stale" })),
    ).toBe("/central/skill");
  });
});

describe("repointCurrentSource", () => {
  it("answers the skill's own kind and ref", () => {
    expect(
      repointCurrentSource(skill({ name: "a", source_type: "git", source_ref: " https://github.com/o/r " })),
    ).toEqual({ kind: "git", ref: "https://github.com/o/r" });
    expect(
      repointCurrentSource(skill({ name: "a", source_type: "local", source_ref: "/Users/me/Downloads/scratchpad" })),
    ).toEqual({ kind: "local", ref: "/Users/me/Downloads/scratchpad" });
  });

  it("is null for an imported skill and for a record without a source", () => {
    expect(repointCurrentSource(skill({ name: "a", source_type: "imported", source_ref: null }))).toBeNull();
    expect(repointCurrentSource(skill({ name: "a", source_type: "local", source_ref: null }))).toBeNull();
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

  it("folds single-skill repos into one Individual sources group, in input order", () => {
    const input = [
      skill({ name: "loner-z", source_type: "git", source_ref: "https://github.com/zoo/z" }),
      skill({ name: "local-one" }),
      skill({ name: "alpha", source_type: "git", source_ref: "https://github.com/acme/alpha" }),
      skill({ name: "alpha-two", source_type: "git", source_ref: "https://github.com/acme/alpha" }),
      skill({ name: "loner-b", source_type: "git", source_ref: "https://github.com/bee/b" }),
    ];
    const labels = { local: "Local", ungrouped: "Ungrouped", individual: "Individual sources" };
    const groups = groupSkillsByRepo(input, labels, { foldSingletons: true });

    expect(groups.map((g) => g.label)).toEqual(["acme/alpha", "Individual sources", "Local"]);
    expect(groups[1].key).toBe(INDIVIDUAL_GROUP_KEY);
    expect(groups[1].href).toBeNull();
    expect(groups[1].skills.map((s) => s.name)).toEqual(["loner-z", "loner-b"]);
    // The local group never folds, even with one skill.
    expect(groups[2].skills.map((s) => s.name)).toEqual(["local-one"]);
    // Off by default: every repo keeps its own group.
    expect(groupSkillsByRepo(input, labels).map((g) => g.label)).toEqual([
      "acme/alpha",
      "bee/b",
      "zoo/z",
      "Local",
    ]);
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

describe("deployAction", () => {
  const targets = (...statuses: SkillTargetDto["status"][]) =>
    statuses.map((status) => ({ status, synced_at: 1 as number | null }));
  const neverSynced = (status: SkillTargetDto["status"]) => ({ status, synced_at: null });
  it.each([
    ["no target at all syncs", null, targets(), { kind: "sync" }],
    ["a live target is deployed", null, targets("synced", "disabled"), { kind: "deployed" }],
    ["an error target that was synced is still deployed (Disable retries it)", null, targets("error"), { kind: "deployed" }],
    ["a failed first sync alone is not deployed: retry = first sync", null, [neverSynced("error")], { kind: "sync" }],
    ["a failed first sync beside a live target is deployed", null, [neverSynced("error"), ...targets("synced")], { kind: "deployed" }],
    ["every target pair-disabled enables them", null, targets("disabled", "disabled"), { kind: "enable", disabledTargets: 2 }],
    ["skill grain enables, whatever the rows", 5, targets("disabled", "error"), { kind: "enable", disabledTargets: 1 }],
    ["skill grain with no rows still enables", 5, targets(), { kind: "enable", disabledTargets: 0 }],
  ] as const)("%s", (_name, disabled_at, rows, expected) => {
    expect(deployAction({ disabled_at, targets: rows })).toEqual(expected);
  });
});

describe("skillSourceLine", () => {
  const t = (key: string, opts?: Record<string, unknown>) =>
    opts ? `${key}:${JSON.stringify(opts)}` : key;
  const base = { central_path: "/hub/alpha", imported_from_tool: null };
  it.each([
    [
      "a GitHub repo links and copies the repo",
      { source_type: "git", source_ref: "https://github.com/acme/skills.git" },
      { kind: "git", label: "acme/skills", href: "https://github.com/acme/skills", copyValue: "https://github.com/acme/skills", icon: "github" },
    ],
    [
      "another git host shows and copies the ref",
      { source_type: "git", source_ref: "https://gitlab.com/acme/skills" },
      { kind: "git", label: "https://gitlab.com/acme/skills", href: null, copyValue: "https://gitlab.com/acme/skills", icon: "git" },
    ],
    [
      "a local skill shows and copies its recorded folder (D15)",
      { source_type: "local", source_ref: "~/skills/alpha" },
      { kind: "local", label: "~/skills/alpha", href: null, copyValue: "~/skills/alpha", icon: "folder" },
    ],
    [
      "a local skill with no recorded folder falls back to the central copy",
      { source_type: "local", source_ref: null },
      { kind: "local", label: "/hub/alpha", copyValue: "/hub/alpha", icon: "folder" },
    ],
    [
      "an imported skill copies nothing and names its found-in Tool as history",
      { source_type: "imported", source_ref: null, imported_from_tool: "cursor" },
      { kind: "imported", label: "provenance.managedHere", copyValue: null, href: null, icon: "imported" },
    ],
  ] as const)("%s", (_name, fields, expected) => {
    expect(skillSourceLine({ ...base, ...fields }, t)).toMatchObject(expected);
  });
});

describe("skillHealth (the fold)", () => {
  const t = (key: string) => key;
  type Fields = Parameters<typeof skillHealth>[0];
  const healthy: Fields = {
    source_type: "local",
    source_ref: "/src/alpha",
    central_path: "/hub/alpha",
    imported_from_tool: null,
    unlocatable: null,
    refreshable: true,
    detachable: true,
    disabled_at: null,
    health: null,
    invocation_override: null,
    targets: [{ tool: "claude", status: "synced", synced_at: 1 }],
  };
  const failure = { code: "OTHER", message: "x" } as const;
  const row = (tool: string, status: SkillTargetDto["status"], synced_at: number | null = 1) => ({ tool, status, synced_at });
  const kinds = (fields: Partial<Fields>) =>
    skillHealth({ ...healthy, ...fields }, { t }).rows.map((r) => r.kind);

  it("a healthy, deployed skill has no rows and is healthy", () => {
    const fold = skillHealth(healthy, { t });
    expect(fold.rows).toEqual([]);
    expect(fold.severity).toBe("healthy");
    expect(fold.source.copyValue).toBe("/src/alpha");
  });

  it.each([
    ["central copy missing", { unlocatable: "central_missing" }, ["central_missing"], "failed", ["restore", "delete"]],
    ["source folder missing", { unlocatable: "source_missing" }, ["source_missing"], "stale", ["repoint", "detach", "delete"]],
    ["fetch failed", { health: { kind: "fetch_failed", error: failure, at: 5 } }, ["fetch_failed"], "stale", ["update", "repoint"]],
    ["edit conflict (stored)", { health: { kind: "edit_conflict", error: failure, at: 5 } }, ["edit_conflict"], "stale", ["editInvocation"]],
    ["edit conflict (live override)", { invocation_override: { conflict: true } }, ["edit_conflict"], "stale", ["editInvocation"]],
    ["delete incomplete: Retry finishes the Delete", { health: { kind: "delete_incomplete", error: failure, at: 5 } }, ["delete_incomplete"], "failed", ["delete"]],
    ["import incomplete: only Dismiss", { health: { kind: "import_failed", error: failure, at: 5 } }, ["import_failed"], "stale", ["dismiss"]],
    ["repoint failed", { health: { kind: "repoint_failed", error: failure, at: 5 } }, ["repoint_failed"], "stale", ["repoint"]],
    ["restore failed", { health: { kind: "restore_failed", error: failure, at: 5 } }, ["restore_failed"], "failed", ["restore", "repoint"]],
    ["disabled at skill grain", { disabled_at: 9, targets: [row("claude", "disabled")] }, ["disabled"], "muted", ["enable"]],
    ["one pair disabled", { targets: [row("claude", "synced"), row("cursor", "disabled")] }, ["targets_disabled"], "muted", ["enable"]],
    ["never deployed", { targets: [] }, ["not_deployed"], "muted", ["deploy"]],
    ["a failed first sync only", { targets: [row("claude", "error", null)] }, ["targets_failed", "not_deployed"], "failed", ["retryTargets"]],
    ["a stale copy: re-synced from the central copy", { targets: [row("claude", "stale")] }, ["targets_stale"], "stale", ["retryTargets"]],
    ["a missing target", { targets: [row("claude", "missing")] }, ["targets_missing"], "failed", ["retryTargets"]],
  ] as const)("%s", (_name, fields, expectedKinds, severity, repairs) => {
    const fold = skillHealth({ ...healthy, ...(fields as Partial<Fields>) }, { t });
    expect(fold.rows.map((r) => r.kind)).toEqual(expectedKinds);
    expect(fold.severity).toBe(severity);
    expect(fold.rows[0].repairs).toEqual(repairs);
  });

  // Target repairs × provenance × skill policy (S5): the central copy is
  // what a target should hold, so a failed / missing / stale target is
  // re-synced from it whatever the skill's source — never an upstream Update
  // an imported or detached skill cannot run; under skill-grain Disable the
  // repair finishes the Disable (a re-sync would meet the Disabled gate).
  it.each([
    ["git", true],
    ["local", true],
    ["imported", false],
    // Detached: imported provenance, not refreshable, no external source.
    ["imported (detached)", false],
  ] as const)("a %s skill's target rows repair from the central copy", (provenance, refreshable) => {
    const source_type = provenance.startsWith("imported") ? "imported" : provenance;
    for (const status of ["error", "missing", "stale"] as const) {
      const fold = skillHealth(
        { ...healthy, source_type, source_ref: source_type === "imported" ? null : "/src/alpha", refreshable, targets: [row("claude", "synced"), row("cursor", status)] },
        { t },
      );
      expect(fold.rows).toEqual([expect.objectContaining({ tools: ["cursor"], repairs: ["retryTargets"] })]);
    }
  });

  it.each(["error", "missing", "stale"] as const)(
    "under skill-grain Disable a %s target offers finishing the Disable, never a retry",
    (status) => {
      const fold = skillHealth({ ...healthy, disabled_at: 4, targets: [row("claude", status)] }, { t });
      const target = fold.rows.find((r) => r.scope === "target");
      expect(target?.repairs).toEqual(["disable"]);
      expect(fold.rows.flatMap((r) => r.repairs)).not.toContain("retryTargets");
    },
  );

  it("an Edit conflict both stored and live is one row", () => {
    expect(
      kinds({ health: { kind: "edit_conflict", error: failure, at: 1 }, invocation_override: { conflict: true } }),
    ).toEqual(["edit_conflict"]);
  });

  it("counts target rows by status and names their Tools", () => {
    const fold = skillHealth(
      { ...healthy, targets: [row("claude", "error"), row("cursor", "error"), row("pi", "synced")] },
      { t },
    );
    expect(fold.rows).toEqual([
      expect.objectContaining({ kind: "targets_failed", scope: "target", count: 2, tools: ["claude", "cursor"] }),
    ]);
  });

  it("one precedence rule orders every row, loudest first, and the severity is the loudest", () => {
    const fold = skillHealth(
      {
        ...healthy,
        unlocatable: "source_missing",
        health: { kind: "fetch_failed", error: failure, at: 1 },
        targets: [row("claude", "stale"), row("cursor", "disabled"), row("pi", "error")],
      },
      { t },
    );
    expect(fold.rows.map((r) => r.kind)).toEqual([
      "targets_failed",
      "source_missing",
      "fetch_failed",
      "targets_stale",
      "targets_disabled",
    ]);
    expect(fold.severity).toBe("failed");
    const order = HEALTH_ROW_ORDER as readonly string[];
    const ranks = fold.rows.map((r) => order.indexOf(r.kind));
    expect([...ranks].sort((a, b) => a - b)).toEqual(ranks);
  });

  it("a skill disabled at skill grain shows no pair-disabled or not-deployed row", () => {
    expect(kinds({ disabled_at: 3, targets: [] })).toEqual(["disabled"]);
  });

  it("the stored row carries its typed error and when it settled", () => {
    const fold = skillHealth({ ...healthy, health: { kind: "fetch_failed", error: failure, at: 42 } }, { t });
    expect(fold.rows[0]).toMatchObject({ error: failure, at: 42, scope: "skill" });
  });
});
