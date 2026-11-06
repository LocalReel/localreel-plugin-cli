// Example LocalReel storage plugin (Node.js).
//
// It exposes one local directory through the sidecar protocol: the host passes
// a token and a JSON config, the plugin prints one `ready` line, then answers
// stat/list/walk/read over loopback HTTP.

'use strict'

const http = require('node:http')
const fs = require('node:fs')
const path = require('node:path')

const token = process.env.LOCALREEL_PLUGIN_TOKEN ?? ''
const config = JSON.parse(process.env.LOCALREEL_PLUGIN_CONFIG ?? '{}')
if (typeof config.root !== 'string' || config.root.length === 0) {
  console.error('config.root is required')
  process.exit(1)
}
const root = fs.realpathSync(config.root)

function fileUri(absolute) {
  const normalized = absolute.split(path.sep).join('/')
  return process.platform === 'win32' ? `file:///${normalized}` : `file://${normalized}`
}

function resolve(uri) {
  let target = uri
  if (target.startsWith('file://')) {
    target = target.slice('file://'.length)
    if (process.platform === 'win32') target = target.replace(/^\//, '').replace(/\//g, '\\')
  } else {
    target = path.join(root, target)
  }
  try {
    const real = fs.realpathSync(target)
    const relative = path.relative(root, real)
    if (relative.startsWith('..') || path.isAbsolute(relative)) return null
    return real
  } catch {
    return null
  }
}

function statOf(absolute) {
  const stat = fs.statSync(absolute)
  return {
    size: stat.size,
    modified: Math.floor(stat.mtimeMs / 1000),
    is_dir: stat.isDirectory(),
  }
}

function describe(absolute) {
  const stat = fs.statSync(absolute)
  return {
    uri: fileUri(absolute),
    name: path.basename(absolute),
    is_dir: stat.isDirectory(),
    size: stat.size,
    modified: Math.floor(stat.mtimeMs / 1000),
  }
}

function walk(directory, recursive, out) {
  for (const name of fs.readdirSync(directory)) {
    const absolute = path.join(directory, name)
    const stat = fs.statSync(absolute)
    if (stat.isDirectory()) {
      if (recursive) walk(absolute, true, out)
      continue
    }
    out.push(describe(absolute))
  }
}

function send(response, status, body, type = 'application/json') {
  const payload = Buffer.isBuffer(body) ? body : Buffer.from(body)
  response.writeHead(status, {
    'Content-Type': type,
    'Content-Length': payload.length,
    Connection: 'close',
  })
  response.end(payload)
}

const server = http.createServer((request, response) => {
  if (token !== '' && request.headers['x-localreel-token'] !== token) {
    send(response, 403, 'forbidden', 'text/plain')
    return
  }
  const url = new URL(request.url ?? '/', 'http://127.0.0.1')
  const uri = url.searchParams.get('uri') ?? ''
  if (url.pathname === '/v1/health') {
    send(response, 200, '{"ok":true}')
    return
  }
  if (url.pathname === '/v1/stat') {
    const absolute = resolve(uri)
    if (!absolute) return send(response, 404, 'missing', 'text/plain')
    try {
      send(response, 200, JSON.stringify(statOf(absolute)))
    } catch {
      send(response, 404, 'missing', 'text/plain')
    }
    return
  }
  if (url.pathname === '/v1/list' || url.pathname === '/v1/walk') {
    const absolute = resolve(uri)
    if (!absolute) return send(response, 404, 'missing', 'text/plain')
    const entries = []
    try {
      walk(absolute, url.pathname === '/v1/walk', entries)
    } catch {
      return send(response, 404, 'missing', 'text/plain')
    }
    send(response, 200, JSON.stringify({ entries }))
    return
  }
  if (url.pathname === '/v1/read') {
    const absolute = resolve(uri)
    if (!absolute) return send(response, 404, 'missing', 'text/plain')
    const start = Number(url.searchParams.get('start') ?? '0')
    const length = Number(url.searchParams.get('len') ?? '0')
    try {
      const fd = fs.openSync(absolute, 'r')
      try {
        const buffer = Buffer.alloc(Math.max(0, length))
        const read = fs.readSync(fd, buffer, 0, buffer.length, start)
        send(response, 200, buffer.subarray(0, read), 'application/octet-stream')
      } finally {
        fs.closeSync(fd)
      }
    } catch {
      send(response, 404, 'missing', 'text/plain')
    }
    return
  }
  send(response, 404, 'unknown route', 'text/plain')
})

server.listen(0, '127.0.0.1', () => {
  console.log(JSON.stringify({ ready: true, port: server.address().port }))
})

// The host closes stdin on shutdown; exit so no orphan keeps the port alive.
process.stdin.resume()
process.stdin.on('end', () => process.exit(0))
