# Commissioning: service-control pre-write recovery gate

Date: 2026-09-30
Agent: codex
Scope: K12 / ADR-0051, offline only; no web files or KNX gateway contacted.

## Finding

The published CLI and HTTP entry points used an in-memory property read before
changing `PID_SERVICE_CONTROL` bit 2. On a client crash or ambiguous readback,
there was no durable original property value to aid recovery. The Settings UI
owner therefore deferred the Debug write action. This gap concerns a two-byte
property element, not the 070nh application-memory image, and does not justify
claiming a full-device backup without evidence of other affected regions.

## Change

- The management-session setter now requires a pre-write callback after
  validating the mask, reading the full property and deciding a write is
  necessary. Callback failure is a distinct error and sends no property write.
- `knx-app::service_control_backup` creates a versioned owner-only JSON file
  containing target address, mask, object/PID and both original property
  octets. The file is new-only, synced, parsed/read back and compared, and its
  directory entry synced before the callback returns. No-op skips the backup.
- Both CLI and HTTP call this writer inside the connected session. CLI defaults
  to `./device-backups/` or `--backup-dir`; the API stores under its data dir
  and returns `backupPath` on success or includes it after a later failure.
  Server opt-in, device phrase and tunnel locks remain unchanged.
- ADR-0051, KNOWN_LIMITATIONS §139, goal-commission and status now distinguish
  property-only recovery evidence from a full device image and from the K13
  reset's separate complete-backup gate. No automatic rollback is claimed.

## Verification

- TDD: `failed_prewrite_backup_refuses_the_property_write` initially failed to
  compile against the old setter. Focused net, application backup, CLI and
  HTTP simulator suites then passed; blocked backup directories refuse before
  any property write. The successful HTTP test reads the saved pre-write
  record and matches its octets and mask to the response.
- `cargo check -p knx-cli -p knx-server` and strict Clippy over all four
  affected crates/all targets passed. `cargo fmt --all --check` passed.
- Corpus-enabled `cargo test --workspace`: 136 suites, 2,766 passed,
  0 failed, 160 ignored, no `SKIP:` output. Explicit house-readiness
  and download-coverage corpus tests both passed (1 + 1). The larger
  coverage test took about 14 minutes on this host; the first run was
  stopped by a too-short 360-second tool timeout and the subsequent
  unbounded background run passed. No product counts were re-pinned.
- Fresh-worktree `xtask`: anchors, headers, layering and corpus gates passed.
  After rebasing onto the concurrently published U12 monitor work, the
  integrated workspace again passed 136 suites / 2,766 tests / 0 failures /
  160 ignored / 0 `SKIP:`; full Web 78 files / 1,258 tests and build passed;
  workspace strict Clippy, fmt, all four xtask checks and diff check passed.
- **No new live hardware evidence.** Restore of potential manufacturer-side
  effects remains unknown; the recorded two bytes alone do not prove them
  absent. Only the operator may approve another device-specific live write.

## Next

UI owner can reassess the explicitly gated Debug action using the new
property-only backup path; the full-device K13 HTTP/restore workflow remains
blocked on its own backup contract. A global bus-activity contract and
partial-download selector remain separate UI/server work packages.
