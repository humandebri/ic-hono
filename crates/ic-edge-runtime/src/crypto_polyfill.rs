//! `crates/ic-edge-runtime` embeds crypto and base64 JS shims.
//! Host callbacks provide the cryptographic primitives.

pub const SOURCE: &str = r#"
const __ic_edge_base64_chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

class CryptoKey {
  constructor(keyData, algorithm, usages) {
    this.keyData = keyData
    this.algorithm = algorithm
    this.usages = usages
    this.type = 'secret'
    this.extractable = false
  }
}

const __ic_edge_bytes = (value) => {
  if (value instanceof ArrayBuffer) return Array.from(new Uint8Array(value))
  if (ArrayBuffer.isView(value)) return Array.from(new Uint8Array(value.buffer, value.byteOffset, value.byteLength))
  return Array.from(new TextEncoder().encode(String(value)))
}

const __ic_edge_json_bytes = (value) => JSON.stringify(__ic_edge_bytes(value))

const __ic_edge_from_json_bytes = (value) => {
  return new Uint8Array(JSON.parse(value)).buffer
}

const __ic_edge_is_integer_typed_array = (value) => {
  if (!ArrayBuffer.isView(value) || value instanceof DataView) return false
  if (value instanceof Float32Array || value instanceof Float64Array) return false
  return true
}

globalThis.atob = (value) => {
  const clean = String(value).replace(/=+$/, '')
  let output = ''
  let buffer = 0
  let bits = 0
  for (const char of clean) {
    const index = __ic_edge_base64_chars.indexOf(char)
    if (index < 0) throw new Error('Invalid base64')
    buffer = (buffer << 6) | index
    bits += 6
    if (bits >= 8) {
      bits -= 8
      output += String.fromCharCode((buffer >> bits) & 0xff)
    }
  }
  return output
}

globalThis.btoa = (value) => {
  let output = ''
  let buffer = 0
  let bits = 0
  for (const char of String(value)) {
    buffer = (buffer << 8) | (char.charCodeAt(0) & 0xff)
    bits += 8
    while (bits >= 6) {
      bits -= 6
      output += __ic_edge_base64_chars[(buffer >> bits) & 0x3f]
    }
  }
  if (bits > 0) output += __ic_edge_base64_chars[(buffer << (6 - bits)) & 0x3f]
  while (output.length % 4) output += '='
  return output
}

const crypto = {
  getRandomValues: (array) => {
    if (!__ic_edge_is_integer_typed_array(array)) {
      throw new TypeError('crypto.getRandomValues requires an integer TypedArray')
    }
    if (array.byteLength > 65536) {
      throw new Error('crypto.getRandomValues exceeds 65536 bytes')
    }
    const bytes = JSON.parse(globalThis.__ic_edge_crypto_random(array.byteLength))
    new Uint8Array(array.buffer, array.byteOffset, array.byteLength).set(bytes)
    return array
  },
  subtle: {
    digest: (algorithm, data) => {
      const name = typeof algorithm === 'string' ? algorithm : algorithm.name
      return Promise.resolve(__ic_edge_from_json_bytes(
        globalThis.__ic_edge_crypto_digest(name, __ic_edge_json_bytes(data))
      ))
    },
    importKey: (format, keyData, algorithm, _extractable, usages) => {
      if (format !== 'raw') return Promise.reject(new Error('only raw keys are supported'))
      return Promise.resolve(new CryptoKey(__ic_edge_bytes(keyData), algorithm, usages))
    },
    sign: (algorithm, key, data) => {
      const name = typeof algorithm === 'string' ? algorithm : algorithm.name
      return Promise.resolve(__ic_edge_from_json_bytes(
        globalThis.__ic_edge_crypto_sign(name, JSON.stringify(key.keyData), __ic_edge_json_bytes(data))
      ))
    },
    verify: (algorithm, key, signature, data) => {
      const name = typeof algorithm === 'string' ? algorithm : algorithm.name
      return Promise.resolve(globalThis.__ic_edge_crypto_verify(
        name,
        JSON.stringify(key.keyData),
        __ic_edge_json_bytes(signature),
        __ic_edge_json_bytes(data)
      ))
    }
  }
}

globalThis.crypto = crypto
globalThis.CryptoKey = CryptoKey
"#;
