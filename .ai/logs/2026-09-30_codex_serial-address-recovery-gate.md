# 2026-09-30 — Serial-address write recovery gate (codex)

## Scope and evidence

Feature `35b4f8f3fd67a94ee7208551f762f4cdbee42cbf` published to `origin/main` with exact remote SHA readback. The production CLI `knx device address-by-serial --confirm` and HTTP `POST /api/device-address/by-serial` now fail closed before opening a tunnel when the durable, complete pre-write recovery contract is unavailable. HTTP uses 412; CLI uses failure. A possible no-op cannot bypass the gate. Validation of address, phrase and serial remains first; read-only lookup, dry-run plan and simulated protocol procedure remain available. ADR-0057, RESEARCH §22, status, limitations and the commissioning goal describe the boundary. No Web source or hardware bus was touched; no keys or credentials recorded.

## Verification and review

- TDD: valid HTTP and CLI write requests were initially red (both exit 101) because they could reach the write path; the new pre-tunnel gate made both green.
- Self-review (no delegated subagent per user preference): IMPORTANT — recovery error suggested an unverified alternative programming path, corrected to a read-only lookup and explicit no-write statement (`crates/knx-app/src/serial_address_recovery.rs:14`). IMPORTANT — property-bit-2 integration checked refusal but not that the second request opened no additional tunnel; added an exact connector-count assertion (`apps/knx-server/tests/http_service_control.rs:397`). MINOR — duplicate CLI assertions and awkward documentation wrapping removed. No remaining blocking findings in the scoped diff.
- Final offline Rust workspace: 137 suites, 2,784 passed, 0 failed, 161 ignored, zero `SKIP:` markers. Final focused HTTP and CLI tests, strict workspace Clippy, fmt, layering, headers, anchors (405 links across 223 Markdown files), corpus-gate static check, and diff check all passed.
- Web dependencies, build and Vitest passed (82 test files / 1,295 tests); no Web source changed. A synthetic CLI plan exited 0 and a confirmed synthetic write exited 1 with the missing-backup diagnostic; neither request targeted real hardware.
- KNX Association-authored Architecture v3.0 §2.6 and the official Device Reader help were checked against the documented statements. Their network-interface/memory-range descriptions do not establish a complete manufacturer storage map for an address write.

## Remaining / handoff

Serial-address writes remain deliberately unavailable from production CLI/HTTP until per-device affected storage, durable pre-send backup/readback and recovery/abort can be demonstrated. The protocol function remains directly callable by custom Rust clients outside the application gate. `groupWrite` remains untracked and has no receiver verification; bus activity coverage stays partial. The Web lock remains with the separate UI session. No access key may be guessed, `1.1.220` may not be queried, and no new live write is authorized. K13 HTTP reset and K14 hardware reset remain blocked; do not mistake property-specific backups for whole-device backups. The next safe package is an offline, route-specific group-write evidence audit/implementation without editing locked Web files or touching the gateway.
