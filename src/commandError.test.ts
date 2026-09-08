// Tests at the commandError seam: the one frontend surface that turns any
// command rejection (structured CommandError payload, Error, string) into
// localized user copy or a silent null. Expected values come from the wire
// contract in docs/adr/0001-tagged-command-error-contract.md and the i18n
// key names in src/i18n/resources.ts.

import { describe, expect, it } from "vitest";
import { describeCommandError, toCommandError } from "./commandError";

// Deterministic translate stub: renders the key plus any interpolation
// values, so assertions can verify both key choice and passed params
// without coupling to real EN/ZH copy.
const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

describe("toCommandError", () => {
  it("passes a structured payload with a known code through unchanged", () => {
    const err = { code: "CANCELLED" };
    expect(toCommandError(err)).toBe(err);
  });

  it("passes another known code through unchanged", () => {
    const err = { code: "TARGET_EXISTS", path: "/x" };
    expect(toCommandError(err)).toBe(err);
  });

  it("wraps payloads with unknown codes and null as OTHER", () => {
    expect(toCommandError({ code: "NOT_A_REAL_CODE" })).toEqual({
      code: "OTHER",
      message: "[object Object]",
    });
    expect(toCommandError(null)).toEqual({ code: "OTHER", message: "null" });
  });

  it("wraps an Error as OTHER with its message", () => {
    expect(toCommandError(new Error("boom"))).toEqual({
      code: "OTHER",
      message: "boom",
    });
  });

  it("stringifies anything else as OTHER", () => {
    expect(toCommandError(42)).toEqual({ code: "OTHER", message: "42" });
  });
});

describe("describeCommandError", () => {
  it("returns null for CANCELLED (silent by contract)", () => {
    expect(describeCommandError({ code: "CANCELLED" }, t)).toBeNull();
  });

  it("interpolates tool and path for TOOL_NOT_WRITABLE", () => {
    expect(
      describeCommandError(
        { code: "TOOL_NOT_WRITABLE", tool: "cursor", path: "/x" },
        t,
      ),
    ).toBe('errors.toolNotWritable {"tool":"cursor","path":"/x"}');
  });

  it("special-cases the missing_skill_md reason of SKILL_INVALID", () => {
    expect(
      describeCommandError(
        { code: "SKILL_INVALID", reason: "missing_skill_md" },
        t,
      ),
    ).toBe("errors.skillInvalidMissingSkillMd");
    expect(
      describeCommandError({ code: "SKILL_INVALID", reason: "empty" }, t),
    ).toBe('errors.skillInvalid {"reason":"empty"}');
  });

  it("appends the path to DUPLICATE_PROJECT only when present", () => {
    expect(
      describeCommandError({ code: "DUPLICATE_PROJECT", path: "/p" }, t),
    ).toBe("projects.duplicateError: /p");
    expect(
      describeCommandError({ code: "DUPLICATE_PROJECT", path: null }, t),
    ).toBe("projects.duplicateError");
  });

  it("distinguishes RATE_LIMITED with and without a reset ETA", () => {
    expect(
      describeCommandError({ code: "RATE_LIMITED", resetMinutes: 7 }, t),
    ).toBe('errors.rateLimited {"minutes":7}');
    expect(
      describeCommandError({ code: "RATE_LIMITED", resetMinutes: 0 }, t),
    ).toBe("errors.rateLimitedNoEta");
  });

  it("maps GIT_CLONE_FAILED kinds to hints and appends detail", () => {
    expect(
      describeCommandError(
        { code: "GIT_CLONE_FAILED", kind: "auth", detail: "401" },
        t,
      ),
    ).toBe("errors.gitCloneAuth\n\n401");
    // Unknown kind falls back to the generic hint.
    expect(
      describeCommandError(
        { code: "GIT_CLONE_FAILED", kind: "martian", detail: null },
        t,
      ),
    ).toBe("errors.gitCloneUnknown");
  });

  it("maps the execFailed git kind to its dedicated hint", () => {
    expect(
      describeCommandError(
        { code: "GIT_CLONE_FAILED", kind: "execFailed", detail: "boom" },
        t,
      ),
    ).toBe("errors.gitCloneExecFailed\n\nboom");
  });

  it("interpolates the checkable URL for GITHUB_SKILL_NOT_FOUND", () => {
    expect(
      describeCommandError(
        { code: "GITHUB_SKILL_NOT_FOUND", url: "https://g/tree/main/s" },
        t,
      ),
    ).toBe('errors.githubSkillNotFound {"url":"https://g/tree/main/s"}');
  });

  it("lists failed paths for DELETE_CLEANUP_FAILED", () => {
    expect(
      describeCommandError(
        { code: "DELETE_CLEANUP_FAILED", failures: ["/a: denied", "/b: busy"] },
        t,
      ),
    ).toBe("errors.deleteCleanupFailed\n- /a: denied\n- /b: busy");
  });

  it("localizes rollback recovery paths and keeps diagnostics separate", () => {
    const error = { code: "FINALIZE_ROLLBACK_FAILED", central: "/central", backup: "/backup", detail: "move failed" };
    expect(toCommandError(error)).toBe(error);
    expect(describeCommandError(error, t)).toBe(
      'errors.finalizeRollbackFailed {"central":"/central","backup":"/backup"}\n\nmove failed',
    );
    expect(describeCommandError({ ...error, backup: null }, t)).toBe(
      'errors.finalizeRollbackFailedNoBackup {"central":"/central"}\n\nmove failed',
    );
  });

  it("names the manifest path and preserves I/O diagnostics", () => {
    expect(describeCommandError({ code: "SKILL_MANIFEST_IO", path: "/skill/SKILL.md", detail: "Permission denied" }, t))
      .toBe('errors.skillManifestIo {"path":"/skill/SKILL.md"}\n\nPermission denied');
  });

  it("names the refused path for PATH_OUTSIDE_TOOL_DIRS", () => {
    expect(
      describeCommandError(
        { code: "PATH_OUTSIDE_TOOL_DIRS", path: "/home/u/Documents" },
        t,
      ),
    ).toBe('errors.pathOutsideToolDirs {"path":"/home/u/Documents"}');
  });

  it("shows the missing path as a detail line for SOURCE_PATH_MISSING", () => {
    expect(
      describeCommandError(
        { code: "SOURCE_PATH_MISSING", path: "/home/u/skills/gone" },
        t,
      ),
    ).toBe("errors.sourcePathMissing\n\n/home/u/skills/gone");
  });

  it("shows the missing path as a detail line for CENTRAL_PATH_MISSING", () => {
    expect(
      describeCommandError(
        { code: "CENTRAL_PATH_MISSING", path: "/home/u/.skillshub/gone" },
        t,
      ),
    ).toBe("errors.centralPathMissing\n\n/home/u/.skillshub/gone");
  });

  it("shows the requested subpath as a detail line for SUBPATH_MISSING", () => {
    expect(
      describeCommandError(
        { code: "SUBPATH_MISSING", subpath: "skills/nope" },
        t,
      ),
    ).toBe("errors.subpathMissing\n\nskills/nope");
  });

  it("shows the opener diagnostics as a detail line for REVEAL_LOG_FAILED", () => {
    expect(
      describeCommandError(
        { code: "REVEAL_LOG_FAILED", detail: "opener: exit 1" },
        t,
      ),
    ).toBe("errors.revealLogFailed\n\nopener: exit 1");
    expect(
      describeCommandError({ code: "REVEAL_LOG_FAILED", detail: "" }, t),
    ).toBe("errors.revealLogFailed");
  });

  it("names the link and its target for SYMLINK_ESCAPES_REPO", () => {
    expect(
      describeCommandError(
        {
          code: "SYMLINK_ESCAPES_REPO",
          subpath: "plugins/all/skills/x",
          target: "../../../../etc",
        },
        t,
      ),
    ).toBe(
      'errors.symlinkEscapesRepo {"subpath":"plugins/all/skills/x","target":"../../../../etc"}',
    );
  });

  it("interpolates the bounded chain's subpath for SYMLINK_CHAIN_TOO_DEEP", () => {
    expect(
      describeCommandError(
        { code: "SYMLINK_CHAIN_TOO_DEEP", subpath: "skills/alias-9" },
        t,
      ),
    ).toBe('errors.symlinkChainTooDeep {"subpath":"skills/alias-9"}');
  });

  it("names the skill for NOT_REFRESHABLE", () => {
    expect(
      describeCommandError({ code: "NOT_REFRESHABLE", name: "taken-over" }, t),
    ).toBe('errors.notRefreshable {"name":"taken-over"}');
  });

  it("names the holding Tool's label and shows the refused path as a detail line for LOCAL_SOURCE_INSIDE_TOOL_DIR", () => {
    expect(
      describeCommandError(
        {
          code: "LOCAL_SOURCE_INSIDE_TOOL_DIR",
          path: "/home/u/.claude/skills/taken",
          tool: "claude_code",
        },
        t,
      ),
    ).toBe(
      'errors.localSourceInsideToolDir {"tool":"tools.claude_code {\\"defaultValue\\":\\"claude_code\\"}"}\n\n/home/u/.claude/skills/taken',
    );
  });

  it("names the unknown tool key for UNKNOWN_TOOL", () => {
    expect(
      describeCommandError({ code: "UNKNOWN_TOOL", tool: "not-a-tool" }, t),
    ).toBe('errors.unknownTool {"tool":"not-a-tool"}');
  });

  it("localizes each INVALID_PATH reason token and falls back for unknown ones", () => {
    expect(
      describeCommandError(
        { code: "INVALID_PATH", path: "/gone", reason: "missing" },
        t,
      ),
    ).toBe('errors.invalidPathMissing {"path":"/gone"}');
    expect(
      describeCommandError(
        { code: "INVALID_PATH", path: "/f.txt", reason: "not_a_directory" },
        t,
      ),
    ).toBe('errors.invalidPathNotADirectory {"path":"/f.txt"}');
    expect(
      describeCommandError(
        { code: "INVALID_PATH", path: "/x", reason: "martian" },
        t,
      ),
    ).toBe('errors.invalidPath {"path":"/x","reason":"martian"}');
  });

  it("names the colliding skill for SKILL_EXISTS", () => {
    expect(
      describeCommandError(
        { code: "SKILL_EXISTS", name: "react-best-practices" },
        t,
      ),
    ).toBe('errors.skillExistsInHubNamed {"name":"react-best-practices"}');
  });

  it("passes unrecognized OTHER prose through verbatim", () => {
    expect(describeCommandError(new Error("weird failure"), t)).toBe(
      "weird failure",
    );
  });
});
