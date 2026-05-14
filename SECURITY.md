# Security Policy

## Supported Versions

`ic-edge-workers` is currently a v1 preview runtime contract with crate semver `0.2.0`.

Security fixes target the latest `main` branch until formal release branches exist.

## Reporting a Vulnerability

Do not open a public issue for a vulnerability.

Report privately to the repository owner through GitHub private vulnerability reporting when available. If GitHub private reporting is not enabled, contact the maintainer directly before publishing details.

## Scope

Security-sensitive areas include:

- canister controller-only APIs
- bundle upload and rollback
- `process.env` secret injection
- HTTPS outcall request construction
- QuickJS host callbacks
- canister-local stable Cache
- resource limit enforcement

## Non-Goals

The runtime does not provide:

- browser sandbox isolation
- DOM security model
- Node.js native addon support
- managed platform identity, KV, R2, Durable Objects, or CDN cache semantics
