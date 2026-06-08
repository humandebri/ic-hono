# Hono x402 Paid API

Canister-friendly Hono port of the official x402 V2 custom server pattern.

It uses `@x402/core/server`, `@x402/evm/exact/server`, manual `PAYMENT-REQUIRED` / `PAYMENT-SIGNATURE` / `PAYMENT-RESPONSE` handling, and `ic.audit` receipt logging instead of framework middleware. The default facilitator is a deterministic mock so local canister smoke can run without wallet keys or testnet USDC. Set `X402_FACILITATOR_URL` to use an HTTP facilitator.

```bash
npm install
npm run build
cargo run -p ic-edge-pack --bin ic-edge -- upload dist/app.qjbc --canister edge --environment local
```

Routes:

- `GET /free/catalog`: route catalog with endpoint, price, payee, and canonical resource
- `GET /demo/payment-signature?endpoint=/paid/report`: local mock `PAYMENT-SIGNATURE` value
- `GET /paid/report`: paid canister report, default `$0.001`
- `GET /paid/outcall?url=https://example.com/`: paid replicated HTTPS outcall, default `$0.003`
- `GET /receipts`: canister-local settlement receipts
- `GET /receipts/:id`: one committed receipt
- `GET /audit/root`: latest audit hash root and event count
- `GET /audit/events`: bounded audit event list

Route catalog environment:

- `X402_REPORT_PRICE`: report price, default `$0.001`
- `X402_OUTCALL_PRICE`: outcall price, default `$0.003`
- `X402_REPORT_PAY_TO`: report recipient, falls back to `X402_PAY_TO`
- `X402_OUTCALL_PAY_TO`: outcall recipient, falls back to `X402_PAY_TO`
- `X402_PAY_TO`: shared recipient fallback before the demo default

Security notes:

- Paid results are returned only after verify and settle succeed.
- Receipt IDs bind `endpoint + PAYMENT-SIGNATURE` and reject replay.
- `reserve` happens before verify/settle so duplicate signatures cannot settle twice.
- Receipt entries store product ID, price, payee, result digest, `payerHash`, transaction, network, amount, canister ID, and canonical resource.
- Paid responses use `Cache-Control: no-store`.
