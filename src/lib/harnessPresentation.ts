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

/** One cross-read: `reader` also reads `other`'s deploy dir, so a skill deployed to both shows up twice in it. */
export type DuplicatePair = { reader: string; other: string; behaviour: DuplicateBehaviour };

/** Every cross-read pair whose two harnesses are both in `selected`, in catalog then backend order. */
export function duplicatePairs(tools: readonly ToolInfoDto[], selected: ReadonlySet<string>): DuplicatePair[] {
  return tools.flatMap((tool) => {
    const reads = tool.duplicate_readers;
    if (!reads || !selected.has(tool.key)) return [];
    return reads.keys
      .filter((other) => selected.has(other))
      .map((other) => ({ reader: tool.key, other, behaviour: reads.behaviour }));
  });
}

/** The pairs `key` takes part in, as the reader or as the one read. */
export function pairsInvolving(pairs: readonly DuplicatePair[], key: string): DuplicatePair[] {
  return pairs.filter((pair) => pair.reader === key || pair.other === key);
}

/** One reader's sentence; the consequence follows its duplicate behaviour. */
function readerSentence(behaviour: DuplicateBehaviour, opts: Record<string, unknown>, t: Translate): string {
  switch (behaviour) {
    case "loads_none":
      return t("settingsDialog.harnesses.duplicates.loadsNone", opts);
    case "flags_conflict":
      return t("settingsDialog.harnesses.duplicates.flagsConflict", opts);
    case "shows_all":
    case "shadows":
    case "unknown":
      return t("settingsDialog.harnesses.duplicates.showsTwice", opts);
  }
}

/** "A", "A and B", "A, B and C". */
function listOf(names: readonly string[], t: Translate): string {
  if (names.length <= 1) return names.join("");
  return names.slice(0, -1).join(t("common.listSeparator")) + t("common.listAnd") + names[names.length - 1];
}

/**
 * The warning line for `pairs` (a row's, or the ones turning a harness on
 * completes), or null when there are none: one sentence per reader, in pair
 * order, naming every folder it also reads. The consequence follows the
 * reader's duplicate behaviour — invocation breaks (`loads_none`), a
 * conflict is flagged (`flags_conflict`), otherwise it shows twice.
 */
export function duplicateWarning(
  pairs: readonly DuplicatePair[],
  labelOf: (key: string) => string,
  t: Translate,
): string | null {
  if (pairs.length === 0) return null;
  const readers: string[] = [];
  for (const pair of pairs) if (!readers.includes(pair.reader)) readers.push(pair.reader);
  return readers
    .map((reader) => {
      const own = pairs.filter((pair) => pair.reader === reader);
      const opts = {
        reader: labelOf(reader),
        harnesses: listOf(own.map((pair) => labelOf(pair.other)), t),
        count: own.length,
      };
      return readerSentence(own[0].behaviour, opts, t);
    })
    .join(" ");
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
