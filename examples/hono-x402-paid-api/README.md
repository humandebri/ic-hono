# Hono x402 Paid API

Canister-friendly Hono port of the official x402 V2 custom server pattern.

It uses `@x402/core/server`, `@x402/evm/exact/server`, manual `PAYMENT-REQUIRED` / `PAYMENT-SIGNATURE` / `PAYMENT-RESPONSE` handling, and `ic.audit` receipt logging instead of framework middleware. The default facilitator is a deterministic mock so local canister smoke can run without wallet keys or testnet USDC. Set `X402_FACILITATOR_URL`, `X402_PAY_TO`, and `X402_PRICE` to use an HTTP facilitator.

```bash
npm install
npm run build
cargo run -p ic-edge-pack --bin ic-edge -- upload dist/app.bundle.js --canister edge --environment local
```

Routes:

- `GET /free/catalog`: payment requirements metadata
- `GET /demo/payment-signature`: local mock `PAYMENT-SIGNATURE` value
- `GET /paid/report`: paid canister report
- `GET /paid/outcall?url=https://example.com/`: paid replicated HTTPS outcall
- `GET /receipts`: canister-local settlement receipts
- `GET /receipts/:id`: one committed receipt
- `GET /audit/root`: latest audit hash root and event count
- `GET /audit/events`: bounded audit event list

Security notes:

- Paid results are returned only after verify and settle succeed.
- Receipt IDs bind `endpoint + PAYMENT-SIGNATURE` and reject replay.
- `reserve` happens before verify/settle so duplicate signatures cannot settle twice.
- Receipt entries store result digest, `payerHash`, transaction, network, amount, canister ID, and canonical resource.
- Paid responses use `Cache-Control: no-store`.
