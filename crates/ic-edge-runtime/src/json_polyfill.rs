//! `crates/ic-edge-runtime` installs JSON stability patches.
//! The wasm QuickJS backend corrupts numeric JSON in some paths, so integer
//! stringification uses explicit decimal construction before TextEncoder sees it.

pub const SOURCE: &str = r#"
(() => {
  const originalStringify = JSON.stringify.bind(JSON)
  const digits = '0123456789'

  const integerToDecimal = (value) => {
    if (value === 0) return '0'
    const negative = value < 0
    let remaining = negative ? -value : value
    let output = ''
    while (remaining > 0) {
      const digit = remaining % 10
      output = digits[digit] + output
      remaining = (remaining - digit) / 10
    }
    return negative ? '-' + output : output
  }

  const stringifyValue = (value, seen) => {
    if (value === null) return 'null'
    if (typeof value === 'string') return originalStringify(value)
    if (typeof value === 'boolean') return value ? 'true' : 'false'
    if (typeof value === 'number') {
      if (!Number.isFinite(value)) return 'null'
      if (Number.isInteger(value)) return integerToDecimal(value)
      return originalStringify(value)
    }
    if (typeof value === 'bigint') throw new TypeError('Do not know how to serialize a BigInt')
    if (typeof value !== 'object') return undefined
    if (typeof value.toJSON === 'function') return stringifyValue(value.toJSON(), seen)
    if (seen.indexOf(value) !== -1) throw new TypeError('Converting circular structure to JSON')
    seen.push(value)
    if (Array.isArray(value)) {
      const items = value.map((item) => {
        const output = stringifyValue(item, seen)
        return output === undefined ? 'null' : output
      })
      seen.pop()
      return '[' + items.join(',') + ']'
    }
    const entries = []
    Object.keys(value).forEach((key) => {
      const output = stringifyValue(value[key], seen)
      if (output !== undefined) entries.push(originalStringify(key) + ':' + output)
    })
    seen.pop()
    return '{' + entries.join(',') + '}'
  }

  JSON.stringify = (value, replacer, space) => {
    if (replacer !== undefined || space !== undefined) return originalStringify(value, replacer, space)
    const output = stringifyValue(value, [])
    return output === undefined ? undefined : output
  }
})()
"#;
