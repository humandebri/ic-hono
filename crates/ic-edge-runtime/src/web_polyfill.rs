//! `crates/ic-edge-runtime` embeds the minimum JS Web API polyfill.
//! It is intentionally small until the Hono compatibility suite demands more.

pub const SOURCE: &str = r#"
class Headers {
  constructor(init = []) {
    this.values = []
    if (init instanceof Headers) {
      init = init.entriesArray()
    } else if (!Array.isArray(init)) {
      init = Object.entries(init)
    }
    for (const [name, value] of init) this.append(name, value)
  }
  append(name, value) {
    this.values.push([String(name).toLowerCase(), String(value)])
  }
  set(name, value) {
    const key = String(name).toLowerCase()
    this.values = this.values.filter(([item]) => item !== key)
    this.values.push([key, String(value)])
  }
  delete(name) {
    const key = String(name).toLowerCase()
    this.values = this.values.filter(([item]) => item !== key)
  }
  get(name) {
    const key = String(name).toLowerCase()
    const found = this.values.filter(([item]) => item === key).map(([, value]) => value)
    return found.length === 0 ? null : found.join(', ')
  }
  has(name) {
    const key = String(name).toLowerCase()
    return this.values.some(([item]) => item === key)
  }
  forEach(callback, thisArg = undefined) {
    for (const [name, value] of this.values) {
      callback.call(thisArg, value, name, this)
    }
  }
  entries() {
    return this.values[Symbol.iterator]()
  }
  [Symbol.iterator]() {
    return this.values[Symbol.iterator]()
  }
  entriesArray() {
    return this.values
  }
}

class Request {
  constructor(input, init = {}) {
    this.url = String(input)
    this.method = String(init.method || 'GET').toUpperCase()
    this.headers = new Headers(init.headers || [])
    this._body = body_from(init.body || '')
    this.signal = init.signal
  }
  text() {
    return Promise.resolve(body_text(this._body))
  }
  json() {
    return Promise.resolve(JSON.parse(body_text(this._body)))
  }
  arrayBuffer() {
    return Promise.resolve(body_bytes(this._body).buffer)
  }
}

class Response {
  constructor(body = '', init = {}) {
    this.status = init.status || 200
    this.statusText = init.statusText || ''
    this.headers = new Headers(init.headers || [])
    this._body = body_from(body === null ? '' : body)
  }
  get ok() {
    return this.status >= 200 && this.status < 300
  }
  get body() {
    return this._body
  }
  text() {
    return Promise.resolve(body_text(this._body))
  }
  json() {
    return Promise.resolve(JSON.parse(body_text(this._body)))
  }
  arrayBuffer() {
    return Promise.resolve(body_bytes(this._body).buffer)
  }
  static json(value, init = {}) {
    const response = new Response(JSON.stringify(value), init)
    response.headers.set('content-type', 'application/json')
    return response
  }
}

class Blob {
  constructor(parts = [], init = {}) {
    this.type = String(init.type || '')
    this._body = body_from(parts.map((part) => body_text(body_from(part))).join(''))
    this.size = body_bytes(this._body).byteLength
  }
  text() {
    return Promise.resolve(body_text(this._body))
  }
  arrayBuffer() {
    return Promise.resolve(body_bytes(this._body).buffer)
  }
}

class AbortController {
  constructor() {
    this.signal = new AbortSignal()
  }
  abort() {
    if (this.signal.aborted) return
    this.signal.aborted = true
    this.signal.dispatchEvent({ type: 'abort' })
  }
}

class AbortSignal {
  constructor() {
    this.aborted = false
    this.listeners = []
  }
  addEventListener(type, callback) {
    if (type === 'abort' && callback) this.listeners.push(callback)
  }
  removeEventListener(type, callback) {
    if (type !== 'abort') return
    this.listeners = this.listeners.filter((listener) => listener !== callback)
  }
  dispatchEvent(event) {
    for (const listener of this.listeners) listener.call(this, event)
    return true
  }
  throwIfAborted() {
    if (this.aborted) throw new Error('This operation was aborted')
  }
}

class EventTarget {
  addEventListener() {}
  removeEventListener() {}
  dispatchEvent() {
    return true
  }
}
class TextEncoder {
  encode(input = '') {
    const text = unescape(encodeURIComponent(String(input)))
    const bytes = []
    for (let i = 0; i < text.length; i++) {
      const char = text[i]
      if (char >= '0' && char <= '9') {
        bytes.push(48 + '0123456789'.indexOf(char))
      } else {
        bytes.push(text.charCodeAt(i))
      }
    }
    return new Uint8Array(bytes)
  }
}
class TextDecoder {
  decode(input = []) {
    const text = Array.from(input).map((byte) => String.fromCharCode(byte)).join('')
    return decodeURIComponent(escape(text))
  }
}
const body_from = (value = '') => {
  if (value instanceof Uint8Array) return new Uint8Array(value)
  if (value instanceof ArrayBuffer) return new Uint8Array(value)
  return String(value)
}
const body_bytes = (value = '') => {
  return value instanceof Uint8Array ? value : new TextEncoder().encode(value)
}

const body_text = (value = '') => {
  return value instanceof Uint8Array ? new TextDecoder().decode(value) : String(value)
}

const setTimeout = (_callback, _ms = 0) => 0
const clearTimeout = (_id) => {}

globalThis.Headers = Headers
globalThis.Request = Request
globalThis.Response = Response
globalThis.Blob = Blob
globalThis.AbortController = AbortController
globalThis.AbortSignal = AbortSignal
globalThis.EventTarget = EventTarget
globalThis.TextEncoder = TextEncoder
globalThis.TextDecoder = TextDecoder
globalThis.setTimeout = setTimeout
globalThis.clearTimeout = clearTimeout
globalThis.process = { env: {} }
globalThis.global = globalThis
globalThis.ic = {
  caller: () => 'anonymous',
  time: () => BigInt(Date.now()) * 1000000n,
  canisterId: () => 'local-canister'
}
globalThis.console = {
  log: () => {},
  error: (...args) => {
    globalThis.__ic_edge_console_error = args.map((arg) => {
      return arg && arg.stack ? arg.stack : String(arg)
    }).join(' ')
  },
  warn: () => {},
  info: () => {}
}

"#;
