# Empty-column preferences — 23 November 2026

Status: implementation prepared; awaiting parent final review and package-gate acceptance.

Owner approved all declared columns as an in-memory fallback when preferences
hide every column of a nonempty schema. Empty schemas stay empty. The shared
`useTableColumnVisibility` validator owns the correction; catalogue results,
Devices and their choosers use it without consumer-specific workarounds.

Reads, initial load, rerender, schema/table-ID changes, cross-tab storage events
(including removal/clear) and same-table local refresh do not write storage.
Explicit toggle/reset starts from the effective normalized state and may persist
that action. Valid subsets, unknown-ID dropping, new-column defaults, table
isolation, mounted synchronization and last-visible-column refusal remain.
No project/backend/bus mutation, mandatory columns, selection-control redesign,
API, schema, dependency or ADR change is included.

Focused evidence covers hook transitions and real production catalogue/Devices
consumers. Browser regressions seed all-false preferences before mount, check
named data columns and exact stored values through reload, and pair each with a
valid-subset control. A per-tab seed marker prevents reload from repairing a
value that the application changed or removed. Execution results and any setup
limitations are recorded below before handoff; no package-gate result is claimed.

OWN-01's empty-preference lead is split from hidden selection controls. The
latter remains open and unconfirmed as a UX defect; no claim of keyboard or
selection inaccessibility being fixed. This package does not reopen or close
broader WP-48, WP-49, stored-profile, beta/REL-01 or hardware scope.

## Focused execution and remaining acceptance

- Focused Vitest run: 3 files, 99 tests passed (hook and both production
  consumer suites). The user-data guard reported no touched or unchecked data.
- Baseline validator reversion: each consumer's empty-preference regression
  failed (2 failures); both valid-subset controls passed (2 passes; 83 unrelated
  tests skipped). The corrected hook was restored byte-identically.
- `npm run build` passed (TypeScript and production Vite bundle); Vite emitted
  its existing large-chunk advisory.
- A focused TypeScript check of both changed browser specs and their imports
  passed with explicit Node and Vite declarations.
- Worker attempts at both focused owning Playwright configurations failed before test execution:
  `spawnSync python3 EPERM` in `tools/browser_tmp.cjs` while calling the normal
  browser-temp helper. Those worker attempts provide no browser assertion,
  screenshot, reload proof or browser baseline-reversion result. The sandbox
  was not weakened.
- Parent execution after the fixture corrections: both owning production
  configurations passed, 2 cases each (empty preferences and a valid subset).
  Real named data columns and exact stored values passed before and after
  reload; both user-data guards reported no touched or unchecked data.
  Screenshots show the recovered catalogue and Devices data after reload.
  Earlier attempts exposed reload-navigation timing and the request ledger's
  mistaken classification of normal achievements reporting; those failed
  attempts remain in the external evidence dossier.
- The new request guards now explicitly allow only that achievements reporting
  POST, answered by the existing fresh-record achievements fixture for startup
  and commands. It is nonproject reporting; achievements reset and other writes
  remain forbidden. Catalogue records API request events before fixture setup;
  bus discovery stays an exact POST exception, and project creation remains
  allowed only in Devices setup.
- Independent review and final package-gate acceptance remain with the parent.
  No browser baseline-reversion result, package gate, publication or release is
  claimed by this focused proof.

The external implementation handoff retains commands, exact outputs and the
byte-restoration digest. No machine-specific evidence paths are tracked here.
