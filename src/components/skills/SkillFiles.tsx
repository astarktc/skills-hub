import { memo } from "react";
import { ChevronDown, ChevronRight, File, Folder, FolderOpen, LoaderCircle } from "lucide-react";
// Class output only (useInlineStyles={false}): colours are prose-layer
// rules reading tokens (styles/prose.css), so the theme is never read here.
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import Markdown from "react-markdown";
import remarkFrontmatter from "remark-frontmatter";
import remarkGfm from "remark-gfm";
import type { TFunction } from "i18next";
import { cn } from "@/lib/utils";
import { focusRing } from "@/components/ui/styles";
import { describeCommandError } from "../../commandError";
import type { SkillDocument } from "../../hooks/useSkillDocument";
import { parseFrontmatter } from "../../lib/manifestPresentation";
import { formatSize, getLang, isMarkdown, type FileTreeNode } from "../../lib/skillFiles";

// An empty theme object for every highlighter: even with
// useInlineStyles={false} the library builds the <code> tag's inline style
// from its `style` prop, whose default is Prism's light theme (colour: black,
// a white text-shadow), measured 1.17:1 on the dark prose ground.
const NO_INLINE_THEME = {};

type TreeProps = {
  nodes: FileTreeNode[];
  depth: number;
  doc: SkillDocument;
};

/** 12 px per level, plus the chevron's column for files. */
const indent = (depth: number, file: boolean) => ({ paddingLeft: `${0.5 + depth * 0.75 + (file ? 1.125 : 0)}rem` });

const TREE_ITEM = cn(
  "flex h-7 w-full min-w-0 cursor-default items-center gap-1.5 rounded-sm pr-2 text-left text-sm text-muted-foreground hover:bg-raised hover:text-foreground",
  focusRing,
  "focus-visible:-outline-offset-2",
);

function Tree({ nodes, depth, doc }: TreeProps) {
  return (
    <ul className="m-0 flex list-none flex-col p-0">
      {nodes.map((node) => {
        if (node.isDir) {
          const open = doc.expanded.has(node.path);
          return (
            <li key={node.path}>
              <button
                type="button"
                aria-expanded={open}
                className={TREE_ITEM}
                style={indent(depth, false)}
                onClick={() => doc.toggleDir(node.path)}
              >
                {open ? <ChevronDown className="size-3.5 shrink-0" aria-hidden="true" /> : <ChevronRight className="size-3.5 shrink-0" aria-hidden="true" />}
                {open ? <FolderOpen className="size-3.5 shrink-0 text-faint" aria-hidden="true" /> : <Folder className="size-3.5 shrink-0 text-faint" aria-hidden="true" />}
                <span className="truncate">{node.name}</span>
              </button>
              {open ? <Tree nodes={node.children} depth={depth + 1} doc={doc} /> : null}
            </li>
          );
        }
        const active = doc.activeFile === node.path;
        return (
          <li key={node.path}>
            <button
              type="button"
              aria-current={active ? "true" : undefined}
              className={cn(TREE_ITEM, active && "bg-raised font-medium text-foreground")}
              style={indent(depth, true)}
              onClick={() => doc.selectFile(node.path)}
            >
              <File className="size-3.5 shrink-0 text-faint" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate">{node.name}</span>
              <span className="shrink-0 font-mono text-xs text-faint">{formatSize(node.size)}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** A file's text: Markdown as prose (frontmatter as a table), code highlighted, anything else numbered plain text. */
const FileContent = memo(({ filename, content }: { filename: string; content: string }) => {
  if (isMarkdown(filename)) {
    const { meta, body } = parseFrontmatter(content);
    return (
      <div className="prose max-w-[72ch] px-5 py-4">
        {meta ? (
          <table>
            <thead>
              <tr>
                {Object.keys(meta).map((key) => (
                  <th key={key}>{key}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              <tr>
                {Object.values(meta).map((value, i) => (
                  <td key={i}>{value}</td>
                ))}
              </tr>
            </tbody>
          </table>
        ) : null}
        <Markdown
          remarkPlugins={[remarkFrontmatter, remarkGfm]}
          components={{
            code: ({ className, children, ...rest }) => {
              const match = /language-(\w+)/.exec(className ?? "");
              if (!match) {
                return (
                  <code className="md-inline-code" {...rest}>
                    {children}
                  </code>
                );
              }
              return (
                <SyntaxHighlighter useInlineStyles={false} style={NO_INLINE_THEME} language={match[1]} PreTag="div">
                  {String(children).replace(/\n$/, "")}
                </SyntaxHighlighter>
              );
            },
          }}
        >
          {body}
        </Markdown>
      </div>
    );
  }
  return (
    <div className="prose">
      <SyntaxHighlighter
        useInlineStyles={false}
        style={NO_INLINE_THEME}
        className="code-file"
        language={getLang(filename) || "text"}
        showLineNumbers
        lineNumberStyle={{ minWidth: "3em", paddingRight: "1em" }}
      >
        {content}
      </SyntaxHighlighter>
    </div>
  );
});
FileContent.displayName = "FileContent";

type SkillFilesProps = {
  doc: SkillDocument;
  /** Names the file list (`nav`) for assistive tech. */
  label: string;
  /** `stacked`: tree above the reader (the inspector); `split`: tree beside it (expanded, Explore preview). */
  layout: "stacked" | "split";
  t: TFunction;
};

/**
 * The skill file browser's view (arch candidate 12): the tree and the
 * reader for one `useSkillDocument`. A listing failure is shown in place,
 * never as a toast.
 */
export default function SkillFiles({ doc, label, layout, t }: SkillFilesProps) {
  if (doc.loadingFiles) {
    return (
      <p className="m-0 flex items-center gap-2 text-sm text-muted-foreground">
        <LoaderCircle className="size-4 motion-safe:animate-spin" aria-hidden="true" />
        {t("detail.loadingFiles")}
      </p>
    );
  }
  if (doc.listError) {
    return (
      <p className="m-0 text-sm text-failed">
        {t("inspector.files.listError", { error: describeCommandError(doc.listError, t) ?? "" })}
      </p>
    );
  }
  if (doc.files.length === 0) return <p className="m-0 text-sm text-muted-foreground">{t("detail.noFiles")}</p>;

  const activeSize = doc.files.find((file) => file.path === doc.activeFile)?.size ?? 0;
  return (
    <div
      className={cn(
        "flex min-h-0 min-w-0 flex-1",
        layout === "split" ? "flex-row" : "flex-col gap-2",
      )}
    >
      <nav
        aria-label={label}
        className={cn(
          "shrink-0 overflow-y-auto",
          layout === "split" ? "w-64 border-r border-border pr-2" : "max-h-48 rounded-md border border-border p-1",
        )}
      >
        <Tree nodes={doc.tree} depth={0} doc={doc} />
      </nav>
      <div
        className={cn(
          "flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden",
          layout === "split" ? "pl-2" : "max-h-[28rem] rounded-md border border-border",
        )}
      >
        {doc.activeFile ? (
          <>
            <div className="flex h-8 shrink-0 items-center gap-2 border-b border-border bg-card px-3">
              <File className="size-3.5 shrink-0 text-faint" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground">{doc.activeFile}</span>
              <span className="shrink-0 font-mono text-xs text-faint">{formatSize(activeSize)}</span>
            </div>
            {/* The reader scrolls; keyboard users reach it as one stop. */}
            <div
              tabIndex={0}
              role="region"
              aria-label={doc.activeFile}
              className={cn("min-h-0 flex-1 overflow-auto bg-background", focusRing, "focus-visible:-outline-offset-2")}
            >
              {doc.loadingContent ? (
                <p className="m-0 flex items-center gap-2 p-4 text-sm text-muted-foreground">
                  <LoaderCircle className="size-4 motion-safe:animate-spin" aria-hidden="true" />
                  {t("detail.loadingContent")}
                </p>
              ) : doc.contentError ? (
                <p className="m-0 p-4 text-sm text-failed">{describeCommandError(doc.contentError, t)}</p>
              ) : (
                <FileContent filename={doc.activeFile} content={doc.content} />
              )}
            </div>
          </>
        ) : null}
      </div>
    </div>
  );
}
