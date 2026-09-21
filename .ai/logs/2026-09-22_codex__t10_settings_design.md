# T10 — settings-surface design gate

The approved conversational direction was converted into the architectural
spec `docs/superpowers/specs/2026-09-22-settings-surface-design.md`. No product
code was changed.

The design keeps ADR-0029's single settings record and existing overlay. It
groups Appearance, Language & data, and Bus & diagnostics; adds one optional
preferred gateway as an initial field seed only; extracts one protected
line-scan exclusion editor for Settings and Line Scan; and replaces English
settings notices with typed diagnostics translated in Settings and Log panels.
Group-address output remains fixed slash notation with no selector.

Review found and the spec closes seven important design defects: localized
session-log events, distinct adoption/quarantine diagnostics, lossless handling
of old invalid exclusion strings, delayed authoritative gateway seeding,
ADR-0029's endpoint/privacy amendment, explicit preservation of retired
`groupAddressNotation`, and a pre-hydration settings mutation journal preventing
the initial GET from erasing a fast user edit.

Final read-only spec review: 0 Critical, 0 Important. `git diff --check` and
anchors (386 links across 181 Markdown files) pass. No network, KNX/LAN, bus or
hardware access occurred.
