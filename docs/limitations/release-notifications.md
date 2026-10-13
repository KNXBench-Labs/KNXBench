# Release notifications

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-release-notifications)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->


## Release notification boundary (2026-10-11)

[Release discovery](../contracts/RELEASE_NOTIFICATIONS.md) polls one page/up to100
GitHub releases at startup/hourly while visible. It is neither instant push nor
an installer/integrity verifier/upgrade approval. Offline, API rate limits,
unknown running versions, refused settings or oversized responses do not block
engineering but cannot establish the newest release. Client IP/browser metadata
reaches GitHub; opt-out stops future automatic requests, not bytes already sent.
Chromium/common-bundle evidence is not interactive WebKitGTK/AppImage or Docker
runtime certification. The feature is not in the published Alpha.9 snapshot.
<!-- END retained catalogue record -->
