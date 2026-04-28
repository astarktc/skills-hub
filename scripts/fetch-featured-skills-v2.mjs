#!/usr/bin/env node

/**
 * Scrapes the skills.sh leaderboard using Playwright and RSC payload extraction.
 *
 * Produces featured-skills.json with:
 *   - skills: top 300 by all-time installs (from /)
 *   - trending: top 100 by recent momentum (from /hot)
 *
 * No GITHUB_TOKEN needed — scrapes skills.sh directly.
 *
 * Usage: node scripts/fetch-featured-skills-v2.mjs
 */

import { existsSync, writeFileSync } from 'node:fs'
import { chromium } from 'playwright'

const OUTPUT_FILE = 'featured-skills.json'
const MAX_SKILLS = 300
const MAX_TRENDING = 100

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
