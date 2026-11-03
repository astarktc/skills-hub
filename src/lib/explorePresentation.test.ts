// Explore's pure folds (ticket 22): the candidate list, the installed match,
// the chips' partition, the query fold, the search mode and popularity.

import { describe, expect, it } from "vitest";
import type { FeaturedSkillDto, OnlineSkillDto } from "../bindings";
import {
  EXPLORE_FILTERS,
  EXPLORE_FILTER_ORDER,
  exploreCandidates,
  exploreFocusReturn,
  exploreHideKey,
  exploreInstalledId,
  exploreInstalledIndex,
  exploreQueryFold,
  exploreSearchMode,
  exploreVisibleOnline,
  formatPopularity,
} from "./explorePresentation";

const featured = (name: string, url: string, downloads = 0, stars = 0, summary = `${name} summary`): FeaturedSkillDto => ({
  slug: name,
  name,
  summary,
  downloads,
  stars,
  source_url: url,
});
const online = (name: string, slug: string, installs = 0): OnlineSkillDto => ({
  name,
  installs,
  source: slug,
  source_url: `https://github.com/${slug}`,
});

const FEATURED = [
  featured("Release-Notes", "https://github.com/Acme/skills/tree/main/release-notes", 1200),
  featured("docx", "https://github.com/anthropics/skills/tree/main/docx", 0, 40),
  featured("scratch", "https://github.com/solo/scratch.git"),
];
const ONLINE = [online("docx", "anthropics/skills", 9), online("pdf", "anthropics/skills", 3), online("pdf", "anthropics/skills", 2)];

describe("exploreHideKey", () => {
  it("keeps the persisted shape: lower-cased name | owner/repo, without host, .git or /tree/", () => {
    expect(exploreHideKey("Release-Notes", "https://github.com/Acme/skills/tree/main/release-notes")).toBe(
      "release-notes|acme/skills",
    );
    expect(exploreHideKey("scratch", "https://github.com/solo/scratch.git")).toBe("scratch|solo/scratch");
    expect(exploreHideKey("x", "HTTPS://GITHUB.COM/a/b")).toBe("x|a/b");
  });
});

describe("exploreCandidates", () => {
  it("lists featured then online in their sources' order, dropping an online repeat of a featured skill", () => {
    const rows = exploreCandidates({ featured: FEATURED, online: ONLINE, managed: [], hidden: new Set() });
    expect(rows.map((row) => row.id)).toEqual([
      "featured:release-notes|acme/skills",
      "featured:docx|anthropics/skills",
      "featured:scratch|solo/scratch",
      "online:pdf|anthropics/skills",
    ]);
    expect(rows[0]).toMatchObject({
      origin: "featured",
      name: "Release-Notes",
      description: "Release-Notes summary",
      sourceLabel: "Acme/skills",
      sourceUrl: "https://github.com/Acme/skills/tree/main/release-notes",
      popularity: { kind: "downloads", count: 1200 },
      installedId: null,
      hidden: false,
    });
    // Stars stand in when there are no downloads; nothing when neither.
    expect(rows[1].popularity).toEqual({ kind: "stars", count: 40 });
    expect(rows[2].popularity).toBeNull();
    expect(rows[3]).toMatchObject({ origin: "online", description: null, popularity: { kind: "installs", count: 3 } });
  });

  it("marks a candidate the library holds with that skill's id, and a hidden one", () => {
    const rows = exploreCandidates({
      featured: FEATURED,
      online: [],
      managed: [
        { id: "s1", name: "release-notes", source_ref: "https://github.com/acme/skills.git" },
        { id: "s2", name: "scratch", source_ref: "/Users/me/scratch" },
      ],
      hidden: new Set(["docx|anthropics/skills"]),
    });
    expect(rows.map((row) => [row.installedId, row.hidden])).toEqual([
      ["s1", false],
      [null, true],
      // A local skill of the same name is not this candidate.
      [null, false],
    ]);
  });
});

describe("EXPLORE_FILTERS", () => {
  it("partition the candidates: Not in library · Installed · Hidden (a hidden installed one is only Hidden)", () => {
    const rows = exploreCandidates({
      featured: FEATURED,
      online: [],
      managed: [
        { id: "s1", name: "release-notes", source_ref: "https://github.com/acme/skills" },
        { id: "s2", name: "docx", source_ref: "https://github.com/anthropics/skills" },
      ],
      hidden: new Set(["docx|anthropics/skills"]),
    });
    const under = (filter: (typeof EXPLORE_FILTER_ORDER)[number]) =>
      rows.filter(EXPLORE_FILTERS[filter]).map((row) => row.name);
    expect(EXPLORE_FILTER_ORDER[0]).toBe("notInLibrary");
    expect(under("notInLibrary")).toEqual(["scratch"]);
    expect(under("installed")).toEqual(["Release-Notes"]);
    expect(under("hidden")).toEqual(["docx"]);
  });
});

describe("exploreQueryFold", () => {
  const rows = exploreCandidates({ featured: FEATURED, online: ONLINE, managed: [], hidden: new Set() });

  it("matches a featured candidate on name, description or source, keeping the ranked order", () => {
    expect(exploreQueryFold(rows, "  ").map((row) => row.name)).toEqual(["Release-Notes", "docx", "scratch", "pdf"]);
    expect(exploreQueryFold(rows, "RELEASE").map((row) => row.name)).toEqual(["Release-Notes", "pdf"]);
    expect(exploreQueryFold(rows, "docx summary").map((row) => row.name)).toEqual(["docx", "pdf"]);
    expect(exploreQueryFold(rows, "solo/").map((row) => row.name)).toEqual(["scratch", "pdf"]);
  });

  it("keeps every online result: skills.sh matched it already", () => {
    expect(exploreQueryFold(rows, "zzz").map((row) => row.name)).toEqual(["pdf"]);
  });
});

describe("exploreSearchMode", () => {
  it("is featured for no query, short below two characters, online from two", () => {
    expect(exploreSearchMode("")).toBe("featured");
    expect(exploreSearchMode("   ")).toBe("featured");
    expect(exploreSearchMode(" a ")).toBe("short");
    expect(exploreSearchMode("ab")).toBe("online");
  });
});

describe("exploreInstalledId (the preview's installed state, audit F2)", () => {
  const MANAGED = [
    { id: "s-jsdoc", name: "jsdoc-to-ts", source_ref: "https://github.com/mattpocock/skills.git" },
    { id: "s-notes", name: "Release-Notes", source_ref: "https://github.com/acme/skills" },
  ];
  const ONLINE_ONLY = [online("jsdoc-to-ts", "mattpocock/skills", 12), online("ts-reset", "total/ts-reset", 4)];
  const preview = { name: "jsdoc-to-ts", sourceUrl: "https://github.com/mattpocock/skills" };

  it("keeps an online-only installed candidate installed whatever the search shows: ts \u2192 '' \u2192 x, and while loading", () => {
    const index = exploreInstalledIndex(MANAGED);
    const states: { query: string; loading: boolean; listed: boolean }[] = [
      { query: "ts", loading: false, listed: true },
      { query: "", loading: false, listed: false },
      { query: "x", loading: false, listed: false },
      { query: "ts", loading: true, listed: false },
    ];
    for (const state of states) {
      const rows = exploreCandidates({
        featured: FEATURED,
        online: exploreVisibleOnline(state.query, state.loading, ONLINE_ONLY),
        managed: MANAGED,
        hidden: new Set(),
      });
      const row = rows.find((candidate) => candidate.name === "jsdoc-to-ts");
      // The search decides whether the row is listed \u2026
      expect(row !== undefined, JSON.stringify(state)).toBe(state.listed);
      // \u2026 never whether the preview is installed.
      expect(exploreInstalledId(index, preview.name, preview.sourceUrl), JSON.stringify(state)).toBe("s-jsdoc");
    }
  });

  it("agrees with the row's mark for featured and online candidates alike", () => {
    const index = exploreInstalledIndex(MANAGED);
    const rows = exploreCandidates({
      featured: FEATURED,
      online: exploreVisibleOnline("ts", false, ONLINE_ONLY),
      managed: MANAGED,
      hidden: new Set(),
    });
    expect(rows.map((row) => [row.origin, row.name, row.installedId])).toEqual([
      ["featured", "Release-Notes", "s-notes"],
      ["featured", "docx", null],
      ["featured", "scratch", null],
      ["online", "jsdoc-to-ts", "s-jsdoc"],
      ["online", "ts-reset", null],
    ]);
    for (const row of rows) expect(exploreInstalledId(index, row.name, row.sourceUrl)).toBe(row.installedId);
  });

  it("lets the first Managed skill of an identity win, as the rows always did", () => {
    const index = exploreInstalledIndex([
      { id: "first", name: "docx", source_ref: "https://github.com/anthropics/skills" },
      { id: "second", name: "DOCX", source_ref: "https://github.com/Anthropics/skills.git" },
    ]);
    expect(exploreInstalledId(index, "docx", "https://github.com/anthropics/skills/tree/main/docx")).toBe("first");
    expect(exploreInstalledId(index, "pdf", "https://github.com/anthropics/skills")).toBeNull();
  });
});

describe("exploreVisibleOnline", () => {
  const results = [online("pdf", "anthropics/skills")];
  it("shows online results only for a searched query that is not loading", () => {
    expect(exploreVisibleOnline("pd", false, results)).toBe(results);
    expect(exploreVisibleOnline("pd", true, results)).toEqual([]);
    expect(exploreVisibleOnline("p", false, results)).toEqual([]);
    expect(exploreVisibleOnline("", false, results)).toEqual([]);
  });
});

describe("exploreFocusReturn (a closing preview / a row taken away, audit F1)", () => {
  it("returns to the row itself while it is still shown", () => {
    expect(exploreFocusReturn(["a", "b", "c"], ["a", "b", "c"], "b")).toEqual({ kind: "row", key: "b" });
  });
  it("moves to the next row still shown in the order it was opened from", () => {
    // The audit's sequence: changelog-writer left Not in library after its Add.
    expect(
      exploreFocusReturn(
        ["ts-reset", "changelog-writer", "ts-migration-planner"],
        ["ts-reset", "ts-migration-planner"],
        "changelog-writer",
      ),
    ).toEqual({ kind: "row", key: "ts-migration-planner" });
    // Skips a following row that left too.
    expect(exploreFocusReturn(["a", "b", "c", "d"], ["a", "d"], "b")).toEqual({ kind: "row", key: "d" });
  });
  it("falls back to the previous row when none follows", () => {
    expect(exploreFocusReturn(["a", "b", "c"], ["a", "b"], "c")).toEqual({ kind: "row", key: "b" });
    expect(exploreFocusReturn(["a", "b", "c", "d"], ["a", "x"], "c")).toEqual({ kind: "row", key: "a" });
  });
  it("falls to the first row shown when none of the opened-from rows survives, or the row was not in it", () => {
    expect(exploreFocusReturn(["a", "b"], ["x", "y"], "a")).toEqual({ kind: "row", key: "x" });
    expect(exploreFocusReturn(["a", "b"], ["a", "b"], "z")).toEqual({ kind: "row", key: "a" });
  });
  it("goes to the search when no row is left", () => {
    expect(exploreFocusReturn(["a"], [], "a")).toEqual({ kind: "search" });
    expect(exploreFocusReturn([], [], "a")).toEqual({ kind: "search" });
  });
});

describe("formatPopularity", () => {
  it("is compact on the row and in full for its name", () => {
    expect(formatPopularity(950)).toEqual({ short: "950", full: "950" });
    expect(formatPopularity(12345)).toEqual({ short: "12.3K", full: "12,345" });
    expect(formatPopularity(2_400_000)).toEqual({ short: "2.4M", full: "2,400,000" });
  });
});
