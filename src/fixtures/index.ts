// Fixture-backed browser mode (`npm run dev:fixture`): the whole app runs in
// a plain browser against an in-memory library, so agents and reviewers can
// see and drive every surface without touching the operator's real skills.
//
// Two seams, both installed before the first render (`src/main.tsx`):
// - App commands: `invokeTauri` (`src/lib/tauri.ts`) routes to
//   `fixtureInvoke` — typed per command, one handler each (`backend.ts`).
// - The native platform: `src/lib/platform.ts`'s second adapter
//   (`installPlatformAdapter`) — the folder picker rotates through the
//   scenario's folders, progress channels are plain message sinks, the
//   updater answers `?update=<version>` (none by default), zoom sets CSS
//   zoom. Tauri's own `mockIPC`/`mockWindows` stay installed underneath as a
//   backstop: a native call that bypasses the platform seam is logged loudly.
//
// Dev-only; reached only through the `VITE_MOCK_BACKEND` gate.

import type { Channel } from "@tauri-apps/api/core";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { version } from "../../package.json";
import { installPlatformAdapter, type PlatformAdapter } from "../lib/platform";
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

/**
 * `?update=<version>`: the updater finds that release (its install takes a
 * moment and succeeds; `?update=<version>&updateFails` makes it fail), so the
 * startup notice and Settings' update section can be driven. Absent: the
 * fixture app is current.
 */
function updateFromUrl(): string | null {
  return new URLSearchParams(window.location.search).get("update");
}

/** The fixture's platform adapter: canned answers, nothing native. */
function fixturePlatform(): PlatformAdapter {
  const params = new URLSearchParams(window.location.search);
  return {
    async pickFolder() {
      // A folder picker: rotate through the fixture's known folders.
      const state = current().state;
      const pick = state.dialogPicks.shift() ?? null;
      if (pick) state.dialogPicks.push(pick);
      return pick ? [pick] : [];
    },
    async createProgressChannel<T>(onMessage: (message: T) => void) {
      // SAFETY: the channel only reaches `fixtureInvoke`, whose handlers only
      // ever call `onmessage` (backend.ts `ProgressSink`); nothing serialises it.
      return { onmessage: onMessage } as unknown as Channel<T>;
    },
    async checkForUpdate() {
      const found = updateFromUrl();
      if (!found) return null;
      const fails = params.has("updateFails");
      return {
        version: found,
        downloadAndInstall: async () => {
          const latency = latencyFromUrl();
          await sleepFor(latency !== null)(1200 * (latency ?? 1));
          if (fails) throw new Error("fixture mode: the download failed");
        },
        releaseNotes: async () =>
          `## What's new in ${found}\n\n- Fixture release notes (\`?update=${found}\`).`,
      };
    },
    async setZoom(factor) {
      document.documentElement.style.zoom = String(factor);
    },
    async appVersion() {
      return `${version}-fixture`;
    },
  };
}

/** Native calls that bypassed the platform seam: logged, then refused. */
function unexpectedNativeCall(cmd: string, args: unknown): never {
  console.warn(`[fixture] unhandled native call ${cmd}`, args);
  throw new Error(`fixture mode: no answer for ${cmd}`);
}

/** Install both seams. Call once, before the app renders. */
export function installFixtureBackend(): void {
  const { scenario } = current().state;
  installPlatformAdapter(fixturePlatform());
  mockWindows("main");
  mockIPC((cmd, args) => unexpectedNativeCall(cmd, args));
  document.title = `Skills Hub · fixture: ${scenario}`;
  document.documentElement.dataset.fixture = FIXTURE_MODE_MARKER;
  console.info(
    `[fixture] ${FIXTURE_MODE_MARKER}: scenario "${scenario}" — in-memory library, nothing on disk is touched. ` +
      `Switch with ?scenario=${SCENARIOS.join("|")}`,
  );
}
