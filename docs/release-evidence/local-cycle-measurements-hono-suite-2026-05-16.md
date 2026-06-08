# Local Cycle Measurements: Hono Suite

- date: 2026-05-16
- canister: `edge`
- environment: `local`
- app entry: `examples/hono-suite/src/app.ts`
- bundle: `examples/hono-suite/dist/app.bundle.js`
- bundle size: 119,560 bytes raw / 34,361 bytes gzip
- repeats: 5; short HTTPS/upload repeats: 3
- delta: `before_cycles - after_cycles - idle_burn_elapsed`
- pricing conversion: `1T cycles = $1.33`
- scope: local PocketIC/icp-cli environment; not mainnet pricing

`hono-suite` includes Hono middleware, zod validation, jose HS256 JWT sign/verify, Web Crypto SHA-256, Cache API storage, audit log state, SSR HTML, and replicated HTTPS fetch.

| operation | repeats | median cycles | USD | avg cycles | min | max | samples |
|---|---:|---:|---:|---:|---:|---:|---|
| `abort_bundle_upload` missing | 5 | 9,103,351 | $0.000012 | 9,116,521 | 9,103,247 | 9,169,373 | 9169373, 9103247, 9103351, 9103351, 9103284 |
| `set_env` replace 16B | 5 | 75,606,532 | $0.000101 | 68,341,681 | 51,149,954 | 75,633,061 | 51149954, 63708847, 75610013, 75606532, 75633061 |
| `GET /api/health` cold after generation change | 3 | 240,354,320 | $0.000320 | 240,281,105 | 239,952,551 | 240,536,444 | 239952551, 240536444, 240354320 |
| `GET /api/health` warm | 5 | 45,929,244 | $0.000061 | 45,955,418 | 45,905,870 | 46,054,791 | 45905870, 45918040, 45929244, 45969149, 46054791 |
| `GET /api/incidents` | 5 | 40,993,993 | $0.000055 | 40,978,091 | 40,934,935 | 41,007,185 | 40993993, 40934935, 40952379, 41001966, 41007185 |
| `GET /` HTML page | 5 | 205,158,959 | $0.000273 | 205,054,131 | 202,737,116 | 207,199,677 | 202737116, 204070562, 205158959, 206104344, 207199677 |
| `GET /demo` Cache write | 5 | 110,508,840 | $0.000147 | 110,523,582 | 102,498,969 | 118,604,409 | 102498969, 106448601, 110508840, 114557095, 118604409 |
| `POST /api/incidents` | 5 | 149,243,137 | $0.000198 | 149,301,544 | 132,574,077 | 166,111,978 | 132574077, 140880592, 149243137, 157697937, 166111978 |
| `POST /api/incidents/:id/resolve` | 5 | 595,254,814 | $0.000792 | 595,514,457 | 515,776,244 | 675,906,578 | 515776244, 555309882, 595254814, 635324771, 675906578 |
| direct `upload_bundle` 119KB | 3 | 307,805,595 | $0.000409 | 305,620,731 | 301,142,295 | 307,914,303 | 301142295, 307914303, 307805595 |
| `GET /api/check` warm + replicated fetch | 3 | 954,397,936 | $0.001269 | 954,412,302 | 950,088,499 | 958,750,473 | 950088499, 954397936, 958750473 |
| direct `fetch_outcall` non-replicated | 3 | 746,832,512 | $0.000993 | 746,832,059 | 746,830,427 | 746,833,240 | 746832512, 746830427, 746833240 |
| direct `fetch_outcall_replicated` | 3 | 746,852,781 | $0.000993 | 746,880,250 | 746,852,672 | 746,935,298 | 746852781, 746852672, 746935298 |

Validation:

- `npm install`, TypeScript check, and bundle build passed in `examples/hono-suite`.
- Measurement was rerun from a local `icp deploy edge --mode reinstall --yes` empty state because stateful routes depend on stored incident/audit list sizes.
- Local canister `http_request_update` smoke passed for `/api/health`, `/api/session`, `/api/crypto`, `/demo`, `/api/report`, and `/api/check?url=https%3A%2F%2Fexample.com%2F`.
- Gateway HTML loaded at `http://t63gs-up777-77776-aaaba-cai.localhost:8000/`.
- `playwright-cli` snapshot passed after adding `/favicon.ico`.
- PBT found and fixed credential-bearing `/api/check` URL acceptance in the suite validator.
- `cargo fuzz run hono_suite_api --sanitizer none -- -max_total_time=10 -max_len=512` completed 702 runs.
- One earlier 5-repeat measurement attempt trapped during cold QuickJS crypto callback installation after deploy; a 1-repeat run and the final empty-state 5-repeat run completed.
