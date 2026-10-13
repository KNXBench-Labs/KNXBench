# 19.19 The presence detector 1.1.8: download, check, restore (2026-10-09)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1919-the-presence-detector-118-download-check-restore-2026-10-09"></a>
<!-- BEGIN preserved record-2-028 -->
### 19.19 The presence detector 1.1.8: download, check, restore (2026-10-09)

The first live download of a program other than MDT `A-0027-15-0BAC`, run
with the maintainer's go for this device. **[V]**

- **Device and plan.** `1.1.8` "PM - Dachboden", Eibmarkt "Universal
  Präsenzmelder 360" (N000520), program `M-006A_A-0001-22-617E-O0079` (ETS
  6.3's conversion of the `.vd4` program), mask `0701h`, manufacturer `006Ah`.
  Project: "Unser Zuhause ets 6.3.0 - 2026-09-02", imported into a scratch
  store; the product database was ingested from the same project. Plan: 25
  steps, 530 octets in four regions (`4000h`, `4003h`–`407Ah`,
  `407Bh`–`40F3h`, `40F4h`–`4213h`), no access key, no inference; the
  machine-5 task segment is not sent (§19.12). Readiness `untested`, so the
  run needed both phrases.
- **Before (read only).** `knx device compare`: mask, manufacturer and three
  parts `Loaded` as planned; 23 of 530 octets differ in 22 runs: 21 group
  object flag octets (`40FDh`–`4151h`, the causes of §19.13: C set on unlinked
  active objects, a documented user decision, and inactive objects' other
  bits) and `brightnessThresholdPIR_0` at `4194h`–`4195h` (device `0064h`,
  project default `012Ch`, §19.13).
- **Download.** Pre-write backup of the 530 octets (four regions) first; it
  matches the compare's device bytes at all 23 differing octets. Compare
  Property `0/78` answered `00 00 00 00 00 03`; the three unloads and every
  load-state step reached the expected state; 46 memory writes, each read back;
  all three parts `Loaded`. `A_Restart` got no T_ACK after four
  transmissions: `RestartOutcome::Unconfirmed`, as on `1.1.67` (§136).
  Run time 09:57:51–09:59:44.
- **After.** 40 s later an independent `knx device compare`: all 530 octets
  equal the plan, three parts `Loaded`.
- **Functional check.** A bus monitor ran while the maintainer walked into the
  attic: `1.1.8 -> 2/0/35` (`SD_AN_AUS_SP02_FLURLICHT`) `GroupValueWrite 1` at
  10:02:01.7, then `0` at 10:03:05.3 after leaving. The device runs the
  downloaded configuration and switches as before.
- **Restore.** `knx device restore` of the pre-write backup (same 25-step
  procedure, its own pre-restore backup first): 530 octets read back, three
  parts `Loaded`, restart unconfirmed. The pre-restore backup differs from the
  pre-write backup at exactly the 23 octets above. 40 s later `knx device
  compare` listed the same 22 runs as before, line for line: the device is
  back to its pre-test bytes, threshold `0064h` included.
- **Consequence.** `M-006A_A-0001-22-617E-O0079` joins
  `crates/knx-app/data/verified_downloads.json` with scope `complete`. Evidence
  is per program, so the house's nine detectors (`1.1.1`–`1.1.9`, all this
  program) now grade `verified`; partial scopes did not run and stay
  untested. The `.vd4` legacy program (ADR-0094 L4) was **not** downloaded;
  its plan differs from this one at `4196h`–`4197h` and stays untested.
- **Artefacts** (gitignored): `OriginalData/DeviceBackups/2026-10-09_pm-118-live/`
  (compare before/after/after-restore, download and restore runs, the
  monitor log, both backups, activity history).


<!-- END preserved record-2-028 -->
