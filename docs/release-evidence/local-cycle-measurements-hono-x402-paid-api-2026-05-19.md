# Hono x402 Paid API Local Cycle Measurements

- canister: edge
- environment: local
- app entry: examples/hono-x402-paid-api/src/app.ts
- bundle: examples/hono-x402-paid-api/dist/app.bundle.js
- bundle size: 295,853 bytes raw / 55,229 bytes gzip
- repeats: 3
- date: 2026-05-19
- delta: `before_cycles - after_cycles - idle_burn_elapsed`
- scope: local PocketIC/icp-cli environment; not mainnet pricing
- pricing: `1T cycles = $1.33`

| operation | repeats | median cycles | median USD | avg cycles | avg USD | min | max | samples |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| GET /free/catalog | 3 | 35,224,816 | $0.000047 | 120,570,806 | $0.000160 | 35,195,030 | 291,292,573 | 291,292,573, 35,195,030, 35,224,816 |
| GET /paid/report unpaid | 3 | 60,817,245 | $0.000081 | 60,788,571 | $0.000081 | 60,715,854 | 60,832,616 | 60,715,854, 60,832,616, 60,817,245 |
| GET /paid/report paid | 3 | 144,257,763 | $0.000192 | 144,346,364 | $0.000192 | 143,995,489 | 144,785,842 | 143,995,489, 144,257,763, 144,785,842 |
| GET /paid/report replay rejected | 3 | 101,106,583 | $0.000134 | 101,109,023 | $0.000134 | 101,020,206 | 101,200,280 | 101,106,583, 101,020,206, 101,200,280 |
| GET /receipts | 3 | 223,752,264 | $0.000298 | 223,783,211 | $0.000298 | 223,588,141 | 224,009,230 | 223,588,141, 223,752,264, 224,009,230 |
| GET /audit/root | 3 | 22,723,443 | $0.000030 | 22,774,739 | $0.000030 | 22,711,272 | 22,889,502 | 22,711,272, 22,889,502, 22,723,443 |
| GET /paid/outcall paid replicated | 3 | 879,461,732 | $0.001170 | 879,397,580 | $0.001170 | 879,247,569 | 879,483,441 | 879,461,732, 879,483,441, 879,247,569 |

Notes:

- Measurement script: `scripts/measure_x402_cycles.mjs`.
- The canister was reinstalled before measurement to start with an empty audit log.
- Paid operation setup requests `/demo/payment-signature` before each measured paid call; setup cost is excluded.
- The first `/free/catalog` sample includes runtime cold-start effects and explains the higher average.
