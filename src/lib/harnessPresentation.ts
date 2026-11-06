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
 * `via` is set for a reader-only harness (fx): the entry it is listed under, whose folder it reads as its own
 * deploy dir — the switch that, with `other`'s, makes the pair. `among` is set instead when that entry is not
 * selected: the reader-only harness reads two or more selected folders (`other` one of them), so a skill deployed
 * to more than one of them is a duplicate in it all the same (round 20 rows review — fx loads none).
 */
export type DuplicatePair = {
  reader: string;
  other: string;
  behaviour: DuplicateBehaviour;
  folder?: string[];
  via?: string;
  among?: true;
};

/**
 * Every cross-read pair whose two deploy dirs are both selected, in catalog then backend order: an entry's own
 * pairs (the entry and `other` selected), then those of its reader-only harnesses (the entry they are listed
 * under and `other` selected — a reader has no switch of its own).
 */
export function duplicatePairs(tools: readonly ToolInfoDto[], selected: ReadonlySet<string>): DuplicatePair[] {
  const sharedWith = new Map(tools.map((tool) => [tool.key, tool.shared_with]));
  const pairsOf = (
    reader: string,
    reads: ToolInfoDto["duplicate_readers"],
    via?: string,
  ): DuplicatePair[] => {
    if (!reads) return [];
    return reads.keys
      .filter((other) => selected.has(other))
      .map((other) => {
        const folder = (sharedWith.get(other) ?? [other]).filter((key) => selected.has(key));
        const pair: DuplicatePair = { reader, other, behaviour: reads.behaviour };
        if (folder.length > 1) pair.folder = folder;
        if (via) pair.via = via;
        return pair;
      });
  };
  /** A reader-only harness whose entry is unselected: its reads among the selected folders, when two or more. */
  const amongPairs = (reader: ReaderHarnessDto): DuplicatePair[] => {
    const pairs = pairsOf(reader.key, reader.duplicate_readers);
    if (byFolder(pairs).length < 2) return [];
    return pairs.map((pair) => ({ ...pair, among: true as const }));
  };
  return tools.flatMap((tool) => {
    if (!selected.has(tool.key)) return tool.readers.filter((reader) => reader.installed).flatMap(amongPairs);
    return [
      ...pairsOf(tool.key, tool.duplicate_readers),
      ...tool.readers.flatMap((reader) => pairsOf(reader.key, reader.duplicate_readers, tool.key)),
    ];
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

/** The pairs `key` takes part in: as the reader, as the one read, or as the entry a reader-only harness reads. */
export function pairsInvolving(pairs: readonly DuplicatePair[], key: string): DuplicatePair[] {
  return pairs.filter((pair) => pair.reader === key || pair.other === key || pair.via === key);
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

/** "R reads the A and B folders — a skill deployed to more than one of them …": a reader-only harness, entry unselected. */
function amongSentence(consequence: Consequence, opts: Record<string, unknown>, t: Translate): string {
  switch (consequence) {
    case "loadsNone":
      return t("settingsDialog.harnesses.duplicates.among.loadsNone", opts);
    case "flagsConflict":
      return t("settingsDialog.harnesses.duplicates.among.flagsConflict", opts);
    case "showsTwice":
      return t("settingsDialog.harnesses.duplicates.among.showsTwice", opts);
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
 * named once — then one per reader-only harness reading the subject's folder
 * as its own (fx under `.agents/skills`: what it also reads), then one per
 * consequence for the harnesses reading the subject's folder, gravest first. The consequence follows the reader's
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
  const readsOf = (reader: string, readerPairs: readonly DuplicatePair[]) => {
    const reads = byFolder(readerPairs);
    if (reads.length === 0) return;
    sentences.push(
      readsSentence(
        consequenceOf(reads[0].behaviour),
        {
          reader: labelOf(reader),
          harnesses: listOf(
            reads.map((pair) => (pair.folder ?? [pair.other]).map(labelOf).join(" / ")),
            t,
          ),
          count: reads.length,
        },
        t,
      ),
    );
  };
  readsOf(
    subject,
    pairs.filter((pair) => pair.reader === subject),
  );
  // The reader-only harnesses reading the subject's folder as their own: what each of them also reads.
  const viaReaders = [...new Set(pairs.filter((pair) => pair.via === subject).map((pair) => pair.reader))];
  for (const reader of viaReaders) {
    readsOf(
      reader,
      pairs.filter((pair) => pair.via === subject && pair.reader === reader),
    );
  }
  // A reader-only harness reading several selected folders, the subject's among them (its entry unselected).
  const amongReaders = [...new Set(pairs.filter((pair) => pair.among && pair.other === subject).map((pair) => pair.reader))];
  for (const reader of amongReaders) {
    const reads = byFolder(pairs.filter((pair) => pair.among && pair.reader === reader));
    sentences.push(
      amongSentence(
        consequenceOf(reads[0].behaviour),
        {
          reader: labelOf(reader),
          harnesses: listOf(
            reads.map((pair) => (pair.folder ?? [pair.other]).map(labelOf).join(" / ")),
            t,
          ),
        },
        t,
      ),
    );
  }
  const readBy = pairs.filter((pair) => pair.other === subject && !pair.among);
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
