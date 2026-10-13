# AP3 / KL-43 — reduction-only category motion

Own feature/ux-feedback-20261010; no integration/push yet.

- Pure default inheritance RED -> GREEN, category/OS priority semantic RED
  before policy implementation, settings-panel native control RED before wiring.
- Missing/unknown category strings inherit; no automatic replacement of unknown
  persisted preferences. independent keys use unchanged settingsStorage.
- Existing global style and Off remain authoritative; OS reduced motion stays
  structural in CSS and live in the flow observer. No project/domain/API changes.
- Runtime category attributes drive Toast/Achievement reference duration; flow Off
  stops actual frames/nudges/pulses. Subtle keeps the legacy bounded flow motion.
- Native settings controls tested through actual adapter, panel reopen and global
  Off override without losing category preferences. Real bootstrap parity tested
  against old/invalid cache entries. Focused92 passed /0 failed /6 files.
- Source/spec/fixture/config typecheck and production build pass. Shared final
  full Web/browser/visual acceptance pending foreign leases; no false pass claim.
- ADR0109 Proposed records policy and deferred independent per-category styles.
- AP1 frozen candidate219e4c3e remains on branch; Session2 surfaces untouched.
- Next: AP2 independent export-format research and bounded capture implementation.
