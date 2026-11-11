# Batch download

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-batch-download)

Recorded 11 November 2026 with WP-38b (server) and WP-38c (UI), issue #42,
[ADR-0172](../adr/0172-batch-programming-stops-at-the-first-doubt.md).

- **Evidence.** Server integration tests drive the batch routes against
  simulated mask-`0701h` devices behind fake tunnels; the browser cases
  intercept every `/api/**` request. No real gateway, line or device took part.
  Whether a gateway or line behaves differently when several devices are
  programmed back to back is unknown.
- **Order.** Address order only. Couplers first or line by line is not
  implemented because no verified rule exists.
- **What stays outside.** A device whose individual address is pending (address
  programming needs a button press per device), System-2 devices (own panel and
  phrase), plans graded `unsupported`, and plans with no verified identity
  layout or application identity, because their post-download check could never
  let the batch continue. The single-device download stays available for them.
- **Durability.** The batch summary is held in server memory until the next
  batch or a restart. Each device keeps its own activity-history row, operation
  recording (with `batchId`), download report and flag settlement. A crash
  leaves the device in progress `interrupted/unknown` and starts nothing more;
  a batch never resumes.
- **Busy wording.** Routes other than the download start keep their refusal
  text while a batch runs; they name the batch in the structured `holder`.
