# ADR-0114: Client release discovery is notification-only

- Status: Accepted implementation decision; source delivery is separate from release/deployment
- Date: 2026-10-11

## Context

The owner requested startup/update toasts when a new KNXBench GitHub release is
published. The browser and desktop share a web workbench and authenticated
`/api/version`; toast history/lifetime and server-owned settings already exist.
The public release endpoint admits anonymous CORS reads. GitHub `latest` excludes
prereleases, so it does not cover the currently published alpha channel.
Primary evidence and bounded admission are in the
[release notification contract](../contracts/RELEASE_NOTIFICATIONS.md).

## Decision

Keep external HTTP in one fixed-origin adapter, semantic eligibility in a pure
module, and cancellation/cooldown/session deduplication in a small client service
hook. Views only present notices, links and the persisted opt-out. Read the actual
running backend version and acknowledged settings; never assume the bundle's
version or a cached preference authorizes an outbound request. Reuse existing
settings, About dialog, toast lifetime/history and localized catalogues.

Poll published releases once at startup and hourly while visible. Stable builds
exclude prereleases; prerelease builds include newer prereleases/stable releases.
Bound JSON to one page/100 records/4 MiB and the complete request chain to ten
seconds. Unknown versions and failed checks remain explicit unavailable states.
No webhook collector, update/install service, GitHub credentials or dependencies
are introduced. The adapter never receives project/manufacturer/bus data.

## Consequences and alternatives

A client IP/browser metadata reaches GitHub; the Settings explanation and opt-out
make this visible. Offline/blocked clients continue working. Notification is not
an installer, integrity verifier or migration/compatibility approval. Session-local
state does not change KNX Core, native/project/product schemas or API route contracts.

A central webhook/push service would require public ingress, deployment, secrets
and maintenance just to announce public metadata. A server HTTP client would add
a new HTTPS dependency to an otherwise dependency-free feature. Both are rejected
for this scope. The bounded polling delay, one-page search, per-IP rate limit and
native GUI verification boundary are disclosed rather than silently promising
instant or universal update detection.
