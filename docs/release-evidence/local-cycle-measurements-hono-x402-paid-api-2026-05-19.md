# Hono x402 Paid API Local Cycle Measurements

- canister: edge
- environment: local
- app entry: examples/hono-x402-paid-api/src/app.ts
- bundle: examples/hono-x402-paid-api/dist/app.bundle.js
- bundle size: 298,651 bytes raw / 55,646 bytes gzip
- repeats: 3
- date: 2026-05-19
- delta: `before_cycles - after_cycles - idle_burn_elapsed`
- scope: local PocketIC/icp-cli environment; not mainnet pricing
- pricing: `1T cycles = $1.33`

| operation | repeats | median cycles | median USD | avg cycles | avg USD | min | max | samples |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| GET /free/catalog | 3 | 54,567,042 | $0.000073 | 140,093,760 | $0.000186 | 54,556,086 | 311,158,154 | 311,158,154, 54,567,042, 54,556,086 |
| GET /paid/report unpaid | 3 | 62,867,345 | $0.000084 | 62,938,606 | $0.000084 | 62,860,442 | 63,088,033 | 63,088,033, 62,860,442, 62,867,345 |
| GET /paid/report paid | 3 | 151,836,330 | $0.000202 | 151,833,451 | $0.000202 | 151,466,429 | 152,197,594 | 151,466,429, 151,836,330, 152,197,594 |
| GET /paid/report replay rejected | 3 | 107,950,007 | $0.000144 | 107,882,487 | $0.000143 | 107,721,262 | 107,976,193 | 107,721,262, 107,976,193, 107,950,007 |
| GET /receipts | 3 | 249,256,691 | $0.000332 | 249,234,947 | $0.000331 | 249,074,984 | 249,373,167 | 249,074,984, 249,256,691, 249,373,167 |
| GET /audit/root | 3 | 22,812,779 | $0.000030 | 22,812,205 | $0.000030 | 22,796,464 | 22,827,372 | 22,796,464, 22,812,779, 22,827,372 |
| GET /paid/outcall paid replicated | 3 | 886,849,413 | $0.001180 | 886,830,084 | $0.001179 | 886,709,817 | 886,931,024 | 886,931,024, 886,849,413, 886,709,817 |

Notes:

- Measurement script: `scripts/measure_x402_cycles.mjs`.
- The canister was reinstalled before measurement to start with an empty audit log.
- Paid operation setup requests `/demo/payment-signature?endpoint=...` before each measured paid call; setup cost is excluded.
- The first `/free/catalog` sample includes runtime cold-start effects and explains the higher average.
