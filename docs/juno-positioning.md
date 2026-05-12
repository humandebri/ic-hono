# Juno Positioning

Durable Edge Runtime on ICP は Juno 代替ではない。

## Scope

目的は platform ではなく runtime kernel。

提供するもの:

- QuickJS-based execution
- Web Standards API subset
- canister-native bindings
- local bundle upload path
- package-compatible subset

提供しないもの:

- auth platform
- storage product
- hosting product
- app dashboard
- managed backend

## README Wording

推奨:

> Run Hono and Edge-compatible npm packages inside ICP canisters.

避ける表現:

- Node / Bun 全面互換を示す表現
- npm 全体の完全互換を示す表現
- serverless platform
- Juno の置換を示す表現
