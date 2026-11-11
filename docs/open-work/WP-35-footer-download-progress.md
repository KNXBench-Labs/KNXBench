# WP-35 — Footer progress for a running device download (#43)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **open.**

## Requested outcome

While a device download runs, the footer shows the device and step/percentage
progress across navigation, with a link back to its download dialog. Data
already exists (`BusActivityLive` session `completedSteps`/`totalSteps`,
`busConnection.ts` occupied state). Cancellation stays in the owning dialog.
