import assert from 'node:assert/strict'
import { copyFile, mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { VerdantBridge } from '../lib/bridge.js'
import { Config } from '../lib/config.js'
import { Workspaces } from '../lib/workspaces.js'

const JOB_A = '11111111-1111-4111-8111-111111111111'
const JOB_B = '22222222-2222-4222-8222-222222222222'
const source = new URL('./fixtures/render-cli.js', import.meta.url)

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'verdant-dsh-render-'))
  const config = Config({ executable: process.execPath, workspaceRoot: root, maxActiveJobs: 1,
    queryTimeoutMs: 5000, renderTimeoutMs: 5000, killGraceMs: 100 })
  // Node runs the relative `request` script just as in transport.test.mjs.
  // Initialize a real managed root before installing this controlled peer.
  const store = await Workspaces.create(config)
  await store.dispose()
  await copyFile(source, join(root, 'request'))
  // Only the host session shell is a fixture; bridge, workspace persistence,
  // CLI envelopes, pipes, supervision, and byte identities use production code.
  const ctx = { on() {}, get() {}, tools: { guard() {} } }
  const exec = { agent: { id: 'render-fixture-session' }, callId: 'render-fixture-call',
    rootCallId: 'render-fixture-call', signal: new AbortController().signal }
  const bridge = await VerdantBridge.create(ctx, config)
  t.after(async () => {
    await bridge.dispose()
    await rm(root, { recursive: true, force: true })
  })
  const workspace = await bridge.store.create(await bridge.host.tool(exec))
  const directory = bridge.store.directory(workspace)
  await copyFile(source, join(directory, 'request'))
  const invoke = (name, args) => bridge.tool(name, { workspace_id: workspace.id, ...args }, exec)
  await invoke('verdant_project', { action: 'new' })
  return { bridge, directory, invoke }
}

async function calls(state) {
  const rows = (await readFile(join(state.directory, 'cli-calls.jsonl'), 'utf8')).trim().split('\n').map(JSON.parse)
  for (const { pid } of rows) assert.throws(() => process.kill(pid, 0), error => error.code === 'ESRCH')
  return rows
}

async function render(state, scenario, exportAudio = false) {
  return state.invoke(exportAudio ? 'verdant_export' : 'verdant_render', {
    ...(exportAudio ? { kind: 'audio' } : {}), seed: scenario, idempotency_key: `fixture-${scenario}`,
  })
}

async function protocolError(promise) {
  const outcome = await promise.then(result => ({ result }), error => ({ error }))
  if (outcome.error !== undefined) {
    assert.equal(outcome.error.code, 'PROTOCOL_ERROR')
    return
  }
  assert.fail('Bridge accepted an inconsistent native job identity: ' + JSON.stringify({
    kind: outcome.result.kind, verdant_job_id: outcome.result.verdant_job_id,
    artifacts: outcome.result.artifacts.length,
  }))
}

for (const [label, scenario] of [
  ['progress and terminal job IDs agree after unassigned progress', 1],
  ['a terminal result establishes identity without progress', 4],
  ['a result without an optional top-level job ID stays compatible', 7],
]) {
  test(label, async t => {
    const state = await fixture(t)
    const result = await render(state, scenario)
    assert.equal(result.kind, 'completed')
    assert.equal(result.verdant_job_id, JOB_A)
    assert.equal(result.artifacts.length, 3)
    const listing = await state.invoke('verdant_artifacts', { action: 'list' })
    assert.equal(listing.active, false)
    assert.equal(listing.runs[0].status, 'succeeded')
    assert.equal(listing.runs[0].verdant_job_id, JOB_A)
    const manifest = JSON.parse(await readFile(join(state.directory, 'manifest.json'), 'utf8'))
    assert.equal(manifest.runs[0].result.verdant_job_id, JOB_A)
    assert.equal((await calls(state)).filter(call => call.op === 'project.pack').length, 1)
  })
}

for (const [label, scenario, expectedJob, exportAudio] of [
  ['a terminal result cannot replace the established progress job ID', 2, JOB_A, false],
  ['export audio rejects a terminal envelope with a different job ID', 3, JOB_A, true],
  ['a terminal envelope must agree with its result even without progress', 5, undefined, false],
  ['successive progress events cannot change the native job ID', 6, JOB_A, false],
]) {
  test(label, async t => {
    const state = await fixture(t)
    await protocolError(render(state, scenario, exportAudio))
    const listing = await state.invoke('verdant_artifacts', { action: 'list' })
    assert.equal(listing.active, false)
    assert.deepEqual(listing.artifacts, [])
    assert.equal(listing.runs[0].status, 'failed')
    assert.equal(listing.runs[0].verdant_job_id, expectedJob)
    assert.notEqual(listing.runs[0].verdant_job_id, JOB_B)
    const manifest = JSON.parse(await readFile(join(state.directory, 'manifest.json'), 'utf8'))
    assert.equal(manifest.runs[0].result, undefined)
    assert.equal((await calls(state)).some(call => call.op === 'project.pack'), false)
    // An identity failure must not strand either the workspace or process slot.
    const retry = await render(state, 1)
    assert.equal(retry.verdant_job_id, JOB_A)
    await calls(state)
  })
}
