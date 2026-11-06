/**
 * Settings → Harnesses' pure presentation (round 20 ticket 02): which
 * harnesses the list shows first, and the cross-read warning.
 *
 * The warning's facts are the backend's: `ToolInfoDto.duplicate_readers`
 * names, per installed harness, the other installed harnesses whose deploy
 * dir it reads by default (resolved from the registry). This module only
 * intersects those pairs with the selection and words them — it never
 * re-derives a read from a dir, the same rule as `shared_with`.
 */

import type { DuplicateBehaviour, ReaderHarnessDto, ToolInfoDto } from "../components/skills/types";

type Translate = (key: string, opts?: Record<string, unknown>) => string;

/**
 * One cross-read: `reader` also reads `other`'s deploy dir, so a skill deployed to both shows up twice in it.
 * `folder` (present only when `other` shares its skills dir with another selected harness) is every selected
 * harness deploying to that one folder, `other` included — the backend's `shared_with`, never re-derived.
 */
export type DuplicatePair = { reader: string; other: string; behaviour: DuplicateBehaviour; folder?: string[] };

/** Every cross-read pair whose two harnesses are both in `selected`, in catalog then backend order. */
export function duplicatePairs(tools: readonly ToolInfoDto[], selected: ReadonlySet<string>): DuplicatePair[] {
  const sharedWith = new Map(tools.map((tool) => [tool.key, tool.shared_with]));
  return tools.flatMap((tool) => {
    const reads = tool.duplicate_readers;
    if (!reads || !selected.has(tool.key)) return [];
    return reads.keys
      .filter((other) => selected.has(other))
      .map((other) => {
        const folder = (sharedWith.get(other) ?? [other]).filter((key) => selected.has(key));
        const pair: DuplicatePair = { reader: tool.key, other, behaviour: reads.behaviour };
        if (folder.length > 1) pair.folder = folder;
        return pair;
      });
  });
}

/** The reader's pairs, one per folder: selected harnesses sharing one skills dir are one folder, named once. */
function byFolder(pairs: readonly DuplicatePair[]): DuplicatePair[] {
  const seen = new Set<string>();
  return pairs.filter((pair) => {
    const id = (pair.folder ?? [pair.other]).slice().sort().join("\u0000");
    if (seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

/** The pairs `key` takes part in, as the reader or as the one read. */
export function pairsInvolving(pairs: readonly DuplicatePair[], key: string): DuplicatePair[] {
  return pairs.filter((pair) => pair.reader === key || pair.other === key);
}

/** A duplicate's consequence, gravest first: invocation breaks › a conflict is flagged › it shows twice. */
type Consequence = "loadsNone" | "flagsConflict" | "showsTwice";
const CONSEQUENCES: readonly Consequence[] = ["loadsNone", "flagsConflict", "showsTwice"];

function consequenceOf(behaviour: DuplicateBehaviour): Consequence {
  switch (behaviour) {
    case "loads_none":
      return "loadsNone";
    case "flags_conflict":
      return "flagsConflict";
    case "shows_all":
    case "shadows":
    case "unknown":
      return "showsTwice";
  }
}

/** "X also reads the A (and B) folder(s) — …": the subject reads the others' folders. */
function readsSentence(consequence: Consequence, opts: Record<string, unknown>, t: Translate): string {
  switch (consequence) {
    case "loadsNone":
      return t("settingsDialog.harnesses.duplicates.reads.loadsNone", opts);
    case "flagsConflict":
      return t("settingsDialog.harnesses.duplicates.reads.flagsConflict", opts);
    case "showsTwice":
      return t("settingsDialog.harnesses.duplicates.reads.showsTwice", opts);
  }
}

/** "A (and B) also read(s) the X folder — …": others read the subject's folder. */
function readBySentence(consequence: Consequence, opts: Record<string, unknown>, t: Translate): string {
  switch (consequence) {
    case "loadsNone":
      return t("settingsDialog.harnesses.duplicates.readBy.loadsNone", opts);
    case "flagsConflict":
      return t("settingsDialog.harnesses.duplicates.readBy.flagsConflict", opts);
    case "showsTwice":
      return t("settingsDialog.harnesses.duplicates.readBy.showsTwice", opts);
  }
}

/** "A", "A and B", "A, B and C". */
function listOf(names: readonly string[], t: Translate): string {
  if (names.length <= 1) return names.join("");
  return names.slice(0, -1).join(t("common.listSeparator")) + t("common.listAnd") + names[names.length - 1];
}

/**
 * The warning line about `subject` (its Harnesses row, or the Deploy-now ask
 * when turning it on), or null when no pair in `pairs` involves it. At most
 * one sentence for what the subject reads — every folder it also reads,
 * named once — then one per consequence for the harnesses reading the
 * subject's folder, gravest first. The consequence follows the reader's
 * duplicate behaviour: invocation breaks (`loads_none`), a conflict is
 * flagged (`flags_conflict`), otherwise it shows twice.
 */
export function duplicateWarning(
  pairs: readonly DuplicatePair[],
  subject: string,
  labelOf: (key: string) => string,
  t: Translate,
): string | null {
  const sentences: string[] = [];
  const reads = byFolder(pairs.filter((pair) => pair.reader === subject));
  if (reads.length > 0) {
    sentences.push(
      readsSentence(
        consequenceOf(reads[0].behaviour),
        {
          reader: labelOf(subject),
          harnesses: listOf(
            reads.map((pair) => (pair.folder ?? [pair.other]).map(labelOf).join(" / ")),
            t,
          ),
          count: reads.length,
        },
        t,
      ),
    );
  }
  const readBy = pairs.filter((pair) => pair.other === subject);
  for (const consequence of CONSEQUENCES) {
    const readers = readBy.filter((pair) => consequenceOf(pair.behaviour) === consequence);
    if (readers.length === 0) continue;
    sentences.push(
      readBySentence(
        consequence,
        {
          readers: listOf(readers.map((pair) => labelOf(pair.reader)), t),
          harness: labelOf(subject),
          count: readers.length,
        },
        t,
      ),
    );
  }
  return sentences.length > 0 ? sentences.join(" ") : null;
}

/** A detected harness's row: the harness, and its detected reader-only harnesses (listed under it). */
export type DetectedHarness = { tool: ToolInfoDto; readers: ReaderHarnessDto[] };

/** An undetected reader-only harness, with the key of the entry whose folder it reads. */
export type UndetectedReader = ReaderHarnessDto & { reads: string };

/**
 * The Harnesses list's two parts: detected harnesses in catalog order (each
 * with its detected readers), and the closed "Not detected (N)" group —
 * every undetected harness, a selected one included (its row explains it),
 * and every undetected reader-only harness.
 */
export function harnessListGroups(tools: readonly ToolInfoDto[]): {
  detected: DetectedHarness[];
  undetected: { tools: ToolInfoDto[]; readers: UndetectedReader[]; count: number };
} {
  const detected = tools
    .filter((tool) => tool.installed)
    .map((tool) => ({ tool, readers: tool.readers.filter((reader) => reader.installed) }));
  const undetectedTools = tools.filter((tool) => !tool.installed);
  const undetectedReaders = tools.flatMap((tool) =>
    tool.readers.filter((reader) => !reader.installed).map((reader) => ({ ...reader, reads: tool.key })),
  );
  return {
    detected,
    undetected: {
      tools: undetectedTools,
      readers: undetectedReaders,
      count: undetectedTools.length + undetectedReaders.length,
    },
  };
}
