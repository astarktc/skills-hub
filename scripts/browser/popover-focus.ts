// Browser regression for the Popover primitive's focus contract (BACKLOG #97), driven with trusted keys
// (`cdp-keys.ts`) against the fixture — what jsdom cannot show, since it moves no focus on Tab and runs none of
// Base UI's focus guards.
//
// Why it exists: `ui/popover.tsx` leans on Base UI behaviour that is not public API — the `data-tabindex`
// bookkeeping it undoes when focus returns from a layer that is gone (`restoreTabOrder`, ticket 16 A1), and the focus
// guards a triggerless popover's Tab-out steps around. `@base-ui/react` is pinned exactly in `package.json` for that
// reason; **run this before bumping the pin** (and after any change to `popover.tsx` or `layer-stack.ts`).
//
// Flows:
//   A. The row switch's inline Enable confirmation (a Popover without a Trigger, `PopoverContent anchor`):
//      Tab past Enable closes it and lands on the row's next control — never an unnamed focus guard; Shift+Tab before
//      Not now closes it and lands on the switch; Escape closes it and returns to the switch.
//   B. Add's harness scope popover → the shared-folder ask → Continue: focus returns to the checkbox that raised the
//      ask with the popover still open, and Tab moves on to the next checkbox inside it; Escape then closes the popover
//      alone and returns to its trigger.
//
// Usage (Bun or Node ≥ 22; `npm run dev:fixture` running; agent-browser installed):
//
//   agent-browser --session fixture-audit open "http://localhost:5175/?scenario=rich"
//   bun scripts/browser/popover-focus.ts
//
// Environment: `AB_SESSION` / `FIXTURE_ORIGIN` as `cdp-keys.ts` reads them. Exits non-zero on the first broken step.
import { agentBrowser, closeKeys, press } from "./cdp-keys.ts";

const ORIGIN = process.env.FIXTURE_ORIGIN ?? "http://localhost:5175";

/** A page expression's value as the checks use it: every expression here yields a primitive. */
type Evaluated = string | number | boolean | null | undefined;
/** Evaluate `js` in the page. */
function evaluate(js: string): Evaluated {
  const result = agentBrowser("eval", js).result;
  return result === null || ["string", "number", "boolean", "undefined"].includes(typeof result)
    ? (result as Evaluated)
    : JSON.stringify(result);
}
const settle = (ms = 300) => new Promise((resolve) => setTimeout(resolve, ms));

/** The focused element as the checks read it: its name, role, and whether it is hidden from AT / a focus guard. */
type Focused = { name: string; role: string; hidden: boolean; guard: boolean };
// The name: `aria-label`, else the `<label>`/field text that names it, else its own text.
const NAME_OF = `(a) => (a?.getAttribute("aria-label") ?? (a?.labels?.[0] ?? a?.closest("label, [data-slot=field]"))?.textContent ?? a?.textContent ?? "").trim().slice(0, 60)`;
function focused(): Focused {
  const raw = evaluate(`JSON.stringify((() => {
    const a = document.activeElement;
    return {
      name: (${NAME_OF})(a),
      role: a?.getAttribute("role") ?? a?.tagName.toLowerCase() ?? "",
      hidden: !!a?.closest("[aria-hidden=true]"),
      guard: !!a?.hasAttribute("data-base-ui-focus-guard"),
    };
  })())`);
  try {
    return JSON.parse(String(raw)) as Focused;
  } catch {
    return { name: String(raw), role: "?", hidden: false, guard: false };
  }
}
/** Focus the first element matching `selector` whose name is `name`. */
const focusNamed = (selector: string, name: string) =>
  evaluate(`[...document.querySelectorAll(${JSON.stringify(selector)})].find((el) => (${NAME_OF})(el) === ${JSON.stringify(name)})?.focus()`);
const clickNamed = (selector: string, name: string) =>
  evaluate(`[...document.querySelectorAll(${JSON.stringify(selector)})].find((el) => (${NAME_OF})(el) === ${JSON.stringify(name)})?.click()`);
const popoverOpen = () => evaluate(`!!document.querySelector("[data-slot=popover-content]")`) === true;

let step = 0;
function check(label: string, ok: boolean, detail: unknown) {
  step += 1;
  console.log(`${ok ? "ok  " : "FAIL"} ${step}. ${label}${ok ? "" : ` — got ${JSON.stringify(detail)}`}`);
  if (!ok) {
    closeKeys();
    process.exit(1);
  }
}
function expectFocus(label: string, name: string) {
  const now = focused();
  check(label, now.name === name && !now.hidden && !now.guard, now);
}

async function open(url: string) {
  agentBrowser("open", url);
  agentBrowser("wait", "1200");
}

// --- A. A Popover without a Trigger ------------------------------------------------------------------------------
await open(`${ORIGIN}/?scenario=rich`);
const SKILL = "api-contract-review";
const SWITCH = `${SKILL} enabled`;
// Turn the skill off (a skill-grain Disable), so its switch raises the inline Enable confirmation.
evaluate(`document.querySelector('[aria-label="${SWITCH}"]').click()`);
await settle(800);
check("the skill is off", evaluate(`document.querySelector('[aria-label="${SWITCH}"]').getAttribute("aria-checked")`) === "false", null);

async function raiseInline() {
  evaluate(`document.querySelector('[aria-label="${SWITCH}"]').focus()`);
  await press("Space");
  await settle();
  check("the switch raises its inline confirmation, focus on Enable", popoverOpen() && focused().name === "Enable", focused());
}

await raiseInline();
await press("Tab");
await settle();
check("Tab past Enable closes the confirmation", !popoverOpen(), null);
expectFocus("…and lands on the row's next control, not a focus guard", `More actions for ${SKILL}`);

await raiseInline();
await press("Shift+Tab");
await settle(150);
check("Shift+Tab stays inside (Enable → Not now)", popoverOpen() && focused().name === "Not now", focused());
await press("Shift+Tab");
await settle();
check("Shift+Tab before Not now closes the confirmation", !popoverOpen(), null);
expectFocus("…and lands on the switch", SWITCH);

await raiseInline();
await press("Escape");
await settle();
check("Escape closes the confirmation", !popoverOpen(), null);
expectFocus("…and returns to the switch", SWITCH);

// --- B. Focus returned from a layer that is gone -----------------------------------------------------------------
await open(`${ORIGIN}/?scenario=rich`);
clickNamed("button", "Add skill");
await settle(600);
clickNamed("[role=dialog] button", "Edit harnesses");
await settle(500);
check("the harness scope popover is open", popoverOpen(), null);
focusNamed("[data-slot=popover-content] [role=checkbox]", "Amp");
expectFocus("focus on the Amp checkbox (a shared skills folder)", "Amp");
await press("Space");
await settle(500);
check("unchecking Amp raises the shared-folder ask", focused().name === "Continue", focused());
await press("Enter");
await settle(500);
check("the ask is gone and the popover stayed open", popoverOpen(), null);
expectFocus("…focus returned to the checkbox that raised it", "Amp");
await press("Tab");
await settle(200);
check("Tab moves on inside the popover (its controls are back in the tab order)", popoverOpen() && focused().role === "checkbox" && focused().name !== "Amp", focused());
await press("Escape");
await settle();
check("Escape closes the popover alone", !popoverOpen() && evaluate(`!!document.querySelector("[role=dialog]")`) === true, null);
expectFocus("…and returns to its trigger", "Edit harnesses");

closeKeys();
console.log(`popover-focus: ${step} checks passed`);
