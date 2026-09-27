// The four fixture scenarios, selected with `?scenario=` (default `rich`).
//
// - `rich`       the round-18 prototype roster (`.scratch/round18/prototypes/
//                BRIEF.md`): 60 skills, 7 detected Tools, 3 projects, mixed
//                Sync states — so the prototypes and the app show one library.
// - `empty`      configured app, nothing managed, nothing to import.
// - `first-run`  never configured: no selection, every Tool newly detected,
//                a sizeable onboarding plan waiting to be imported.
// - `failures`   failure-heavy: per-target errors, TARGET_EXISTS, skipped
//                rows, Unlocatable skills, failing acquisitions, a missing
//                project folder.
//
// Only states the real backend emits appear. In particular `stale` is
// target drift of a copy-mode *project assignment* (reconcile), never
// "update available" — the backend has no such state.

import type { CommandError, FeaturedSkillDto, OnlineSkillDto } from "../bindings";
import {
  HOME,
  defaultSettings,
  fingerprint,
  globalRoot,
  makeProject,
  makeSkill,
  type FixtureLocalFolder,
  type FixtureRepo,
  type FixtureRepoCandidate,
  type FixtureState,
  type ForeignDir,
  type ScenarioName,
  type SkillSpec,
} from "./model";

/** The operator's detected Tools in every scenario (the brief's seven). */
export const DETECTED_TOOLS = [
  "cursor",
  "claude_code",
  "codex",
  "amp",
  "kimi_cli",
  "pi",
  "gemini_cli",
];

const ALL = DETECTED_TOOLS;
const without = (...tools: string[]) => ALL.filter((t) => !tools.includes(t));

// ---------------------------------------------------------------------------
// The shared upstream world: repositories, local folders, Explore data
// ---------------------------------------------------------------------------

type RosterEntry = [name: string, description: string];

const MATTPOCOCK: RosterEntry[] = [
  ["ts-error-fixer", "Diagnose and fix TypeScript errors by category"],
  ["vitest-migration", "Migrate a Jest suite to Vitest without behaviour changes"],
  ["react-perf-audit", "Find wasted renders and heavy effects in React trees"],
  ["zod-schema-first", "Design API boundaries schema-first with Zod"],
  ["prisma-review", "Review Prisma schemas and queries for N+1 and missing indexes"],
  ["ts-generics-coach", "Explain and simplify generic type signatures"],
  ["api-contract-review", "Review a public API surface for breaking changes"],
  ["type-safe-env", "Validate environment variables at startup with types"],
  ["effect-ts-basics", "Introduce Effect gradually into an existing codebase"],
  ["node-esm-migration", "Move a Node package from CommonJS to ESM"],
  ["tsconfig-doctor", "Audit tsconfig options against the project's runtime"],
  ["monorepo-boundaries", "Enforce package boundaries in a workspace monorepo"],
  ["react-hook-form-patterns", "Build forms with react-hook-form and typed resolvers"],
  ["tanstack-query-review", "Review TanStack Query keys, caching and invalidation"],
  ["strict-null-migration", "Turn on strictNullChecks one module at a time"],
  ["dependency-audit", "Find unused, duplicated and risky dependencies"],
  ["pnpm-workspace-setup", "Set up a pnpm workspace with shared tooling"],
  ["branded-types", "Model IDs and units with branded types"],
  ["error-handling-result", "Replace thrown errors with typed Result values"],
  ["jsdoc-to-ts", "Convert JSDoc-typed JavaScript to TypeScript"],
  ["test-data-builders", "Write test data builders instead of fixtures"],
  ["playwright-e2e-review", "Review Playwright tests for flakiness"],
  ["improve-codebase-architecture", "Find deepening opportunities in a codebase"],
  ["write-a-prd", "Turn a conversation into a product requirements doc"],
  ["prd-to-issues", "Split a PRD into independently shippable issues"],
  ["discriminated-unions", "Model state machines with discriminated unions"],
  ["satisfies-operator", "Use satisfies to keep literal types and still check shape"],
  ["ts-library-publishing", "Publish a dual-format TypeScript library"],
];

const ANTHROPICS: RosterEntry[] = [
  ["docx", "Create and edit Word documents with tracked changes"],
  ["pdf", "Extract, fill and merge PDF documents"],
  ["pptx", "Build and edit PowerPoint decks"],
  ["xlsx", "Read, write and analyse spreadsheets with formulas"],
  ["frontend-design", "Build distinctive, production-grade frontend interfaces"],
  ["mcp-builder", "Build high-quality MCP servers"],
  ["webapp-testing", "Test local web apps with Playwright"],
  ["canvas-design", "Create visual art and posters as PNG and PDF"],
  ["artifacts-builder", "Build multi-component HTML artifacts"],
];

const SUPERPOWERS: RosterEntry[] = [
  ["brainstorming", "Refine rough ideas into designs through questions"],
  ["tdd", "Red-green-refactor, one failing test at a time"],
  ["systematic-debugging", "Four-phase debugging: root cause before fixes"],
  ["verification-before-completion", "Prove a change works before claiming it is done"],
  ["writing-plans", "Write implementation plans an engineer can follow cold"],
  ["executing-plans", "Execute a written plan in batches with review checkpoints"],
  ["subagent-driven-development", "Dispatch a fresh subagent per task with review between"],
  ["condition-based-waiting", "Replace arbitrary sleeps with condition polling"],
  ["root-cause-tracing", "Trace a bug backwards through the call stack"],
  ["defense-in-depth", "Validate at every layer data passes through"],
  ["using-git-worktrees", "Isolate feature work in git worktrees"],
  ["finishing-a-development-branch", "Decide how to integrate a finished branch"],
];

const VERCEL: RosterEntry[] = [
  ["react-best-practices", "React and Next.js performance rules from Vercel"],
  ["composition-patterns", "React composition patterns that scale"],
  ["web-design-guidelines", "Review UI code against web interface guidelines"],
  ["next-app-router", "Structure Next.js App Router projects"],
  ["edge-runtime", "Write code that runs on the edge runtime"],
  ["turbo-monorepo", "Configure Turborepo pipelines and caching"],
];

function repo(
  slug: string,
  base: string,
  entries: RosterEntry[],
  extra: RosterEntry[] = [],
): FixtureRepo {
  const candidates: FixtureRepoCandidate[] = [...entries, ...extra].map(
    ([name, description], i) => ({
      name,
      description,
      subpath: `${base}/${name}`,
      installs: Math.round(1_400_000 / (i + 2) + fingerprint(name).charCodeAt(3) * 311),
    }),
  );
  return { slug, candidates };
}

function sharedRepos(): FixtureRepo[] {
  return [
    repo("mattpocock/skills", "skills/engineering", MATTPOCOCK, [
      ["grill-me", "Interview the operator relentlessly about a plan"],
      ["git-guardrails", "Block dangerous git commands before they run"],
      ["setup-matt-pocock-skills", "Configure a repo for the rest of these skills"],
    ]),
    repo("anthropics/skills", "skills", ANTHROPICS, [
      ["skill-creator", "Create and iterate on new skills"],
      ["brand-guidelines", "Apply brand colours and typography to artifacts"],
    ]),
    repo("obra/superpowers", "skills", SUPERPOWERS, [
      ["writing-skills", "Write skills that agents actually follow"],
    ]),
    repo("vercel-labs/agent-skills", "skills", VERCEL, [
      ["vercel-deploy", "Deploy a project to Vercel from the terminal"],
    ]),
    repo("vercel-labs/skills", "skills", [
      ["find-skills", "Discover and install agent skills for a task"],
    ]),
    repo("alexstark/handoff", ".", [
      ["handoff", "Compact a conversation into a handoff document"],
    ]),
    repo("ego-browser/skill", ".", [
      ["ego-browser", "Drive the operator's browser for web tasks"],
    ]),
    repo("t3-code/orchestration", "skills", [
      ["t3-orchestration", "Delegate work to T3 Code child threads"],
    ]),
    repo("some-org/ui-ux-pro-max", ".", [
      ["ui-ux-pro-max", "UI/UX design intelligence for any stack"],
    ]),
    // The Add flow's demo repositories.
    repo("total-typescript/skills", "skills", [
      ["ts-reset", "Apply ts-reset and explain what it changes"],
      ["type-transformations", "Practise mapped and conditional type transformations"],
      ["generics-deep-dive", "Work through generic inference step by step"],
      ["ts-migration-planner", "Plan an incremental JavaScript to TypeScript migration"],
    ]),
    repo("acme/changelog-skill", ".", [
      ["changelog-writer", "Draft a changelog entry from merged pull requests"],
    ]),
  ];
}

function sharedLocalFolders(): FixtureLocalFolder[] {
  return [
    {
      path: `${HOME}/Projects/pi-lens/skill`,
      candidates: [
        {
          name: "pi-lens",
          description: "Run pi-lens diagnostics before handing work back",
          subpath: ".",
          valid: true,
          reason: null,
        },
      ],
    },
    {
      path: `${HOME}/Projects/team-skills`,
      candidates: [
        { name: "deploy-checklist", description: "Walk the release checklist before a deploy", subpath: "deploy-checklist", valid: true, reason: null },
        { name: "incident-runbook", description: "Run the incident process from page to postmortem", subpath: "incident-runbook", valid: true, reason: null },
        { name: "oncall-handoff", description: "Hand the pager over with open threads listed", subpath: "oncall-handoff", valid: true, reason: null },
        { name: "draft-notes", description: null, subpath: "draft-notes", valid: false, reason: "missing_name" },
        { name: "scratch", description: null, subpath: "scratch", valid: false, reason: "missing_skill_md" },
      ],
    },
  ];
}

function featuredFrom(repos: FixtureRepo[]): FeaturedSkillDto[] {
  const out: FeaturedSkillDto[] = [];
  for (const r of repos) {
    for (const c of r.candidates.slice(0, 6)) {
      out.push({
        slug: c.name,
        name: c.name,
        summary: `${c.description}.`,
        downloads: c.installs,
        stars: 0,
        source_url: `https://github.com/${r.slug}/tree/main/${c.subpath}`,
      });
    }
  }
  return out.sort((a, b) => b.downloads - a.downloads);
}

function onlineFrom(repos: FixtureRepo[]): OnlineSkillDto[] {
  return repos.flatMap((r) =>
    r.candidates.map((c) => ({
      name: c.name,
      installs: c.installs,
      source: r.slug,
      source_url: `https://github.com/${r.slug}`,
    })),
  );
}

function foreign(tool: string, name: string, content: string, linkTo?: string): ForeignDir {
  return {
    tool,
    name,
    fingerprint: fingerprint(content),
    linkTarget: linkTo ? `${globalRoot(linkTo)}/${name}` : null,
  };
}

function baseState(scenario: ScenarioName, now: number): FixtureState {
  const repos = sharedRepos();
  return {
    scenario,
    settings: defaultSettings(ALL),
    installedTools: [...DETECTED_TOOLS],
    seenTools: [...DETECTED_TOOLS],
    skills: [],
    projects: [],
    foreign: [],
    repos,
    localFolders: sharedLocalFolders(),
    featured: featuredFrom(repos),
    online: onlineFrom(repos),
    hiddenExplore: [],
    unwritableTools: [],
    unwritableProjectTools: [],
    lockedPaths: [],
    dialogPicks: [`${HOME}/Projects/team-skills`, `${HOME}/Projects/new-service`],
    gitCacheEntries: 6,
    nextId: Math.floor(now / 1000) % 100_000,
    cancelRequested: false,
  };
}

// ---------------------------------------------------------------------------
// rich — the prototype roster
// ---------------------------------------------------------------------------

function rosterSpecs(): SkillSpec[] {
  const git = (repo: string) => ({ kind: "git" as const, repo });
  const specs: SkillSpec[] = [];
  for (const [name, description] of MATTPOCOCK) {
    const spec: SkillSpec = { name, description, source: git("mattpocock/skills"), synced: ALL };
    if (name === "vitest-migration") spec.synced = without("amp", "kimi_cli");
    if (name === "zod-schema-first") spec.synced = without("cursor");
    if (name === "prisma-review") spec.synced = [];
    if (name === "effect-ts-basics") spec.synced = without("gemini_cli");
    if (name === "tsconfig-doctor") spec.synced = without("pi");
    if (name === "ts-generics-coach") spec.override = { mode: "user-only" };
    if (name === "api-contract-review") spec.override = { mode: "model-only" };
    if (name === "write-a-prd") spec.invocation = "user-only";
    specs.push(spec);
  }
  for (const [name, description] of ANTHROPICS) {
    specs.push({ name, description, source: git("anthropics/skills"), synced: ALL });
  }
  for (const [name, description] of SUPERPOWERS) {
    const spec: SkillSpec = { name, description, source: git("obra/superpowers"), synced: ALL };
    if (name === "using-git-worktrees") {
      spec.synced = ["cursor", "claude_code", "codex", "pi"];
      spec.errored = ["amp", "kimi_cli"];
    }
    specs.push(spec);
  }
  for (const [name, description] of VERCEL) {
    specs.push({ name, description, source: git("vercel-labs/agent-skills"), synced: ALL });
  }
  specs.push(
    { name: "handoff", description: "Compact a conversation into a handoff document", source: git("alexstark/handoff"), synced: ALL, invocation: "user-only", ageDays: 4 },
    { name: "ego-browser", description: "Drive the operator's browser for web tasks", source: git("ego-browser/skill"), synced: ALL, ageDays: 9 },
    { name: "pi-lens", description: "Run pi-lens diagnostics before handing work back", source: { kind: "local", path: `${HOME}/Projects/pi-lens/skill` }, synced: ALL, ageDays: 2 },
    { name: "t3-orchestration", description: "Delegate work to T3 Code child threads", source: git("t3-code/orchestration"), synced: ALL, ageDays: 1 },
    { name: "ui-ux-pro-max", description: "UI/UX design intelligence for any stack", source: git("some-org/ui-ux-pro-max"), synced: ALL, override: { mode: "user-only" }, ageDays: 12 },
  );
  return specs;
}

function rich(now: number): FixtureState {
  const state = baseState("rich", now);
  state.skills = rosterSpecs().map((spec, i) => makeSkill(spec, now, i));
  // using-git-worktrees' Kimi/Amp rows failed their last propagation (the
  // shared dir was read-only then); it is writable now, so a re-sync or
  // Refresh recovers them. Refresh lands new revisions for a few skills.
  for (const name of ["using-git-worktrees", "vitest-migration", "tdd", "t3-orchestration", "react-perf-audit"]) {
    const skill = state.skills.find((s) => s.dto.name === name);
    if (skill) skill.acquisition.upstreamChanged = true;
  }
  state.projects = [
    makeProject(
      {
        id: "prj-skills-hub",
        path: `${HOME}/Projects/skills-hub`,
        tools: ["claude_code", "agents_skills", "pi"],
        ageDays: 40,
        gitignore: { in_gitignore: true },
        assignments: [
          { skill: "react-perf-audit", tool: "claude_code" },
          { skill: "react-perf-audit", tool: "agents_skills" },
          { skill: "frontend-design", tool: "claude_code" },
          { skill: "t3-orchestration", tool: "claude_code", status: "stale" },
          { skill: "handoff", tool: "claude_code" },
          { skill: "handoff", tool: "pi" },
          { skill: "ts-error-fixer", tool: "agents_skills" },
        ],
      },
      state.skills,
      now,
    ),
    makeProject(
      {
        id: "prj-quartermaster",
        path: `${HOME}/Projects/quartermaster`,
        tools: ["claude_code", "agents_skills"],
        ageDays: 25,
        assignments: [
          { skill: "react-perf-audit", tool: "claude_code" },
          { skill: "vitest-migration", tool: "agents_skills", status: "stale" },
          { skill: "systematic-debugging", tool: "claude_code" },
          { skill: "writing-plans", tool: "claude_code" },
          { skill: "executing-plans", tool: "claude_code" },
        ],
      },
      state.skills,
      now,
    ),
    makeProject(
      {
        id: "prj-agent-skills",
        path: `${HOME}/Projects/agent-skills`,
        tools: ["claude_code", "pi"],
        ageDays: 12,
        gitignore: { in_exclude: true },
        assignments: [
          { skill: "tdd", tool: "pi", status: "stale" },
          { skill: "brainstorming", tool: "claude_code" },
          { skill: "brainstorming", tool: "pi" },
          { skill: "mcp-builder", tool: "claude_code" },
        ],
      },
      state.skills,
      now,
    ),
  ];
  state.foreign = [
    // Why zod-schema-first is 6/7: Cursor already holds a different copy,
    // so the next sync there settles TARGET_EXISTS and raises the ask.
    foreign("cursor", "zod-schema-first", "cursor's own zod notes"),
    // Collides with total-typescript/skills' ts-reset (Add → sync → ask).
    foreign("claude_code", "ts-reset", "a hand-written ts-reset"),
    foreign("claude_code", "commit-message", "commit-message v1"),
    foreign("codex", "commit-message", "commit-message v1", "claude_code"),
    foreign("pi", "release-notes", "release notes"),
    foreign("claude_code", "sql-review", "sql review A"),
    foreign("gemini_cli", "sql-review", "sql review B"),
  ];
  return state;
}

// ---------------------------------------------------------------------------
// empty / first-run
// ---------------------------------------------------------------------------

function empty(now: number): FixtureState {
  return baseState("empty", now);
}

function firstRun(now: number): FixtureState {
  const state = baseState("first-run", now);
  state.settings = defaultSettings(null);
  state.seenTools = [];
  state.gitCacheEntries = 0;
  const plan: Array<[string, string[], boolean]> = [
    // name, tools holding it, divergent copies?
    ["frontend-design", ["claude_code", "cursor"], false],
    ["pdf", ["claude_code"], false],
    ["tdd", ["claude_code", "codex", "pi"], false],
    ["systematic-debugging", ["claude_code", "pi"], false],
    ["handoff", ["claude_code", "pi"], true],
    ["ego-browser", ["claude_code", "codex", "amp"], false],
    ["commit-message", ["codex", "gemini_cli"], true],
    ["mcp-builder", ["cursor"], false],
    ["writing-plans", ["claude_code"], false],
    ["web-design-guidelines", ["cursor", "codex"], false],
    ["release-notes", ["pi"], false],
    ["sql-review", ["gemini_cli", "claude_code"], true],
  ];
  for (const [name, tools, divergent] of plan) {
    tools.forEach((tool, i) => {
      const linkBack = !divergent && i > 0 && name.length % 2 === 0 ? tools[0] : undefined;
      state.foreign.push(foreign(tool, name, divergent ? `${name} variant ${i}` : `${name} body`, linkBack));
    });
  }
  return state;
}

// ---------------------------------------------------------------------------
// failures — every report fold has something to say
// ---------------------------------------------------------------------------

const RATE_LIMITED: CommandError = { code: "RATE_LIMITED", resetMinutes: 12 };

function failures(now: number): FixtureState {
  const state = baseState("failures", now);
  // Windsurf is selected but no longer detected: every global sync reports
  // it skipped (TOOL_NOT_INSTALLED) — the operator's stale-selection signal.
  state.settings = defaultSettings([...ALL, "windsurf"]);
  // Kimi and Amp share ~/.config/agents/skills, which refuses writes.
  state.unwritableTools = ["amp", "kimi_cli"];
  state.unwritableProjectTools = ["prj-monorepo:agents_skills"];
  state.lockedPaths = [`${globalRoot("claude_code")}/release-train`];
  state.repos.push(
    { slug: "corp/private-skills", candidates: [], listingError: { code: "GIT_CLONE_FAILED", kind: "auth", detail: "remote: Repository not found.\nfatal: Authentication failed for 'https://github.com/corp/private-skills/'" } },
    { slug: "rate/limited", candidates: [], listingError: RATE_LIMITED },
  );
  const git = (repo: string) => ({ kind: "git" as const, repo });
  const specs: SkillSpec[] = [
    { name: "tdd", description: "Red-green-refactor, one failing test at a time", source: git("obra/superpowers"), synced: without("amp", "kimi_cli"), errored: ["amp", "kimi_cli"] },
    { name: "using-git-worktrees", description: "Isolate feature work in git worktrees", source: git("obra/superpowers"), synced: ["claude_code", "codex", "pi"], errored: ["amp", "kimi_cli", "cursor"] },
    { name: "frontend-design", description: "Build distinctive, production-grade frontend interfaces", source: git("anthropics/skills"), synced: without("amp", "kimi_cli"), acquisition: { upstreamChanged: true } },
    { name: "docx", description: "Create and edit Word documents with tracked changes", source: git("anthropics/skills"), synced: ALL, unlocatable: "central_missing" },
    { name: "local-notes", description: "Keep running notes in the project's docs folder", source: { kind: "local", path: `${HOME}/Projects/notes-skill` }, synced: ["claude_code", "pi"], unlocatable: "source_missing" },
    { name: "scratchpad", description: "A scratch skill that lived in a deleted folder", source: { kind: "local", path: `${HOME}/Downloads/scratchpad` }, synced: ["claude_code"], unlocatable: "source_missing", centralGone: true },
    { name: "pr-review", description: "Review a pull request against the team checklist", source: git("mattpocock/skills"), synced: without("amp", "kimi_cli"), acquisition: { fail: RATE_LIMITED } },
    { name: "deprecated-helper", description: "Helper removed upstream last month", source: git("mattpocock/skills"), synced: ["claude_code", "codex"], acquisition: { fail: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/mattpocock/skills/tree/main/skills/deprecated-helper" } } },
    { name: "corp-conventions", description: "Company coding conventions", source: git("corp/private-skills"), synced: ["claude_code", "cursor"], acquisition: { fail: { code: "GIT_CLONE_FAILED", kind: "auth", detail: "fatal: Authentication failed for 'https://github.com/corp/private-skills/'" } } },
    { name: "mirror-tools", description: "Tools mirrored from an internal host", source: git("corp/mirror-tools"), synced: ["claude_code"], acquisition: { fail: { code: "GIT_CLONE_FAILED", kind: "tls", detail: "SSL certificate problem: self-signed certificate in certificate chain" } } },
    { name: "alias-skill", description: "Upstream points this skill at a path outside the repo", source: git("some-org/aliases"), synced: ["claude_code"], acquisition: { fail: { code: "SYMLINK_ESCAPES_REPO", subpath: "skills/alias-skill", target: "/etc/passwd" } } },
    { name: "design-tokens", description: "Keep design tokens in sync across platforms", source: git("some-org/design-tokens"), synced: without("amp", "kimi_cli"), override: { mode: "user-only", conflict: true }, acquisition: { upstreamChanged: true, editConflict: { base_mode: "user-and-model", upstream_mode: "model-only", override_mode: "user-only" } } },
    { name: "flaky-upstream", description: "Upstream force-pushes during every refresh", source: git("some-org/flaky"), synced: ["claude_code", "codex"], acquisition: { skip: "stale_acquisition" } },
    { name: "store-hiccup", description: "Refreshes fine, then the re-assert hits a locked database", source: git("some-org/hiccup"), synced: ["claude_code"], acquisition: { upstreamChanged: true, reassertError: { code: "OTHER", message: "database is locked" } } },
    { name: "release-train", description: "Cut and ship a release train", source: git("acme/release-train"), synced: ["claude_code", "codex", "pi"], mode: "copy" },
    { name: "legacy-lint", description: "Imported from Cursor during onboarding", source: { kind: "imported", fromTool: "cursor" }, synced: ["cursor", "claude_code"] },
    { name: "mystery-skill", description: "Imported; the Tool it came from was not recorded", source: { kind: "imported", fromTool: null }, synced: ["claude_code"] },
    { name: "windsurf-rules", description: "Synced to Windsurf before it was uninstalled", source: git("some-org/windsurf-rules"), synced: ["windsurf", "claude_code"] },
    { name: "zod-schema-first", description: "Design API boundaries schema-first with Zod", source: git("mattpocock/skills"), synced: ["claude_code"] },
    { name: "prisma-review", description: "Review Prisma schemas and queries for N+1 and missing indexes", source: git("mattpocock/skills"), synced: [] },
  ];
  state.skills = specs.map((spec, i) => makeSkill(spec, now, i));
  state.projects = [
    makeProject(
      {
        id: "prj-monorepo",
        path: `${HOME}/Projects/monorepo`,
        tools: ["claude_code", "agents_skills", "windsurf"],
        ageDays: 60,
        assignments: [
          { skill: "tdd", tool: "claude_code" },
          { skill: "tdd", tool: "agents_skills", status: "error", lastError: "Permission denied (os error 13): /Users/alex/Projects/monorepo/.agents/skills/tdd" },
          { skill: "frontend-design", tool: "claude_code", status: "stale" },
          { skill: "frontend-design", tool: "windsurf", status: "missing", mode: "copy" },
          { skill: "release-train", tool: "claude_code", status: "pending" },
          { skill: "docx", tool: "claude_code", status: "missing" },
          { skill: "design-tokens", tool: "claude_code", status: "stale" },
        ],
      },
      state.skills,
      now,
    ),
    makeProject(
      {
        id: "prj-archived-app",
        path: `${HOME}/Projects/archived-app`,
        pathExists: false,
        tools: ["claude_code"],
        ageDays: 200,
        assignments: [
          { skill: "tdd", tool: "claude_code", status: "missing" },
          { skill: "pr-review", tool: "claude_code", status: "missing" },
        ],
      },
      state.skills,
      now,
    ),
    makeProject(
      {
        id: "prj-quartermaster",
        path: `${HOME}/Projects/quartermaster`,
        tools: ["claude_code", "pi"],
        ageDays: 25,
        assignments: [
          { skill: "tdd", tool: "claude_code" },
          { skill: "using-git-worktrees", tool: "pi" },
        ],
      },
      state.skills,
      now,
    ),
    makeProject(
      { id: "prj-empty-shell", path: `${HOME}/Projects/empty-shell`, tools: [], ageDays: 3, assignments: [] },
      state.skills,
      now,
    ),
  ];
  state.foreign = [
    // Occupied targets: syncing these skills there raises the overwrite ask.
    foreign("cursor", "zod-schema-first", "cursor zod"),
    foreign("cursor", "prisma-review", "cursor prisma"),
    foreign("codex", "prisma-review", "codex prisma"),
    foreign("gemini_cli", "frontend-design", "gemini frontend"),
    foreign("claude_code", "ts-reset", "a hand-written ts-reset"),
    // Onboarding: a name taken by a Managed skill (SKILL_EXISTS on import),
    // a conflicting group, and an original in the unwritable shared dir.
    foreign("pi", "tdd", "pi's tdd fork"),
    foreign("claude_code", "sql-review", "sql review A"),
    foreign("gemini_cli", "sql-review", "sql review B"),
    foreign("amp", "commit-message", "commit-message v1"),
    foreign("codex", "commit-message", "commit-message v1"),
    foreign("cursor", "broken-skill", "no frontmatter at all"),
  ];
  return state;
}

const BUILDERS: Record<ScenarioName, (now: number) => FixtureState> = {
  rich,
  empty,
  "first-run": firstRun,
  failures,
};

export function buildScenario(name: ScenarioName, now = Date.now()): FixtureState {
  return BUILDERS[name](now);
}

/** Onboarding groups whose import finalize fails (bad manifest). */
export const BROKEN_IMPORT_NAMES = new Set(["broken-skill"]);
