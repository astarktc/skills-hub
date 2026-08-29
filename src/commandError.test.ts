// Tests at the commandError seam: the one frontend surface that turns any
// command rejection (structured CommandError payload, Error, string) into
// localized user copy or a silent null. Expected values come from the wire
// contract in docs/adr/0001-tagged-command-error-contract.md and the i18n
// key names in src/i18n/resources.ts.

import { describe, expect, it } from "vitest";
import {
  describeCommandError,
  isCommandError,
  toCommandError,
} from "./commandError";

// Deterministic translate stub: renders the key plus any interpolation
// values, so assertions can verify both key choice and passed params
// without coupling to real EN/ZH copy.
const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

describe("isCommandError", () => {
  it("accepts a structured payload with a known code", () => {
    expect(isCommandError({ code: "TARGET_EXISTS" })).toBe(true);
  });

  it("rejects unknown codes, plain Errors, and strings", () => {
    expect(isCommandError({ code: "NOT_A_REAL_CODE" })).toBe(false);
    expect(isCommandError(new Error("boom"))).toBe(false);
    expect(isCommandError("boom")).toBe(false);
    expect(isCommandError(null)).toBe(false);
  });
});

describe("toCommandError", () => {
  it("passes a structured payload through unchanged", () => {
    const err = { code: "CANCELLED" };
    expect(toCommandError(err)).toBe(err);
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

  it("recovers the skill name from the legacy central-repo prose", () => {
    expect(
      describeCommandError(
        new Error(
          'skill already exists in central repo: "/home/x/.skillshub/react-best-practices"',
        ),
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
