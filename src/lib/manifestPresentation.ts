/** Pure presentation adapter for manifest metadata and Markdown body. */
export function parseFrontmatter(raw: string): {
  meta: Record<string, string> | null;
  body: string;
} {
  if (!raw.startsWith("---")) return { meta: null, body: raw };
  const end = raw.indexOf("\n---", 3);
  if (end === -1) return { meta: null, body: raw };
  const block = raw.slice(4, end);
  const entries: Record<string, string> = {};
  const lines = block.split("\n");
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
  const body = raw.slice(end + 4).replace(/^\n+/, "");
  return { meta: entries, body };
}
