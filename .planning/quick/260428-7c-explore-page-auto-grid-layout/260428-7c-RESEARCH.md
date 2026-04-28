# Quick Task: Explore Page Auto Grid Layout - Research

**Researched:** 2026-04-28
**Domain:** CSS Grid layout
**Confidence:** HIGH

## Summary

The My Skills page uses CSS Grid with `auto-fill` and `minmax()` to create a dynamic grid that adjusts columns based on available space. The Explore page currently uses a static `repeat(2, 1fr)` two-column grid. The change is purely CSS -- one line in `.explore-grid` needs to switch from fixed columns to the same `auto-fill`/`minmax()` pattern, with a doubled minimum width.

**Primary recommendation:** Change `.explore-grid` from `grid-template-columns: repeat(2, 1fr)` to `grid-template-columns: repeat(auto-fill, minmax(720px, 1fr))`.

## Current Implementation

### My Skills Auto Grid (reference pattern)

**CSS classes** (App.css lines 237-248): [VERIFIED: codebase]

```css
.skills-grid {
  display: grid !important;
  gap: 12px;
}

.skills-grid--auto-grid {
  grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
}

.skills-grid--dense-grid {
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
}
```

**JS logic** (SkillsList.tsx line 106-107): [VERIFIED: codebase]

```typescript
const gridClass =
  viewMode !== "list" ? `skills-grid skills-grid--${viewMode}` : "";
```

The `viewMode` state is managed in App.tsx and passed as a prop. The Explore page does NOT need the view mode dropdown -- only the auto-fill behavior.

### Explore Page Current Layout

**CSS** (App.css lines 2040-2045): [VERIFIED: codebase]

```css
.explore-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
  margin-bottom: 24px;
}
```

**JSX** (ExplorePage.tsx lines 125, 197): [VERIFIED: codebase]

```tsx
<div className="explore-grid">
  {filteredSkills.map((skill) => { ... })}
</div>
```

The `explore-grid` class is used in two places in ExplorePage.tsx: once for featured skills (line 125) and once for online search results (line 197). Both will inherit the CSS change automatically.

## Required Changes

### Single CSS change

In `src/App.css`, change the `.explore-grid` rule:

```css
/* FROM */
.explore-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
  margin-bottom: 24px;
}

/* TO */
.explore-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(720px, 1fr));
  gap: 10px;
  margin-bottom: 24px;
}
```

**720px** = 360px (My Skills auto-grid min) x 2 (user's "DOUBLE" requirement).

No JS changes needed. No new props, no view mode state, no component changes.

### Behavior at different widths

| Available width | Columns | Notes                                        |
| --------------- | ------- | -------------------------------------------- |
| < 720px         | 1       | Card stretches to fill                       |
| 720-1439px      | 1       | Single column, cards remain readable         |
| 1440-2159px     | 2       | Two columns (typical wide/ultrawide display) |
| 2160px+         | 3+      | Three or more columns                        |

Note: The app's content area (`.explore-scroll`) has 32px padding on each side, so the effective grid width is `window_width - 64px` minus any other chrome. In a typical 1200-1400px Tauri window, this will likely show 1 column. At 1600px+ it will show 2 columns. This is by design -- the doubled min-width prioritizes description readability.

## Gotchas

### Gap consistency

The explore grid uses `gap: 10px` while the skills grid uses `gap: 12px`. This is an existing difference and should be preserved unless the user wants them aligned.

### No interaction with view mode state

The Explore page has no `viewMode` prop and no view mode dropdown. The auto-grid behavior comes purely from CSS -- no JS wiring needed. The `viewMode` state in App.tsx only affects the My Skills page via SkillsList.tsx.

### Both grids in ExplorePage share the class

Both the featured skills grid and the online search results grid use `className="explore-grid"`. The CSS change applies to both simultaneously, which is the desired behavior.

## Sources

### Primary (HIGH confidence)

- `src/App.css` lines 237-248: My Skills grid CSS classes
- `src/App.css` lines 2040-2045: Explore page grid CSS
- `src/components/skills/SkillsList.tsx` lines 106-107: Grid class application logic
- `src/components/skills/ExplorePage.tsx` lines 125, 197: Explore grid JSX usage
