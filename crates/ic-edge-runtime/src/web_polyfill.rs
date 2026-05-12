//! `crates/ic-edge-runtime` embeds the minimum JS Web API polyfill.
//! It is intentionally small until the Hono compatibility suite demands more.

pub const SOURCE: &str = r#"
class Headers {
  constructor(init = []) {
    this._values = []
    if (init instanceof Headers) {
      init = init.entriesArray()
    } else if (!Array.isArray(init)) {
      init = Object.entries(init)
    }
    for (const [name, value] of init) this.append(name, value)
  }
  append(name, value) {
    this._values.push([String(name).toLowerCase(), String(value)])
  }
  set(name, value) {
    const key = String(name).toLowerCase()
    this._values = this._values.filter(([item]) => item !== key)
    this._values.push([key, String(value)])
  }
  delete(name) {
    const key = String(name).toLowerCase()
    this._values = this._values.filter(([item]) => item !== key)
  }
  get(name) {
    const key = String(name).toLowerCase()
    const found = this._values.filter(([item]) => item === key).map(([, value]) => value)
    return found.length === 0 ? null : found.join(', ')
  }
  has(name) {
    const key = String(name).toLowerCase()
    return this._values.some(([item]) => item === key)
  }
  forEach(callback, thisArg = undefined) {
    for (const [name, value] of this._values) {
      callback.call(thisArg, value, name, this)
    }
  }
  entries() {
    return this._values[Symbol.iterator]()
  }
  keys() {
    return this._values.map(([name]) => name)[Symbol.iterator]()
  }
  values() {
    return this._values.map(([, value]) => value)[Symbol.iterator]()
  }
  getSetCookie() {
    return this._values.filter(([name]) => name === 'set-cookie').map(([, value]) => value)
  }
  [Symbol.iterator]() {
    return this._values[Symbol.iterator]()
  }
  entriesArray() {
    return this._values
  }
  get [Symbol.toStringTag]() {
    return 'Headers'
  }
}

class Request {
  constructor(input, init = {}) {
    const isRequest = input instanceof Request
    const hasBody = Object.prototype.hasOwnProperty.call(init, 'body')
    this.url = isRequest ? input.url : String(input)
    this.method = String(init.method || (isRequest ? input.method : 'GET')).toUpperCase()
    this.headers = init.headers ? new Headers(init.headers) : new Headers(isRequest ? input.headers : [])
    this._body = body_from(hasBody ? init.body : (isRequest ? input.body : ''))
    this.signal = init.signal || (isRequest ? input.signal : undefined)
    this.bodyUsed = false
  }
  text() {
    return Promise.resolve(body_text(consume_body(this)))
  }
  json() {
    return Promise.resolve(JSON.parse(body_text(consume_body(this))))
  }
  arrayBuffer() {
    return Promise.resolve(body_bytes(consume_body(this)).buffer)
  }
  get body() {
    return this._body
  }
  formData() {
    const contentType = this.headers.get('content-type') || ''
    if (!contentType.toLowerCase().split(';')[0].trim().endsWith('application/x-www-form-urlencoded')) {
      return Promise.reject(new TypeError('formData only supports application/x-www-form-urlencoded'))
    }
    const form = new FormData()
    const params = new URLSearchParams(body_text(consume_body(this)))
    for (const [name, value] of params) form.append(name, value)
    return Promise.resolve(form)
  }
  clone() {
    if (this.bodyUsed) throw new TypeError('Body has already been used')
    return new Request(this)
  }
  get [Symbol.toStringTag]() {
    return 'Request'
  }
}

class Response {
  constructor(body = '', init = {}) {
    this.status = init.status || 200
    this.statusText = init.statusText || ''
    this.headers = new Headers(init.headers || [])
    this._body = body_from(body === null ? '' : body)
    this.bodyUsed = false
    this.url = ''
    this.redirected = false
    this.type = 'default'
  }
  get ok() {
    return this.status >= 200 && this.status < 300
  }
  get body() {
    return this._body
  }
  text() {
    return Promise.resolve(body_text(consume_body(this)))
  }
  json() {
    return Promise.resolve(JSON.parse(body_text(consume_body(this))))
  }
  arrayBuffer() {
    return Promise.resolve(body_bytes(consume_body(this)).buffer)
  }
  formData() {
    const contentType = this.headers.get('content-type') || ''
    if (!contentType.toLowerCase().split(';')[0].trim().endsWith('application/x-www-form-urlencoded')) {
      return Promise.reject(new TypeError('formData only supports application/x-www-form-urlencoded'))
    }
    const form = new FormData()
    const params = new URLSearchParams(body_text(consume_body(this)))
    for (const [name, value] of params) form.append(name, value)
    return Promise.resolve(form)
  }
  clone() {
    if (this.bodyUsed) throw new TypeError('Body has already been used')
    return new Response(this._body, {
      status: this.status,
      statusText: this.statusText,
      headers: this.headers
    })
  }
  static json(value, init = {}) {
    const response = new Response(JSON.stringify(value), init)
    response.headers.set('content-type', 'application/json')
    return response
  }
  get [Symbol.toStringTag]() {
    return 'Response'
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
  if (value === null || value === undefined) return ''
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

const consume_body = (target) => {
  if (target.bodyUsed) throw new TypeError('Body has already been used')
  target.bodyUsed = true
  return target._body
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
