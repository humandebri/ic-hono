//! `crates/ic-edge-runtime` wires host fetch and app dispatch into the Web API shim.
//! Keeping dispatch separate lets the core Web API subset stay compact.

pub const SOURCE: &str = r#"
const __ic_edge_body_json = (body) => JSON.stringify(Array.from(body_bytes(body_from(body))))
const __ic_edge_body_from_json = (bodyJson) => new Uint8Array(JSON.parse(bodyJson))

globalThis.fetch = (input, init = {}) => {
  const signal = init.signal || (input instanceof Request ? input.signal : undefined)
  if (signal && signal.aborted) {
    return Promise.reject(new Error('This operation was aborted'))
  }
  if (!globalThis.__ic_edge_host_fetch) {
    return Promise.reject(new Error('fetch is not configured'))
  }
  const isRequest = input instanceof Request
  const url = isRequest ? input.url : String(input && input.href ? input.href : input)
  const method = String(init.method || (isRequest ? input.method : 'GET')).toUpperCase()
  const headers = new Headers(input instanceof Request ? input.headers : [])
  if (init.headers) {
    for (const [name, value] of new Headers(init.headers)) headers.set(name, value)
  }
  const body = init.body || (isRequest ? input.body : '')
  const responseJson = globalThis.__ic_edge_host_fetch(
    method,
    url,
    JSON.stringify(headers.entriesArray()),
    __ic_edge_body_json(body)
  )
  const response = JSON.parse(responseJson)
  const responseBody = Array.isArray(response.body) ? new Uint8Array(response.body) : response.body
  return Promise.resolve(new Response(responseBody, {
    status: response.status,
    headers: response.headers
  }))
}

globalThis.__ic_edge_dispatch = (method, url, headersJson, bodyJson) => {
  const requestUrl = url.startsWith('http://') || url.startsWith('https://')
    ? url
    : `https://ic-edge.local${url.startsWith('/') ? url : `/${url}`}`
  const request = new Request(requestUrl, { method, headers: JSON.parse(headersJson), body: __ic_edge_body_from_json(bodyJson) })
  globalThis.__ic_edge_output = undefined
  globalThis.__ic_edge_error = undefined
  globalThis.__ic_edge_console_error = undefined
  Promise.resolve(globalThis.__ic_edge_app.fetch(request)).then((response) => {
    return Promise.resolve(response).then((response) => {
      globalThis.__ic_edge_output = JSON.stringify({
        status: response.status,
        headers: response.headers.entriesArray(),
        body: Array.from(body_bytes(body_from(response.body)))
      })
    })
  }).catch((error) => {
    const message = error && error.message ? error.message : String(error)
    const stack = error && error.stack ? error.stack : ''
    globalThis.__ic_edge_error = `${error && error.name ? error.name : 'Error'}: ${message}\n${stack}`
  })
}
"#;
