import { execFileSync } from 'node:child_process'
import { appendFileSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const componentVersion = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/
const panelVersionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/

export function createReleasePlan({ panelVersion, channel, coreVersion, webVersion, sourceSha }) {
  if (!panelVersionPattern.test(panelVersion || '')) throw new Error('Panel version must be a semantic version')
  const prerelease = panelVersion.split('-').slice(1).join('-')
  if (prerelease.split('.').some(part => /^0\d+$/.test(part))) throw new Error('Prerelease numeric identifiers cannot have leading zeros')
  if (!['stable', 'preview'].includes(channel)) throw new Error('Channel must be stable or preview')
  if (channel === 'stable' && !componentVersion.test(panelVersion)) throw new Error('Stable releases cannot use a prerelease version')
  if (!componentVersion.test(coreVersion || '') || !componentVersion.test(webVersion || '')) {
    throw new Error('Core and Web must each declare a numeric version')
  }
  if (!/^[0-9a-f]{40}$/.test(sourceSha || '')) throw new Error('Source must be an immutable Git commit')
  return {
    panel_version: panelVersion,
    channel,
    core_tag: `Core-v${coreVersion}`,
    web_tag: `web-v${webVersion}`,
    source_sha: sourceSha,
  }
}

// Validate every tag before creating any: an existing release identity must never move.
export async function prepareComponentTags(plan, { lookup, create }) {
  const componentTags = [plan.core_tag, plan.web_tag]
  const existing = new Map()
  for (const tag of [...componentTags, `v${plan.panel_version}`]) {
    const sha = await lookup(tag)
    if (sha && sha !== plan.source_sha) {
      throw new Error(`${tag} already belongs to ${sha}; bump the version instead of replacing its source`)
    }
    existing.set(tag, sha)
  }
  for (const tag of componentTags) {
    if (!existing.get(tag)) await create(tag, plan.source_sha)
  }
}

async function main() {
  const sourceSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim()
  const coreVersion = readFileSync('niupanel/Cargo.toml', 'utf8').match(/^version\s*=\s*"([^"]+)"/m)?.[1]
  const webVersion = JSON.parse(readFileSync('niupanelweb/package.json', 'utf8')).version
  const plan = createReleasePlan({
    panelVersion: process.env.PANEL_VERSION,
    channel: process.env.UPDATE_CHANNEL,
    coreVersion,
    webVersion,
    sourceSha,
  })
  if (process.argv.includes('--dry-run')) {
    console.log(JSON.stringify(plan, null, 2))
    return
  }
  const repository = process.env.GITHUB_REPOSITORY
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository || '')) throw new Error('GITHUB_REPOSITORY is required')
  if (!process.env.GITHUB_OUTPUT) throw new Error('GITHUB_OUTPUT is required')
  await prepareComponentTags(plan, {
    lookup(tag) {
      const refs = execFileSync('git', ['ls-remote', '--tags', 'origin', `refs/tags/${tag}`, `refs/tags/${tag}^{}`], { encoding: 'utf8' })
        .trim().split('\n').filter(Boolean).map(line => line.split(/\s+/))
      return refs.find(([, ref]) => ref.endsWith('^{}'))?.[0] || refs[0]?.[0] || null
    },
    create(tag, sha) {
      execFileSync('gh', ['api', '--method', 'POST', `repos/${repository}/git/refs`, '-f', `ref=refs/tags/${tag}`, '-f', `sha=${sha}`], { stdio: 'pipe' })
    },
  })
  for (const [key, value] of Object.entries(plan)) appendFileSync(process.env.GITHUB_OUTPUT, `${key}=${value}\n`)
  console.log(`Prepared Panel ${plan.panel_version}: ${plan.core_tag} + ${plan.web_tag} from ${plan.source_sha}`)
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) await main()
