# Additive outcome achievements and optional humour (AP5 / FUTURE-08)

Four permanent new IDs reuse existing rules and observed UI events:

| ID | Evidence | Threshold |
| --- | --- | --- |
| `save-point-regular` | successful `projectSaved` outcome, including autosave/save-before-replace | 10 saves |
| `palette-resident` | `commandPaletteUsed` emitted before invocation, not successful completion | 100 invocations |
| `room-atlas` | existing `observeProject` recursive Room count | 50 rooms |
| `address-librarian` | existing nonblank group-address-name count, not name uniqueness/quality | 1,000 names |

The original 38 catalogue definitions are unchanged; the catalogue now has 42.
Existing records, IDs, progress, earliest timestamps and `bus-master` unlocks
are not reset. A not-yet-earned `bus-master` continues to require all other
catalogue entries; previously earned versions remain earned. Action counters
start on introduction; existing state can unlock immediately when observed.
No account, schema, backend, CLI or domain change. The existing per-installation
shared record/concurrent-maximum limitations in KNOWN_LIMITATIONS §164 remain.
No claim of security, compatibility or risky bus-operation achievement added.
Existing disabled tracking and bounded achievement-summary notifications remain.

Three EN/DE optional error wrappers enter the existing humour pool (33 instead
of 30). Raw diagnostics/history remain unchanged; humour Off and localized
archive-size refusal bypasses still apply. Bundled additional languages retain
the existing English fallback; no private language-pack content was rewritten.
Achievement descriptions state facts, not optional joke wrappers.

Evidence: each new rule had a missing-definition RED followed by GREEN; below,
at and after threshold are tested with injected time. Three wrapper RED/GREEN
controls retain literal `{msg}`/`$&` input exactly. Scoped tests: 111 passed in
7 files. Actual production workbench EN/DE1440/400: 4 cases prove one new popup,
server-acknowledged progress/unlock, unchanged old timestamps (including
`bus-master`) and no repeat popup after reload. Existing achievement Off/exit/
OS-reduction cases passed in the 30-case production browser suite. Initial
reload keyboard-before-mount harness failures were corrected by waiting for
the real toolbar/language/theme before invoking the palette. Screenshots saved,
but the current image tool refuses inspection; no new visual acceptance claimed.
Whole-web admission and coordinated integration remain separate.
