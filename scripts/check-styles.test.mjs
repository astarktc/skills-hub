// Regression suite for the styling guard (scripts/check-styles.mjs, ADR-0006 §6).
// Each case applies ONE violation to a temp copy of the repository's styling
// inputs (src/, the guard, its baseline) and asserts the guard exits 1 with
// the rule's tag; the unmodified copy must exit 0. Includes the round-19
// ticket-07 review's counterexamples (S1), which the regex-based guard passed.
//
// Run: node --test scripts/check-styles.test.mjs   (styles:check runs it first)

import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  appendFileSync,
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
let workspace;
let template;
let caseNumber = 0;

before(() => {
  workspace = mkdtempSync(join(tmpdir(), "styles-check-test-"));
  template = join(workspace, "template");
  mkdirSync(join(template, "scripts"), { recursive: true });
  cpSync(join(ROOT, "src"), join(template, "src"), { recursive: true });
  for (const file of ["check-styles.mjs", "styles-baseline.json"]) {
    cpSync(join(ROOT, "scripts", file), join(template, "scripts", file));
  }
});

after(() => {
  if (workspace) rmSync(workspace, { recursive: true, force: true });
});

/** A fresh copy, mutated by `mutate(root)`, checked; returns the guard's result. */
function runGuard(mutate) {
  const root = join(workspace, `case-${++caseNumber}`);
  cpSync(template, root, { recursive: true });
  symlinkSync(join(ROOT, "node_modules"), join(root, "node_modules"), "dir");
  mutate?.(root);
  const run = spawnSync(process.execPath, [join(root, "scripts", "check-styles.mjs"), "--no-self-test"], {
    cwd: root,
    encoding: "utf8",
  });
  return { status: run.status, output: `${run.stdout}${run.stderr}` };
}

const write = (root, rel, text) => {
  mkdirSync(dirname(join(root, rel)), { recursive: true });
  writeFileSync(join(root, rel), text);
};
const appendProse = (root, css) => appendFileSync(join(root, "src/styles/prose.css"), `\n${css}\n`);
const markMigrated = (root, file) => {
  const path = join(root, "scripts/styles-baseline.json");
  const baseline = JSON.parse(readFileSync(path, "utf8"));
  baseline.migrated.push(file);
  writeFileSync(path, JSON.stringify(baseline, null, 2));
};

function expectRejected(result, rule, fragment) {
  assert.equal(result.status, 1, `expected exit 1, got ${result.status}:\n${result.output}`);
  assert.match(result.output, new RegExp(`\\[${rule}\\]`), result.output);
  if (fragment) assert.ok(result.output.includes(fragment), `missing "${fragment}" in:\n${result.output}`);
}

describe("styles:check", () => {
  it("passes the unmodified tree", () => {
    const result = runGuard();
    assert.equal(result.status, 0, result.output);
    assert.match(result.output, /styles:check ok/);
  });

  describe("rule 1 — the App.css ratchet", () => {
    it("rejects a new App.css line", () => {
      const result = runGuard((root) => appendFileSync(join(root, "src/App.css"), "\n.probe { display: block; }\n"));
      expectRejected(result, "growth", "above its ceiling");
    });
  });

  describe("rule 2 — migrated files reference no legacy class", () => {
    it("rejects a literal legacy class", () => {
      const result = runGuard((root) => {
        write(root, "src/ReviewProbe.tsx", 'export const Probe = () => <div className="btn" />;\n');
        markMigrated(root, "src/ReviewProbe.tsx");
      });
      expectRejected(result, "migrated", "btn");
    });

    it("rejects a legacy class held in a local constant", () => {
      const result = runGuard((root) => {
        write(
          root,
          "src/ReviewProbe.tsx",
          'const legacy = "btn";\nexport const Probe = () => <div className={legacy} />;\n',
        );
        markMigrated(root, "src/ReviewProbe.tsx");
      });
      expectRejected(result, "migrated", "btn");
    });

    it("rejects a legacy class behind a conditional, an object and a template", () => {
      const result = runGuard((root) => {
        write(
          root,
          "src/ReviewProbe.tsx",
          [
            'const looks = { primary: "btn-primary" } as const;',
            "export const Probe = ({ on }: { on: boolean }) => (",
            "  <div className={`${on ? looks.primary : \"\"} flex`} />",
            ");",
            "",
          ].join("\n"),
        );
        markMigrated(root, "src/ReviewProbe.tsx");
      });
      expectRejected(result, "migrated", "btn-primary");
    });

    it("rejects a legacy class imported from a sibling module", () => {
      const result = runGuard((root) => {
        write(root, "src/probeClasses.ts", 'export const legacy = "btn";\n');
        write(
          root,
          "src/ReviewProbe.tsx",
          'import { legacy } from "./probeClasses";\nexport const Probe = () => <div className={legacy} />;\n',
        );
        markMigrated(root, "src/ReviewProbe.tsx");
      });
      expectRejected(result, "migrated", "btn");
    });

    it("rejects a legacy class inside a primitive (born migrated)", () => {
      const result = runGuard((root) =>
        write(root, "src/components/ui/probe.tsx", 'export const Probe = () => <span className="btn" />;\n'),
      );
      expectRejected(result, "migrated", "btn");
    });
  });

  describe("rule 3 — no new stylesheet", () => {
    it("rejects a new .css file", () => {
      const result = runGuard((root) => write(root, "src/review.css", ".x {}\n"));
      expectRejected(result, "stylesheet", "src/review.css");
    });
  });

  describe("rule 4 — the prose layer stays scoped and token-only", () => {
    it("rejects a selector outside the root", () => {
      expectRejected(runGuard((root) => appendProse(root, "button { color: var(--foreground); }")), "prose");
    });

    it("rejects a sibling selector that escapes the root", () => {
      const result = runGuard((root) => appendProse(root, ".prose + button { color: var(--foreground); }"));
      expectRejected(result, "prose", "sibling combinator");
    });

    it("rejects a nested sibling selector that escapes the root", () => {
      const result = runGuard((root) => appendProse(root, ".prose { & ~ p { color: var(--foreground); } }"));
      expectRejected(result, "prose", "sibling combinator");
    });

    it("rejects a colour literal hidden in a var() fallback", () => {
      const result = runGuard((root) => appendProse(root, ".prose { color: var(--missing, #ff0000); }"));
      expectRejected(result, "prose", "#ff0000");
    });

    it("rejects a named colour", () => {
      const result = runGuard((root) => appendProse(root, ".prose a { color: rebeccapurple; }"));
      expectRejected(result, "prose", "rebeccapurple");
    });

    it("accepts sibling combinators inside the root", () => {
      const result = runGuard((root) => appendProse(root, ".prose p + p { margin-top: 0; }"));
      assert.equal(result.status, 0, result.output);
    });
  });

  describe("rule 5 — no raw colour in JSX", () => {
    it("rejects an arbitrary hex colour", () => {
      const result = runGuard((root) =>
        write(root, "src/RawProbe.tsx", 'export const Probe = () => <div className="bg-[#123456]" />;\n'),
      );
      expectRejected(result, "raw-colour", "bg-[#123456]");
    });

    it("rejects an arbitrary named colour", () => {
      const result = runGuard((root) =>
        write(root, "src/RawProbe.tsx", 'export const Probe = () => <div className="bg-[red]" />;\n'),
      );
      expectRejected(result, "raw-colour", "bg-[red]");
    });

    it("rejects an arbitrary colour property", () => {
      const result = runGuard((root) =>
        write(root, "src/RawProbe.tsx", 'export const Probe = () => <div className="[color:tomato]" />;\n'),
      );
      expectRejected(result, "raw-colour", "[color:tomato]");
    });

    it("rejects a palette utility", () => {
      const result = runGuard((root) =>
        write(root, "src/RawProbe.tsx", 'export const Probe = () => <div className="text-red-500" />;\n'),
      );
      expectRejected(result, "raw-colour", "text-red-500");
    });

    it("rejects a colour literal in an inline style", () => {
      const result = runGuard((root) =>
        write(root, "src/RawProbe.tsx", 'export const Probe = () => <div style={{ color: "#ff0000" }} />;\n'),
      );
      expectRejected(result, "raw-colour", "color: #ff0000");
    });

    it("rejects a named colour in an inline style held in a constant", () => {
      const result = runGuard((root) =>
        write(
          root,
          "src/RawProbe.tsx",
          'const look = { background: "white" };\nexport const Probe = () => <div style={look} />;\n',
        ),
      );
      expectRejected(result, "raw-colour", "background: white");
    });

    it("accepts token utilities and colour-free inline styles", () => {
      const result = runGuard((root) =>
        write(
          root,
          "src/RawProbe.tsx",
          [
            "export const Probe = () => (",
            '  <div className="bg-card text-muted-foreground bg-[color-mix(in_oklab,var(--primary)_10%,var(--popover))]"',
            '    style={{ display: "none", paddingLeft: 12 }} />',
            ");",
            "",
          ].join("\n"),
        ),
      );
      assert.equal(result.status, 0, result.output);
    });
  });
});
