// The single frontend consumer of the backend's structured CommandError
// (see src-tauri/src/commands/error.rs and the generated union in
// src/bindings/index.ts). All user-facing error copy is composed here
// via i18n; nothing else in the frontend should inspect command failures.

import type { CommandError } from "./bindings";

type TranslateFn = (key: string, opts?: Record<string, unknown>) => string;

// Compiler-derived whitelist of wire codes: `satisfies` forces this map to
// stay in exact sync with the generated union, so adding a Rust variant
// (which regenerates src/bindings/index.ts) fails `npm run build`
// until the frontend handles the new code here and in describeCommandError.
const COMMAND_ERROR_CODE_MAP = {
  TOOL_NOT_INSTALLED: true,
  TARGET_EXISTS: true,
  TOOL_NOT_WRITABLE: true,
  SKILL_INVALID: true,
  MULTI_SKILLS: true,
  SKILL_EXISTS: true,
  DUPLICATE_PROJECT: true,
  ASSIGNMENT_EXISTS: true,
  NOT_FOUND: true,
  CANCELLED: true,
  RATE_LIMITED: true,
  GIT_CLONE_FAILED: true,
  GITHUB_SKILL_NOT_FOUND: true,
  DELETE_CLEANUP_FAILED: true,
  OTHER: true,
} as const satisfies Record<CommandError["code"], true>;

const COMMAND_ERROR_CODES: ReadonlySet<string> = new Set(
  Object.keys(COMMAND_ERROR_CODE_MAP),
);

function isCommandError(err: unknown): err is CommandError {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    typeof (err as { code: unknown }).code === "string" &&
    COMMAND_ERROR_CODES.has((err as { code: string }).code)
  );
}

/** Normalize any rejection (structured payload, Error, string) to the union. */
export function toCommandError(err: unknown): CommandError {
  if (isCommandError(err)) return err;
  if (err instanceof Error) return { code: "OTHER", message: err.message };
  return { code: "OTHER", message: String(err) };
}

const GIT_CLONE_HINT_KEYS: Record<string, string> = {
  tls: "errors.gitCloneTls",
  auth: "errors.gitCloneAuth",
  notFound: "errors.gitCloneNotFound",
  dns: "errors.gitCloneDns",
  timeout: "errors.gitCloneTimeout",
  refused: "errors.gitCloneRefused",
  execFailed: "errors.gitCloneExecFailed",
  unknown: "errors.gitCloneUnknown",
};

/**
 * Localized user-facing message for a command failure, or `null` when the
 * failure should be silently ignored (user-initiated cancellation).
 */
export function describeCommandError(
  err: unknown,
  t: TranslateFn,
): string | null {
  const e = toCommandError(err);
  switch (e.code) {
    case "TOOL_NOT_INSTALLED":
      return t("errors.toolNotInstalled");
    case "TARGET_EXISTS":
      return t("errors.targetExists");
    case "TOOL_NOT_WRITABLE":
      return t("errors.toolNotWritable", { tool: e.tool, path: e.path });
    case "SKILL_INVALID":
      return e.reason === "missing_skill_md"
        ? t("errors.skillInvalidMissingSkillMd")
        : t("errors.skillInvalid", { reason: e.reason });
    case "MULTI_SKILLS":
      return t("errors.multiSkillsRepo");
    case "SKILL_EXISTS":
      return t("errors.skillExistsInHubNamed", { name: e.name });
    case "DUPLICATE_PROJECT":
      return t("projects.duplicateError") + (e.path ? `: ${e.path}` : "");
    case "ASSIGNMENT_EXISTS":
      return t("projects.assignmentExistsError");
    case "NOT_FOUND":
      return t("projects.notFoundError") + `: ${e.kind}:${e.id}`;
    case "CANCELLED":
      return null;
    case "RATE_LIMITED":
      return e.resetMinutes > 0
        ? t("errors.rateLimited", { minutes: e.resetMinutes })
        : t("errors.rateLimitedNoEta");
    case "GIT_CLONE_FAILED": {
      const hint = t(GIT_CLONE_HINT_KEYS[e.kind] ?? "errors.gitCloneUnknown");
      return e.detail ? `${hint}\n\n${e.detail}` : hint;
    }
    case "GITHUB_SKILL_NOT_FOUND":
      return t("errors.githubSkillNotFound", { url: e.url });
    case "DELETE_CLEANUP_FAILED":
      return (
        t("errors.deleteCleanupFailed") + "\n- " + e.failures.join("\n- ")
      );
    case "OTHER":
      return e.message;
  }
}
