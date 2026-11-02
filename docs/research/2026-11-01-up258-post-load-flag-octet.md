# UP258 allocation mismatch: a firmware-maintained flag octet

1 November 2026. Target: UP 258D12, application `M-0001_A-9205-04-9270`, mask `0025h`
(System 2), the allocation mismatch reported in GitHub issues #30 and #22. Offline analysis
of data the reporter already supplied; no bus access, no new hardware run. No private bytes,
project content or capture is reproduced here. Vocabulary as in the
[rule sources](2026-10-20-system2-validation-rules.md): **source observation**, **strong
hypothesis**, **unresolved**.

## Inputs

- The public application of the exact program (source observation): parameter `Download_Flag`,
  `Access="None"`, default value 1, in absolute segment `3C00h` at offset 244, i.e. octet
  `3CF4h`. Hidden from the user; written by every download that writes the segment.
- One private reporter set, matched to one private project by digest and serial: two
  declared-segment memory snapshots taken with Alpha 15 before and after an ETS unload, and a
  bus-monitor recording of an ETS download of the same device on the same day.
- KNXBench's own offline plan for that device on current `main` (configured profile v2), built
  from the same project with the shipped CLI in a scratch store.

## Observations

1. The two snapshots are identical over all declared segments.
2. ETS writes the device memory and does not read it back afterwards.
3. Before the ETS download, the device held exactly the values ETS then writes at every written
   octet except one: `3CF4h`, where the device held `00h` and both ETS and KNXBench write `01h`.
4. KNXBench verifies the whole allocation only after the Load Completed controls (the last plan
   step); the reported failure was a verification mismatch in that step.

## Strong hypothesis

The firmware clears `Download_Flag` after a completed load. A strict post-load read-back then
sees `00h` where `01h` was written and refuses, although the download itself succeeded — which
matches the reporter's statement that the device works. ETS never reads back, so it never sees
the difference.

Not established: when the firmware clears the octet (before or after the KNXBench read-back),
whether it can hold any other value, and whether other UP258 firmware revisions behave alike. A
snapshot of the same device after any successful download that shows `00h` at `3CF4h` would
corroborate it; that was requested from the reporter in #30.

## Corroboration (2 November 2026)

The reporter sent two further read-only `system2-declared-segments-v1` snapshots, taken with
Alpha 19 about three minutes apart, of the same device while it still held the earlier KNXBench
Alpha 17 download (no ETS download since). No bus access by us; the bytes stay private.

1. Both snapshots are identical: no ongoing self-modification in the declared segments.
2. Against KNXBench's plan for the same project, every written octet matches except two: one
   parameter octet the reporter had changed in KNXBench himself, and `3CF4h`, where the plan
   writes `01h` and the device holds `00h`. Every unwritten octet still holds its prior value.
3. Alpha 17 wrote with Verify Mode and checked each memory chunk on write, and the run passed the
   step that writes segment `3C00h`. The octet therefore held `01h` right after the write; the
   device cleared it itself.

This settles the open question above: the download took effect, and the firmware clears
`Download_Flag` afterwards. Still not observed: whether it clears at Load Completed or at the
following restart (only a read-back between the write and Load Completed would show it), and
whether other UP258 firmware revisions behave alike.

## Consequence for KNXBench

The strict comparison stays the default. Net can accept a *declared* post-load value at a
*declared* written octet of the download and report each such octet with its prestate as an
observation. Every other byte stays strict, as does every restore that writes other bytes than
the download (as here, where the prestate differs at `3CF4h`)
([contract](../contracts/system2-download-result.md#declared-post-load-octets)).

Since 2 November 2026 a configured UP258 preparation observed with that build (hardware record
revision 3) declares exactly this octet: object 3, `3CF4h`, accepted `00h`, only when the plan
writes it with another value. Older records stay strict. The preview names the octet before
consent; the journal, status and panel record each tolerated value with its prestate, the
shareable report only that an octet of object 3 was tolerated. An opt-in test confirms on the
pinned public package that its plan writes `01h` at exactly that octet.

The prestate caveat of the first analysis no longer decides anything: the reporter's device held
`00h` there before the download too, but the per-chunk Verify Mode check proves the write itself,
so a tolerated `00h` after load is the firmware's own value, not a lost write.
