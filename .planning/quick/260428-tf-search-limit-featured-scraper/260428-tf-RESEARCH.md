# Quick Task 260428-tf: Featured Skills Scraper v2 - Research

**Researched:** 2026-04-28
**Domain:** Web scraping (skills.sh leaderboard), Playwright headless browser
**Confidence:** HIGH

## Summary

The skills.sh leaderboard is a Next.js RSC (React Server Components) application hosted on Vercel. It renders skill rankings across three views: All Time (`/`), Trending 24h (`/trending`), and Hot (`/hot`). The page uses virtual scrolling to display up to 600 skills per view, but only renders ~31 DOM nodes at a time.

**Critical discovery:** All skill data is embedded in the initial HTML response as RSC payloads inside `<script>` tags. Each page's `self.__next_f.push([1, "..."])` call contains a JSON object with an `initialSkills` array of up to 600 entries. This data is accessible via a simple `page.content()` + regex extraction -- no scrolling, clicking expand buttons, or DOM traversal needed.

**Primary recommendation:** Use Playwright to load the page and extract the RSC `initialSkills` payload from script tags. This bypasses virtual scrolling entirely, is faster, and captures all 600 skills per view in a single extraction.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Use headless browser (Playwright) to render skills.sh pages and extract data via DOM queries
- Purely scripted -- no LLM integration, just standard DOM querying of rendered pages
- Playwright added as a dev dependency
- Produce **separate lists**: `featured` from All Time tab, `trending` from Hot/Trending tab
- Two distinct arrays in the output JSON
- Frontend can later show tabs or sections (but frontend changes are NOT in scope)
- Expand all "+N more from repo" collapsed entries to get full coverage
- Click each collapse button and wait for content to appear before extracting

### Claude's Discretion

- Output JSON schema: maintain compatibility with existing `featured-skills.json` shape where possible, add a `trending` key alongside `skills`
- Max entries: keep 300 for featured (All Time), ~100 for trending

### Out of Scope

- Frontend changes (no new tabs/sections in UI)
- Chinese i18n strings

</user_constraints>

## Key Finding: RSC Payload Extraction (Eliminates Virtual Scroll Complexity)

The CONTEXT.md says "Expand all +N more collapsed entries" and "Click each collapse button." However, research reveals this complexity is unnecessary because the full data exists in RSC script tags embedded in the server-rendered HTML.

**What the RSC payload contains per page:**

| Page     | URL         | View Name  | Skills Count | Extra Fields                                                           |
| -------- | ----------- | ---------- | ------------ | ---------------------------------------------------------------------- |
| All Time | `/`         | `all-time` | 600          | `source`, `skillId`, `name`, `installs`                                |
| Trending | `/trending` | `trending` | 600          | `source`, `skillId`, `name`, `installs`                                |
| Hot      | `/hot`      | `hot`      | 600          | `source`, `skillId`, `name`, `installs`, `installsYesterday`, `change` |

**RSC payload structure (per page):**

```json
{
  "initialSkills": [
    { "source": "vercel-labs/skills", "skillId": "find-skills", "name": "find-skills", "installs": 1242339 },
    ...
  ],
  "totalSkills": 91009,
  "allTimeTotal": 91009,
  "view": "all-time"
}
```

**Extraction method:**

```javascript
// RSC data is in script tags as: self.__next_f.push([1, "...escaped JSON..."])
const html = await page.content();
const match = html.match(/self\.__next_f\.push\(\[1,"(.+?)"\]\)<\/script>/);
// Unescape and parse to find the initialSkills array
```

[VERIFIED: live skills.sh page scraping via agent-browser, 2026-04-28]

## skills.sh Page DOM Structure

### Page Layout

- **Header:** `<header>` with nav links (Official, Audits, Docs)
- **Hero section:** ASCII art banner, "Try it now" install command, agent logos
- **Leaderboard section:** `<main class="py-6">` containing:
  - `<h2>Skills Leaderboard</h2>`
  - Tab bar: `<div class="flex gap-4 mb-4 font-mono">` with `<a>` links
  - Content: `<div class="relative min-h-[400px]">` containing virtual scroll container

### Tab Navigation (Links, Not Buttons)

| Tab      | URL         | Text Example        |
| -------- | ----------- | ------------------- |
| All Time | `/`         | "All Time (91,009)" |
| Trending | `/trending` | "Trending (24h)"    |
| Hot      | `/hot`      | "Hot"               |

Tabs are `<a>` tags, not JavaScript tabs. Each is a full page navigation. [VERIFIED: DOM inspection]

### Virtual Scrolling Container

- Container: `<div style="height: 26156px; position: relative;">`
- Each item: `<div style="position: absolute; height: 52px; transform: translateY(Npx);">`
- Only ~31 items rendered in DOM at any time (viewport window)
- Total items = container height / 52px (e.g., 26156/52 = ~503 visible items, but RSC has 600)

### Skill Entry HTML Structure (All Time / Trending)

```html
<div
  style="position: absolute; top: 0px; left: 0px; width: 100%; height: 52px; transform: translateY(0px);"
>
  <a
    class="group grid grid-cols-[auto_1fr_auto] lg:grid-cols-16 ..."
    href="/vercel-labs/skills/find-skills"
  >
    <div class="lg:col-span-1 text-left">
      <span class="... font-mono">1</span>
      <!-- rank -->
    </div>
    <div class="lg:col-span-13 min-w-1 flex flex-col ...">
      <h3 class="font-semibold ...">find-skills</h3>
      <!-- skill name -->
      <p class="... font-mono ...">vercel-labs/skills</p>
      <!-- source repo -->
    </div>
    <div class="lg:col-span-2 text-right">
      <span class="font-mono text-sm ...">1.2M</span>
      <!-- installs (formatted) -->
    </div>
  </a>
</div>
```

### "+N More" Collapsed Entry Structure

```html
<div style="...">
  <div
    role="button"
    tabindex="0"
    class="grid grid-cols-[auto_1fr_auto] lg:grid-cols-16 ..."
  >
    <div class="lg:col-span-1"></div>
    <div class="lg:col-span-13 min-w-1 flex items-center gap-1.5 ...">
      <span>+19 more from</span>
      <span>microsoft/azure-skills</span>
      <span>(5.1M total)</span>
      <svg><!-- chevron down --></svg>
    </div>
  </div>
</div>
```

- These are `div[role="button"]` elements (NOT `<button>`)
- Clicking expands to show individual skills, increases container height
- After expand: button becomes `<button>` with "Collapse {repo}" text
- NOT needed if using RSC extraction (all skills are already in the payload)

### Link/URL Pattern

- Skill detail: `/{owner}/{repo}/{skillId}` (e.g., `/vercel-labs/skills/find-skills`)
- Construct GitHub source: `https://github.com/{source}/tree/main/skills/{skillId}` or `https://github.com/{source}/tree/main/{skillId}`

[VERIFIED: DOM inspection via agent-browser]

### Hot Page Differences

- Uses `lg:col-span-11` for skill info (vs col-span-13 on All Time)
- Uses `lg:col-span-4` for installs + growth
- Shows both total installs and delta: `<span>259</span><span class="text-green-500">+257</span>`

## Playwright Usage for mjs Scripts

### Installation

```bash
npm install --save-dev playwright
npx playwright install chromium
```

[VERIFIED: npm registry -- playwright@1.59.1]

### Script Pattern (Library Mode, Not Test Runner)

```javascript
import { chromium } from "playwright";

const browser = await chromium.launch();
const page = await browser.newPage();

await page.goto("https://skills.sh/");
await page.waitForLoadState("domcontentloaded");

const html = await page.content();
// Extract RSC payload from script tags

await browser.close();
```

[CITED: github.com/microsoft/playwright/blob/main/docs/src/library-js.md]

### Key APIs

| API                                         | Purpose                                           |
| ------------------------------------------- | ------------------------------------------------- |
| `chromium.launch()`                         | Launch headless Chromium                          |
| `page.goto(url)`                            | Navigate to URL                                   |
| `page.waitForLoadState('domcontentloaded')` | Wait for HTML parsed (RSC data is in script tags) |
| `page.content()`                            | Get full page HTML                                |
| `page.evaluate(() => ...)`                  | Run JS in browser context                         |
| `browser.close()`                           | Clean shutdown                                    |

### Wait Strategy

For RSC extraction, `domcontentloaded` is sufficient since the `initialSkills` data is embedded in `<script>` tags that are part of the server-rendered HTML. No need to wait for `networkidle` or `load` -- the data is present as soon as the HTML is parsed.

## Output Schema Compatibility

### Current `featured-skills.json` Shape

```json
{
  "updated_at": "2026-04-07T01:07:23.235Z",
  "total": 300,
  "categories": ["ai-assistant", "development"],
  "skills": [
    {
      "slug": "algorithmic-art",
      "name": "algorithmic-art",
      "summary": "Creating algorithmic art...",
      "downloads": 0,
      "stars": 111625,
      "category": "ai-assistant",
      "tags": ["agent-skills"],
      "source_url": "https://github.com/anthropics/skills/tree/main/skills/algorithmic-art",
      "updated_at": "2026-04-07T01:05:21Z"
    }
  ]
}
```

### Field Mapping from Leaderboard Data

| Output Field | Leaderboard Source               | Notes                                                     |
| ------------ | -------------------------------- | --------------------------------------------------------- |
| `slug`       | `skillId`                        | Direct mapping                                            |
| `name`       | `name`                           | Direct mapping (same as skillId on leaderboard)           |
| `summary`    | --                               | **Not available** from leaderboard. Use empty string `""` |
| `downloads`  | `installs`                       | Direct mapping (exact count, not formatted)               |
| `stars`      | --                               | **Not available**. Use `0`                                |
| `category`   | --                               | **Not available**. Use `"general"`                        |
| `tags`       | --                               | **Not available**. Use `[]`                               |
| `source_url` | Derive from `source` + `skillId` | `https://github.com/{source}` (repo URL)                  |
| `updated_at` | --                               | Use script run timestamp                                  |

### Proposed v2 Output Schema

```json
{
  "updated_at": "2026-04-28T...",
  "total": 300,
  "categories": ["general"],
  "skills": [
    {
      "slug": "find-skills",
      "name": "find-skills",
      "summary": "",
      "downloads": 1242339,
      "stars": 0,
      "category": "general",
      "tags": [],
      "source_url": "https://github.com/vercel-labs/skills",
      "updated_at": "2026-04-28T..."
    }
  ],
  "trending": [
    {
      "slug": "ai-image-generation",
      "name": "ai-image-generation",
      "summary": "",
      "downloads": 28229,
      "stars": 0,
      "category": "general",
      "tags": [],
      "source_url": "https://github.com/infsh-skills/skills",
      "updated_at": "2026-04-28T..."
    }
  ]
}
```

**Key changes from v1:**

- `downloads` now contains real install counts (v1 always had `0`)
- `stars` is `0` (not available from leaderboard; v1 used GitHub API stars)
- `summary` is empty (v1 pulled from SKILL.md frontmatter via GitHub API)
- New `trending` array alongside existing `skills` array

### Source URL Construction

The leaderboard provides `source` as `owner/repo`. The `source_url` can be:

- **Repo URL:** `https://github.com/{source}` (simplest, always correct)
- **Skill path URL:** `https://github.com/{source}/tree/main/skills/{skillId}` (may not be accurate for all repos -- some repos have different directory structures)

**Recommendation:** Use repo URL (`https://github.com/{source}`) since we cannot determine the exact directory path from leaderboard data alone. [ASSUMED]

## Common Pitfalls

### Pitfall 1: Virtual Scroll DOM Extraction

**What goes wrong:** Attempting to extract all skills by querying DOM elements fails because virtual scrolling only renders ~31 items at a time.
**Why it happens:** The leaderboard uses a virtual scroll container.
**How to avoid:** Use RSC payload extraction from script tags instead of DOM traversal.
**Warning signs:** Getting only ~30-60 skills when expecting 300+.

### Pitfall 2: RSC Payload Escaping

**What goes wrong:** The RSC data in script tags is double-escaped JSON inside a string literal.
**Why it happens:** `self.__next_f.push([1, "escaped_string"])` format requires unescaping.
**How to avoid:** Parse the outer string first (`JSON.parse('"' + match + '"')`), then find and parse the JSON object within.
**Warning signs:** JSON parse errors on the first attempt.

### Pitfall 3: Formatted vs Raw Install Counts

**What goes wrong:** DOM shows formatted values like "1.2M", "356.6K" that need parsing.
**Why it happens:** The rendered DOM formats numbers for display.
**How to avoid:** RSC payload has exact integer values (1242339, 356554) -- no parsing needed.

### Pitfall 4: Playwright Browser Install

**What goes wrong:** Script fails with "browser not installed" error on first run or CI.
**Why it happens:** Playwright requires a separate browser binary download step.
**How to avoid:** Add `npx playwright install chromium` to package.json scripts or document in README. Consider `npx playwright install --with-deps chromium` for CI.

### Pitfall 5: Stale/Renamed Skills on Leaderboard

**What goes wrong:** Some skills on the leaderboard may reference repos that have been renamed or deleted.
**Why it happens:** skills.sh caches install data; repos can change after initial tracking.
**How to avoid:** Don't try to validate every source URL. Just pass through the data as-is.
**Context:** See memory `project_skillsh_stale_data_investigation.md` for full investigation.

## RSC Extraction Algorithm

Verified step-by-step approach for extracting skills from any skills.sh page:

```javascript
async function extractSkillsFromPage(page, url) {
  await page.goto(url);
  await page.waitForLoadState("domcontentloaded");

  const html = await page.content();

  // Find the RSC push containing initialSkills
  // Pattern: self.__next_f.push([1,"...initialSkills..."])
  const pushRegex = /self\.__next_f\.push\(\[1,"((?:[^"\\]|\\.)*)"\]\)/g;
  let match;
  while ((match = pushRegex.exec(html)) !== null) {
    if (match[1].includes("initialSkills")) {
      // Unescape the string content
      const unescaped = JSON.parse('"' + match[1] + '"');

      // Find the JSON object containing initialSkills
      const jsonStart = unescaped.indexOf('{"initialSkills"');
      if (jsonStart === -1) continue;

      // Find matching closing brace
      let depth = 0,
        jsonEnd = -1;
      for (let i = jsonStart; i < unescaped.length; i++) {
        if (unescaped[i] === "{") depth++;
        if (unescaped[i] === "}") {
          depth--;
          if (depth === 0) {
            jsonEnd = i + 1;
            break;
          }
        }
      }

      const data = JSON.parse(unescaped.substring(jsonStart, jsonEnd));
      return data.initialSkills; // Array of { source, skillId, name, installs }
    }
  }
  throw new Error(`No initialSkills found in ${url}`);
}
```

[VERIFIED: tested extraction logic via agent-browser eval on all three pages]

## Don't Hand-Roll

| Problem                  | Don't Build                    | Use Instead            | Why                                                           |
| ------------------------ | ------------------------------ | ---------------------- | ------------------------------------------------------------- |
| Browser automation       | Custom fetch + JSDOM           | Playwright `chromium`  | RSC rendering needs real browser JS execution for reliability |
| Number formatting parse  | Parse "1.2M" strings           | RSC raw integers       | RSC payload has exact integers, no formatting                 |
| Virtual scroll traversal | Scroll + extract + dedupe loop | RSC payload extraction | All data is in script tags, no scrolling needed               |

## Assumptions Log

| #   | Claim                                                          | Section                     | Risk if Wrong                                                                                        |
| --- | -------------------------------------------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------- |
| A1  | `source_url` should use repo URL not skill subdirectory URL    | Output Schema Compatibility | Minor -- links to repo instead of skill dir; still navigable                                         |
| A2  | RSC payload format will remain stable across skills.sh deploys | RSC Extraction Algorithm    | Medium -- if format changes, regex extraction breaks; add error handling                             |
| A3  | `domcontentloaded` is sufficient wait state for RSC data       | Playwright Usage            | Low -- data is in initial HTML; if dynamic loading is added later, switch to `load` or `networkidle` |

## Open Questions

1. **Should trending use Hot or Trending tab?**
   - What we know: CONTEXT.md says "trending from Hot/Trending tab" (ambiguous -- could mean either or both)
   - Hot shows real-time growth with change deltas; Trending shows weekly installs
   - Recommendation: Use `/hot` for the `trending` array since it captures real-time momentum. The `change` field provides useful ranking signal.

2. **Should the script fall back to DOM extraction if RSC parsing fails?**
   - What we know: RSC format could change on any Vercel deploy
   - Recommendation: Implement RSC extraction as primary with a clear error message on failure. DOM fallback would add significant complexity for an edge case.

## Sources

### Primary (HIGH confidence)

- Live skills.sh pages (`/`, `/trending`, `/hot`) -- DOM structure and RSC payloads verified via agent-browser scraping [2026-04-28]
- Context7 `/microsoft/playwright` -- library mode usage, page.evaluate, waitForLoadState docs

### Secondary (MEDIUM confidence)

- npm registry -- playwright@1.59.1 verified current

## Metadata

**Confidence breakdown:**

- Page structure: HIGH -- directly verified via live scraping
- RSC extraction approach: HIGH -- tested on all three page views
- Output schema mapping: HIGH -- compared against existing `featured-skills.json`
- Playwright patterns: HIGH -- verified via Context7 official docs

**Research date:** 2026-04-28
**Valid until:** 2026-05-12 (skills.sh could redeploy and change RSC format at any time)
