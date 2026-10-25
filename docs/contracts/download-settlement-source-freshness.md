# Download settlement and product-source freshness

ADR-0129. When a finished download may mark device memory parts programmed.
Change this file in the same commit as the behaviour it describes.

## Rule

A completed download credits Application, Parameters and Group addresses
only if the product source *now* still plans the same download. At the end
of the run, under the project lock and one catalogue read lease, the server:

1. reads the project's current request, exactly as before (it decides which
   parts the project still describes: address, application, parameters,
   tables);
2. plans the **original written request** again from the current catalogue
   through the same preparation as a download (kind, plug-in, image and plan
   refusals included; the same partial scope), and compares the result with
   the plan that ran, step for step (`knx_app::device_download::source_still_plans`).

Equal plans keep the per-part rules unchanged. Metadata or unrelated program
changes that leave the selected plan identical keep their credit; a partial
download of the group address tables is not invalidated by a changed
parameter segment it never wrote. A combined parameters-and-tables run loses
both memory credits if either part changes the selected combined plan.

A different, refused, missing or unreadable plan leaves the memory parts the
run wrote **pending**. Their undo/redo claims are still forgotten. A complete
download still confirms the address it reached the device at when the
project's address is unchanged; a partial run never gains Address or
Application. The transfer result, restart note, post-download check,
interrupted and nothing-written classification and download/activity history are untouched.

The original request is used for the comparison, not the edited one, so an
ordinary parameter, link or address edit during a run keeps the existing
selective outcome.

## Limits

- A snapshot at settlement, not a subscription: a catalogue change after the
  settlement does not revoke a mark. Mark it pending by hand or download again.
- A source-bound software check. It is not a read of the device, not exemplar
  or recovery evidence, and not proof that the device holds the planned bytes.
- A reinstall that keeps the previous winning source is, correctly, not a change.
- Only parts written by this run are settled. Other marks are not downgraded,
  even when the current source would plan those parts differently.
- Equality covers the executable selected plan, not inference annotations,
  byte-order warnings or product identity outside that plan. It is not proof
  that earlier memory contents or unselected parts fit the current source.

## What the tests execute

Synthetic fixtures only (a mask-0701 program the tests write themselves, whose
`hardware2program` row is inserted by hand with foreign keys off; that models a
prepared download, not a whole source install, and says nothing about package
admission). The source is replaced through the owner's public backup restore.

- **Persistence boundary.** With a saved `.knxdb` (`store_path` set) the
  settlement itself persists the *working journal* of that file, the same
  recoverable journal every project command writes. It is not an explicit
  save. A fresh server that opens the file
  recovers the settled flags and the history, and undo and redo there (and in
  the live state) bring no memory claim back. Flags a partial run did not write
  keep their value and their history.
- **Current snapshot.** The check reads one catalogue lease when the run ends;
  nothing observes later catalogue changes.
- **Unhealthy owner.** If the lease is refused (the owner quarantined by an
  uncertain mutation or poisoned by a panicking one) the callback still
  settles, conservatively: no request is readable, so nothing is credited,
  not even the address. The callback has no access to change the transfer result.
- **Edits during a run.** A parameter or link edit over an unchanged source
  keeps the credit of the parts the edit leaves current (the original request
  is replanned, not the edited one). The same edit together with a changed
  source credits the address only.
- **Admission.** A changed mask check, task segment or load control, a newly
  declared download plug-in and an absent program are each not the same plan.
