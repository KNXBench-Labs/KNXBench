# OWN-01 — Owner decisions and deferred items

**Status:** Owner decision. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Each item stays as it is until the owner decides. Do not operate on
containers, backups or remote refs in a planning task.

## Decisions pending

- Root checkout: publish or keep local the untracked root-only research and
  triage notes.
- Old container and old source path from the 17 October move: remove on
  request (last checked 18 October).
- GitHub pull-request refs `refs/pull/3` and `refs/pull/24` still serve
  pre-rewrite commits: optionally ask GitHub Support to purge them.
- Local `refs/kl185-rewritten/*` copies of delivered packages may go once
  each is proven on `main`.
- The parked `wip/manufacturer-plugin-research-20261013` branch stays by the
  14 October decision.
- CI is disabled until the owner re-enables it.
- Docker Hub: image signing, description sync, scheduled base-image rebuilds
  and the `latest` rule at the first stable release.
- Retention of the private pre-launch rollback repository; private mailbox
  verification.
- Website / Evolution Story: the owner's privacy review and optional WebKit
  evidence.

## Deferred by recorded decision (reopen only on a new decision)

- ADR-0039 phases 3–5, sealing the ID allocator (KL-129).
- Package/application version pinning and a selector (KL-135).
- Live tunnel from a Docker bridge container (Route Back): only with a go.
- New real-hardware, power-loss, vendor and ETS validation for
  commissioning.
- Durable pre-write recovery for K6 button / serial / K13 address writes;
  every new write needs its own go.
- Reset UI (UI-04): refused with a user notice.

## References

[P6–P8](../goals/open-work-2026-10-17/p6-p8-not-goal-ready.md) ·
[parked-work triage](../status/2026-10-12-parked-work-triage.md) ·
[ledger](../status/LEDGER.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#3-deferred-by-a-recorded-user-decision)
