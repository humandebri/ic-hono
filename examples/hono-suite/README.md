# Hono Suite

Full practical Hono example for `ic-edge`.

It combines SSR HTML, JSON APIs, zod validation, jose JWT signing and verification, Cache API state, audit log storage, Web Crypto, and replicated HTTPS checks.

```bash
npm install
npm run build
cargo run -p ic-edge-pack --bin ic-edge -- upload dist/app.bundle.js --canister edge --environment local
```
