//! `crates/ic-edge-runtime` installs the Worker Cache API subset.
//! Rust callbacks own persistence, while JS keeps the Web-shaped surface.

pub const SOURCE: &str = r#"
const __ic_edge_cache_key = (input, options = {}) => {
  const request = input instanceof Request ? input : new Request(input)
  if (request.method !== 'GET' && !options.ignoreMethod) return null
  return new URL(request.url).href
}

const __ic_edge_cache_response_json = async (response) => {
  const bodyText = await response.text()
  const bodyValue = response.body instanceof Uint8Array
    ? Array.from(response.body)
    : bodyText
  const expiresAt = __ic_edge_cache_expires_at(response.headers.get('cache-control'))
  return JSON.stringify({
    status: response.status,
    headers: response.headers.entriesArray(),
    body: bodyValue,
    expiresAt
  })
}

const __ic_edge_cache_now_ms = () => {
  if (globalThis.ic && typeof globalThis.ic.time === 'function') {
    return Number(globalThis.ic.time() / 1000000n)
  }
  return Date.now()
}

const __ic_edge_cache_expires_at = (cacheControl) => {
  if (!cacheControl) return null
  const match = String(cacheControl).match(/(?:^|,)\s*max-age\s*=\s*(\d+)\s*(?:,|$)/i)
  if (!match) return null
  return __ic_edge_cache_now_ms() + Number(match[1]) * 1000
}

class Cache {
  constructor(name) {
    this.name = String(name)
  }
  async match(input, options = {}) {
    const key = __ic_edge_cache_key(input, options)
    if (key === null) return undefined
    const responseJson = globalThis.__ic_edge_cache_match(this.name, key)
    if (!responseJson) return undefined
    const response = JSON.parse(responseJson)
    if (typeof response.expiresAt === 'number' && response.expiresAt <= __ic_edge_cache_now_ms()) {
      globalThis.__ic_edge_cache_delete(this.name, key)
      return undefined
    }
    const body = Array.isArray(response.body) ? new Uint8Array(response.body) : response.body
    return new Response(body, { status: response.status, headers: response.headers })
  }
  async put(input, response) {
    const request = input instanceof Request ? input : new Request(input)
    if (request.method !== 'GET') throw new Error('cache.put only supports GET requests')
    if (response.status === 206) throw new Error('cache.put does not support 206 responses')
    if (response.headers.get('vary') === '*') throw new Error('cache.put does not support Vary: *')
    if (response.headers.has('set-cookie')) throw new Error('cache.put does not support Set-Cookie')
    const key = __ic_edge_cache_key(request)
    globalThis.__ic_edge_cache_put(this.name, key, await __ic_edge_cache_response_json(response))
  }
  async delete(input, options = {}) {
    const key = __ic_edge_cache_key(input, options)
    if (key === null) return false
    return globalThis.__ic_edge_cache_delete(this.name, key)
  }
}

globalThis.Cache = Cache
globalThis.caches = {
  default: new Cache('default'),
  open: async (name) => new Cache(name)
}
"#;
