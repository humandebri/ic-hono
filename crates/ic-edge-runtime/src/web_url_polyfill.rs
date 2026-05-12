//! `crates/ic-edge-runtime` provides URL and FormData Web API shims.
//! They are separate from the core body/headers shim to keep files small.

pub const SOURCE: &str = r#"
const __ic_edge_decode_query = (value) => decodeURIComponent(String(value).replace(/\+/g, ' '))

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
  [Symbol.iterator]() {
    return this.values[Symbol.iterator]()
  }
}

class URL {
  constructor(input, base = undefined) {
    const value = String(input)
    const absolute = /^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(value)
      ? value
      : `${String(base || 'https://ic-edge.local').replace(/\/$/, '')}/${value.replace(/^\//, '')}`
    const match = absolute.match(/^([^:]+):\/\/([^/?#]*)([^?#]*)(\?[^#]*)?(#.*)?$/)
    if (!match) throw new TypeError('Invalid URL')
    this.protocol = `${match[1]}:`
    this.host = match[2]
    this.hostname = match[2].split(':')[0]
    this.pathname = match[3] || '/'
    this.search = match[4] || ''
    this.hash = match[5] || ''
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
}

globalThis.URL = URL
globalThis.URLSearchParams = URLSearchParams
globalThis.FormData = FormData
"#;
