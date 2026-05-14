# Release Evidence

Public release evidence lives here.

Mainnet preflight evidence files use `mainnet-preflight-YYYY-MM-DD.md` and must not include secrets.

Each evidence file should record:

- Date
- Command: `IC_EDGE_PREFLIGHT_EVIDENCE=docs/release-evidence/mainnet-preflight-YYYY-MM-DD.md IC_EDGE_MAINNET_PREFLIGHT=1 scripts/mainnet_preflight.sh`
- Identity principal
- `edge` canister status
- Wasm import audit result
- Confirmation that no secrets are included
