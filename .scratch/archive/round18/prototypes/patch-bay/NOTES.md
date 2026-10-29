# The Patch Bay — notes (seed 4a508bda)

## Decisions where the world was silent
- **Cord = violet** `#a594ff` dark / `#5a44dc` light. The one hue not already spoken for: LEDs own green/amber/red, and the incumbent is blue (#2563eb/#38bdf8). Violet reads as "a patch cable" rather than "a link".
- **No per-row LED.** The jack strip *is* the row's LED row: patched = cord, a stale skill's patched jacks = amber, faulted = red, unpatched = ash ring. A separate status dot would have doubled the signal. Green appears only in panel summaries and inspector health.
- **Cord uses:** primary action, selected-row tint (13% mix), focus ring and checkbox. The switch, tab underline and Deploy all are neutral. The *open* row uses the neutral `lift` colour, so "open" never reads as "selected".
- **Legend** lives in the foot and in the inspector hintline, next to the jacks it explains.
- **Bypassed rows** show ash jacks plus "—" for count and scope; the switch is the only control on the row that moves.
- **Motion is stricter than asked.** The bulk rail, bypass knob and 960 sheet appear instantly, via visibility. Selecting, toggling and opening are frequent actions, and the world says nothing else moves. Only the jack opacity fade remains (100ms, ease-out, off under reduced-motion).
- **960 collapse:** the inspector becomes a fixed 440px sheet over the rack. It is the only structural change; the description column shrinks to fit.
- **Default open skill:** zod-schema-first. It is faulted, so the first screen shows the TARGET_EXISTS recovery.

## What felt wrong while building
- **Colour-blindness:** patched vs faulted jacks differ by hue only at 8px. The count text ("Synced 6/7") and the inspector carry the fault in words, but the row itself does not.
- **Stale as amber jacks** overloads the jack: it mixes "where it is deployed" with "how fresh it is".
- **Tight 7-column target grid** in the 440px sheet: "Claude Code" / "Gemini CLI" wrap to two lines.
- **Light-theme status colours:** ok (3.85:1) and stale (3.44:1) pass only as non-text (dots and jacks). They are never used as text in light.
- **Button hairlines:** `rule-strong` on `face` is 1.55:1 dark / 1.71:1 light. Buttons are identified by their label, but this is a possible 1.4.11 finding.

## What I'd change
- Add a shape cue to faulted jacks (notch or ×) and move stale to a jack *ring* rather than a fill.
- Switch the sheet's target grid to 4+3 or to a list below 1199px.

## Process and caveats
- **Fixture count:** the fixture sums to 60 (28+9+12+6+5), not the brief's 61. I kept it internally consistent rather than pad it.
- **Serving:** the preview could not load the `file://` path, so the captures were served from `python3 -m http.server`. The file itself still opens by double-click.
- **Tailwind:** the browser CDN is a network script, not a font. Offline, the page loses its utilities; tokens and components are plain CSS and survive.
- **innerHTML:** the 24 hook findings are false positives. The fixture is static and every value passes through `esc()`.
- **960 capture:** the first one raced the resize (blank sheet and headers). I retook it after a forced reflow plus two frames; DOM checks gave width 960, no horizontal scroll, sheet visible.
- **Keyboard verified at 960:** j/k/↑↓ move a 2px cord ring; x selects and raises the bulk rail; Enter opens the sheet and focuses it; Esc returns focus to the row.
- **Detector:** 1 finding (placeholder with no inset) and 1 advisory (menu hairline plus 24px shadow). Both are fixed; `detect --json` now returns `[]`.

## Tokens
Dark: ground #0f1114 · face #171a1f · hover #1d2127 · lift #232830 · rule #262b33 · rule-strong #353b45 · ink #e3e6eb · ink-2 #9ba2ad · label #8c939e · ash #666d78 · cord #a594ff (cord-ink #0f1114) · ok #3ddc84 · stale #f5b400 · fault #ff5d5d
Light: ground #d8dce1 · face #eef0f3 · hover #e4e7eb · lift #dde1e6 · rule #cdd2d8 · rule-strong #b3bac3 · ink #15181c · ink-2 #4a515b · label #555c66 · ash #7d848e · cord #5a44dc (cord-ink #fff) · ok #188a4a · stale #a87800 · fault #c62f2f
Type: system-ui plus ui-monospace; label 11/16 caps +0.08em · meta 12/16 · body 13/20 · title 15/20 (ratio ≤1.16); tabular figures for counts.
Space: 4px grid · rail 48 · filter bar 40 · column/panel headers 28/36 · rows 36 (1U) · foot 28 · inspector 480 (sheet 440 below 1199px).
Radius 2px on controls, 50% on jacks. Motion: jack opacity 100ms cubic-bezier(0.23,1,0.32,1); everything else instant.
