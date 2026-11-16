# LS-02 — Online catalogue source

**Work state (16 November 2026):** owner-authorized Reko implementation in progress
on `work/reko-catalogue-20261115`, based on `f6ef90fb4`. Backend and UI/test work
exist in the branch; no accepted final review/package gate, delivery commit or
publication is recorded here. Alpha 22 and the accepted file-based workflow are
not retroactively changed by this candidate.

**Ledger:** `FUTURE-05`; per-ID status of record is the
[ledger](../status/LEDGER.md). Listed in [OPEN_WORK](../OPEN_WORK.md).

## Current owner decisions

The owner approved implementation and continuation: add Reko beside local files,
configurable HTTPS base (default `https://reko.x3muha.de`), manual-only sync, local
bounded search, one encrypted account per installation and verified acquisition
through the existing modern/legacy import boundaries. Native and web share the
backend. There is no startup/timer/search sync, bulk update, automatic project
change or new VD1/VD2 importer.

**Scope exception, 14 November 2026:** the owner excluded the rights/provenance
audit from this implementation package. It is neither a start nor package
acceptance blocker. This is not licence clearance, manufacturer authorization
or proof that the historical condition below was met. Keep technical source
identity and hashes. Release and deployment remain separate authorizations.

## Bounded packages and actual status

| Package | Actual state | Still required |
| --- | --- | --- |
| Decision and provider/API contract | ADR-0176 and the branch contract written from the agreed scope and inspected source | Final integration review against the finished candidate; no authenticated-live proof implied |
| Manual index snapshot and search | Backend WIP for HTTPS/gzip, ETag/304, last-good snapshot, bounded local search and explicit sync | Source-bound regression and final package acceptance |
| Shared encrypted account | Backend/UI WIP for password/token, AES-256-GCM, separate key, source binding and explicit forget | Final security/integrity review and failure/leak/concurrency acceptance |
| Verified acquisition and import handoff | Backend/UI WIP for selected SHA, bounded download cache, modern owned-job feed and separate legacy import | Complete size/hash/cancellation/stale-selection/retry and importer-composition evidence |
| Web/native and operator documentation | Branch manual describes actual source controls and separate Docker `/config` retention | Final UI composition and controlled replacement/recovery evidence; native runtime claims only if actually exercised |
| Delivery | Not accepted, committed or published at this checkpoint | Parent-owned independent review, complete selected package gate, final source/evidence binding and authorized delivery |

The initial backend compile failed; this checkpoint does not turn that attempt
into a pass or guess later test totals. Parent-owned final evidence must name the
candidate, reviews, executed checks, skips/failures and what was actually frozen.
Controlled local HTTPS responses are test fixtures, not successful calls to the
real provider. No authenticated Reko login/product download, hardware/bus run,
release or deployment is claimed.

## Acceptance boundaries

- Manual update is the only index-network trigger. Search/status/opening the
  selector stay local; absence or damage prompts instead of fetching.
- Preserve last good state; distinguish successful `lastCheckedAt` (also 304)
  from `fetchedAt` and provider `generatedAt`; enforce transfer/decode/count/time
  budgets and report omitted invalid entries.
- Credentials never enter ordinary settings/browser persistence/logs/readback.
  One shared server account/quota is explicit, not per-user access control.
  Missing/wrong keys refuse; restore the separate key or explicitly forget and
  re-enter. Persist `/config` separately and reattach it across Docker replacement.
- Every download/feed/legacy action requires the selected SHA; changed selection
  refuses instead of substituting another result. Same-source HTTPS and no
  redirects; verify actual size and streaming SHA before importer admission.
- Existing modern ownership/preview/consent/backup/apply and separate legacy
  file-password rules remain. VD1/VD2 search visibility is not import support.
- One password re-login/repeat only for download `INVALID_TOKEN`; token-only has
  no refresh. 403/404 stop; 429 respects provider waits/reset; 503/network requires
  manual retry. No post-wait background action.

## Historical lifting condition (superseded for this package, not fulfilled)

The earlier later-scope condition was a public, licence-clear feed usable without
vendor approval, followed by its own provenance/integrity/offline-fallback ADR.
File-based catalogue update/restore was already complete in CLI and web. The
owner's package exception changes the audit scope; it is not evidence of legally
cleared rights. General online-source expansion remains separate work.

## References

[ADR-0176](../adr/0176-manual-reko-catalogue-acquisition.md) ·
[Reko provider/API contract](../contracts/reko-online-catalogue.md) ·
[Manual workflow](../manual/user-guide/05-devices-and-products.md#reko-online-catalogue-branch-wip) ·
[Docker key persistence](../manual/user-guide/11-web-and-docker.md#reko-configuration-and-key-persistence-branch-wip) ·
[ADR-0117](../adr/0117-previewed-recoverable-catalogue-updates.md) ·
[ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md) ·
[catalogue update contract](../contracts/catalogue-update.md) ·
[catalogue job contract](../contracts/catalogue-update-jobs.md)
