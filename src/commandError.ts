// The single frontend consumer of the backend's structured CommandError
// (see src-tauri/src/commands/error.rs and the generated union in
// src/bindings/index.ts). All user-facing error copy is composed here
// via i18n; nothing else in the frontend should inspect command failures.

import type { CommandError } from "./bindings";
import { toolLabel } from "./lib/skillPresentation";

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
  FINALIZE_ROLLBACK_FAILED: true,
  DUPLICATE_PROJECT: true,
  ASSIGNMENT_EXISTS: true,
  NOT_FOUND: true,
  UNKNOWN_TOOL: true,
  INVALID_PATH: true,
  CANCELLED: true,
  RATE_LIMITED: true,
  GIT_CLONE_FAILED: true,
  GITHUB_SKILL_NOT_FOUND: true,
  INVALID_GITHUB_URL: true,
  GIT_REPOINT_REQUIRES_GIT: true,
  DELETE_CLEANUP_FAILED: true,
  PATH_OUTSIDE_TOOL_DIRS: true,
  SKILL_MANIFEST_IO: true,
  SOURCE_PATH_MISSING: true,
  CENTRAL_PATH_MISSING: true,
  SUBPATH_MISSING: true,
  REVEAL_LOG_FAILED: true,
  SYMLINK_ESCAPES_REPO: true,
  SYMLINK_CHAIN_TOO_DEEP: true,
  NOT_REFRESHABLE: true,
  LOCAL_SOURCE_INSIDE_TOOL_DIR: true,
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

const INVALID_PATH_KEYS: Record<string, string> = {
  missing: "errors.invalidPathMissing",
  not_a_directory: "errors.invalidPathNotADirectory",
};

/** A localized sentence followed by a structured detail (a path, a subpath,
 * diagnostics) on its own line — the detail never travels inside the prose. */
function withDetail(message: string, detail: string): string {
  return detail ? `${message}\n\n${detail}` : message;
}

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
    case "FINALIZE_ROLLBACK_FAILED":
      return withDetail(
        e.backup !== null
          ? t("errors.finalizeRollbackFailed", { central: e.central, backup: e.backup })
          : t("errors.finalizeRollbackFailedNoBackup", { central: e.central }),
        e.detail,
      );
    case "DUPLICATE_PROJECT":
      return t("projects.duplicateError") + (e.path ? `: ${e.path}` : "");
    case "ASSIGNMENT_EXISTS":
      return t("projects.assignmentExistsError");
    case "NOT_FOUND":
      return t("projects.notFoundError") + `: ${e.kind}:${e.id}`;
    case "UNKNOWN_TOOL":
      return t("errors.unknownTool", { tool: e.tool });
    case "INVALID_PATH": {
      const key = INVALID_PATH_KEYS[e.reason];
      return key
        ? t(key, { path: e.path })
        : t("errors.invalidPath", { path: e.path, reason: e.reason });
    }
    case "CANCELLED":
      return null;
    case "RATE_LIMITED":
      return e.resetMinutes > 0
        ? t("errors.rateLimited", { minutes: e.resetMinutes })
        : t("errors.rateLimitedNoEta");
    case "GIT_CLONE_FAILED":
      return withDetail(
        t(GIT_CLONE_HINT_KEYS[e.kind] ?? "errors.gitCloneUnknown"),
        e.detail,
      );
    case "GITHUB_SKILL_NOT_FOUND":
      return t("errors.githubSkillNotFound", { url: e.url });
    case "INVALID_GITHUB_URL":
      return t("errors.invalidGithubUrl", { url: e.url });
    case "GIT_REPOINT_REQUIRES_GIT":
      return t("errors.gitRepointRequiresGit", { name: e.name });
    case "DELETE_CLEANUP_FAILED":
      return t("errors.deleteCleanupFailed") + "\n- " + e.failures.join("\n- ");
    case "PATH_OUTSIDE_TOOL_DIRS":
      return t("errors.pathOutsideToolDirs", { path: e.path });
    case "SKILL_MANIFEST_IO":
      return withDetail(t("errors.skillManifestIo", { path: e.path }), e.detail);
    case "SOURCE_PATH_MISSING":
      return withDetail(t("errors.sourcePathMissing"), e.path);
    case "CENTRAL_PATH_MISSING":
      return withDetail(t("errors.centralPathMissing"), e.path);
    case "SUBPATH_MISSING":
      return withDetail(t("errors.subpathMissing"), e.subpath);
    case "REVEAL_LOG_FAILED":
      return withDetail(t("errors.revealLogFailed"), e.detail);
    case "SYMLINK_ESCAPES_REPO":
      return t("errors.symlinkEscapesRepo", {
        subpath: e.subpath,
        target: e.target,
      });
    case "SYMLINK_CHAIN_TOO_DEEP":
      return t("errors.symlinkChainTooDeep", { subpath: e.subpath });
    case "NOT_REFRESHABLE":
      return t("errors.notRefreshable", { name: e.name });
    case "LOCAL_SOURCE_INSIDE_TOOL_DIR":
      // The wire carries the Tool's registry key; its label is the same
      // rule the cards use.
      return withDetail(
        t("errors.localSourceInsideToolDir", { tool: toolLabel(t, e.tool) }),
        e.path,
      );
    case "OTHER":
      return e.message;
  }
}
