// A controlled protocol peer launched through the real subprocess transport.
// Its files only exercise bridge identity/byte checks, not native rendering,
// portable-project validity, an installed DSH host, or channel delivery.
const fs = require('node:fs')
const { createHash } = require('node:crypto')

const JOB_A = '11111111-1111-4111-8111-111111111111'
const JOB_B = '22222222-2222-4222-8222-222222222222'
const operations = ['doctor', 'schema', 'project.new', 'effect.status', 'render', 'export.audio', 'project.pack']
let input = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', chunk => { input += chunk })
process.stdin.on('end', () => {
  const request = JSON.parse(input)
  fs.appendFileSync('cli-calls.jsonl', JSON.stringify({ op: request.op, pid: process.pid }) + '\n')
  const envelope = { protocol: 'verdant-cli/1', request_id: request.request_id, ok: true, diagnostics: [] }
  const send = value => process.stdout.write(JSON.stringify(value) + '\n')
  const artifact = (path, role, bytes) => {
    fs.writeFileSync(path, bytes)
    return { path, role, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }
  }
  let result
  switch (request.op) {
    case 'capabilities':
      result = { protocol: envelope.protocol, version: '0.1.0', operations }
      break
    case 'doctor': result = { fixture: true }; break
    case 'schema': result = { params: { properties: { max_duration_seconds: {} } } }; break
    case 'project.new':
      fs.writeFileSync(request.params.output, JSON.stringify({ fixture: 'project input' }))
      result = { revision: 0 }
      break
    case 'effect.status': result = { native_code_loaded: false, inserts: [] }; break
    case 'project.pack': {
      const packed = artifact(request.params.output, 'desktop_bundle', Buffer.from('fixture project bundle'))
      result = { byte_len: packed.bytes, sha256: packed.sha256 }
      break
    }
    case 'render':
    case 'export.audio': {
      const scenario = request.params.seed
      let sequence = 0
      const progress = jobId => send({ ...envelope, sequence: ++sequence, event: 'progress', job_id: jobId,
        data: { stage: 'fixture', job_id: jobId } })
      // Native progress may precede job admission with no job_id, and replay
      // may return a terminal result without any progress at all.
      if (scenario === 1) progress(null)
      if (![4, 5, 7].includes(scenario)) progress(JOB_A)
      if (scenario === 6) progress(JOB_B)
      result = {
        job_id: scenario === 2 ? JOB_B : JOB_A,
        frames: 4, sample_rate: 44100, project_id: 'fixture-project', revision: 0,
        artifacts: [
          artifact(request.params.output, 'audio', Buffer.from('fixture audio bytes')),
          artifact(request.params.report, 'report', Buffer.from('{"fixture":true}')),
        ],
      }
      send({ ...envelope, sequence: ++sequence, event: 'result', result,
        ...(scenario === 7 ? {} : { job_id: [3, 5].includes(scenario) ? JOB_B : result.job_id }) })
      return
    }
    default: throw new Error(`Unexpected fixture operation ${request.op}`)
  }
  send({ ...envelope, result })
})
