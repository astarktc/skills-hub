/**
 * PROTOTYPE — throwaway (round 20, Wave B grill Q14/Q25/Q27). Not product code.
 *
 * Question: which Harnesses-pane model is clearer once shared folders serve
 * several harnesses at once? Three variants on one fixture-only route,
 * switchable with `?variant=a|b|c` and the floating bar at the bottom:
 *   A — per-harness switches, folders derived (today's list + explanation lines)
 *   B — flat folder list, paths first (cheap extra)
 *   C — folder cards, one switch per thing written (the grill's pick, to confirm)
 * Each variant also renders the Add dialog's "Deploy after adding" picker in
 * its own model (Q25: one model on every surface).
 *
 * Reached only in fixture mode: `npm run dev:fixture` →
 * http://localhost:5175/?proto=harnesses&variant=c (main.tsx's fixture branch
 * is a compile-time constant, so production never bundles this file).
 * Read facts are a hand copy of the registry (`core/tool_adapters/mod.rs`
 * `reads`, `READER_ONLY_HARNESSES`); paths are post-#108 (Q12). Strings are
 * prototype labels, not product copy.
 */
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { ChevronLeft, ChevronRight, Moon, Sun, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldDescription, FieldLabel, Select } from "@/components/ui/field";
import { Segmented, SegmentedItem } from "@/components/ui/segmented";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { PaneHeader, SettingGroup, SettingRow } from "../PaneDialog";

// ---------------------------------------------------------------------------
// Data (registry facts, operator's machine)
// ---------------------------------------------------------------------------

const AGENTS = "~/.agents/skills";
const CLAUDE = "~/.claude/skills";
const SHARED = [AGENTS, CLAUDE] as const;
const LIBRARY_SKILLS = 33;

type Read = "yes" | "no" | "conditional" | "unknown";
type Dup = "shows_all" | "shadows" | "loads_none" | "flags_conflict" | "unknown";
type Harness = {
  key: string;
  label: string;
  /** Own global folder ("" for a reader-only harness). */
  own: string;
  agents: Read;
  claude: Read;
  /** Other folders read by default. */
  also: readonly string[];
  link: boolean;
  dup: Dup;
  installed: boolean;
  readerOnly?: true;
};

const H = (h: Omit<Harness, "also" | "link"> & Partial<Pick<Harness, "also" | "link">>): Harness => ({
  also: [],
  link: true,
  ...h,
});

const HARNESSES: readonly Harness[] = [
  // installed (the operator's three + the ones that make the cards non-trivial)
  H({ key: "claude_code", label: "Claude Code", own: CLAUDE, agents: "no", claude: "yes", dup: "unknown", installed: true }),
  H({ key: "codex", label: "Codex", own: AGENTS, agents: "yes", claude: "no", also: ["~/.codex/skills"], dup: "shows_all", installed: true }),
  H({ key: "pi", label: "Pi", own: "~/.pi/agent/skills", agents: "yes", claude: "no", dup: "shadows", installed: true }),
  H({ key: "cursor", label: "Cursor", own: "~/.cursor/skills", agents: "yes", claude: "yes", also: ["~/.codex/skills"], dup: "shows_all", installed: true }),
  H({ key: "gemini_cli", label: "Gemini CLI", own: "~/.gemini/skills", agents: "yes", claude: "no", dup: "shadows", installed: true }),
  H({ key: "cortex_code", label: "Cortex Code", own: "~/.snowflake/cortex/skills", agents: "no", claude: "yes", dup: "flags_conflict", installed: true }),
  H({ key: "deep_agents_code", label: "Deep Agents Code", own: "~/.deepagents/agent/skills", agents: "yes", claude: "yes", link: false, dup: "shadows", installed: true }),
  H({ key: "antigravity", label: "Antigravity", own: "~/.gemini/config/skills", agents: "no", claude: "no", dup: "unknown", installed: true }),
  // not detected
  H({ key: "opencode", label: "OpenCode", own: "~/.config/opencode/skills", agents: "yes", claude: "yes", dup: "shows_all", installed: false }),
  H({ key: "goose", label: "Goose", own: AGENTS, agents: "yes", claude: "yes", dup: "unknown", installed: false }),
  H({ key: "openhands", label: "OpenHands", own: AGENTS, agents: "yes", claude: "no", dup: "unknown", installed: false }),
  H({ key: "zencoder", label: "Zencoder / Zenflow", own: AGENTS, agents: "yes", claude: "unknown", dup: "unknown", installed: false }),
  H({ key: "neovate", label: "Neovate", own: "~/.neovate/skills", agents: "no", claude: "yes", dup: "unknown", installed: false }),
  H({ key: "augment", label: "Augment", own: "~/.augment/skills", agents: "unknown", claude: "unknown", dup: "unknown", installed: false }),
  // reader-only (no folder of their own we deploy to)
  H({ key: "zed", label: "Zed", own: "", agents: "yes", claude: "no", dup: "unknown", installed: true, readerOnly: true }),
  H({
    key: "warp",
    label: "Warp",
    own: "",
    agents: "yes",
    claude: "yes",
    also: ["~/.warp/skills", "~/.codex/skills", "~/.cursor/skills", "~/.gemini/skills"],
    dup: "shows_all",
    installed: true,
    readerOnly: true,
  }),
  H({
    key: "fx",
    label: "fx",
    own: "",
    agents: "yes",
    claude: "yes",
    also: ["~/.fx/skills", "~/.config/opencode/skills", "~/.codex/skills"],
    dup: "loads_none",
    installed: true,
    readerOnly: true,
  }),
  H({ key: "dexto", label: "Dexto", own: "", agents: "yes", claude: "no", also: ["~/.dexto/skills"], dup: "unknown", installed: false, readerOnly: true }),
  H({ key: "crow_cli", label: "crow-cli", own: "", agents: "yes", claude: "no", dup: "unknown", installed: false, readerOnly: true }),
];

/** Today's saved selection on the operator's machine (Claude Code, Codex, Pi; `.agents/skills` folds into Codex). */
const INITIAL_SELECTED = ["claude_code", "codex", "pi"];

const DUP_WORDS: Record<Dup, string> = {
  shows_all: "lists every copy",
  shadows: "one copy hides the other",
  loads_none: "invoking a skill by name loads neither copy",
  flags_conflict: "flags differing copies as a conflict",
  unknown: "effect unknown",
};

// ---------------------------------------------------------------------------
// The rule (Q15, Q16, Q21)
// ---------------------------------------------------------------------------

type Mode = "link" | "copy";
type Settings = { pref: boolean; both: "agents" | "claude"; modes: Record<string, Mode> };

const short = (path: string) => path.replace("~/", "").replace("/skills", "");
const isShared = (path: string) => (SHARED as readonly string[]).includes(path);
const modeOf = (path: string, s: Settings): Mode => s.modes[path] ?? "link";
const listJoin = (names: readonly string[]) => names.join(", ");

/** The folder that serves a harness. */
function serving(h: Harness, s: Settings): string {
  if (!s.pref) return h.own;
  let shared: string | null = null;
  if (h.agents === "yes" && h.claude !== "yes") shared = AGENTS;
  else if (h.claude === "yes" && h.agents !== "yes") shared = CLAUDE;
  else if (h.agents === "yes" && h.claude === "yes") shared = s.both === "agents" ? AGENTS : CLAUDE;
  if (!shared) return h.own;
  // Q21: a copy-only harness is served by its own folder unless the shared folder copies.
  if (!h.link && modeOf(shared, s) !== "copy") return h.own;
  return shared;
}

function reads(h: Harness, path: string): boolean {
  if (path === AGENTS && h.agents === "yes") return true;
  if (path === CLAUDE && h.claude === "yes") return true;
  if (!h.readerOnly && h.own === path) return true;
  return h.also.includes(path);
}

type FolderGroup = { path: string; served: Harness[] };

/** Every folder that serves at least one (non-reader) harness, shared ones first. */
function folderGroups(s: Settings): FolderGroup[] {
  const map = new Map<string, Harness[]>();
  for (const path of SHARED) map.set(path, []);
  for (const h of HARNESSES) {
    if (h.readerOnly) continue;
    const path = serving(h, s);
    map.set(path, [...(map.get(path) ?? []), h]);
  }
  return [...map.entries()]
    .map(([path, served]) => ({ path, served }))
    .filter((g) => g.served.length > 0 || isShared(g.path))
    .sort((a, b) => Number(isShared(b.path)) - Number(isShared(a.path)));
}

const installedReaders = (path: string, except: readonly Harness[] = []) =>
  HARNESSES.filter((h) => h.installed && reads(h, path) && !except.includes(h));

type DupLine = { harness: Harness; folders: string[] };

/** Q20: one line per installed reader that sees the library in ≥ 2 written folders. */
function duplicateLines(written: ReadonlySet<string>): DupLine[] {
  return HARNESSES.filter((h) => h.installed)
    .map((harness) => ({ harness, folders: [...written].filter((path) => reads(harness, path)) }))
    .filter((line) => line.folders.length >= 2);
}

function dupText({ harness, folders }: DupLine) {
  return `${harness.label} reads ${folders.map(short).join(" and ")}: sees each skill twice — ${DUP_WORDS[harness.dup]}.`;
}

/** Carry each harness's "served" state across a regroup (a redeploy, not a reset). */
function migrateEnabled(enabled: ReadonlySet<string>, from: Settings, to: Settings): Set<string> {
  const next = new Set<string>();
  for (const h of HARNESSES) {
    if (h.readerOnly || !h.installed) continue;
    if (enabled.has(serving(h, from))) next.add(serving(h, to));
  }
  return next;
}

// ---------------------------------------------------------------------------
// Page: pane frame + Add picker + state, and the switcher bar
// ---------------------------------------------------------------------------

const VARIANTS = [
  { key: "a", name: "Per-harness switches" },
  { key: "b", name: "Flat folder list" },
  { key: "c", name: "Folder cards" },
] as const;
type VariantKey = (typeof VARIANTS)[number]["key"];

function variantFromUrl(): VariantKey {
  const v = new URLSearchParams(window.location.search).get("variant")?.toLowerCase();
  return VARIANTS.some((x) => x.key === v) ? (v as VariantKey) : "c";
}

type Model = {
  s: Settings;
  setS: (next: Settings) => void;
  /** Variant A: per-harness selection. */
  selected: ReadonlySet<string>;
  setSelected: (key: string, on: boolean) => void;
  /** Variants B/C: per-folder switches. */
  enabled: ReadonlySet<string>;
  setEnabled: (path: string, on: boolean) => void;
  setMode: (path: string, mode: Mode) => void;
};

export default function HarnessesPrototype() {
  const [variant, setVariant] = useState<VariantKey>(variantFromUrl);
  const [s, setSRaw] = useState<Settings>({ pref: true, both: "agents", modes: {} });
  const [selected, setSelectedSet] = useState<ReadonlySet<string>>(() => new Set(INITIAL_SELECTED));
  // Q26: a shared card is on when any harness it serves was selected.
  const [enabled, setEnabledSet] = useState<ReadonlySet<string>>(
    () => new Set(INITIAL_SELECTED.map((k) => serving(HARNESSES.find((h) => h.key === k)!, { pref: true, both: "agents", modes: {} }))),
  );

  const setS = (next: Settings) => {
    setEnabledSet((current) => migrateEnabled(current, s, next));
    setSRaw(next);
  };
  const model: Model = {
    s,
    setS,
    selected,
    setSelected: (key, on) =>
      setSelectedSet((cur) => {
        const next = new Set(cur);
        if (on) next.add(key);
        else next.delete(key);
        return next;
      }),
    enabled,
    setEnabled: (path, on) =>
      setEnabledSet((cur) => {
        const next = new Set(cur);
        if (on) next.add(path);
        else next.delete(path);
        return next;
      }),
    setMode: (path, mode) => setS({ ...s, modes: { ...s.modes, [path]: mode } }),
  };

  const written = useMemo(
    () =>
      variant === "a"
        ? new Set(HARNESSES.filter((h) => selected.has(h.key)).map((h) => serving(h, s)))
        : new Set(enabled),
    [variant, selected, enabled, s],
  );

  const go = (next: VariantKey) => {
    const url = new URL(window.location.href);
    url.searchParams.set("variant", next);
    window.history.replaceState(null, "", url);
    setVariant(next);
  };

  return (
    <div className="min-h-screen bg-background p-6 pb-24 text-foreground">
      <div className="flex flex-wrap items-start gap-6">
        <PaneFrame>
          {variant === "a" ? <VariantA m={model} written={written} /> : null}
          {variant === "b" ? <VariantB m={model} written={written} /> : null}
          {variant === "c" ? <VariantC m={model} written={written} /> : null}
        </PaneFrame>
        <div className="flex w-80 flex-col gap-6">
          <AddPicker key={`${variant}|${[...enabled].join()}|${[...selected].join()}|${s.pref}|${s.both}`} variant={variant} m={model} />
          <WrittenState written={written} s={s} />
        </div>
      </div>
      <Switcher current={variant} onChange={go} />
    </div>
  );
}

/** A copy of the Settings pane frame (880 × 640, pane list on the left). */
function PaneFrame({ children }: { children: ReactNode }) {
  return (
    <div
      className={cn(
        "flex w-[880px] flex-col overflow-hidden rounded-lg border border-border-strong bg-popover shadow-lg",
        // `&frame=full` unclips the pane for full-length screenshots; the default is the real 640 px frame.
        new URLSearchParams(window.location.search).get("frame") === "full" ? "" : "h-[640px]",
      )}
    >
      <div className="flex h-12 shrink-0 items-center border-b border-border px-5 text-sm font-semibold">Settings</div>
      <div className="flex min-h-0 flex-1">
        <nav className="flex w-44 shrink-0 flex-col gap-0.5 border-r border-border p-2 text-sm">
          {["General", "Harnesses", "Appearance", "Updates", "Diagnostics"].map((pane) => (
            <span
              key={pane}
              className={cn(
                "flex h-8 items-center rounded-md px-2",
                pane === "Harnesses" ? "bg-raised text-foreground" : "text-muted-foreground",
              )}
            >
              {pane}
            </span>
          ))}
        </nav>
        <div className="min-w-0 flex-1 overflow-y-auto">
          <div className="flex flex-col gap-5 px-6 py-5">{children}</div>
        </div>
      </div>
    </div>
  );
}

/** The global preference + the dropdown (every variant needs them: the rule is global). */
function GlobalRule({ m }: { m: Model }) {
  return (
    <SettingGroup title="Folder rule">
      <SettingRow control={<Switch size="sm" checked={m.s.pref} onCheckedChange={(pref) => m.setS({ ...m.s, pref })} aria-label="Use shared folders" />}>
        <span className="text-sm text-foreground">Use shared folders when a harness can read them</span>
        <span className="text-xs text-muted-foreground">
          One copy in ~/.agents/skills or ~/.claude/skills serves every harness that reads it. Off: each harness gets its own folder.
        </span>
      </SettingRow>
      <SettingRow
        control={
          <Select
            size="sm"
            className="w-36"
            value={m.s.both}
            disabled={!m.s.pref}
            onChange={(e) => m.setS({ ...m.s, both: e.target.value === "claude" ? "claude" : "agents" })}
            aria-label="When a harness reads both"
          >
            <option value="agents">.agents/skills</option>
            <option value="claude">.claude/skills</option>
          </Select>
        }
      >
        <span className="text-sm text-foreground">When a harness reads both shared folders, use</span>
        <span className="text-xs text-muted-foreground">Cursor, Deep Agents Code, OpenCode, Goose, Warp, fx read both.</span>
      </SettingRow>
    </SettingGroup>
  );
}

function ModeControl({ path, m, fixed }: { path: string; m: Model; fixed?: string }) {
  if (fixed) {
    return (
      <Badge variant="outline" tone="neutral" title={fixed}>
        Copy only
      </Badge>
    );
  }
  return (
    <Segmented aria-label={`Deploy ${path} as`} value={modeOf(path, m.s)} onValueChange={(v) => m.setMode(path, v === "copy" ? "copy" : "link")}>
      <SegmentedItem value="link">Link</SegmentedItem>
      <SegmentedItem value="copy">Copy</SegmentedItem>
    </Segmented>
  );
}

function Warn({ children }: { children: ReactNode }) {
  return (
    <p className="m-0 flex items-start gap-1 text-xs text-stale">
      <TriangleAlert aria-hidden="true" className="mt-px size-3.5 shrink-0" />
      <span>{children}</span>
    </p>
  );
}

function DuplicatesSection({ written }: { written: ReadonlySet<string> }) {
  const lines = duplicateLines(written);
  if (lines.length === 0) return null;
  return (
    <section className="flex flex-col gap-1.5">
      <h4 className="m-0 text-xs font-medium text-faint">Seen twice ({lines.length})</h4>
      {lines.map((line) => (
        <Warn key={line.harness.key}>{dupText(line)}</Warn>
      ))}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Variant A — per-harness switches, folders derived
// ---------------------------------------------------------------------------

function VariantA({ m, written }: { m: Model; written: ReadonlySet<string> }) {
  const [undetectedOpen, setUndetectedOpen] = useState(false);
  const deployable = HARNESSES.filter((h) => !h.readerOnly);
  const detected = deployable.filter((h) => h.installed);
  const undetected = HARNESSES.filter((h) => !h.installed);
  const readersOnly = HARNESSES.filter((h) => h.readerOnly && h.installed);
  const dups = duplicateLines(written);
  /** Who switched a folder on (A: the selected harnesses it serves). */
  const onFor = (path: string) => HARNESSES.filter((h) => m.selected.has(h.key) && serving(h, m.s) === path).map((h) => h.label);

  const row = (h: Harness) => {
    const path = serving(h, m.s);
    const on = m.selected.has(h.key);
    const others = installedReaders(path, [h]).map((x) => x.label);
    const stillSees = !on && h.installed ? [...written].filter((p) => reads(h, p)) : [];
    const dup = dups.find((d) => d.harness === h);
    const copyOnlyOwn = !h.link && path === h.own;
    return (
      <li key={h.key} className="flex flex-col gap-1 py-2 pr-3 pl-4">
        <div className="flex items-center gap-3">
          <span className="min-w-0 flex-1 truncate text-sm">{h.label}</span>
          <Badge variant={h.installed ? "soft" : "outline"} tone="neutral">
            {h.installed ? "Detected" : "Not detected"}
          </Badge>
          {isShared(path) ? null : (
            <ModeControl path={path} m={m} fixed={copyOnlyOwn ? "Deep Agents Code rejects links" : undefined} />
          )}
          <Switch size="sm" checked={on} onCheckedChange={(v) => m.setSelected(h.key, v)} aria-label={`Deploy to ${h.label}`} />
        </div>
        <p className="m-0 text-xs text-muted-foreground">
          via <span className="font-mono">{path}</span>
          {isShared(path) ? ` (${modeOf(path, m.s)}, shared)` : ""}
          {isShared(path) && others.length > 0 ? ` · also seen by ${listJoin(others)}` : ""}
          {copyOnlyOwn && m.s.pref ? " · own folder: copy-only, and the shared folder links" : ""}
        </p>
        {stillSees.map((p) => (
          <Warn key={p}>
            Off, but still sees your skills: <span className="font-mono">{short(p)}</span> is on for {listJoin(onFor(p)) || "another harness"}.
          </Warn>
        ))}
        {on && isShared(path) && onFor(path).length > 1 ? (
          <p className="m-0 text-xs text-faint">
            Switching this off alone changes nothing on disk while {listJoin(onFor(path).filter((x) => x !== h.label))} {onFor(path).length > 2 ? "are" : "is"} on.
          </p>
        ) : null}
        {dup ? <Warn>{dupText(dup)}</Warn> : null}
      </li>
    );
  };

  return (
    <>
      <PaneHeader
        title="Harnesses"
        description="Choose which harnesses get your skills. Harnesses that read a shared folder are served from it."
        note="Changes save automatically"
      />
      <GlobalRule m={m} />
      <section className="flex flex-col gap-2">
        <h4 className="m-0 text-xs font-medium text-faint">Harnesses</h4>
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">{detected.map(row)}</ul>
      </section>
      <section className="flex flex-col gap-2">
        <h4 className="m-0 text-xs font-medium text-faint">Read-only harnesses (no switch: they read other harnesses' folders)</h4>
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {readersOnly.map((h) => {
            const seen = [...written].filter((p) => reads(h, p));
            const dup = dups.find((d) => d.harness === h);
            return (
              <li key={h.key} className="flex flex-col gap-1 py-2 pr-3 pl-4">
                <div className="flex items-center gap-3">
                  <span className="min-w-0 flex-1 truncate text-sm">{h.label}</span>
                  <Badge variant="soft" tone="neutral">
                    Detected
                  </Badge>
                </div>
                <p className="m-0 text-xs text-muted-foreground">
                  {seen.length > 0 ? `Sees your skills via ${seen.map(short).join(", ")}` : "Sees none of your skills"}
                </p>
                {dup ? <Warn>{dupText(dup)}</Warn> : null}
              </li>
            );
          })}
        </ul>
      </section>
      <section className="flex flex-col gap-2">
        <Button variant="ghost" className="self-start" aria-expanded={undetectedOpen} onClick={() => setUndetectedOpen((o) => !o)}>
          <ChevronRight aria-hidden="true" className={cn(undetectedOpen && "rotate-90")} />
          Not detected ({undetected.length})
        </Button>
        {undetectedOpen ? (
          <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
            {undetected.map((h) =>
              h.readerOnly ? (
                <li key={h.key} className="flex h-row items-center gap-3 pr-3 pl-4 text-sm">
                  <span className="flex-1">{h.label}</span>
                  <span className="text-xs text-muted-foreground">reads {AGENTS}</span>
                </li>
              ) : (
                row(h)
              ),
            )}
          </ul>
        ) : null}
      </section>
    </>
  );
}

// ---------------------------------------------------------------------------
// Variant B — flat folder list, paths first
// ---------------------------------------------------------------------------

function VariantB({ m, written }: { m: Model; written: ReadonlySet<string> }) {
  const groups = folderGroups(m.s).filter((g) => g.served.some((h) => h.installed));
  return (
    <>
      <PaneHeader title="Harnesses" description="Your skills are written to these folders. Every harness that reads a folder sees them." note="Changes save automatically" />
      <GlobalRule m={m} />
      <section className="flex flex-col gap-2">
        <h4 className="m-0 text-xs font-medium text-faint">Folders written</h4>
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {groups.map((g) => {
            const readers = installedReaders(g.path).map((h) => h.label);
            const copyOnly = g.served.length > 0 && g.served.every((h) => !h.link);
            return (
              <li key={g.path} className="grid grid-cols-[1fr_auto_auto] items-center gap-x-3 gap-y-0.5 py-2 pr-3 pl-4">
                <span className="truncate font-mono text-sm">{g.path}</span>
                <ModeControl path={g.path} m={m} fixed={copyOnly ? "copy-only reader" : undefined} />
                <Switch size="sm" checked={m.enabled.has(g.path)} onCheckedChange={(v) => m.setEnabled(g.path, v)} aria-label={`Write ${g.path}`} />
                <span className="col-span-3 text-xs text-muted-foreground">Read by {listJoin(readers)}</span>
              </li>
            );
          })}
        </ul>
      </section>
      <DuplicatesSection written={written} />
    </>
  );
}

// ---------------------------------------------------------------------------
// Variant C — folder cards
// ---------------------------------------------------------------------------

function VariantC({ m, written }: { m: Model; written: ReadonlySet<string> }) {
  const groups = folderGroups(m.s);
  const shared = groups.filter((g) => isShared(g.path));
  const own = groups.filter((g) => !isShared(g.path) && g.served.some((h) => h.installed));
  const ownUndetected = groups.filter((g) => !isShared(g.path) && !g.served.some((h) => h.installed));
  const [undetectedOpen, setUndetectedOpen] = useState(false);
  return (
    <>
      <PaneHeader
        title="Harnesses"
        description="One switch per folder your skills are written to. Every harness that reads a folder sees what is in it."
        note="Changes save automatically"
      />
      <GlobalRule m={m} />
      <section className="flex flex-col gap-2">
        <h4 className="m-0 text-xs font-medium text-faint">Shared folders</h4>
        {shared.map((g) => (
          <SharedCard key={g.path} g={g} m={m} written={written} />
        ))}
      </section>
      <section className="flex flex-col gap-2">
        <h4 className="m-0 text-xs font-medium text-faint">Own folders</h4>
        <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
          {own.map((g) => (
            <OwnCard key={g.path} g={g} m={m} />
          ))}
        </ul>
        {ownUndetected.length > 0 ? (
          <>
            <Button variant="ghost" className="self-start" aria-expanded={undetectedOpen} onClick={() => setUndetectedOpen((o) => !o)}>
              <ChevronRight aria-hidden="true" className={cn(undetectedOpen && "rotate-90")} />
              Not detected ({ownUndetected.length})
            </Button>
            {undetectedOpen ? (
              <ul className="m-0 flex list-none flex-col divide-y divide-border rounded-lg border border-border p-0">
                {ownUndetected.map((g) => (
                  <OwnCard key={g.path} g={g} m={m} />
                ))}
              </ul>
            ) : null}
          </>
        ) : null}
      </section>
      <DuplicatesSection written={written} />
    </>
  );
}

function SharedCard({ g, m, written }: { g: FolderGroup; m: Model; written: ReadonlySet<string> }) {
  const [open, setOpen] = useState(false);
  const served = g.served.filter((h) => h.installed);
  const alsoSeen = HARNESSES.filter((h) => h.installed && !h.readerOnly && reads(h, g.path) && !g.served.includes(h));
  const readerOnly = HARNESSES.filter((h) => h.installed && h.readerOnly && reads(h, g.path));
  const undetected = HARNESSES.filter((h) => !h.installed && (g.served.includes(h) || (h.readerOnly && reads(h, g.path))));
  const on = m.enabled.has(g.path);
  const otherShared = g.path === AGENTS ? CLAUDE : AGENTS;
  const copyOnlyNote = HARNESSES.filter(
    (h) => h.installed && !h.link && !g.served.includes(h) && m.s.pref && serving({ ...h, link: true }, m.s) === g.path,
  );

  const alsoNote = (h: Harness) => {
    const dupWith = [...written].filter((p) => p !== g.path && reads(h, p));
    if (on && dupWith.length > 0) return { dup: true, text: `reads ${dupWith.map(short).join(", ")} too: duplicates` };
    return { dup: false, text: `served by ${short(serving(h, m.s))}` };
  };

  return (
    <div className={cn("flex flex-col gap-2 rounded-lg border p-3", on ? "border-border-strong bg-card" : "border-border")}>
      <div className="flex items-center gap-3">
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="text-sm font-semibold">{served.length > 0 ? listJoin(served.map((h) => h.label)) : "No detected harness"}</span>
          <span className="text-xs text-muted-foreground">
            Shared — <span className="font-mono">{g.path}</span>
          </span>
        </div>
        <ModeControl path={g.path} m={m} />
        <Switch size="sm" checked={on} onCheckedChange={(v) => m.setEnabled(g.path, v)} aria-label={`Deploy to ${g.path}`} />
      </div>
      {alsoSeen.length > 0 ? (
        <div className="flex flex-col gap-0.5 text-xs">
          <span className="text-faint">Also seen by</span>
          {alsoSeen.map((h) => {
            const note = alsoNote(h);
            return (
              <span key={h.key} className={note.dup ? "text-stale" : "text-muted-foreground"}>
                <span className="text-foreground">{h.label}</span> — {note.text}
              </span>
            );
          })}
        </div>
      ) : null}
      {readerOnly.length > 0 ? (
        <p className="m-0 text-xs text-muted-foreground">
          <span className="text-faint">Read-only harnesses: </span>
          {readerOnly.map((h, i) => {
            const dup = on && [...written].some((p) => p !== g.path && reads(h, p));
            return (
              <span key={h.key}>
                {i > 0 ? ", " : ""}
                <span className="text-foreground">{h.label}</span>
                {dup ? <span className="text-stale"> (duplicates)</span> : null}
              </span>
            );
          })}
        </p>
      ) : null}
      {copyOnlyNote.map((h) => (
        <p key={h.key} className="m-0 text-xs text-faint">
          {h.label} needs copies: it is served by its own folder while this folder links.
        </p>
      ))}
      {!on && served.length > 0 ? (
        <p className="m-0 text-xs text-faint">
          Off: {listJoin(served.map((h) => h.label))} get no skills{" "}
          {served.some((h) => reads(h, otherShared) && written.has(otherShared)) ? `(except via ${short(otherShared)})` : ""}.
        </p>
      ) : null}
      {undetected.length > 0 ? (
        <div className="flex flex-col gap-1">
          <Button variant="ghost" size="sm" className="self-start" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
            <ChevronRight aria-hidden="true" className={cn(open && "rotate-90")} />
            Not detected ({undetected.length})
          </Button>
          {open ? <p className="m-0 pl-6 text-xs text-muted-foreground">{listJoin(undetected.map((h) => h.label))}</p> : null}
        </div>
      ) : null}
    </div>
  );
}

function OwnCard({ g, m }: { g: FolderGroup; m: Model }) {
  const h = g.served[0];
  const copyOnly = !h.link;
  return (
    <li className="flex flex-col gap-0.5 py-2 pr-3 pl-4">
      <div className="flex items-center gap-3">
        <span className="min-w-0 flex-1 truncate text-sm">
          {g.served.map((x) => x.label).join(", ")} <span className="text-xs text-muted-foreground">— </span>
          <span className="font-mono text-xs text-muted-foreground">{g.path}</span>
        </span>
        {h.installed ? null : (
          <Badge variant="outline" tone="neutral">
            Not detected
          </Badge>
        )}
        <ModeControl path={g.path} m={m} fixed={copyOnly ? `${h.label} rejects links` : undefined} />
        <Switch size="sm" checked={m.enabled.has(g.path)} onCheckedChange={(v) => m.setEnabled(g.path, v)} aria-label={`Deploy to ${g.path}`} />
      </div>
      {copyOnly && m.s.pref ? (
        <span className="text-xs text-faint">Needs copies; set {short(serving({ ...h, link: true }, m.s))} to Copy to serve it from there.</span>
      ) : null}
    </li>
  );
}

// ---------------------------------------------------------------------------
// The Add dialog's "Deploy after adding" picker, per variant (Q25)
// ---------------------------------------------------------------------------

function AddPicker({ variant, m }: { variant: VariantKey; m: Model }) {
  // Its own draft, seeded from the pane's state (the Add dialog's checkboxes default to the selection).
  const [harnesses, setHarnesses] = useState<ReadonlySet<string>>(() => new Set(m.selected));
  const [folders, setFolders] = useState<ReadonlySet<string>>(() => new Set(m.enabled));
  const flip = <T,>(set: ReadonlySet<T>, v: T, on: boolean) => {
    const next = new Set(set);
    if (on) next.add(v);
    else next.delete(v);
    return next;
  };
  const groups = folderGroups(m.s).filter((g) => g.served.some((h) => h.installed));
  const pickedWrites =
    variant === "a"
      ? new Set(HARNESSES.filter((h) => harnesses.has(h.key)).map((h) => serving(h, m.s)))
      : new Set(folders);

  return (
    <div className="flex flex-col gap-2">
      <span className="text-xs font-medium text-faint">Add dialog → Edit harnesses (popover, w-80)</span>
      <div className="flex flex-col gap-2 rounded-md border border-border bg-popover p-3 text-sm shadow-lg">
        <p className="m-0 text-sm font-semibold">Deploy after adding</p>
        <p className="m-0 text-sm text-muted-foreground">
          {variant === "a" ? "Each added skill is deployed to the harnesses checked here." : "Each added skill is written to the folders checked here."}
        </p>
        {variant === "a" ? (
          <div className="grid grid-cols-2 gap-x-4 gap-y-1.5">
            {HARNESSES.filter((h) => h.installed && !h.readerOnly).map((h) => {
              const path = serving(h, m.s);
              const via = isShared(path) ? `via ${short(path)}` : null;
              const stillGets = !harnesses.has(h.key) && [...pickedWrites].some((p) => reads(h, p));
              return (
                <Field key={h.key} className="flex-row items-start gap-2">
                  <Checkbox className="mt-0.5" checked={harnesses.has(h.key)} onCheckedChange={(c) => setHarnesses(flip(harnesses, h.key, c))} />
                  <div className="flex min-w-0 flex-col">
                    <FieldLabel>{h.label}</FieldLabel>
                    {stillGets ? <FieldDescription>gets it anyway</FieldDescription> : via ? <FieldDescription>{via}</FieldDescription> : null}
                  </div>
                </Field>
              );
            })}
          </div>
        ) : (
          <div className="flex flex-col gap-1.5">
            {groups.map((g) => {
              const served = g.served.filter((h) => h.installed).map((h) => h.label);
              const others = installedReaders(g.path, g.served).map((h) => h.label);
              const label = variant === "b" ? g.path : isShared(g.path) ? listJoin(served) || g.path : listJoin(served);
              const desc =
                variant === "b"
                  ? `read by ${listJoin(installedReaders(g.path).map((h) => h.label))}`
                  : isShared(g.path)
                    ? `shared ${short(g.path)}${others.length ? ` · also ${listJoin(others)}` : ""}`
                    : g.path;
              return (
                <Field key={g.path} className="flex-row items-start gap-2">
                  <Checkbox className="mt-0.5" checked={folders.has(g.path)} onCheckedChange={(c) => setFolders(flip(folders, g.path, c))} />
                  <div className="flex min-w-0 flex-col">
                    <FieldLabel className={variant === "b" ? "font-mono" : undefined}>{label}</FieldLabel>
                    <FieldDescription>{desc}</FieldDescription>
                  </div>
                </Field>
              );
            })}
          </div>
        )}
        <p className="m-0 text-xs text-muted-foreground tabular-nums">
          Writes {pickedWrites.size} folder{pickedWrites.size === 1 ? "" : "s"}; seen by {HARNESSES.filter((h) => h.installed && [...pickedWrites].some((p) => reads(h, p))).length} detected harnesses
        </p>
      </div>
    </div>
  );
}

/** Rule 5: surface the derived state — what is written where, and who sees it. */
function WrittenState({ written, s }: { written: ReadonlySet<string>; s: Settings }) {
  return (
    <div className="flex flex-col gap-2">
      <span className="text-xs font-medium text-faint">State: what lands on disk ({LIBRARY_SKILLS} skills each)</span>
      <ul className="m-0 flex list-none flex-col gap-2 rounded-md border border-border p-3 text-xs">
        {[...written].length === 0 ? <li className="text-muted-foreground">Nothing written.</li> : null}
        {[...written].map((p) => (
          <li key={p} className="flex flex-col gap-0.5">
            <span className="font-mono text-foreground">
              {p} <span className="text-muted-foreground">({modeOf(p, s)})</span>
            </span>
            <span className="text-muted-foreground">seen by {listJoin(installedReaders(p).map((h) => h.label))}</span>
          </li>
        ))}
        <li className="text-faint">
          preference {s.pref ? "on" : "off"} · both → .{s.both}
        </li>
      </ul>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Floating switcher (prototype chrome, not part of the design)
// ---------------------------------------------------------------------------

function Switcher({ current, onChange }: { current: VariantKey; onChange: (v: VariantKey) => void }) {
  const index = VARIANTS.findIndex((v) => v.key === current);
  const step = (d: number) => onChange(VARIANTS[(index + d + VARIANTS.length) % VARIANTS.length].key);
  const [theme, setTheme] = useState(() => document.documentElement.dataset.theme ?? "dark");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
  }, [theme]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.closest("input, textarea, select, [contenteditable], [role=group], [role=switch]") !== null)) return;
      if (e.key === "ArrowLeft") step(-1);
      if (e.key === "ArrowRight") step(1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
  if (!import.meta.env.DEV) return null;
  return (
    <div className="fixed bottom-4 left-1/2 z-50 flex -translate-x-1/2 items-center gap-1 rounded-full bg-primary p-1 text-primary-foreground shadow-lg">
      <Button variant="solid" tone="accent" size="icon-sm" aria-label="Previous variant" onClick={() => step(-1)}>
        <ChevronLeft aria-hidden="true" />
      </Button>
      <span className="px-2 text-sm font-medium tabular-nums">
        {current.toUpperCase()} · {VARIANTS[index].name} <span className="opacity-70">({index + 1}/{VARIANTS.length})</span>
      </span>
      <Button variant="solid" tone="accent" size="icon-sm" aria-label="Next variant" onClick={() => step(1)}>
        <ChevronRight aria-hidden="true" />
      </Button>
      <Button
        variant="solid"
        tone="accent"
        size="icon-sm"
        aria-label="Toggle theme"
        onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
      >
        {theme === "dark" ? <Sun aria-hidden="true" /> : <Moon aria-hidden="true" />}
      </Button>
    </div>
  );
}
