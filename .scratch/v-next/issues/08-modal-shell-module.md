# One `<Modal>` shell module for the ~9 copied modal skeletons

Status: resolved

Type: grilling

## Question

~9 modals repeat the same `.modal*` JSX skeleton by hand: `modal-backdrop` (click-to-close) → `modal` (stopPropagation, `role="dialog"`, `aria-modal`) → `modal-header` (title + ✕ close button) → `modal-body` → `modal-footer`, plus the `open`-gate + `memo` wrapper pattern. Scan finding 10: styling or fixing the shell means auditing every copy; ticket 04's merge shrank the surface by one but the pattern now has a natural home in `src/components/shared/`.

Decide, then implement (per map Notes):

- The shell's interface: likely `title`, `onRequestClose`, `children` (body), and a footer slot; does the `open`-gate/`memo` wrapper live in the shell or stay per-modal? Do variant modals (e.g. ones without footers, or with custom header content) fit one shell or argue for a narrower extraction?
- Which modals adopt it in this pass: the 7 in `src/components/skills/modals/`, the projects modals (`RemoveProjectModal`, `AddProjectModal`, `EditProjectModal`), and `shared/ToolConfigModal` — enumerate and check each actually matches the skeleton before converting.
- Styling stays global CSS (`.modal*` classes untouched) — this is a JSX dedupe, not a CSS architecture change (no CSS Modules, per scan).

Context: scan finding 10; ticket 04's Answer (established `components/shared/`). Independent of the Rust tickets and parallel-safe with 07; overlaps App.tsx only trivially — coordinate with ticket 05 if worked concurrently.

## Answer

Landed green on main as **`936a835`** (−112 net lines; frontend-only, zero visual change). Grilled one round (Q1–Q6), all recommendations accepted.

**Fact base**: 13 `.modal-backdrop` users — 11 modal components, App.tsx's inline update modal, and LoadingOverlay. Variant axes found: headerless (Delete/Remove), no-✕ (NewTools/SharedDir), close-disabled (AddSkill `canClose`), extra dialog classes (`modal-delete`, `modal-lg modal-discovered`, `update-modal`), body/footer class variants (`delete-body`, `space-between`), fully custom insides (update modal). 4 skills modals (AddSkill, GitPick, Import, LocalPick) were missing `role="dialog"`/`aria-modal`. No modal handled Escape. `.modal-actions`/`.modal-xl` CSS had zero users.

**Decisions:**

1. **Interface** — `src/components/shared/Modal.tsx`: `open`, `onRequestClose?` (backdrop + ✕; `undefined` = inert backdrop), `title?` (absent = headerless), `showCloseButton?` (default: title && onRequestClose), `closeDisabled?` (disables ✕ **and** backdrop — AddSkill's `canClose`), `className?`/`bodyClassName?`/`footerClassName?` (extra classes on dialog/body/footer), `footer?` slot, `children` = body. Plus **`plain`**: chrome-only mode (backdrop + dialog + gate, children verbatim, no header/body/footer wrappers) — used solely by the update modal, which keeps its custom `update-modal-*` insides byte-for-byte rather than being restyled.
2. **Shell owns the `open` gate** (`if (!open) return null`); unmounted children never execute, so mount semantics are preserved exactly. Per-modal `memo` stays; the shell itself is not memo'd (children identity defeats it). EditProjectModal and ToolConfigModal **keep their Inner components** — their hooks must not run while closed, so Inner renders `<Modal open>` and the outer wrapper keeps the gate (Edit's also guards `!project`). AddProjectModal's state deliberately persists across close/reopen (its gate sat after hooks) — preserved, since the component stays mounted and only Modal returns null.
3. **Adoption**: all 11 modal components + the update modal (via `plain`). **LoadingOverlay excluded** — shares only CSS classes; it's an overlay, not a dialog (no close affordance).
4. **a11y fix by construction**: shell renders `role="dialog"`/`aria-modal` unconditionally, silently fixing the 4 modals missing them. **No Escape-to-close added** — behavior parity; recorded as a possible v-next nicety.
5. **i18n**: shell calls `useTranslation()` internally for the ✕ aria-label (ProjectsPage precedent); callers keep passing `t` for their own content.
6. **Dead CSS deleted**: `.modal-actions`, `.modal-xl`. All other `.modal*` styling untouched — JSX dedupe, not a CSS architecture change.

**Gate**: `npm run version:check` exit 0; `npm run check` exit 0 (unmasked); `lens_diagnostics mode=all` — only App.tsx's pre-existing binder-by-design warnings.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
