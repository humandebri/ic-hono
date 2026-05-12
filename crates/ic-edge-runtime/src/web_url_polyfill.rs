//! `crates/ic-edge-runtime` provides URL and FormData Web API shims.
//! They are separate from the core body/headers shim to keep files small.

pub const SOURCE: &str = r#"
const __ic_edge_decode_query = (value) => decodeURIComponent(String(value).replace(/\+/g, ' '))
const __ic_edge_parse_absolute_url = (value) => {
  const match = String(value).match(/^([^:]+):\/\/([^/?#]*)([^?#]*)(\?[^#]*)?(#.*)?$/)
  if (!match) throw new TypeError('Invalid URL')
  return {
    protocol: `${match[1]}:`,
    host: match[2],
    pathname: match[3] || '/',
    search: match[4] || '',
    hash: match[5] || ''
  }
}
const __ic_edge_url_origin = (url) => `${url.protocol}//${url.host}`
const __ic_edge_resolve_url = (input, base) => {
  const value = String(input)
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(value)) return value
  const baseUrl = __ic_edge_parse_absolute_url(base || 'https://ic-edge.local/')
  const origin = __ic_edge_url_origin(baseUrl)
  if (value.startsWith('/')) return `${origin}${value}`
  if (value.startsWith('?')) return `${origin}${baseUrl.pathname}${value}`
  if (value.startsWith('#')) return `${origin}${baseUrl.pathname}${baseUrl.search}${value}`
  const directory = baseUrl.pathname.endsWith('/')
    ? baseUrl.pathname
    : baseUrl.pathname.slice(0, baseUrl.pathname.lastIndexOf('/') + 1)
  return `${origin}${directory}${value}`
}

class URLSearchParams {
  constructor(init = undefined) {
    this.values = []
    if (typeof init === 'string') {
      const input = init.startsWith('?') ? init.slice(1) : init
      for (const part of input.split('&')) {
        if (part) {
          const index = part.indexOf('=')
          const name = index === -1 ? part : part.slice(0, index)
          const value = index === -1 ? '' : part.slice(index + 1)
          this.append(__ic_edge_decode_query(name), __ic_edge_decode_query(value))
        }
      }
    } else if (Array.isArray(init)) {
      for (const [name, value] of init) this.append(name, value)
    } else if (init && typeof init === 'object') {
      for (const [name, value] of Object.entries(init)) this.append(name, value)
    }
  }
  append(name, value) {
    this.values.push([String(name), String(value)])
  }
  get(name) {
    const key = String(name)
    const found = this.values.find(([item]) => item === key)
    return found ? found[1] : null
  }
  getAll(name) {
    const key = String(name)
    return this.values.filter(([item]) => item === key).map(([, value]) => value)
  }
  has(name) {
    const key = String(name)
    return this.values.some(([item]) => item === key)
  }
  delete(name) {
    const key = String(name)
    this.values = this.values.filter(([item]) => item !== key)
  }
  entries() {
    return this.values[Symbol.iterator]()
  }
  set(name, value) {
    const key = String(name)
    this.values = this.values.filter(([item]) => item !== key)
    this.append(key, value)
  }
  toString() {
    return this.values
      .map(([name, value]) => `${encodeURIComponent(name)}=${encodeURIComponent(value)}`)
      .join('&')
  }
  sort() {
    this.values.sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
  }
  forEach(callback, thisArg = undefined) {
    for (const [name, value] of this.values) {
      callback.call(thisArg, value, name, this)
    }
  }
  [Symbol.iterator]() {
    return this.values[Symbol.iterator]()
  }
  get [Symbol.toStringTag]() {
    return 'URLSearchParams'
  }
}

class URL {
  constructor(input, base = undefined) {
    const parsed = __ic_edge_parse_absolute_url(__ic_edge_resolve_url(input, base))
    this.protocol = parsed.protocol
    this.host = parsed.host
    this.hostname = parsed.host.split(':')[0]
    this.pathname = parsed.pathname
    this.search = parsed.search
    this.hash = parsed.hash
    this.searchParams = new URLSearchParams(this.search)
  }
  toString() {
    const search = this.searchParams.toString()
    return `${this.protocol}//${this.host}${this.pathname}${search ? `?${search}` : ''}${this.hash}`
  }
  get href() {
    return this.toString()
  }
}

class FormData {
  constructor() {
    this.values = []
  }
  append(name, value) {
    this.values.push([String(name), String(value)])
  }
  get(name) {
    const key = String(name)
    const found = this.values.find(([item]) => item === key)
    return found ? found[1] : null
  }
  entries() {
    return this.values[Symbol.iterator]()
  }
  [Symbol.iterator]() {
    return this.values[Symbol.iterator]()
  }
  get [Symbol.toStringTag]() {
    return 'FormData'
  }
}

globalThis.URL = URL
globalThis.URLSearchParams = URLSearchParams
globalThis.FormData = FormData
"#;
