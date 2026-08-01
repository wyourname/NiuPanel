import { readFileSync, writeFileSync } from 'node:fs'

const [channel, panelVersion, corePath, webPath, repository, outputPath] = process.argv.slice(2)
if (
  !['preview', 'stable'].includes(channel) ||
  !panelVersion ||
  !corePath ||
  !webPath ||
  !repository ||
  !outputPath
) {
  console.error(
    'Usage: node scripts/compose-update-channel-index.mjs <preview|stable> <panel-version> <core.json> <web.json> <owner/repo> <output.json>'
  )
  process.exit(1)
}

const core = JSON.parse(readFileSync(corePath, 'utf8'))
const web = JSON.parse(readFileSync(webPath, 'utf8'))
const tag = `v${panelVersion}`
const index = {
  schema_version: 2,
  channel,
  updated_at: new Date().toISOString(),
  release: {
    version: panelVersion,
    tag,
    release_url: `https://github.com/${repository}/releases/tag/${tag}`,
    notes: '',
    launcher_protocol: core.launcher_protocol,
    core,
    web
  }
}

writeFileSync(outputPath, `${JSON.stringify(index, null, 2)}\n`)
