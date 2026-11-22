# LS-02 — Online catalogue source

**Work state (19 November 2026):** **delivered** — the owner-authorized Reko
implementation shipped on main 16 November 2026 as `c079ec8d6` (merged from
`work/reko-catalogue-20261115`, base `f6ef90fb4`): backend online-catalogue
module with index sync, verified downloads, encrypted account store, routes and
tests. Alpha 22 and the accepted file-based workflow are not retroactively
changed; release/deployment remain separate authorizations.

**Ledger:** `FUTURE-05`; per-ID status of record is the
[ledger](../status/LEDGER.md). Not listed in [OPEN_WORK](../OPEN_WORK.md): the row left
the list on 20 November 2026 when this package closed; a stale-base merge re-added it
with the pre-delivery text ("delivery pending") until WP-48.9 removed it again.

**Follow-up WP-48.8 (F17):** cache availability in search now follows the verified SHA-256
of the selected index entry (PR #135, merge `9cd7c0d98`), so a same-id, same-size,
different-digest revision no longer looks already downloaded. No other scope change.

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
| Decision and provider/API contract | ADR-0176 and the contract written from the agreed scope and inspected source | — |
| Manual index snapshot and search | Delivered in `c079ec8d6`: HTTPS/gzip, ETag/304, last-good snapshot, bounded local search, explicit sync | — |
| Shared encrypted account | Delivered in `c079ec8d6`: password/token, AES-256-GCM, separate key, source binding, explicit forget | — |
| Verified acquisition and import handoff | Delivered in `c079ec8d6`: selected SHA, bounded download cache, modern owned-job feed, separate legacy import | — |
| Web/native and operator documentation | Delivered in `c079ec8d6` with the server routes; Docker `/config` retention documented | — |
| Delivery | **Delivered on main 16 November 2026 as `c079ec8d6`** | Release/deployment remain separate authorizations; not in Alpha 22 |

Controlled local HTTPS responses remain test fixtures, not successful calls to
the real provider. No authenticated Reko login/product download, hardware/bus
run, release or deployment is claimed.

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
[Manual workflow](../manual/user-guide/05-devices-and-products.md#reko-online-catalogue) ·
[Docker key persistence](../manual/user-guide/11-web-and-docker.md#reko-configuration-and-key-persistence) ·
[ADR-0117](../adr/0117-previewed-recoverable-catalogue-updates.md) ·
[ADR-0118](../adr/0118-catalogue-updates-through-the-web-server.md) ·
[catalogue update contract](../contracts/catalogue-update.md) ·
[catalogue job contract](../contracts/catalogue-update-jobs.md)
