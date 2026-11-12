# WP-35 — Footer progress for a running device download (#43)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
**Closed 12 November 2026** (removed from [OPEN_WORK](../OPEN_WORK.md); issue closed where one exists). Status: **implemented 11 November 2026.** `DownloadFooterProgress` reads the
bus-connection observation the status bar already polls (no new request);
**Show** opens the download tab without selecting or starting anything. Not
done: a short "finished" note in the status bar after the run ends (the result
stays in the tab and the activity history).

## Requested outcome

While a device download runs, the footer shows the device and step/percentage
progress across navigation, with a link back to its download dialog. Data
already exists (`BusActivityLive` session `completedSteps`/`totalSteps`,
`busConnection.ts` occupied state). Cancellation stays in the owning dialog.
