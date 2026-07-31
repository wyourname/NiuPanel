import { createHash } from 'node:crypto'
import { readFileSync, writeFileSync } from 'node:fs'

const [binaryPath, version, target, outputPath] = process.argv.slice(2)

if (!binaryPath || !version || !target || !outputPath) {
  console.error('Usage: node scripts/generate-core-release-manifest.mjs <binary> <version> <target> <output>')
  process.exit(1)
}

const sourceDateEpoch = Number.parseInt(process.env.SOURCE_DATE_EPOCH || '', 10)
const versionContract = readFileSync(new URL('../niupanel-common/src/version.rs', import.meta.url), 'utf8')
const contractValue = (name) => {
  const match = versionContract.match(new RegExp(`pub const ${name}: u32 = (\\d+);`))
  if (!match) throw new Error(`Unable to read ${name} from niupanel-common/src/version.rs`)
  return Number.parseInt(match[1], 10)
}
const builtAt = Number.isFinite(sourceDateEpoch)
  ? new Date(sourceDateEpoch * 1000).toISOString()
  : undefined
const manifest = {
  component: 'core',
  version: version.replace(/^v/, ''),
  launcher_protocol: contractValue('RELEASE_PROTOCOL_VERSION'),
  api_contract: contractValue('API_CONTRACT_VERSION'),
  schema_epoch: contractValue('SCHEMA_EPOCH'),
  schema_revision: contractValue('SCHEMA_REVISION'),
  target,
  binary_sha256: createHash('sha256').update(readFileSync(binaryPath)).digest('hex'),
  ...(builtAt ? { built_at: builtAt } : {})
}

writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`)
