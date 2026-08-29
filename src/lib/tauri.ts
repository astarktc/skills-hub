// Narrow Tauri seam shared by every hook: command invocation fails loudly
// when the app runs outside a Tauri webview (plain `npm run dev` in a
// browser) instead of hanging on a missing IPC bridge.

export const isTauri =
  typeof window !== "undefined" &&
  Boolean(
    (window as { __TAURI__?: unknown }).__TAURI__ ||
      (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
  );

export async function invokeTauri<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri) {
    throw new Error("Tauri API is not available");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}
