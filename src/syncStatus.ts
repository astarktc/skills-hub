// The single frontend consumer of the backend's sync-status lifecycle
// (see src-tauri/src/core/sync_status.rs and the generated unions in
// src/bindings/SyncStatus.ts and src/bindings/ProjectSyncStatus.ts).
// The maps below are compiler-derived whitelists: `satisfies` forces them to
// stay in exact sync with the generated unions, so adding a Rust variant
// fails `npm run build` until the frontend decides how to render it.

import type { ProjectSyncStatus } from "./bindings/ProjectSyncStatus";
import type { SyncStatus } from "./bindings/SyncStatus";

/** CSS modifier class for an assignment cell (`.matrix-cell.<class>`). */
export const SYNC_STATUS_CLASS = {
  pending: "pending",
  synced: "synced",
  stale: "stale",
  missing: "missing",
  error: "error",
} as const satisfies Record<SyncStatus, string>;

/** CSS modifier class for the project list dot (`.project-status-dot.<class>`). */
export const PROJECT_SYNC_STATUS_CLASS = {
  none: "none",
  error: "error",
  stale: "stale",
  pending: "pending",
  synced: "synced",
} as const satisfies Record<ProjectSyncStatus, string>;
