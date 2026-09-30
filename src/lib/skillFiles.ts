/**
 * The skill file browser's pure half (arch candidate 12): a flat file list
 * becomes a sorted tree, a filename a highlighter language, a byte count a
 * size label. `useSkillDocument` loads the files; the inspector's Files
 * section and Explore's preview render them.
 */

import type { SkillFileEntry } from "../components/skills/types";

export type { SkillFileEntry };

export type FileTreeNode = {
  name: string;
  /** The file's relative path, or the folder's prefix. */
  path: string;
  isDir: boolean;
  /** Bytes (0 for a folder). */
  size: number;
  children: FileTreeNode[];
};

/** "512 B", "1.5 KB", "2.0 MB". */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const EXT_LANG: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  js: "javascript",
  jsx: "jsx",
  py: "python",
  rs: "rust",
  go: "go",
  rb: "ruby",
  java: "java",
  kt: "kotlin",
  swift: "swift",
  c: "c",
  cpp: "cpp",
  h: "c",
  hpp: "cpp",
  cs: "csharp",
  css: "css",
  scss: "scss",
  less: "less",
  html: "html",
  xml: "xml",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  sql: "sql",
  graphql: "graphql",
  dockerfile: "docker",
  lua: "lua",
  r: "r",
  dart: "dart",
  php: "php",
  pl: "perl",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  hs: "haskell",
  vim: "vim",
  ini: "ini",
  cfg: "ini",
  diff: "diff",
  patch: "diff",
};

/** The highlighter language of a file name; "" = plain text. */
export function getLang(filename: string): string {
  const lower = (filename.split("/").pop() ?? filename).toLowerCase();
  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) return "docker";
  if (lower === "makefile" || lower === "gnumakefile") return "makefile";
  if (!lower.includes(".")) return "";
  const ext = lower.split(".").pop() ?? "";
  return EXT_LANG[ext] ?? "";
}

/** Rendered as Markdown (prose), not as code. */
export function isMarkdown(filename: string): boolean {
  return /\.(md|mdx|markdown)$/i.test(filename);
}

/**
 * The tree of a flat file list: folders first (alphabetical), then files
 * with `SKILL.md` first, then alphabetical — at every level.
 */
export function buildTree(files: readonly SkillFileEntry[]): FileTreeNode[] {
  const root: FileTreeNode[] = [];
  for (const file of files) {
    const parts = file.path.split("/").filter((part) => part !== "");
    let level = root;
    parts.forEach((name, i) => {
      if (i === parts.length - 1) {
        level.push({ name, path: file.path, isDir: false, size: file.size, children: [] });
        return;
      }
      let dir = level.find((node) => node.isDir && node.name === name);
      if (!dir) {
        dir = { name, path: parts.slice(0, i + 1).join("/"), isDir: true, size: 0, children: [] };
        level.push(dir);
      }
      level = dir.children;
    });
  }
  const sort = (nodes: FileTreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      if (!a.isDir) {
        const aSkill = a.name.toLowerCase() === "skill.md";
        const bSkill = b.name.toLowerCase() === "skill.md";
        if (aSkill !== bSkill) return aSkill ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    });
    for (const node of nodes) if (node.isDir) sort(node.children);
  };
  sort(root);
  return root;
}

/**
 * The file a document opens on: `SKILL.md` at the root when there is one,
 * else the first file in tree order; null for an empty folder.
 */
export function defaultFile(files: readonly SkillFileEntry[]): string | null {
  const manifest = files.find((file) => file.path.toLowerCase() === "skill.md");
  if (manifest) return manifest.path;
  const first = (nodes: FileTreeNode[]): string | null => {
    for (const node of nodes) {
      if (!node.isDir) return node.path;
      const inner = first(node.children);
      if (inner) return inner;
    }
    return null;
  };
  return first(buildTree(files));
}

/** The folders enclosing a file (`a/b/c.md` → `a`, `a/b`), to expand it into view. */
export function ancestorDirs(path: string): string[] {
  const parts = path.split("/");
  return parts.slice(0, -1).map((_, i) => parts.slice(0, i + 1).join("/"));
}
