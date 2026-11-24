import assert from 'node:assert/strict'
import { copyFile, mkdtemp, readFile, rm, stat } from 'node:fs/promises'
import { randomUUID } from 'node:crypto'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout as delay } from 'node:timers/promises'
import test from 'node:test'
import { CliTransport } from '../lib/transport.js'

async function fixture(t) {
  const cwd = await mkdtemp(join(tmpdir(), 'verdant-dsh-protocol-'))
  await copyFile(new URL('./fixtures/cli.js', import.meta.url), join(cwd, 'request'))
  const cli = await CliTransport.create({
    executable: process.execPath,
    maxActiveJobs: 1,
    maxRequestBytes: 4096,
    maxResponseBytes: 16384,
    maxStderrBytes: 4096,
    killGraceMs: 100,
  })
  t.after(async () => {
    await cli.dispose()
    await rm(cwd, { recursive: true, force: true })
  })
  return { cwd, cli }
}

function request({ cwd, cli }, scenario, events, signal = new AbortController().signal) {
  const progress = []
  const done = cli.request('fixture', { scenario }, signal, 5000, cwd, randomUUID(),
    events ? event => { progress.push(event) } : undefined)
  return { done, progress }
}

async function retired(cwd) {
  const pid = Number(await readFile(join(cwd, 'pid'), 'utf8'))
  assert.ok(Number.isSafeInteger(pid) && pid > 0)
  assert.throws(() => process.kill(pid, 0), error => error.code === 'ESRCH')
}

async function protocolError(run) {
  let accepted
  try { accepted = await run.done } catch (error) {
    assert.equal(error.code, 'PROTOCOL_ERROR')
    return
  }
  assert.fail('Transport accepted damaged response: ' + JSON.stringify(accepted))
}

async function reusable(state) {
  const { done } = request(state, 'valid', false)
  assert.equal((await done).result.lyric, '青🙂')
  await retired(state.cwd)
}

test('valid multibyte text survives split writes in JSON and event responses', async t => {
  const state = await fixture(t)
  for (const events of [false, true]) {
    const run = request(state, 'valid', events)
    const response = await run.done
    assert.equal(response.ok, true)
    assert.equal(response.result.lyric, '青🙂')
    assert.equal(run.progress.length, events ? 1 : 0)
    await retired(state.cwd)
  }
})

test('invalid UTF-8 in a JSON success envelope is refused and releases its slot', async t => {
  const state = await fixture(t)
  await protocolError(request(state, 'invalid', false))
  await retired(state.cwd)
  await reusable(state)
})

test('invalid UTF-8 in an event result is refused and releases its slot', async t => {
  const state = await fixture(t)
  const run = request(state, 'invalid', true)
  await protocolError(run)
  assert.equal(run.progress.length, 1)
  assert.equal(run.progress[0].phase, 'fixture')
  await retired(state.cwd)
  await reusable(state)
})

test('an incomplete UTF-8 sequence at event-stream EOF is a protocol error', async t => {
  const state = await fixture(t)
  await protocolError(request(state, 'truncated', true))
  await retired(state.cwd)
  await reusable(state)
})

test('a leading BOM keeps the existing JSON rejection semantics', async t => {
  const state = await fixture(t)
  await protocolError(request(state, 'bom', false))
  await retired(state.cwd)
  await reusable(state)
})

test('abort during a partial UTF-8 event retires the process before returning', async t => {
  const state = await fixture(t)
  const controller = new AbortController()
  const reason = new DOMException('Fixture cancelled', 'AbortError')
  const { done } = request(state, 'abort', true, controller.signal)
  void done.catch(() => {})
  try {
    const deadline = Date.now() + 3000
    for (;;) {
      try { await stat(join(state.cwd, 'partial-ready')); break } catch (error) {
        if (error.code !== 'ENOENT' || Date.now() >= deadline) throw error
        await delay(10)
      }
    }
    controller.abort(reason)
    await assert.rejects(done, error => error === reason)
    await retired(state.cwd)
    await reusable(state)
  } finally {
    controller.abort(reason)
  }
})
