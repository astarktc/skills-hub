# Model Routing Gaps in Auto-Mode: No Per-Unit Thinking Level, No Review Bucket, Subagent Model Overrides Limited

## Problem

GSD v2's model selection system is impressively sophisticated for _which model_ to use per unit type, but has structural gaps in three dimensions:

1. **No reasoning effort / thinking level configurability per unit type** -- it's session-global
2. **No dedicated model bucket for review/verification work** -- reviews happen as subagent dispatches embedded inside completion/validation units
3. **Subagent model routing is a soft prompt instruction, not a runtime gate** -- and frontmatter `model: sonnet` on 10/13 agent definitions means the `models.subagent` preference is partially dead weight

These gaps prevent users from expressing common cost/quality tradeoffs like:

- "Opus with high thinking for planning, Sonnet with low thinking for execution"
- "Use a different model for code review than for code generation"
- "Don't use Sonnet for security review just because the agent definition says so"

---

## Gap 1: Thinking Level is Session-Global

### Current behavior

The thinking level (reasoning effort) is captured once at auto-mode start:

```javascript
// auto-start.js:286
const startThinkingSnapshot = pi.getThinkingLevel();

// auto-start.js:704
s.autoModeStartThinkingLevel = startThinkingSnapshot ?? null;
```

It is then re-applied uniformly after every model swap:

```javascript
// auto-model-selection.js:85-88
function reapplyThinkingLevel(pi, level) {
  if (!level) return;
  pi.setThinkingLevel(level);
}
```

Called at lines 410, 479, and 485 of `auto-model-selection.js` -- always with the same `autoModeStartThinkingLevel` value, regardless of unit type.

### What's missing

There is no `thinking_level` or `reasoning_effort` field anywhere in:

- `KNOWN_PREFERENCE_KEYS` (preferences-types.js)
- The preferences validation logic (preferences-validation.js)
- The prefs wizard (commands-prefs-wizard.js)
- The PREFERENCES.md schema documentation

The preference system has no mechanism to express "high thinking for planning units, low thinking for execution units."

### Impact

Users paying for reasoning tokens (o-series, extended thinking) can't optimize spend by reducing effort on mechanical tasks (copy file, run command) while maintaining high effort on architectural decisions.

---

## Gap 2: No Review/Verification Model Bucket

### Current behavior

The `models:` preferences block has these buckets (from `preferences-models.js:36-93`):

- `research` -- research-milestone, research-slice
- `planning` -- plan-milestone, plan-slice, refine-slice, replan-slice
- `discuss` -- discuss-milestone, discuss-slice (falls back to planning)
- `execution` -- execute-task, reactive-execute
- `execution_simple` -- execute-task-simple (falls back to execution)
- `completion` -- complete-slice, complete-milestone, worktree-merge, run-uat
- `validation` -- reassess-roadmap, rewrite-docs, gate-evaluate, validate-milestone (falls back to planning)
- `subagent` -- subagent, subagent/\*

Code review happens in two places:

1. **complete-slice** dispatches `reviewer`, `security`, `tester` subagents (unit-context-manifest.js:289-299)
2. **validate-milestone** dispatches the same subagents (unit-context-manifest.js:181-198)

Both route through the `completion` or `validation` model bucket for the _orchestrating unit_, and whatever model the reviewer/security/tester subagent uses is determined by either:

- The agent's frontmatter `model: sonnet` (hard default), OR
- The `models.subagent` preference (if the LLM follows the system prompt instruction to pass it)

### What's missing

There's no `review` model bucket. You can't say "use Opus for reviews" without also changing the model for all completion/validation work. The post_unit_hooks feature (preferences-reference.md line 251) allows attaching a `model:` override to a hook, which is the closest escape hatch, but:

- It requires writing a full hook definition
- It only fires _after_ a unit, not for the review subagents dispatched _during_ complete-slice/validate-milestone
- It's a workaround, not a first-class routing path

### Impact

Review is arguably the highest-leverage place for a capable model -- catching bugs in generated code. But the architecture routes it through the cheapest buckets (completion = light tier in budget/balanced profiles).

---

## Gap 3: Subagent Model Frontmatter vs Preferences

### Current behavior

10 of 13 agents in `~/.gsd/agent/agents/` hardcode `model: sonnet` in frontmatter:

- debugger, doc-writer, git-ops, javascript-pro, planner, refactorer, reviewer, security, tester, typescript-pro

Only 3 agents lack a frontmatter model: researcher, scout, worker.

The subagent tool processes models with this precedence (subagent/index.js:198-202):

```javascript
export function buildSubagentProcessArgs(agent, task, tmpPromptPath, modelOverride) {
    const args = ["--mode", "json", "-p", "--no-session"];
    const effectiveModel = modelOverride ?? agent.model;
    if (effectiveModel) args.push("--model", effectiveModel);
    ...
}
```

The `modelOverride` comes from the tool call's `model` parameter. To get this set, GSD injects a system prompt instruction (bootstrap/system-context.js:186-188):

```javascript
const subagentModelConfig = resolveModelWithFallbacksForUnit("subagent");
const subagentModelBlock = subagentModelConfig
  ? `\n\n## Subagent Model\n\nWhen spawning subagents via the \`subagent\` tool, always pass \`model: "${subagentModelConfig.primary}"\` in the tool call parameters. Never omit this — always specify it explicitly.`
  : "";
```

### The problem

This is a _prompt-based_ enforcement mechanism. It asks the orchestrating LLM to pass the model parameter. But:

1. If the LLM doesn't follow the instruction (or the prompt is compacted away), the frontmatter `model: sonnet` becomes the effective model for 10/13 agents.
2. Even when the instruction is followed, it's a single value for ALL subagent types. You can't configure "Opus for reviewer, Haiku for scout" through preferences alone.
3. The `models.subagent` preference key is technically functional, but its effectiveness depends on LLM compliance rather than runtime enforcement.

### Impact

- The `models.subagent` preference gives users a false sense of control -- it works when the LLM complies, silently falls back to `sonnet` when it doesn't.
- No differentiation between high-stakes subagents (reviewer, security) and low-stakes ones (scout, doc-writer).

---

## Gap 4: No Orchestrator/Subagent Model Split

This is a consequence of gaps 2 and 3 combined. The orchestrator IS the session model. Subagents either use their frontmatter model or the single `models.subagent` bucket. You cannot express:

- "Opus orchestrates, GPT-5.5 executes, Haiku scouts"
- "The reviewer subagent should use a different model than the execute-task subagent"

---

## Possible Solutions (not prescriptive)

These are directions, not a spec -- the architecture is well-designed enough that the right approach will emerge from understanding the constraints.

### For thinking level

A `thinking:` preference block parallel to `models:`, mapping unit types to thinking levels:

```yaml
thinking:
  planning: high
  execution: low
  validation: medium
```

The `reapplyThinkingLevel` function in auto-model-selection.js already has the unit type available -- it just needs a per-unit-type lookup instead of always using the session-level snapshot.

### For review model bucket

Add a `review` key to the models map and route `complete-slice` and `validate-milestone` subagent dispatches through it. This might mean:

- A new case in `resolveModelWithFallbacksForUnit` for unit types like `"subagent/reviewer"`, `"subagent/security"`, `"subagent/tester"`
- OR a more general `models.subagent.<agent-name>` syntax

### For subagent model enforcement

Consider a runtime hook in the subagent extension that intercepts the tool call and injects the model override from preferences _before_ process spawn, rather than relying on the LLM to pass it. This would make `models.subagent` a hard gate rather than a soft instruction.

The existing `register-hooks.js` infrastructure and the `before_model_select` hook pattern could serve as a template.

---

## Files Referenced

| File                                                              | Role                                             |
| ----------------------------------------------------------------- | ------------------------------------------------ |
| `~/.gsd/agent/extensions/gsd/auto-model-selection.js`             | Model selection + thinking level reapplication   |
| `~/.gsd/agent/extensions/gsd/preferences-models.js`               | Model bucket definitions, unit type mapping      |
| `~/.gsd/agent/extensions/gsd/preferences-types.js`                | KNOWN_PREFERENCE_KEYS (no thinking field)        |
| `~/.gsd/agent/extensions/gsd/preferences-validation.js`           | Validation (no thinking field)                   |
| `~/.gsd/agent/extensions/gsd/model-router.js`                     | Complexity classification, capability scoring    |
| `~/.gsd/agent/extensions/gsd/bootstrap/system-context.js:186-188` | Soft subagent model injection                    |
| `~/.gsd/agent/extensions/subagent/index.js:198-202`               | Model override precedence logic                  |
| `~/.gsd/agent/extensions/gsd/unit-context-manifest.js:84-86`      | Review subagent allowlist                        |
| `~/.gsd/agent/extensions/gsd/auto-start.js:286,704`               | Thinking level capture                           |
| `~/.gsd/agent/extensions/gsd/auto/session.js:84`                  | Session state field                              |
| `~/.gsd/agent/agents/*.md`                                        | Agent frontmatter with hardcoded `model: sonnet` |
| `~/.gsd/agent/extensions/gsd/docs/preferences-reference.md`       | User-facing docs (no thinking field)             |
| `~/.gsd/agent/extensions/gsd/commands-prefs-wizard.js:561-669`    | Model phase wizard (no thinking config)          |

---

## Summary

The model routing system is architecturally clean and handles multi-provider complexity well. These gaps are in the _expressiveness_ layer -- users can't describe common workflows that involve heterogeneous reasoning effort, review-specific models, or fine-grained subagent control. The infrastructure to support these features largely exists (per-unit-type dispatch, hook system, capability scoring); what's missing is the preference schema and the wiring to connect user intent to runtime behavior.
