# OWN-01 — Owner decisions and deferred items

**Status:** Decisions recorded; remaining evidence and owner review pending. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

## Decisions recorded — 27 October 2026

These decisions are not delivery evidence or blanket execution authority.

- [WP-11](WP-11-native-linux-acceptance.md): desktop control is authorised for
  bounded native acceptance with isolated fictional/simulator data. No bus
  access or real-project changes. The first bounded Alpha 17 slice is now
  [recorded](../status/2026-10-27-wp11-native-acceptance.md); WP-11 remains open.
- [S2-03](S2-03-up258-image-interpretation.md): retain the existing source-based
  communication-bit policy. Invisible-parameter precedence stays open; no ETS
  equivalence or hardware approval follows.
- [WP-10 / MODEL-05](WP-10-import-and-model-evidence.md): no current native-site
  need across separate installations; explicitly deferred, not implemented.
  Other import/model evidence cases remain independent.
- Root-only research and triage originals remain private; this resolves their
  former publish-or-keep-local question. A separately authored,
  publicly supportable summary may be prepared for review after confidentiality
  and provenance checks; publication is not authorised.
- CI stays manually disabled; established local gates remain authoritative.
  This is not an instruction to disable release, website or Docker Hub
  description workflows, or to reactivate CI.
- KL-185: accept the residual risk of `refs/pull/3` and `refs/pull/24` serving
  pre-rewrite commits. No GitHub Support request or other new contact.
- Technical leftovers may be considered for later cleanup only after fresh
  ownership, reachability and use checks. No cleanup or deletion is authorised
  now. Active worktrees, used symlinks, other tracks, private notes, backups and
  the parked `wip/manufacturer-plugin-research-20261013` branch remain protected.
  This folds in the old container/source-path and local
  `refs/kl185-rewritten/*` leftovers: none may be removed without that fresh
  ownership/reachability/use check; previously used paths are not presumed idle.
- Keep the private pre-launch rollback repository; it is not a cleanup target.
- Docker signing: plan a bounded design before the first stable release,
  covering identity, verification and recovery. No key creation, external login,
  publication or retrospective Alpha 17 change is authorised.
- Scheduled Docker base-image rebuilds remain deferred; reconsider before the
  first stable release. No automatic republication or silent version-tag reuse.
- Docker `latest`: from the first stable release, follow only the newest verified
  stable release. A separate prerelease channel's name and implementation remain
  future work. Until then the existing alpha rule is unchanged; no tag or
  workflow change is commissioned here.

## Already delivered foundation

Docker Hub description sync is already documented in
[ADR-0097](../adr/0097-docker-hub-release-image.md#consequences) and implemented in
[its separate workflow](../../.github/workflows/dockerhub-description.yml),
including exact public readback against the maintained source. It is not a new
owner-decision gap. This reconciliation does not claim a fresh workflow run or
live Docker Hub verification.

## Remaining evidence and owner review

- WP-11 has a first bounded native slice; the remaining cells and missing
  environments stay open, not accepted by consent alone.
- S2-03 needs defining evidence for invisible-parameter precedence; other
  import/model and hardware evidence boundaries are unchanged.
- Website / Evolution Story privacy acceptance requires a concrete review copy;
  no blanket privacy approval was given. Optional WebKit evidence remains open.
- Private mailbox verification remains a separate external/owner action and was
  not confirmed complete.
- Any public research summary still needs confidentiality/provenance review and
  explicit publication approval. The originals stay private.

## Deferred by recorded decision (reopen only on a new decision)

- ADR-0039 phases 3–5, sealing the ID allocator (KL-129).
- Package/application version pinning and a selector (KL-135).
- Live tunnel from a Docker bridge container (Route Back): only with a go.
- New real-hardware, power-loss, vendor and ETS validation for
  commissioning.
- Durable pre-write recovery for K6 button / serial / K13 address writes;
  every new write needs its own go.
- Reset UI (UI-04): refused with a user notice.

Existing later scope (LS-01–LS-07), parked research and other tracks remain
unchanged. No new reporter attempt, bus action, release or deployment follows
from this reconciliation.

## References

[P6–P8](../goals/open-work-2026-10-17/p6-p8-not-goal-ready.md) ·
[parked-work triage](../status/2026-10-12-parked-work-triage.md) ·
[ledger](../status/LEDGER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#3-deferred-by-a-recorded-user-decision)
