# ADR 0176: Reko supplies files; the existing catalogue importer still decides

Date: 2026-11-16
Status: Accepted implementation direction (owner); branch WIP, not software-accepted or released
Session: Reko catalogue
Scope: LS-02 / FUTURE-05, Reko acquisition for the shared web/native backend

## Context

The existing catalogue installs files through bounded import paths. Modern
packages have catalogue jobs with preview, explicit consent, backup and apply;
legacy VD files have a separate importer and file-password policy. An online
source must acquire bytes without becoming another installer or changing the
project's application bindings.

The owner approved a first Reko source, configurable HTTPS address, manual index
updates and one account per installation. The service is privately hosted behind
a limited connection: opening the application or searching must not generate
provider traffic. Native and web use the same server implementation.

The provider/API description supplied by the owner is contract evidence, **not an
authenticated live-service test**. No successful real-provider login or product
download is claimed. The [provider/API contract](../contracts/reko-online-catalogue.md)
records the wire surface, local bounds and evidence boundary. Implementation is
in `apps/knx-server/src/online_catalogue/`; current work and remaining acceptance
are recorded in [LS-02](../open-work/LS-02-online-catalogue.md).

## Decision

### 1. Acquisition stays outside the domain and installed product database

Add Reko to the catalogue-source dropdown beside local files. Default address:
`https://reko.x3muha.de`. A small server-side source registry is sufficient; a
plugin framework is not a prerequisite. Provider DTOs, credentials, index and
verified-download cache stay in the server adapter, separate from installed
product data. The browser receives bounded result pages, never the whole index
or a provider bearer token. Clients identify a source and entry, not a free URL.

### 2. Only an explicit manual sync contacts the index endpoint

No startup sync, timer, automatic first fetch, dropdown-triggered fetch or
search-triggered provider request. Missing or damaged local state prompts for a
manual update; it does not repair itself over the network. Search uses a bounded
local snapshot. Status polling talks only to KNXBench's own backend.

A sync uses HTTPS, gzip and an unchanged `ETag` in `If-None-Match`. Schema and
resource validation precede atomic publication. Whole-document failure leaves
the last good snapshot intact; invalid individual entries are omitted with a
visible count. A valid 304 advances `lastCheckedAt`, not `fetchedAt` or the
provider's `generatedAt`. Failure advances none of these successful timestamps.
There is one active sync per source and conservative shared request spacing.

### 3. The account belongs to this installation and configured source address

Offer username/password or a manually supplied token. Request an account on the
first requested uncached download if none is usable; account settings are also
available under Settings → Catalogues. Registration/account management stays on
the operator website; no registration API or undocumented registration path is
invented. Saving an account is local storage, not proof that login succeeded.
After a first-download prompt, resume only that still-current requested download.

One account per catalogue and installation means all users of a shared server
share the account and provider quota. Per-user accounts, KNXBench user management
and account migration are outside this package.

Persist the account in a separate encrypted file, using `ring` AES-256-GCM,
fresh random nonces and a separately generated random 256-bit key. New private
files use 0600 and private directories 0700. Password-login bearer tokens live
only in memory; a manually entered token is part of the encrypted account.
This is reversible encryption because provider login needs the original
password, not a password verifier. It is not a new master-password/vault system.

The data directory alone must not contain the decryption key. Keys use the
configuration directory, not normal settings. Secrets must not enter browser
persistence, ordinary settings/readback, logs, diagnostics or projects. Responses
return only credential status, mode and optional username. Account/address
changes invalidate token state; a different configured address cannot use the
old account or index. Admitted operations and configuration changes are
serialized so stale work cannot publish into a new account/source context.

An existing unreadable credential file is not silently replaced. Restore its
matching key or explicitly forget the account before re-entering it. Encryption
protects a data-only copy without the key; it does **not** protect against a
compromised host/process or an attacker holding both key and encrypted file.

### 4. Selection and verified bytes are prerequisites, not installation consent

Every download, modern-job feed and legacy-install request includes the selected
entry's SHA-256. If that entry changed in the current local index, refuse with
`selectionChanged`; do not substitute a newer search hit or automatically sync.
Index download paths must be root-relative on the configured HTTPS source;
preserve query parameters, refuse foreign origins and do not follow redirects.
No TLS bypass is offered.

Stream to bounded private temporary storage; check actual size and SHA-256 before
publishing a reusable copy. Recheck retained bytes at their import boundary.
Cache reuse avoids another provider download; the acquisition cache is bounded
and is not a permanent archive. Network acquisition holds no installed-catalogue
write lock. No failed or cancelled acquisition may mutate installed product data.

`.knxprod` feeds the existing catalogue job, with its ownership, admission,
preview, consent, backup and transactional apply unchanged. `.vd3`–`.vd5` use the
existing separate legacy import, including its independent file-password prompt;
the Reko account password is never a legacy file password. `.vd1`/`.vd2` may be
searched/downloaded but are not importable through this feature. Online,
downloaded/verified and imported are different states. No automatic product
upgrade, application rebinding, project change or bus operation follows a download.

### 5. Retry budgets and provider backpressure are explicit

Reuse a usable token. Only download 401 with `INVALID_TOKEN` permits one password
re-login and one repeat of that download. Token-only mode has no refresh or
password fallback. A login 401, a second rejection, other authorization failures,
403 and 404 stop without retry. A 404 may suggest a manual sync, never start it.
Honor 429 `Retry-After` and distinguish `DOWNLOAD_DAILY_LIMIT` with the provider's
`resetAt`; expiry of a wait never starts an action. Network/503 failures retain
last-good local state and require manual retry. Exact bounds are in the contract.

### 6. Configuration storage is independently persistent in Docker

Resolve provider state beside the default product database and keys from the
legacy configuration-directory resolver, in separate private subdirectories.
The Docker source sets `XDG_DATA_HOME=/data/.knxbench-share` and
`XDG_CONFIG_HOME=/config`, declaring both volumes. Operators must explicitly bind
or name `/config` and reattach that same storage when replacing the container;
a Docker-created anonymous volume is not automatically reattached to a new one.
Back up the key/configuration independently from `/data`.
[Paths, replacement and recovery](../manual/user-guide/11-web-and-docker.md#reko-configuration-and-key-persistence-branch-wip).

### 7. Certificate-root data has a package-scoped licence exception

The production trust store uses `webpki-roots` 1.0.12. Its root data is declared
`CDLA-Permissive-2.0`, outside the repository's general dependency allowlist.
`deny.toml` permits it only for this exact package/version, not for arbitrary
new dependencies. The no-GPL runtime policy and project licence are unchanged.
The exact upstream agreement is retained in
[the bundled licence text](../third-party/webpki-roots-CDLA-Permissive-2.0.txt),
embedded in both server/native builds and available as plain text through
`GET /api/online-catalogue/trust-store-license` under existing server auth.

Basis: the package's published `LICENSE` and
[CDLA clause 2.1](https://cdla.dev/permissive-2-0/) require the agreement text to
accompany shared data; [cargo-deny's exception mechanism](https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html#exceptions)
allows package-scoped admission. This is dependency-policy implementation, not
a legal clearance of Reko product data or a change to the owner's audit exception.
A dependency-version update must revisit the pinned exception and agreement.

## Alternatives considered

- **Automatic first fetch/periodic sync/search at the provider:** rejected; it
  adds unrequested load and makes ordinary browsing depend on the remote host.
- **Browser download and re-upload:** rejected; duplicates the shared backend
  acquisition boundary and risks browser-persisted tokens and weaker selection
  integrity. Native and web need the same guarantees.
- **Hash the provider password:** unsuitable; a verifier cannot perform the
  provider's password login. Plain settings storage and a data-co-located key
  were rejected in favor of authenticated encryption with a separate key.
- **Another installer or generic provider framework:** rejected for this slice;
  reuse existing modern/legacy semantics and a small explicit source registry.
- **Personal accounts and a new master-password vault:** deferred; neither is
  required by the approved shared-installation account model.

## Owner scope exception: rights/provenance audit

On 14 November 2026 the owner explicitly excluded the rights/provenance audit
from **this implementation package**. It is not a start or package-acceptance
blocker. This changes the historic LS-02 lifting condition for this package; it
does not establish that it was satisfied. Technical source identity and SHA-256
integrity remain required. No licence clearance, manufacturer authorization,
KNX certification, deployment permission or release permission is implied.

## Consequences and acceptance boundary

Local files remain the offline fallback; the last good index and verified copies
can be used without provider availability. Shared credentials and quota are an
explicit server trust limitation. Key recovery is an operator responsibility.
A provider hash checks bytes against its own index, not independent authenticity,
authorization, application compatibility or device safety.

The branch has implementation and test work in progress, with no accepted final
review/package gate or publication at this checkpoint. Tests against a controlled
HTTPS provider are synthetic contract tests, not proof of real-provider behavior.
Software acceptance must cover manual-only networking, index publication and
limits, account/key failure modes, origin isolation, stale selection refusal,
retry quotas and verified handoff to both existing import paths. Parent-owned
final evidence belongs in LS-02; this ADR grants no acceptance by itself.

Related decisions: [ADR-0117](0117-previewed-recoverable-catalogue-updates.md),
[ADR-0118](0118-catalogue-updates-through-the-web-server.md),
[ADR-0150](0150-cancellable-catalogue-jobs.md),
[ADR-0094](0094-legacy-exim-product-files.md).
