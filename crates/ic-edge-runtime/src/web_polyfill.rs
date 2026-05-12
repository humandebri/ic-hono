//! `crates/ic-edge-runtime` embeds the minimum JS Web API polyfill.
//! It is intentionally small until the Hono compatibility suite demands more.

pub const SOURCE: &str = r#"
const __ic_edge_header_token = /^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/
const __ic_edge_validate_header_name = (name) => {
  const value = String(name)
  if (!__ic_edge_header_token.test(value)) throw new TypeError('Invalid header name')
  return value.toLowerCase()
}
const __ic_edge_validate_header_value = (value) => {
  const text = String(value)
  if (/[\r\n\0]/.test(text)) throw new TypeError('Invalid header value')
  return text
}
const __ic_edge_is_response = (value) => {
  try {
    return Boolean(value && typeof value === 'object' && value.__ic_edge_response === true)
  } catch (_error) {
    return false
  }
}

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
    this._values.push([__ic_edge_validate_header_name(name), __ic_edge_validate_header_value(value)])
  }
  set(name, value) {
    const key = __ic_edge_validate_header_name(name)
    this._values = this._values.filter(([item]) => item !== key)
    this._values.push([key, __ic_edge_validate_header_value(value)])
  }
  delete(name) {
    const key = __ic_edge_validate_header_name(name)
    this._values = this._values.filter(([item]) => item !== key)
  }
  get(name) {
    const key = __ic_edge_validate_header_name(name)
    const found = this._values.filter(([item]) => item === key).map(([, value]) => value)
    return found.length === 0 ? null : found.join(', ')
  }
  has(name) {
    const key = __ic_edge_validate_header_name(name)
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
    if (isRequest && !hasBody && input.bodyUsed) throw new TypeError('Body has already been used')
    if ((this.method === 'GET' || this.method === 'HEAD') && hasBody && init.body !== null && init.body !== undefined) {
      throw new TypeError('Request with GET/HEAD method cannot have body')
    }
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
    return Promise.resolve(body_array_buffer(consume_body(this)))
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
    if (init === null || init === undefined) init = {}
    const isResponse = __ic_edge_is_response(body)
    const hasStatus = Object.prototype.hasOwnProperty.call(init, 'status') && init.status !== undefined
    const hasStatusText = Object.prototype.hasOwnProperty.call(init, 'statusText') && init.statusText !== undefined
    this.status = isResponse
      ? (hasStatus ? Number(init.status) : body.status)
      : (hasStatus ? Number(init.status) : 200)
    if (!Number.isInteger(this.status) || this.status < 200 || this.status > 599) {
      throw new RangeError('Response status must be in the range 200 to 599')
    }
    this.statusText = isResponse
      ? (hasStatusText ? init.statusText : body.statusText)
      : (init.statusText || '')
    this.headers = new Headers(isResponse ? (init.headers || body.headers) : (init.headers || []))
    this._body = body_from(body === null ? '' : (isResponse ? body.body : body))
    this.__ic_edge_response = true
    this.bodyUsed = false
    this.url = isResponse ? body.url : ''
    this.redirected = isResponse ? body.redirected : false
    this.type = isResponse ? body.type : 'default'
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
    return Promise.resolve(body_array_buffer(consume_body(this)))
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
    if (!response.headers.has('content-type')) response.headers.set('content-type', 'application/json')
    return response
  }
  get [Symbol.toStringTag]() {
    return 'Response'
  }
}

class Blob {
  constructor(parts = [], init = {}) {
    this.type = String(init.type || '')
    this._body = body_concat(parts.map((part) => body_from(part)))
    this.size = body_bytes(this._body).byteLength
  }
  text() {
    return Promise.resolve(body_text(this._body))
  }
  arrayBuffer() {
    return Promise.resolve(body_array_buffer(this._body))
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
  constructor(_label = 'utf-8', options = {}) {
    this.fatal = Boolean(options.fatal)
  }
  decode(input = []) {
    const bytes = body_bytes(body_from(input))
    let output = ''
    for (let index = 0; index < bytes.length;) {
      const first = bytes[index]
      let codePoint = 0
      let needed = 0
      let minimum = 0
      if (first <= 0x7f) {
        output += String.fromCharCode(first)
        index += 1
        continue
      } else if (first >= 0xc2 && first <= 0xdf) {
        codePoint = first & 0x1f
        needed = 1
        minimum = 0x80
      } else if (first >= 0xe0 && first <= 0xef) {
        codePoint = first & 0x0f
        needed = 2
        minimum = 0x800
      } else if (first >= 0xf0 && first <= 0xf4) {
        codePoint = first & 0x07
        needed = 3
        minimum = 0x10000
      } else {
        if (this.fatal) throw new TypeError('Invalid UTF-8')
        output += '\ufffd'
        index += 1
        continue
      }
      if (index + needed >= bytes.length) {
        if (this.fatal) throw new TypeError('Invalid UTF-8')
        output += '\ufffd'
        index += 1
        continue
      }
      let valid = true
      for (let offset = 1; offset <= needed; offset++) {
        const byte = bytes[index + offset]
        if ((byte & 0xc0) !== 0x80) {
          valid = false
          break
        }
        codePoint = (codePoint << 6) | (byte & 0x3f)
      }
      if (!valid || codePoint < minimum || codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) {
        if (this.fatal) throw new TypeError('Invalid UTF-8')
        output += '\ufffd'
        index += 1
        continue
      }
      output += codePoint <= 0xffff
        ? String.fromCharCode(codePoint)
        : String.fromCharCode(0xd800 + ((codePoint - 0x10000) >> 10), 0xdc00 + ((codePoint - 0x10000) & 0x3ff))
      index += needed + 1
    }
    return output
  }
}
const body_from = (value = '') => {
  if (value === null || value === undefined) return ''
  if (value instanceof Blob) return new Uint8Array(value._body)
  if (value instanceof Uint8Array) return new Uint8Array(value)
  if (value instanceof ArrayBuffer) return new Uint8Array(value)
  if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength)
  return String(value)
}
const body_bytes = (value = '') => {
  return value instanceof Uint8Array ? value : new TextEncoder().encode(value)
}

const body_array_buffer = (value = '') => {
  const bytes = body_bytes(value)
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength)
}

const body_concat = (parts) => {
  const chunks = parts.map(body_bytes)
  const size = chunks.reduce((total, chunk) => total + chunk.byteLength, 0)
  const output = new Uint8Array(size)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
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
