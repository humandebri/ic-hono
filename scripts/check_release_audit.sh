#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

node -e '
const fs = require("fs")
const audit = JSON.parse(fs.readFileSync("docs/release-audit.json", "utf8"))
if (!Array.isArray(audit.verified) || audit.verified.length === 0) {
  throw new Error("release audit has no verified evidence")
}
if (!Array.isArray(audit.pending)) {
  throw new Error("release audit pending evidence must be an array")
}
if (audit.complete === true) {
  if (audit.pending.length !== 0) {
    throw new Error("complete release audit must not contain pending evidence")
  }
  if (audit.current_blocker !== null) {
    throw new Error("complete release audit must not record a current blocker")
  }
} else if (audit.complete === false) {
  if (audit.pending.length === 0) {
    throw new Error("incomplete release audit must record pending evidence")
  }
  if (!audit.current_blocker) {
    throw new Error("incomplete release audit must record the current blocker")
  }
} else {
  throw new Error("release audit complete field must be boolean")
}
if (!Array.isArray(audit.optional) || audit.optional.length === 0) {
  throw new Error("release audit must record optional non-blocking evidence")
}
for (const item of audit.pending) {
  if (!item.requirement || !item.evidence_command) {
    throw new Error("pending evidence must name requirement and evidence command")
  }
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
console.log(`${audit.verified.length} verified, ${audit.pending.length} pending`)
'
