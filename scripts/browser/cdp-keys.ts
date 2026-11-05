// Trusted keyboard input for fixture audits, through the Chrome DevTools Protocol.
//
// Promoted from round 19's gitignored `evidence/{13,17}-fix/` (since pruned) at its closure (BACKLOG #69): every round-19 audit child
// drove its keyboard transcripts with this, and the gitignored evidence copies were about to be pruned.
//
// Why not `agent-browser press`: agent-browser 0.31.1 against Chrome 154 flooded the page with thousands of phantom
// trusted keydowns after each key ("x|Numpad6", "Unidentified|Minus"; >10 000 in 1.5 s), which toggled selection,
// stole focus and eventually reset the tab to about:blank; `press Meta+a` left Meta held. Re-test on a newer
// agent-browser before relying on `press` again.
//
// Model (Puppeteer's): `keyDown` with `text` for printable keys, else `rawKeyDown`; then `keyUp`. ⌘A also sends the
// `selectAll` editing command, as Chromium on macOS does.
//
// Usage (Bun or Node ≥ 22, an agent-browser session already open on the fixture):
//
//   import { press, typeText, closeKeys } from "./scripts/browser/cdp-keys.ts";
//   await press("Tab"); await press("Shift+Tab"); await press("Meta+a"); await typeText("pico");
//   closeKeys();
//
// Environment: `AB_SESSION` (agent-browser session name, default `fixture-audit`), `FIXTURE_ORIGIN` (the page to attach
// to, default `http://localhost:5175` — `npm run dev:fixture`).
import { spawnSync } from "node:child_process";

const SESSION = process.env.AB_SESSION ?? "fixture-audit";
const ORIGIN = process.env.FIXTURE_ORIGIN ?? "http://localhost:5175";

type KeyDef = { key: string; code: string; keyCode: number; text?: string };
const DEFS: Record<string, KeyDef> = {
  Tab: { key: "Tab", code: "Tab", keyCode: 9 },
  Enter: { key: "Enter", code: "Enter", keyCode: 13, text: "\r" },
  Escape: { key: "Escape", code: "Escape", keyCode: 27 },
  Space: { key: " ", code: "Space", keyCode: 32, text: " " },
  ArrowUp: { key: "ArrowUp", code: "ArrowUp", keyCode: 38 },
  ArrowDown: { key: "ArrowDown", code: "ArrowDown", keyCode: 40 },
  ArrowLeft: { key: "ArrowLeft", code: "ArrowLeft", keyCode: 37 },
  ArrowRight: { key: "ArrowRight", code: "ArrowRight", keyCode: 39 },
  Backspace: { key: "Backspace", code: "Backspace", keyCode: 8 },
  Home: { key: "Home", code: "Home", keyCode: 36 },
  End: { key: "End", code: "End", keyCode: 35 },
  "/": { key: "/", code: "Slash", keyCode: 191, text: "/" },
};
for (const c of "abcdefghijklmnopqrstuvwxyz") {
  DEFS[c] = { key: c, code: "Key" + c.toUpperCase(), keyCode: c.toUpperCase().charCodeAt(0), text: c };
}

/** One agent-browser CLI call, parsed from `--json`. */
export function agentBrowser(...args: string[]): Record<string, unknown> {
  const run = spawnSync("agent-browser", ["--session", SESSION, ...args, "--json"], { timeout: 90_000 });
  const raw = run.stdout.toString();
  let parsed: { success?: boolean; data?: Record<string, unknown> };
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error(`agent-browser ${args.join(" ")}: ${raw}${run.stderr.toString()}`);
  }
  if (!parsed.success) throw new Error(JSON.stringify(parsed));
  return parsed.data ?? {};
}

type Cdp = { send: (method: string, params?: Record<string, unknown>) => Promise<unknown>; close: () => void };
let conn: Cdp | null = null;

async function connect(): Promise<Cdp> {
  const cdpUrl = String(agentBrowser("get", "cdp-url").cdpUrl);
  const ws = new WebSocket(cdpUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let id = 0;
  const pending = new Map<number, [(value: unknown) => void, (reason: unknown) => void]>();
  ws.onmessage = (event) => {
    let message: { id?: number; error?: unknown; result?: unknown };
    try {
      message = JSON.parse(String(event.data));
    } catch {
      return; // not a CDP reply
    }
    const waiter = message.id ? pending.get(message.id) : undefined;
    if (!waiter) return;
    pending.delete(message.id as number);
    if (message.error) waiter[1](message.error);
    else waiter[0](message.result);
  };
  ws.onclose = () => {
    conn = null;
  };
  const call = (method: string, params: Record<string, unknown> = {}, sessionId?: string) =>
    new Promise<unknown>((resolve, reject) => {
      const n = ++id;
      pending.set(n, [resolve, reject]);
      ws.send(JSON.stringify({ id: n, method, params, ...(sessionId ? { sessionId } : {}) }));
      setTimeout(() => {
        if (pending.delete(n)) reject(new Error("CDP timeout " + method));
      }, 15_000);
    });
  const { targetInfos } = (await call("Target.getTargets")) as {
    targetInfos: { type: string; url: string; targetId: string }[];
  };
  const page = targetInfos.find((t) => t.type === "page" && t.url.startsWith(ORIGIN));
  if (!page) throw new Error(`no page target at ${ORIGIN}`);
  const { sessionId } = (await call("Target.attachToTarget", { targetId: page.targetId, flatten: true })) as {
    sessionId: string;
  };
  return { send: (method, params = {}) => call(method, params, sessionId), close: () => ws.close() };
}

async function withConnection(body: (cdp: Cdp) => Promise<void>): Promise<void> {
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      if (!conn) conn = await connect();
      await body(conn);
      return;
    } catch (error) {
      closeKeys();
      if (attempt) throw error;
    }
  }
}

/** Press one key or chord, e.g. `Tab`, `Shift+Tab`, `Meta+a`, `Control+ArrowDown`. */
export async function press(combo: string): Promise<void> {
  const parts = combo.split("+");
  const name = parts.pop() ?? "";
  const def = DEFS[name];
  if (!def) throw new Error("no key definition for " + name);
  const modifiers =
    (parts.includes("Alt") ? 1 : 0) |
    (parts.includes("Control") ? 2 : 0) |
    (parts.includes("Meta") ? 4 : 0) |
    (parts.includes("Shift") ? 8 : 0);
  // A chord with Alt/Control/Meta types no text; Shift alone upper-cases it.
  const text = modifiers & ~8 ? undefined : parts.includes("Shift") && def.text ? def.text.toUpperCase() : def.text;
  const base = { key: def.key, code: def.code, windowsVirtualKeyCode: def.keyCode, modifiers };
  const commands = modifiers === 4 && name === "a" ? ["selectAll"] : undefined;
  await withConnection(async (cdp) => {
    await cdp.send("Input.dispatchKeyEvent", {
      type: text ? "keyDown" : "rawKeyDown",
      ...base,
      ...(text ? { text, unmodifiedText: text } : {}),
      ...(commands ? { commands } : {}),
    });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
  });
}

/** Trusted text entry into the focused field (`Input.insertText` — an IME-style commit, one input event). */
export async function typeText(text: string): Promise<void> {
  await withConnection(async (cdp) => {
    await cdp.send("Input.insertText", { text });
  });
}

/** Close the shared CDP connection (call once at the end of a script). */
export function closeKeys(): void {
  try {
    conn?.close();
  } catch {
    // already closed
  }
  conn = null;
}
