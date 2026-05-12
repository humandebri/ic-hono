//! `crates/ic-edge-runtime` installs async fetch queue glue for wasm QuickJS.
//! JS keeps Promise resolvers while Rust performs canister HTTPS outcalls.

pub const SOURCE: &str = r#"
globalThis.__ic_edge_fetch_requests = []
globalThis.__ic_edge_fetch_pending = {}
globalThis.__ic_edge_fetch_next_id = 1

globalThis.fetch = (input, init = {}) => {
  const signal = init.signal || (input instanceof Request ? input.signal : undefined)
  if (signal && signal.aborted) {
    return Promise.reject(new Error('This operation was aborted'))
  }
  const isRequest = input instanceof Request
  const url = isRequest ? input.url : String(input && input.href ? input.href : input)
  const method = String(init.method || (isRequest ? input.method : 'GET')).toUpperCase()
  const headers = new Headers(input instanceof Request ? input.headers : [])
  if (init.headers) {
    for (const [name, value] of new Headers(init.headers)) headers.set(name, value)
  }
  const hasInitBody = Object.prototype.hasOwnProperty.call(init, 'body')
  if ((method === 'GET' || method === 'HEAD') && hasInitBody && init.body !== null && init.body !== undefined) {
    return Promise.reject(new TypeError('Request with GET/HEAD method cannot have body'))
  }
  const body = hasInitBody ? init.body : (isRequest ? consume_body(input) : '')
  const bodyValue = Array.from(body_bytes(body_from(body)))
  const id = globalThis.__ic_edge_fetch_next_id++
  globalThis.__ic_edge_fetch_requests.push({
    id,
    method,
    url,
    headers: headers.entriesArray(),
    body: bodyValue
  })
  return new Promise((resolve, reject) => {
    globalThis.__ic_edge_fetch_pending[id] = { resolve, reject }
  })
}

globalThis.__ic_edge_take_fetch_requests = () => {
  const requests = globalThis.__ic_edge_fetch_requests
  globalThis.__ic_edge_fetch_requests = []
  return JSON.stringify(requests)
}

globalThis.__ic_edge_resolve_fetch = (id, responseJson) => {
  const pending = globalThis.__ic_edge_fetch_pending[id]
  if (!pending) return
  delete globalThis.__ic_edge_fetch_pending[id]
  const response = JSON.parse(responseJson)
  const responseBody = Array.isArray(response.body) ? new Uint8Array(response.body) : response.body
  pending.resolve(new Response(responseBody, {
    status: response.status,
    headers: response.headers
  }))
}

globalThis.__ic_edge_reject_fetch = (id, message) => {
  const pending = globalThis.__ic_edge_fetch_pending[id]
  if (!pending) return
  delete globalThis.__ic_edge_fetch_pending[id]
  pending.reject(new Error(message))
}
"#;
