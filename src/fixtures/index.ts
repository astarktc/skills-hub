// Fixture-backed browser mode (`npm run dev:fixture`): the whole app runs in
// a plain browser against an in-memory library, so agents and reviewers can
// see and drive every surface without touching the operator's real skills.
//
// Two seams, both installed before the first render (`src/main.tsx`):
// - App commands: `invokeTauri` (`src/lib/tauri.ts`) routes to
//   `fixtureInvoke` — typed per command, one handler each (`backend.ts`).
// - Native plumbing: Tauri's own `mockIPC`/`mockWindows` stand in for the
//   webview internals, so `new Channel()` works and plugin calls (dialog,
//   updater, app version, webview zoom) get canned answers.
//
// Dev-only; reached only through the `VITE_MOCK_BACKEND` gate.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { version } from "../../package.json";
import type { CommandName, Commands } from "../lib/tauri";
import { createFixtureBackend, type FixtureBackend } from "./backend";
import { SCENARIOS, type ScenarioName } from "./model";
import { buildScenario } from "./scenarios";

/** Distinctive marker: the tree-shake proof greps `dist/` for it. */
export const FIXTURE_MODE_MARKER = "skills-hub-fixture-backend";

function scenarioFromUrl(): ScenarioName {
  const requested = new URLSearchParams(window.location.search).get("scenario");
  if (requested && (SCENARIOS as readonly string[]).includes(requested)) {
    return requested as ScenarioName;
  }
  if (requested) {
    console.warn(`[fixture] unknown scenario "${requested}"; using "rich" (${SCENARIOS.join(" | ")})`);
  }
  return "rich";
}

/** `?latency=<scale>` (e.g. `0` for instant, `3` for slow-motion progress). */
function latencyFromUrl(): number | null {
  const raw = new URLSearchParams(window.location.search).get("latency");
  const value = raw === null ? Number.NaN : Number(raw);
  return Number.isFinite(value) && value >= 0 ? value : null;
}

/** Yield one macrotask without a timer (not throttled in a hidden tab). */
const yieldTask = () =>
  new Promise<void>((resolve) => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => resolve();
    channel.port2.postMessage(null);
  });

/**
 * Simulated latency. A hidden tab (an agent's background preview) throttles
 * `setTimeout` to once a minute, which would stall every flow; there the
 * fixture only yields, unless `?latency=` asked for real delays explicitly.
 */
function sleepFor(explicit: boolean) {
  return (ms: number) =>
    !explicit && document.visibilityState === "hidden"
      ? yieldTask()
      : new Promise<void>((resolve) => setTimeout(resolve, ms));
}

let backend: FixtureBackend | null = null;

function current(): FixtureBackend {
  if (!backend) {
    const latency = latencyFromUrl();
    backend = createFixtureBackend(buildScenario(scenarioFromUrl()), {
      latencyScale: latency ?? 1,
      sleep: sleepFor(latency !== null),
    });
  }
  return backend;
}

/** The fixture counterpart of `invokeTauri`. */
export function fixtureInvoke<K extends CommandName>(
  command: K,
  ...args: Parameters<Commands[K]>
): Promise<Awaited<ReturnType<Commands[K]>>> {
  return current().invoke(command, ...args);
}

/** What a native call answers: a picked folder, a version string, or nothing. */
type NativeAnswer = string | null;

/** Canned answers for the native plugin calls the app makes. */
function pluginAnswer(cmd: string, args: unknown): NativeAnswer {
  const state = current().state;
  switch (cmd) {
    case "plugin:dialog|open": {
      // A folder picker: rotate through the fixture's known folders.
      const pick = state.dialogPicks.shift() ?? null;
      if (pick) state.dialogPicks.push(pick);
      return pick;
    }
    case "plugin:updater|check":
      return null;
    case "plugin:app|version":
      return `${version}-fixture`;
    case "plugin:app|name":
      return "Skills Hub";
    case "plugin:app|tauri_version":
      return "2";
    case "plugin:webview|set_webview_zoom": {
      const value = (args as { value?: number } | undefined)?.value;
      if (typeof value === "number") document.documentElement.style.zoom = String(value);
      return null;
    }
    default:
      console.warn(`[fixture] unhandled native call ${cmd}`, args);
      throw new Error(`fixture mode: no answer for ${cmd}`);
  }
}

/** Install both seams. Call once, before the app renders. */
export function installFixtureBackend(): void {
  const { scenario } = current().state;
  mockWindows("main");
  mockIPC((cmd, args) => pluginAnswer(cmd, args));
  document.title = `Skills Hub · fixture: ${scenario}`;
  document.documentElement.dataset.fixture = FIXTURE_MODE_MARKER;
  console.info(
    `[fixture] ${FIXTURE_MODE_MARKER}: scenario "${scenario}" — in-memory library, nothing on disk is touched. ` +
      `Switch with ?scenario=${SCENARIOS.join("|")}`,
  );
}
