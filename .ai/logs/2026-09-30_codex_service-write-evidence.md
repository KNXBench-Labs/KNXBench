# Service-control write activity — 2026-09-30

Agent: codex. Scope: server-only, offline/simulated; no Web source, real gateway, device write, access key, or credentials touched.

## Change

- Added a write-specific one-shot guard with typed terminal states: `noChange`, `notSent`, `effectUnverified`, `verified`, plus in-flight `running` and dropped-future `unknown`. `writeEvidence` carries only `backupRecorded` and `sendPossible`, not raw property bytes, mask, gateway, key, serial, or backup path.
- `POST /api/device/service-control` starts observing only after Debug opt-in, validated address, typed confirmation, key-plan and retained-session conflict checks. The protocol's pre-write callback marks backup evidence only after the exact property's durable backup and readback. Exact-octet response is required for `verified`; a later error is `effectUnverified`, not proof of a send or of no send. Cancellation remains `unknown`. The original route response and property-specific recovery record remain authoritative.
- Replaced the route-local tunnel macro with a shared reservation function so read/write use the same three-lock exclusion order but separate evidence semantics. Group and serial-address writes remain `untracked`; snapshot is still volatile/partial.
- Extended simulated HTTP tests for verified change, no-op, backup failure, transport failure before delivery and after simulator mutation, cancellation before connect and at the write send boundary, plus typed guard unit tests. RED phase: focused HTTP suite exited 101 before route integration. Mutations: removing the verified evidence guard and moving send-possible marking before backup each made their respective targeted test fail with exit 101; both mutations were restored and focused tests returned green.

## Verification

- Rust workspace: 137 suites, 2,782 passed, 0 failed, 161 ignored, no `SKIP:` marker. Fresh target directory used. Strict workspace Clippy and fmt passed.
- Initial Web dependencies/build and 80 Vitest files / 1,270 tests passed before the concurrent UI package landed. After rebasing on its published UI changes, Web build and 82 files / 1,295 tests passed; the full Rust suite, strict Clippy, fmt and all four repository gates passed again. No Web source was edited in this package.
- Repository check-layering, check-headers, check-anchors, check-corpus-gates and diff check passed. Feature commit `65c4f43476a06c136d2427262c99c163a1687c19` was fast-forward-pushed to `main` and the remote SHA was read back exactly.
- The first fresh-target workspace run failed only because this worktree had no `knx-web/dist` required by Tauri; `npm ci && npm run build` produced it, then full workspace passed. This is a build prerequisite, not a product failure.

## Boundaries

- `effectUnverified` means the send **may** have happened, not that a frame reached the bus. Activity is not persistent audit or recovery evidence, and neither the Debug setting nor telemetry grants live permission. The property backup is not a full device image.
- The serial-address HTTP write path still lacks durable pre-write recovery; group writes cannot claim receiver verification. The Web lock belongs to the UI/Ground-root session. K13/K14 remain separately gated; no live operation occurred here.
