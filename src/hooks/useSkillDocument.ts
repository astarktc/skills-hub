import { useCallback, useEffect, useMemo, useState } from "react";
import type { CommandError } from "../bindings";
import { toCommandError } from "../commandError";
import { ancestorDirs, buildTree, defaultFile, type DocumentFile, type FileTreeNode } from "../lib/skillFiles";
import { invokeTauri } from "../lib/tauri";

/**
 * What a file browser shows: a skill folder on disk (`root`) and when its
 * bytes last changed (`version` — a new value re-reads the files). A Managed
 * skill passes its central copy and `updated_at`; Explore's preview passes
 * its clone and `fetchedAt` — neither forges the other's record.
 */
export type SkillDocumentRef = {
  root: string;
  version: number;
  /**
   * Compare the folder with this skill's Library copy (round 20 #70): `root`
   * is a folder an overwrite ask reported in a target's way, read only
   * through the backend's occupant fence (`listOccupantFiles` /
   * `readOccupantFile`) — each file says how it compares, and the open
   * file's skill text comes beside it (`skillContent`).
   */
  comparedWith?: string;
};

export type SkillDocument = {
  files: DocumentFile[];
  tree: FileTreeNode[];
  /** The file open in the reader (null = none: loading, empty, or unreadable). */
  activeFile: string | null;
  content: string;
  /**
   * A compared document's open file as the skill has it (null: not a
   * compared document, or the skill has no such file).
   */
  skillContent: string | null;
  /** Whether this document is compared with a skill (`comparedWith`). */
  compared: boolean;
  /**
   * A compared folder holds more files than its bounded listing shows (the
   * skill's files are always compared in full).
   */
  truncated: boolean;
  /** Listing the folder failed: shown in place of the tree, never a toast. */
  listError: CommandError | null;
  /** Reading the open file failed. */
  contentError: CommandError | null;
  loadingFiles: boolean;
  loadingContent: boolean;
  /** Expanded folders of the tree (every folder starts collapsed but the open file's). */
  expanded: ReadonlySet<string>;
  selectFile: (path: string) => void;
  toggleDir: (path: string) => void;
};

type Listing = {
  key: string;
  files: DocumentFile[];
  truncated: boolean;
  error: CommandError | null;
};

type Reading = {
  key: string;
  content: string;
  skillContent: string | null;
  error: CommandError | null;
};

const NO_FILES: DocumentFile[] = [];

/**
 * The skill file browser's loading half (arch candidate 12): the folder's
 * files, the open file's text, the tree's expansion. Reads go through the
 * typed seam; a stale answer (the document or the file changed meanwhile)
 * is dropped. Pure shaping lives in `lib/skillFiles.ts`.
 */
export function useSkillDocument(doc: SkillDocumentRef | null): SkillDocument {
  const docKey = doc ? `${doc.root}\n${doc.version}\n${doc.comparedWith ?? ""}` : null;
  const root = doc?.root ?? null;
  const comparedWith = doc?.comparedWith ?? null;

  const [listing, setListing] = useState<Listing | null>(null);
  // The operator's choice, remembered per document (a new document opens on its default).
  const [choice, setChoice] = useState<{ key: string; file: string } | null>(null);
  const [toggled, setToggled] = useState<{ key: string; dirs: ReadonlySet<string> } | null>(null);
  const [reading, setReading] = useState<Reading | null>(null);

  useEffect(() => {
    if (!docKey || !root) return;
    let stale = false;
    void (async () => {
      try {
        const { files, truncated }: { files: DocumentFile[]; truncated: boolean } =
          comparedWith === null
            ? { files: await invokeTauri("listSkillFiles", root), truncated: false }
            : await invokeTauri("listOccupantFiles", root, comparedWith).then((listing) => ({
                files: listing.entries,
                truncated: listing.truncated,
              }));
        if (!stale) setListing({ key: docKey, files, truncated, error: null });
      } catch (err) {
        if (!stale) setListing({ key: docKey, files: [], truncated: false, error: toCommandError(err) });
      }
    })();
    return () => {
      stale = true;
    };
  }, [comparedWith, docKey, root]);

  const current = listing?.key === docKey ? listing : null;
  const files = current?.files ?? NO_FILES;
  const tree = useMemo(() => buildTree(files), [files]);
  const chosen = choice?.key === docKey ? choice.file : null;
  const activeFile =
    chosen && files.some((file) => file.path === chosen) ? chosen : current ? defaultFile(files) : null;
  const readKey = docKey && activeFile ? `${docKey}\n${activeFile}` : null;

  useEffect(() => {
    if (!readKey || !root || !activeFile) return;
    let stale = false;
    void (async () => {
      try {
        if (comparedWith === null) {
          const content = await invokeTauri("readSkillFile", root, activeFile);
          if (!stale) setReading({ key: readKey, content, skillContent: null, error: null });
        } else {
          const both = await invokeTauri("readOccupantFile", root, comparedWith, activeFile);
          if (!stale)
            setReading({ key: readKey, content: both.occupant ?? "", skillContent: both.skill, error: null });
        }
      } catch (err) {
        if (!stale) setReading({ key: readKey, content: "", skillContent: null, error: toCommandError(err) });
      }
    })();
    return () => {
      stale = true;
    };
  }, [activeFile, comparedWith, readKey, root]);

  const read = reading?.key === readKey ? reading : null;
  const expanded = useMemo(() => {
    const dirs = new Set(toggled?.key === docKey ? toggled.dirs : []);
    // The open file's folders are always open, so the selection is in view.
    if (activeFile && toggled?.key !== docKey) for (const dir of ancestorDirs(activeFile)) dirs.add(dir);
    return dirs;
  }, [activeFile, docKey, toggled]);

  const selectFile = useCallback(
    (path: string) => {
      if (docKey) setChoice({ key: docKey, file: path });
    },
    [docKey],
  );
  const toggleDir = useCallback(
    (path: string) => {
      if (!docKey) return;
      setToggled((prev) => {
        const next = new Set(prev?.key === docKey ? prev.dirs : expanded);
        if (next.has(path)) next.delete(path);
        else next.add(path);
        return { key: docKey, dirs: next };
      });
    },
    [docKey, expanded],
  );

  return {
    files,
    tree,
    activeFile,
    content: read?.content ?? "",
    skillContent: read?.skillContent ?? null,
    compared: comparedWith !== null,
    truncated: current?.truncated ?? false,
    listError: current?.error ?? null,
    contentError: read?.error ?? null,
    loadingFiles: doc !== null && current === null,
    loadingContent: activeFile !== null && read === null,
    expanded,
    selectFile,
    toggleDir,
  };
}
