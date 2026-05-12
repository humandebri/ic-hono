#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

node -e '
const fs = require("fs")
const audit = JSON.parse(fs.readFileSync("docs/release-audit.json", "utf8"))
if (audit.complete !== true) {
  throw new Error("release audit must be complete once every v1 release evidence item exists")
}
if (!Array.isArray(audit.verified) || audit.verified.length === 0) {
  throw new Error("release audit has no verified evidence")
}
if (!Array.isArray(audit.pending) || audit.pending.length !== 0) {
  throw new Error("release audit must not contain pending v1 release evidence")
}
if (audit.current_blocker !== null) {
  throw new Error("release audit must not record a current blocker")
}
if (!Array.isArray(audit.optional) || audit.optional.length === 0) {
  throw new Error("release audit must record optional non-blocking evidence")
}
for (const item of audit.optional) {
  if (item.release_blocker !== false) {
    throw new Error(`optional evidence must be non-blocking: ${item.requirement}`)
  }
}
if (!audit.success_command || !audit.success_command.includes("scripts/icp_local_smoke.sh")) {
  throw new Error("release audit must name the final smoke command")
}
if (!audit.success_command.includes("scripts/mainnet_preflight.sh")) {
  throw new Error("release audit must name the mainnet preflight gate")
}
if (!audit.success_command.includes("scripts/check_compatibility_matrix.sh")) {
  throw new Error("release audit must name the compatibility matrix gate")
}
if (!audit.success_command.includes("scripts/package_crates.sh")) {
  throw new Error("release audit must name the crate package gate")
}
console.log(`${audit.verified.length} verified, release audit complete`)
'
