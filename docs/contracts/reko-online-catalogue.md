# Reko online catalogue: provider and KNXBench API contract

Status: delivered on main 16 November 2026 as `c079ec8d6` (not in Alpha 22,
release/deployment separate). [ADR-0176](../adr/0176-manual-reko-catalogue-acquisition.md) owns the
decision; [LS-02](../open-work/LS-02-online-catalogue.md) owns the work record.

## Evidence and trust boundary

- **Provider behavior:** the owner-supplied API description is the integration
  contract, not authenticated live evidence. The planning record reports one
  public schema-1 index observation with gzip and an ETag. It establishes neither
  continued availability nor successful authentication/download behavior.
- **Implementation mapping:** `apps/knx-server/src/online_catalogue/` contains
  `source.rs`, `index.rs`, `store.rs`, `transport.rs`, `service.rs`, `vault.rs`,
  `download.rs`, `body.rs`, `routes.rs` and `error.rs`. The UI uses
  `apps/knx-web/src/onlineCatalogueApi.ts` and `OnlineCatalogue*.tsx`.
- **Verification:** source inspection is not an executed test. Controlled HTTPS
  fixtures are contract tests, not real-provider login/downloads. No final
  review, accepted package gate, real-provider acceptance or deployment is
  recorded by this document.
- **Rights:** the owner excluded rights/provenance auditing for this package on
  14 November 2026; it is not a package blocker. That is not a finding of cleared
  licences or manufacturer authorization. Technical source/hash evidence stays.

The provider index is not installed product data. A matching provider SHA-256
proves agreement with that index, not independent authenticity, KNX certification,
application compatibility or permission to distribute/use the file. Native and
web call the same backend; no browser request sends an account to Reko directly.

## Provider-facing requests

Source ID `reko`, label `Reko`, default base `https://reko.x3muha.de`.
The backend accepts a configurable HTTPS origin with an optional path prefix;
userinfo, query and fragment are refused. Credentials and index are bound to the
canonical configured base, including the prefix. Requests retain TLS certificate
validation using the production rustls/Mozilla root set; there is no insecure
mode. All redirects are refused, including same-origin redirects.

| Operation | Request | Required handling |
| --- | --- | --- |
| Manual index sync | `GET /api/catalog/index.json`, `Accept: application/json`, `Accept-Encoding: gzip`; `If-None-Match` only with a usable matching local snapshot | No account required. Accept bounded 200 or valid 304. No implicit calls from startup, timers, view opening, search or status. |
| Password login for a requested download | `POST /api/auth/token`, JSON `{user,password}` | Accept bounded 200 bearer-token JSON; login 401 ends the action. Saving an account alone does not call this endpoint. |
| Selected file | `GET` of that validated index entry's `download_url`, `Authorization: Bearer <token>`, `Accept: application/octet-stream` | Preserve the complete relative path/query. Accept only a size/hash-verified 200 body; no arbitrary client URL and no redirect following. |

Paths in this table are appended to a configured prefix. An index URL such as
`/api/download?path=Hersteller%2F0042_Example%2Fexample.vd5` is a **synthetic shape
example**, not a real downloadable product. The backend does not construct a
new download path from manufacturer/product display text. Registration is on
the operator website at the configured base; no registration route is assumed.

### Index schema 1

The decoded document uses these fields:

| Scope | Fields consumed by KNXBench |
| --- | --- |
| Document | `schema` (integer, exactly 1), `version` (nonempty string), `generatedAt` (string), `count` (integer matching raw entry count), `basePath` (currently empty string), `downloadRequiresToken` (currently true), `entries` (array) |
| Entry | `id`, `manufacturer`, `manufacturerId`, `name`, `orderNumbers` (string array), `fileName`, `fileType`, `size` (positive byte count), `sha256`, `updated`, `programs`, **`download_url`** (the deliberate snake_case field) |
| Program | `displayKey`, `name`, `applicationNumber`, `version`, `maskVersion`, `productNames` (string array) |

`orderNumbers` and `programs` may be absent and default to empty arrays; consumed
program fields default to empty strings/arrays when absent. Unconsumed provider
fields are not installed into the domain. An unknown schema, inconsistent raw
count, unsupported document policy or malformed document refuses the snapshot
whole. Individually invalid/duplicate entries are omitted and counted in
`skippedEntries`, not silently treated as installed or usable.

Entry IDs are 1–64 ASCII alphanumeric/`-`/`_` bytes. Digests are exactly 64
lowercase hexadecimal SHA-256 characters. Names are plain file names, with no
path separators, colon or control characters; extension must match `fileType`
and length must meet catalogue-job source-name admission. Allowed types are
`knxprod`, `vd1`, `vd2`, `vd3`, `vd4`, `vd5`. Root-relative download URLs cannot
start with `//` or name an authority; queries are preserved. Display strings,
program lists and order-number lists are independently bounded in `index.rs`.

### Snapshot, ETag and local search

A fully validated decoded document and its metadata are published atomically,
with a stored document SHA-256 and source binding. Failed transfer, decode,
validation or storage leaves the previous good snapshot in place. A 304 is valid
only with the matching usable snapshot; it cannot create a missing index.
ETags are opaque, retained without removing quotes or weak-validator prefixes;
the implementation retains validators up to 256 bytes.

- `generatedAt`: provider-reported document generation time, not a local sync time.
- `fetchedAt`: time this index body was last successfully accepted from a 200.
- `lastCheckedAt`: last successful sync confirmation, including 304.
- A failed sync advances neither local successful timestamp.
- `entryCount` counts raw entries, including `skippedEntries`; search `total`
  counts admitted matches only. A skipped count is not a pagination defect.

Search is case-insensitive, whitespace-word AND matching over manufacturer/name,
manufacturer ID, file name, order numbers and consumed application details.
It has no provider side effect. A missing/damaged/wrong-base cache returns
`noIndex`, not an automatic fetch. A verified local download can be reused
without another provider request, subject to fresh integrity checks and current
selection identity. Cache availability is not import success.

### Bounds and request policy

These are local implementation bounds, not promises about the provider's maximum
catalogue size or quota. Source constants are authoritative.

| Resource | Bound |
| --- | --- |
| Index response body on wire | 64 MiB, including an identity-encoded response |
| Decoded index | 256 MiB |
| Raw index entries | 500,000; larger snapshots refused, not truncated |
| Search | 200 query characters, default page 25, clamp 1–100, offset at most 10,000,000 |
| File bytes | `MAX_FILE_BYTES = knx_core::limits::MAX_RETAINED_BYTES`, currently 1,000,000,000 bytes |
| Verified acquisition cache | At most 16 files and `2 * MAX_FILE_BYTES`; oldest eligible files evicted after acquisition, current file retained |
| Download duration | One hour total; 60-second idle body deadline |
| Index body duration | 15 minutes total; 60-second idle body deadline |
| Connect / response headers | 15 seconds per TCP/TLS connect stage / 60 seconds for response headers |
| Small login/error replies | 16 KiB; bounded read deadlines |
| Remote request spacing | At least one second between request starts through the shared backend client |
| Retry wait | `Retry-After` seconds or HTTP date; local maximum 24 hours, fallback 60 seconds when unusable/missing |

The backend permits one download slot and one manual sync per source, serializes
account/config changes against admitted work and shares the login session lock.
It does not hold an installed-catalogue writer reservation during network
acquisition. Other installations behind the same public IP can consume a
provider-wide request allowance; remote 429 remains authoritative.

### Provider error/retry matrix

| Provider response | KNXBench behavior |
| --- | --- |
| Login 401 | `invalidCredentials`; no automatic follow-up login. |
| Download 401 with `code: INVALID_TOKEN` | In password mode at most one fresh login and one repeat of that download. A second rejection ends as `tokenRejected`. Token-only mode requests new input; no invented refresh token. |
| Other download 401 | Authorization refusal; no password-login retry. |
| 403 | `forbidden`; no retry or type substitution. |
| 404 | `notFound`; offer manual index update, never start one. |
| 429 with `code: DOWNLOAD_DAILY_LIMIT` | `dailyLimit`; retain provider `resetAt`, block downloads for the announced wait. |
| Other 429 | `rateLimited`; block remote requests for the announced wait. |
| 503 or network/TLS/timeout failure | Stop; retain last-good index, allow manual retry later. |
| 3xx, oversized/unknown encoding, invalid size/hash/schema | Refuse, without following or admitting bytes for installation. |

Only bounded machine-readable `code`/`resetAt` are consumed from error JSON;
provider response text is not echoed into UI/logs. Daily blocking honors the later
of usable `Retry-After` and future `resetAt`; a zero/short header cannot advance
the provider's reset. Origin changes discard old-origin limits/download status.
Account replacement/forget clears the old account's daily block, not the origin's
per-IP rate block. Do not replace provider
calendar/reset semantics with a fixed 24-hour-after-download timer. Block expiry
never schedules a login, sync or download. A process restart is not quota renewal;
the provider may still reject the next explicitly requested action.

## KNXBench-facing HTTP API

Base `/api/online-catalogue`; source placeholder `S` is currently `reko`.
The existing server authentication covers this surface. It creates no new
KNXBench users or per-user account isolation. JSON fields are camelCase except
where explicitly stated; no response contains passwords/tokens or `download_url`.

| Method / route | Request / result |
| --- | --- |
| `GET /api/online-catalogue` | `{available,sources:[{id,label,defaultBaseUrl}]}`; unavailable gives an empty source list. |
| `GET /api/online-catalogue/trust-store-license` | Embedded plain-text root-data agreement; no provider request. |
| `GET /api/online-catalogue/S` | Local `Status` below. No provider request. |
| `PUT /api/online-catalogue/S/config` | `{baseUrl:string\|null}`; null restores default. Returns status; no sync/login. |
| `POST /api/online-catalogue/S/sync` | Explicit provider sync. `{outcome:"updated",entryCount,status}` or `{outcome:"notModified",status}`. |
| `GET /api/online-catalogue/S/search` | Query `q`, `type`, `offset`, `limit`; local `{total,offset,limit,entries}`. No provider request. |
| `PUT /api/online-catalogue/S/credentials` | `{mode:"password",user,password}` or `{mode:"token",token}`; encrypted local save, invalidate session; returns status, not live-login proof. |
| `DELETE /api/online-catalogue/S/credentials` | Explicit forget, invalidate session; returns status. Key is retained. |
| `POST /api/online-catalogue/S/entries/E/download?sha256=H` | Selected source/entry/hash only. 202 running or 200 reused verified cache; returns download status. |
| `DELETE /api/online-catalogue/S/download` | Request cancellation; returns current download status. Cancellation is not confirmed until the slot settles. |
| `POST /api/online-catalogue/S/jobs/J/feed/E?sha256=H` | Feed verified cached `.knxprod` to the existing owned job using its ownership headers and existing response semantics. No new provider download. |
| `POST /api/online-catalogue/S/entries/E/install-legacy?sha256=H` | JSON `{password?:string,remember?:boolean}`; default remember false. Existing `.vd3`–`.vd5` importer/report and password refusals. No modern-preview guarantee added. |

`sha256` is **required** for all three entry actions (download, job feed, legacy
install). It is the search selection's expected digest, not a request to find a
replacement. A mismatch against the current entry returns `selectionChanged`
(409), without auto-sync, auto-research or silent substitution. Reselect locally;
if freshness is needed, separately request sync first. Unknown entries are 404.

`Status` carries source/label/default and configured base, `baseUrlIsDefault`,
optional `index` (`entryCount`, `skippedEntries`, `version`, `generatedAt`,
`lastCheckedAt`, `fetchedAt`), optional `indexProblem`, `credentials`, optional
`limit`, `syncRunning` and `download`.

`credentials` is `{state,mode,user,problem}`: state `empty`, `stored` or `problem`.
The latter reports safe problem kinds, never the secret. `limit` carries `kind`,
`retryAfterSecs`, optional `resetAt`. `download` carries `state` (`idle`, `running`,
`done`, `failed`), `entryId`, `receivedBytes`, optional `totalBytes` and optional
safe `error` (`kind`, `message`, `resetAt`, `retryAfterSecs`). Poll only this local
status for acquisition progress; polling is not a provider retry loop.

Search entries contain the consumed display/application metadata and SHA-256,
plus `importable` and `downloaded`. The latter describes an acquisition-cache
candidate verified when written, not an import receipt or a permanent guarantee
that local bytes remain intact. Reuse/import revalidates bytes. VD1/VD2 stay
visible with `importable:false`.

Errors use the existing API error envelope with stable `kind`; rate/daily-limit
errors include `retryAfterSecs`/`resetAt` details. Important local codes: 409
`noIndex`, `needsLogin`, `invalidCredentials`, `tokenRejected`, `busy`,
`selectionChanged`, `cancelled`, `vaultKeyMissing`, `vaultOriginMismatch`; 429
`rateLimited`/`dailyLimit`; 502 `transport`/`integrity`; 503 `unavailable` or
`serviceUnavailable`. Other vault/storage failures are 500; malformed request
rejection follows the ordinary server extractor contract. Do not interpret local
409 account errors as a raw provider 401.

## Credential, filesystem and import invariants

The encrypted account is `credentials.enc` in the source data directory. A
versioned AES-256-GCM record uses a fresh random nonce, source-bound additional
authenticated data and a sealed configured-base identity. A separate random
32-byte key is never returned by the API. Password-login tokens are memory-only;
manual tokens persist encrypted. Private files/directories are created 0600/0700.
Secrets stay out of settings, browser persistence, logs, diagnostics and projects.
Missing/wrong keys or corrupt records refuse; no replacement key silently repairs
an existing encrypted file. Explicit forget removes that account record, not
installed products or the key.

Default source data root:
`parent(knx_productdb::default_path())/.knxbench-online-catalogue/reko`.
Default key:
`parent(default_remembered_password_path())/.knxbench-config/online-catalogue-reko.key`.
Neither is derived solely from `KNX_DATA_DIR` or the current project. Exact XDG,
Docker and backup/recovery guidance is in the
[manual](../manual/user-guide/11-web-and-docker.md#reko-configuration-and-key-persistence).
Private acquisition/configuration directories are fenced from project-file and
static-file routes; filesystem access by the host administrator remains outside
that API protection.

Download bytes are streamed into a private partial file. Enforce actual length
independently of `Content-Length`, compare index length and SHA-256, then publish
a reusable cache copy. Invalid/truncated/cancelled files never become import
inputs. The modern feed verifies the stream again before the existing job can
admit/preview it; legacy reads verify before invoking its existing importer.
Modern ownership/preview/consent/backup/apply and legacy file-password/report
semantics remain authoritative. Acquisition is not installation, a project
mutation, a version migration or a device write.

## Required acceptance (pending, not results)

Cover no implicit remote calls; empty/damaged/last-good index; schema/entry/budget
validation; gzip/ETag/304 timestamps; account restart, permissions, missing/wrong
key and tampering; no secret readback/persistence leaks; address/account changes
and concurrent login; all retry cases; missing/stale selection SHA on every entry
action; redirects/query preservation; actual-size/hash/timeout/cancel boundaries;
cache reuse/eviction; real owned-job handoff and separate legacy password/import;
shared web/native backend composition and independently persistent Docker key
storage. Use controlled HTTPS fixtures, label their scope and retain failures.
Final independent review/package-gate evidence is still parent-owned. A real
provider login/download, deployment and release are separate claims, not results
inferred from these tests.
