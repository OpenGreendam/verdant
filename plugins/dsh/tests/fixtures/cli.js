// A byte-level protocol producer for transport tests, run by the native Node
// executable outside this package. This is not a Verdant engine or DSH host.
const fs = require('node:fs')

fs.writeFileSync('pid', String(process.pid))
let input = ''
process.stdin.setEncoding('utf8')
process.stdin.on('data', chunk => { input += chunk })
process.stdin.on('end', () => {
  const request = JSON.parse(input)
  const events = process.argv.includes('--events')
  const envelope = {
    protocol: 'verdant-cli/1',
    request_id: request.request_id,
    ok: true,
    result: { lyric: '青🙂' },
    diagnostics: [],
  }
  const progress = {
    protocol: envelope.protocol,
    request_id: envelope.request_id,
    sequence: 1,
    event: 'progress',
    phase: 'fixture',
  }
  const response = events ? { ...envelope, sequence: 2, event: 'result' } : envelope
  const encode = value => Buffer.from(
    (events ? JSON.stringify(progress) + '\n' : '') + JSON.stringify(value) + (events ? '\n' : ''),
  )
  let bytes = encode(response)
  switch (request.params.scenario) {
    case 'invalid': {
      response.result.lyric = 'BROKEN_UTF8'
      bytes = encode(response)
      const offset = bytes.indexOf('BROKEN_UTF8')
      bytes = Buffer.concat([
        bytes.subarray(0, offset),
        Buffer.from([0xff]),
        bytes.subarray(offset + 'BROKEN_UTF8'.length),
      ])
      process.stdout.end(bytes)
      break
    }
    case 'bom':
      process.stdout.end(Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), bytes]))
      break
    case 'truncated':
      process.stdout.end(bytes.subarray(0, bytes.indexOf(Buffer.from('🙂')) + 2))
      break
    case 'abort':
      // Keep an incomplete record alive and require the transport's forced
      // cleanup path on POSIX instead of exiting when stdin reaches EOF.
      process.on('SIGTERM', () => {})
      setInterval(() => {}, 1000)
      process.stdout.write(bytes.subarray(0, bytes.indexOf(Buffer.from('🙂')) + 2), () => {
        fs.writeFileSync('partial-ready', '')
      })
      break
    default: {
      const offset = bytes.indexOf(Buffer.from('🙂')) + 2
      process.stdout.write(bytes.subarray(0, offset), () => {
        setTimeout(() => process.stdout.end(bytes.subarray(offset)), 15)
      })
    }
  }
})
