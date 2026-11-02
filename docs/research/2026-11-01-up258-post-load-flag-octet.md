# UP258 allocation mismatch: a firmware-maintained flag octet (strong hypothesis)

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

## Consequence for KNXBench

The strict comparison stays the default. Net can now accept a *declared* post-load value at a
*declared* written octet, report each such octet as an observation and keep every other byte
strict ([contract](../contracts/system2-download-result.md#declared-post-load-octets)). No
shipped profile declares an octet yet. The UP258 declaration waits for the corroborating
snapshot, so no verification is relaxed on a single unconfirmed observation.
