/** Pure presentation adapter for manifest metadata and Markdown body. */
export function parseFrontmatter(raw: string): {
  meta: Record<string, string> | null;
  body: string;
} {
  // Match core/manifest.rs: complete column-zero lines, trailing whitespace
  // allowed. Keep raw offsets so CRLF and later body fences survive untouched.
  const rawLines = raw.split("\n");
  if (rawLines[0].trimEnd() !== "---") return { meta: null, body: raw };
  const end = rawLines.findIndex((line, i) => i > 0 && line.trimEnd() === "---");
  if (end === -1) return { meta: null, body: raw };
  const entries: Record<string, string> = {};
  const lines = rawLines.slice(1, end).map((line) => line.replace(/\r$/, ""));
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const idx = line.indexOf(":");
    if (idx === -1) continue;
    const key = line.slice(0, idx).trim();
    let val = line.slice(idx + 1).trim();
    if (val === "|" || val === ">") {
      const blockLines: string[] = [];
      while (i + 1 < lines.length) {
        const next = lines[i + 1];
        if (next.trim() !== "" && !/^\s/.test(next)) break;
        blockLines.push(next.replace(/^\s{2}/, ""));
        i++;
      }
      val =
        val === "|"
          ? blockLines.join("\n").trim()
          : blockLines
              .map((v) => v.trim())
              .filter(Boolean)
              .join(" ");
    }
    // strip surrounding quotes
    if (
      val.length >= 2 &&
      ((val[0] === '"' && val[val.length - 1] === '"') ||
        (val[0] === "'" && val[val.length - 1] === "'"))
    ) {
      val = val.slice(1, -1);
    }
    if (key) entries[key] = val;
  }
  const keys = Object.keys(entries);
  if (keys.length === 0) return { meta: null, body: raw };
  const bodyStart = rawLines
    .slice(0, end + 1)
    .reduce((offset, line) => offset + line.length + 1, 0);
  const body = raw.slice(bodyStart).replace(/^(?:\r?\n)+/, "");
  return { meta: entries, body };
}
