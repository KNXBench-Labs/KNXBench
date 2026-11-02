# Rail and Rail Dark — source acceptance, 2 November 2026

## Delivered source and scope

Published main-first source: **`ed01e2bd3`**, first parent `249036fde`;
feature `58760ae32` and exact browser-list correction `60e9dd83c`.
Local, tracking and live main refs matched on publication, 2 November 2026.

Two additional built-ins: Rail (light) and Rail Dark, with Geist/Geist Mono,
square controls, fixed KNX-cable green accents and turquoise success. A shared
trusted presentation adds the toolbar rail and finite action-bound effects,
behind the reduction-only Theme effects category and OS motion preference.
The saved theme choice is not changed automatically. See
[ADR-0157](../adr/0157-rail-built-in-presentation.md) and the
[appearance guide](../manual/user-guide/09-settings-and-appearance.md#rail-and-rail-dark).

## Review and regressions

Independent review identified insufficient light-primary text contrast under
the shared white hover/press veil. A real-browser RED regression confirmed the
defect. Presentation-scoped foreground veils now pass the explicit 4.5:1 minimum
for hover and held press in both palettes. These are settled constant-colour-layer
checks using browser-resolved styles and canvas compositing, not full WCAG
certification. The final static closure review was APPROVE.
The strict palette/component boundary remained unchanged.

The initial presentation contract was calibrated with seven isolated defect
mutants and a harmless passing control. Subsequent focused proof covered
actual hover/held press, no project-creation request, motion cancellation,
bootstrap/runtime parity, role/focus/separator matrices and dialogs.

The first branch package attempt failed on one stale literal dropdown list
(988 passed, one failed); its later Rust/corpus stages did not run and that
attempt is not accepted. The exact list was corrected without weakening the
assertions. Earlier failed/interrupted attempts remain historical evidence.

## Authoritative main-first package result

The maintained selected package gate completed **all 15 planned stages** on
`ed01e2bd3`: format, documentation, browser lanes, five xtask checks, Python
tooling, web gate, desktop clean, Clippy, Rust tests, selected corpus and diff
check. Start/end source fingerprints matched; checked user data was untouched,
with no unchecked user-data paths. This is the final acceptance, not a sum of
passing stages from earlier attempts.

- Web gate: 81 stages; **4,997 Vitest tests**.
- Default Chromium lane: **1,009 functional + 69 exclusive**, no failed,
  skipped or flaky cases.
- Built-bundle Chromium: **16 suites, 625 functional + 29 exclusive**.
  These are execution instances in separate configurations, not additional
  distinct tests beyond the default lane.
- Selected Rust packages: **1,787 passed, 72 ignored**, nine result blocks.
- Selected private-corpus regressions: **51/51**; no private input identities
  or contents are published here.

| Production suite | Functional | Exclusive |
| --- | ---: | ---: |
| Catalogue | 22 | 0 |
| Communication | 87 | 1 |
| Device download support | 77 | 0 |
| Devices | 65 | 1 |
| Dialogs | 26 | 0 |
| GA members download | 23 | 0 |
| Group addresses | 21 | 1 |
| Instructions | 15 | 0 |
| Notes | 13 | 0 |
| Parameters | 27 | 0 |
| Reload guard | 2 | 0 |
| Structure | 23 | 0 |
| Table columns | 156 | 4 |
| Task status | 27 | 0 |
| Updates | 7 | 0 |
| UX feedback | 34 | 22 |

Raw receipts, stage logs, source bindings, review evidence and synthetic
browser artifacts are retained privately. No release tag, deployment, native
WebKitGTK/Orca or Firefox acceptance, full accessibility certification or real
bus operation follows from these results. Imported v1 packs cannot request the
trusted presentation or its executable effects; see the
[remaining boundary](../KNOWN_LIMITATIONS.md#catalogue-rail-presentation).
