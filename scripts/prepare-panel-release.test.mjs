import assert from 'node:assert/strict'
import { test } from 'node:test'
import { createReleasePlan, prepareComponentTags } from './prepare-panel-release.mjs'

const sourceSha = 'a'.repeat(40)
const inputs = { panelVersion: '0.8.6', channel: 'preview', coreVersion: '0.8.6', webVersion: '2.0.6', sourceSha }

test('one Panel release selects both declared component versions from one commit', () => {
  assert.deepEqual(createReleasePlan(inputs), {
    panel_version: '0.8.6', channel: 'preview', core_tag: 'Core-v0.8.6', web_tag: 'web-v2.0.6', source_sha: sourceSha,
  })
})

test('rejects invalid release identities and prereleases on stable', () => {
  for (const patch of [
    { panelVersion: 'v0.8.6' }, { panelVersion: '0.8.6;echo bad' }, { panelVersion: '0.8.6-beta.01' },
    { panelVersion: '0.8.6-beta.1', channel: 'stable' }, { channel: 'latest' },
    { coreVersion: '0.8.6-beta.1' }, { webVersion: '2.00.6' }, { sourceSha: 'main' },
  ]) assert.throws(() => createReleasePlan({ ...inputs, ...patch }))
  assert.equal(createReleasePlan({ ...inputs, panelVersion: '0.8.6-beta.1' }).panel_version, '0.8.6-beta.1')
})

test('creates only missing component tags; leaves Panel publication to the final stage', async () => {
  const calls = []
  await prepareComponentTags(createReleasePlan(inputs), {
    lookup: tag => { calls.push(['lookup', tag]); return null },
    create: (tag, sha) => { calls.push(['create', tag, sha]) },
  })
  assert.deepEqual(calls, [
    ['lookup', 'Core-v0.8.6'], ['lookup', 'web-v2.0.6'], ['lookup', 'v0.8.6'],
    ['create', 'Core-v0.8.6', sourceSha], ['create', 'web-v2.0.6', sourceSha],
  ])
})

test('same-commit reruns reuse tags without modifying them', async () => {
  const creates = []
  await prepareComponentTags(createReleasePlan(inputs), {
    lookup: () => sourceSha,
    create: (...args) => creates.push(args),
  })
  assert.equal(creates.length, 0)
})

for (const conflictingTag of ['Core-v0.8.6', 'web-v2.0.6', 'v0.8.6']) {
  test(`rejects conflicting ${conflictingTag} before creating any tags`, async () => {
    const creates = []
    await assert.rejects(prepareComponentTags(createReleasePlan(inputs), {
      lookup: tag => tag === conflictingTag ? 'b'.repeat(40) : null,
      create: (...args) => creates.push(args),
    }), /bump the version/)
    assert.equal(creates.length, 0)
  })
}

test('remote lookup failure is not treated as a missing tag', async () => {
  const creates = []
  await assert.rejects(prepareComponentTags(createReleasePlan(inputs), {
    lookup: () => { throw new Error('Remote unavailable') },
    create: (...args) => creates.push(args),
  }), /Remote unavailable/)
  assert.equal(creates.length, 0)
})
