#!/usr/bin/env node

/**
 * Scrapes the skills.sh leaderboard using Playwright and RSC payload extraction,
 * then enriches each skill with SKILL.md metadata from GitHub.
 *
 * Produces featured-skills.json with:
 *   - skills: top 300 by all-time installs (from /)
 *   - trending: top 100 by recent momentum (from /hot)
 *
 * Phase 1: Playwright scrapes skills.sh for skill list + install counts
 * Phase 2: GitHub API fetches SKILL.md frontmatter for summaries, names, and full source_urls
 *
 * Usage: node scripts/fetch-featured-skills-v2.mjs
 *   Requires GITHUB_TOKEN for enrichment (falls back to empty summaries without it)
 */

import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { chromium } from 'playwright'

// Load .env file from project root
const envPath = resolve(import.meta.dirname, '..', '.env')
if (existsSync(envPath)) {
  for (const line of readFileSync(envPath, 'utf-8').split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue
    const idx = trimmed.indexOf('=')
    if (idx === -1) continue
    const key = trimmed.slice(0, idx).trim()
    const value = trimmed.slice(idx + 1).trim()
    if (!process.env[key]) process.env[key] = value
  }
}

const OUTPUT_FILE = 'featured-skills.json'
const MAX_SKILLS = 300
const MAX_TRENDING = 100

const GITHUB_TOKEN = process.env.GITHUB_TOKEN || ''
const CONCURRENCY = 10
const MAX_RATE_LIMIT_WAIT_SECS = 60

// Known scan bases for skill directories (matches installer.rs)
const SKILL_SCAN_BASES = [
  'skills',
  'skills/.curated',
  'skills/.experimental',
  'skills/.system',
  '.claude/skills',
]

// ─── HTTP / concurrency helpers ───

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms))
}

async function fetchJson(url, retries = 3) {
  const headers = {
    Accept: 'application/vnd.github+json',
    'User-Agent': 'skills-hub-aggregator',
  }
  if (GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${GITHUB_TOKEN}`
  }

  for (let attempt = 0; attempt <= retries; attempt++) {
    const res = await fetch(url, { headers })

    if (res.status === 403 || res.status === 429) {
      const resetHeader = res.headers.get('x-ratelimit-reset')
      let waitSecs = resetHeader
        ? Math.max(Number(resetHeader) - Math.floor(Date.now() / 1000), 1)
        : Math.pow(2, attempt + 1)

      if (waitSecs > MAX_RATE_LIMIT_WAIT_SECS) {
        console.warn(`Rate limited, reset in ${waitSecs}s (exceeds max ${MAX_RATE_LIMIT_WAIT_SECS}s) -- skipping`)
        return null
      }
      console.warn(`Rate limited (${res.status}), waiting ${waitSecs}s (attempt ${attempt + 1}/${retries + 1})...`)
      await sleep(waitSecs * 1000)
      continue
    }

    if (!res.ok) {
      if (attempt < retries) {
        await sleep(Math.pow(2, attempt) * 1000)
        continue
      }
      return null
    }

    return res.json()
  }
  return null
}

async function pMap(items, fn, concurrency) {
  const results = new Array(items.length)
  let idx = 0

  async function worker() {
    while (idx < items.length) {
      const i = idx++
      results[i] = await fn(items[i], i)
    }
  }

  const workers = Array.from({ length: Math.min(concurrency, items.length) }, () => worker())
  await Promise.all(workers)
  return results
}

// ─── GitHub SKILL.md enrichment helpers ───

function parseSkillMdFrontmatter(content) {
  const lines = content.split('\n')
  if (lines[0].trim() !== '---') return null
  let name = null
  let description = null
  for (let i = 1; i < lines.length; i++) {
    const l = lines[i].trim()
    if (l === '---') break
    if (l.startsWith('name:')) {
      name = l.slice(5).trim().replace(/^["']|["']$/g, '')
    } else if (l.startsWith('description:')) {
      description = l.slice(12).trim().replace(/^["']|["']$/g, '')
    }
  }
  return { name, description }
}

async function getRepoTree(owner, repo, branch) {
  const url = `https://api.github.com/repos/${owner}/${repo}/git/trees/${encodeURIComponent(branch)}?recursive=1`
  const data = await fetchJson(url, 2)
  if (!data || !data.tree) return null
  if (data.truncated) {
    console.warn(`  Warning: tree for ${owner}/${repo} was truncated, some skills may be missed`)
  }
  return data.tree
}

async function fetchSkillMdContent(owner, repo, branch, dirPath) {
  const filePath = dirPath ? `${dirPath}/SKILL.md` : 'SKILL.md'
  const url = `https://api.github.com/repos/${owner}/${repo}/contents/${encodeURIComponent(filePath)}?ref=${encodeURIComponent(branch)}`
  const data = await fetchJson(url, 1)
  if (!data || !data.content) return null
  const content = Buffer.from(data.content, 'base64').toString('utf-8')
  return parseSkillMdFrontmatter(content)
}

/**
 * Post-scrape enrichment: fetches SKILL.md from GitHub for each skill to
 * populate summary, name, and full source_url.
 */
async function enrichWithGitHub(allSkills) {
  // Group skills by repo (owner/repo extracted from source_url)
  const repoSkillsMap = new Map() // "owner/repo" -> skill[]
  for (const skill of allSkills) {
    const match = skill.source_url.match(/github\.com\/([^/]+\/[^/]+)/)
    if (!match) continue
    const repoFullName = match[1]
    if (!repoSkillsMap.has(repoFullName)) {
      repoSkillsMap.set(repoFullName, [])
    }
    repoSkillsMap.get(repoFullName).push(skill)
  }

  const repoNames = Array.from(repoSkillsMap.keys())
  console.log(`Enriching ${allSkills.length} skills across ${repoNames.length} repos...`)

  // Fetch default branch for each repo
  const repoBranches = new Map() // "owner/repo" -> default_branch
  await pMap(
    repoNames,
    async (fullName) => {
      const url = `https://api.github.com/repos/${fullName}`
      const data = await fetchJson(url)
      if (data && data.default_branch) {
        repoBranches.set(fullName, data.default_branch)
      }
    },
    CONCURRENCY,
  )
  console.log(`  Fetched metadata for ${repoBranches.size}/${repoNames.length} repos`)

  // Fetch tree for each repo
  const repoTrees = new Map() // "owner/repo" -> tree[]
  await pMap(
    Array.from(repoBranches.entries()),
    async ([fullName, branch]) => {
      const [owner, repo] = fullName.split('/')
      const tree = await getRepoTree(owner, repo, branch)
      if (tree) {
        repoTrees.set(fullName, tree)
        console.log(`  Tree: ${fullName} (${tree.length} items)`)
      }
    },
    CONCURRENCY,
  )
  console.log(`  Fetched trees for ${repoTrees.size}/${repoBranches.size} repos`)

  // Match each skill to a directory in its repo's tree
  const skillDirMap = new Map() // skill -> { dirPath, repoFullName, branch }
  for (const [repoFullName, skills] of repoSkillsMap.entries()) {
    const tree = repoTrees.get(repoFullName)
    const branch = repoBranches.get(repoFullName)
    if (!tree || !branch) continue

    // Build lookup structures from tree
    const blobPaths = new Set()
    const dirPaths = new Set()
    for (const item of tree) {
      if (item.type === 'blob') blobPaths.add(item.path)
      else if (item.type === 'tree') dirPaths.add(item.path)
    }

    for (const skill of skills) {
      // Find directories whose last segment matches the slug (exact, then containment)
      const candidates = []
      for (const dp of dirPaths) {
        const segments = dp.split('/')
        const lastSeg = segments[segments.length - 1]
        const isExact = lastSeg === skill.slug
        // Bidirectional containment: "react-best-practices" ↔ "vercel-react-best-practices"
        const isContainment =
          !isExact && (lastSeg.includes(skill.slug) || skill.slug.includes(lastSeg))
        if (isExact || isContainment) {
          const hasSkillMd = blobPaths.has(`${dp}/SKILL.md`)
          const isUnderScanBase = SKILL_SCAN_BASES.some(
            (base) => dp === `${base}/${skill.slug}` || dp.startsWith(`${base}/`),
          )
          candidates.push({ dirPath: dp, hasSkillMd, isUnderScanBase, isExact })
        }
      }

      if (candidates.length === 0) continue

      // Only accept containment matches when exactly one has SKILL.md (avoid ambiguity)
      const exact = candidates.filter((c) => c.isExact)
      const fuzzy = candidates.filter((c) => !c.isExact && c.hasSkillMd)
      const usable = exact.length > 0 ? exact : fuzzy.length === 1 ? fuzzy : []
      if (usable.length === 0) continue

      // Pick best match: prefer has SKILL.md + under scan base
      usable.sort((a, b) => {
        if (a.hasSkillMd !== b.hasSkillMd) return a.hasSkillMd ? -1 : 1
        if (a.isUnderScanBase !== b.isUnderScanBase) return a.isUnderScanBase ? -1 : 1
        return a.dirPath.length - b.dirPath.length
      })

      skillDirMap.set(skill, {
        dirPath: usable[0].dirPath,
        repoFullName,
        branch,
      })
    }
  }

  console.log(`  Matched ${skillDirMap.size}/${allSkills.length} skills to directories`)

  // Fetch SKILL.md for matched skills
  const skillsToFetch = Array.from(skillDirMap.entries())
  let enrichedCount = 0

  await pMap(
    skillsToFetch,
    async ([skill, { dirPath, repoFullName, branch }]) => {
      const [owner, repo] = repoFullName.split('/')
      const md = await fetchSkillMdContent(owner, repo, branch, dirPath)

      // Update source_url to include full path
      skill.source_url = `https://github.com/${repoFullName}/tree/${branch}/${dirPath}`

      if (md) {
        if (md.description) {
          skill.summary = md.description
          enrichedCount++
        }
        if (md.name) {
          skill.name = md.name
        }
      }
    },
    CONCURRENCY,
  )

  console.log(`Enriched ${enrichedCount}/${allSkills.length} skills with descriptions`)
}

/**
 * Extract the initialSkills array from RSC payloads embedded in page HTML.
 *
 * skills.sh is a Next.js RSC app. All skill data is embedded in <script> tags
 * as self.__next_f.push([1, "..."]) calls containing JSON with initialSkills.
 */
async function extractSkillsFromPage(page, url) {
  console.log(`  Navigating to ${url}...`)
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 })

  const html = await page.content()

  // Find RSC push calls containing initialSkills
  const pushRegex = /self\.__next_f\.push\(\[1,"((?:[^"\\]|\\.)*)"\]\)/g
  let match
  while ((match = pushRegex.exec(html)) !== null) {
    if (!match[1].includes('initialSkills')) continue

    // Unescape the string content (it's a JS string literal inside quotes)
    let unescaped
    try {
      unescaped = JSON.parse('"' + match[1] + '"')
    } catch {
      continue
    }

    // Find the JSON object containing initialSkills
    const jsonStart = unescaped.indexOf('{"initialSkills"')
    if (jsonStart === -1) continue

    // Find matching closing brace (brace-balanced parsing)
    let depth = 0
    let jsonEnd = -1
    for (let i = jsonStart; i < unescaped.length; i++) {
      if (unescaped[i] === '{') depth++
      if (unescaped[i] === '}') {
        depth--
        if (depth === 0) {
          jsonEnd = i + 1
          break
        }
      }
    }

    if (jsonEnd === -1) continue

    try {
      const data = JSON.parse(unescaped.substring(jsonStart, jsonEnd))
      if (Array.isArray(data.initialSkills)) {
        return data.initialSkills
      }
    } catch {
      continue
    }
  }

  throw new Error(`No initialSkills found in RSC payload at ${url}`)
}

/**
 * Transform a raw leaderboard skill entry to the featured-skills.json schema.
 */
function transformSkill(skill, timestamp) {
  return {
    slug: skill.skillId,
    name: skill.name,
    summary: '',
    downloads: skill.installs,
    stars: 0,
    category: 'general',
    tags: [],
    source_url: `https://github.com/${skill.source}`,
    updated_at: timestamp,
  }
}

/**
 * Deduplicate skills by slug, keeping the first occurrence (higher rank).
 */
function deduplicateBySlug(skills) {
  const seen = new Set()
  return skills.filter((s) => {
    if (seen.has(s.slug)) return false
    seen.add(s.slug)
    return true
  })
}

async function main() {
  const timestamp = new Date().toISOString()
  let browser

  try {
    console.log('Launching Playwright (headless Chromium)...')
    browser = await chromium.launch()
    const page = await browser.newPage()

    // Scrape All Time page for the skills array
    console.log('Scraping All Time leaderboard (/)...')
    const allTimeRaw = await extractSkillsFromPage(page, 'https://skills.sh/')
    console.log(`  Found ${allTimeRaw.length} skills from All Time page`)

    // Scrape Hot page for the trending array
    console.log('Scraping Hot leaderboard (/hot)...')
    const hotRaw = await extractSkillsFromPage(page, 'https://skills.sh/hot')
    console.log(`  Found ${hotRaw.length} skills from Hot page`)

    // Transform and deduplicate
    const allTimeTransformed = allTimeRaw.map((s) => transformSkill(s, timestamp))
    const hotTransformed = hotRaw.map((s) => transformSkill(s, timestamp))

    const skills = deduplicateBySlug(allTimeTransformed).slice(0, MAX_SKILLS)
    const trending = deduplicateBySlug(hotTransformed).slice(0, MAX_TRENDING)

    console.log(`After dedup: ${skills.length} skills, ${trending.length} trending`)

    // Phase 2: Enrich with GitHub SKILL.md metadata
    if (GITHUB_TOKEN) {
      const allSkills = [...skills, ...trending]
      await enrichWithGitHub(allSkills)
    } else {
      console.warn(
        'GITHUB_TOKEN not set -- skipping SKILL.md enrichment (summaries will be empty)',
      )
    }

    if (skills.length === 0 && trending.length === 0) {
      console.error('ERROR: No skills extracted from any page')
      if (existsSync(OUTPUT_FILE)) {
        console.warn('Keeping existing featured-skills.json')
        return
      }
      process.exit(1)
    }

    const output = {
      updated_at: timestamp,
      total: skills.length,
      categories: ['general'],
      skills,
      trending,
    }

    writeFileSync(OUTPUT_FILE, JSON.stringify(output, null, 2) + '\n')
    console.log(`Wrote ${skills.length} skills + ${trending.length} trending to ${OUTPUT_FILE}`)
  } catch (err) {
    console.error('Fatal error:', err.message || err)

    if (existsSync(OUTPUT_FILE)) {
      console.warn('Keeping existing featured-skills.json')
    } else {
      process.exit(1)
    }
  } finally {
    if (browser) {
      await browser.close()
    }
  }
}

main()
