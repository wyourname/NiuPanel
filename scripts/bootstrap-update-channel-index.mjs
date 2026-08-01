import { readFileSync, writeFileSync } from 'node:fs'

const [channel, corePath, webPath, outputPath] = process.argv.slice(2)
if (!channel || !corePath || !webPath || !outputPath) {
  console.error(
    'Usage: node scripts/bootstrap-update-channel-index.mjs <preview|stable> <core.json> <web.json> <output.json>'
  )
  process.exit(1)
}
if (!['preview', 'stable'].includes(channel)) {
  throw new Error(`Unsupported update channel: ${channel}`)
}

const index = {
  schema_version: 1,
  channel,
  updated_at: new Date().toISOString(),
  core: JSON.parse(readFileSync(corePath, 'utf8')),
  web: JSON.parse(readFileSync(webPath, 'utf8'))
}

writeFileSync(outputPath, `${JSON.stringify(index, null, 2)}\n`)
