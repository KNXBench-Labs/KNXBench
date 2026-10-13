# Shared first-run guide state

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-160)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 160. The first-run guide is remembered per installation, not per person

**Limitation.** The first-run guide ([ADR-0084](../adr/0084-first-run-guide-once-per-release-stage.md))
records that it was seen in the server's `settings.json`
(`onboardingGuide.seenStage`). In a web/Docker deployment several people
share that file, so the first person to close the guide closes it for
everyone until the release stage changes. Two browser windows started at the
same moment on a fresh installation can both show it.

**Why.** The settings file is the application's only preference store, and
there are no user accounts to key a per-person record by (ADR-0026: one
password). Keeping the flag in the browser would have added a second store.
User decision 2026-10-07 (first-run guide interview, Q9).

**Impact.** Someone who missed the guide opens it with **File → Show
introduction…** or the command palette. Nothing is gated on it: the
programming consent and the library's write authorisation are unaffected.

**Also not verified.** The guide is tested in Vitest (happy-dom) and in
Chromium (Playwright). WebKitGTK in the desktop shell and screen-reader
output were not checked.

**Lifted when.** Per-user settings exist, or a per-browser record is wanted
despite the second store.
<!-- END retained catalogue record -->
