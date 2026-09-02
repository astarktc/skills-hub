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
