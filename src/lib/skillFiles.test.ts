import { describe, expect, it } from "vitest";
import { ancestorDirs, buildTree, defaultFile, formatSize, getLang, isMarkdown } from "./skillFiles";

describe("formatSize", () => {
  it("names bytes, kilobytes and megabytes", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(1023)).toBe("1023 B");
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(1536)).toBe("1.5 KB");
    expect(formatSize(2 * 1024 * 1024)).toBe("2.0 MB");
  });
});

describe("getLang", () => {
  it("maps extensions to highlighter languages", () => {
    expect(getLang("scripts/run.sh")).toBe("bash");
    expect(getLang("index.TSX")).toBe("tsx");
    expect(getLang("config.yml")).toBe("yaml");
  });

  it("knows the extensionless build files", () => {
    expect(getLang("Dockerfile")).toBe("docker");
    expect(getLang("Dockerfile.dev")).toBe("docker");
    expect(getLang("tools/Makefile")).toBe("makefile");
  });

  it("answers plain text for anything else, including a bare name that looks like an extension", () => {
    expect(getLang("notes.unknown")).toBe("");
    expect(getLang("LICENSE")).toBe("");
    expect(getLang("sh")).toBe("");
  });
});

describe("isMarkdown", () => {
  it("covers md, mdx and markdown, case-insensitively", () => {
    expect(isMarkdown("SKILL.md")).toBe(true);
    expect(isMarkdown("doc.MDX")).toBe(true);
    expect(isMarkdown("a.markdown")).toBe(true);
    expect(isMarkdown("a.mdown.txt")).toBe(false);
  });
});

describe("buildTree", () => {
  const files = [
    { path: "zeta.txt", size: 1 },
    { path: "reference/b.md", size: 2 },
    { path: "reference/nested/c.md", size: 3 },
    { path: "SKILL.md", size: 4 },
    { path: "alpha.md", size: 5 },
    { path: "reference/a.md", size: 6 },
    { path: "assets/logo.png", size: 7 },
  ];

  it("puts folders first, then SKILL.md, then files alphabetically — at every level", () => {
    const tree = buildTree(files);
    expect(tree.map((n) => n.name)).toEqual(["assets", "reference", "SKILL.md", "alpha.md", "zeta.txt"]);
    const reference = tree[1];
    expect(reference).toMatchObject({ isDir: true, path: "reference", size: 0 });
    expect(reference.children.map((n) => n.name)).toEqual(["nested", "a.md", "b.md"]);
    expect(reference.children[0].children).toEqual([
      { name: "c.md", path: "reference/nested/c.md", isDir: false, size: 3, children: [] },
    ]);
  });

  it("is empty for no files and ignores empty path segments", () => {
    expect(buildTree([])).toEqual([]);
    expect(buildTree([{ path: "a//b.md", size: 1 }])[0].children[0].path).toBe("a//b.md");
  });
});

describe("defaultFile", () => {
  it("opens on the root SKILL.md", () => {
    expect(defaultFile([{ path: "a.md", size: 1 }, { path: "SKILL.md", size: 1 }])).toBe("SKILL.md");
  });

  it("falls back to the first file in tree order, and null for an empty folder", () => {
    expect(defaultFile([{ path: "z.md", size: 1 }, { path: "docs/a.md", size: 1 }])).toBe("docs/a.md");
    expect(defaultFile([])).toBeNull();
  });
});

describe("ancestorDirs", () => {
  it("lists every enclosing folder, outermost first", () => {
    expect(ancestorDirs("a/b/c.md")).toEqual(["a", "a/b"]);
    expect(ancestorDirs("SKILL.md")).toEqual([]);
  });
});
