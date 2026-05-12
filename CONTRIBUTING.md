# Contributing

Contributions are welcome.

## Development

Run the local gate before opening a pull request:

```bash
scripts/ci_local.sh
```

For changes that affect canister behavior, also run:

```bash
scripts/icp_local_smoke.sh
```

## Change Guidelines

- Keep changes small and easy to review.
- Prefer existing patterns over new abstractions.
- Add focused tests for changed behavior.
- Update docs when public behavior, limits, APIs, examples, or release gates change.
- Do not claim full Cloudflare Workers, Node.js, Bun, or npm compatibility.

## Pull Request Checklist

- `cargo fmt --all --check`
- `cargo test`
- `scripts/package_smoke.sh`
- `scripts/check_release_audit.sh`
- docs updated when behavior changed

## Versioning

The v1 label is the preview API/product contract. Crate semver remains `0.x` until the runtime contract is stable enough for `1.0.0`.
