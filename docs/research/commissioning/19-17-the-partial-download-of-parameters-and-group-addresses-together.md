# 19.17 The partial download of parameters and group addresses together (2026-09-30)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="1917-the-partial-download-of-parameters-and-group-addresses-together-2026-09-30"></a>
<!-- BEGIN preserved record-2-026 -->
### 19.17 The partial download of parameters and group addresses together (2026-09-30)

User request "starte mit teildownload", the last partial scope not yet run.
Evidence in `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-30_kboth-*`
and `1.1.67-prewrite-kboth/` (private).

- **[V] `--partial both` completes on `1.1.67`.** Binary built from
  `786ec9f2`, no shell timeout. Read-only first: scan `1.1.67` occupied,
  dump 180 lines byte-identical to the K13 post-dump, `device compare` all
  three `Loaded` and 1416 octets as planned. 16:00:53–16:03:35 `knx device
  download 1.1.67 --partial both`, 24 steps: checks 1–6 (mask, property
  0/78, `PID_PROGRAM_VERSION`, three load states), backup of 4 regions
  (`4000h`, `4003h`–`4200h`, `4201h`–`43FFh`, `4400h`–`4589h`; 1416 octets)
  before step 7, matching the pre-dump in every octet; unload both tables,
  load them, load the application program *without* unloading it (the CP
  §3.9.2.4 difference from a complete download), 1416 octets each read back,
  all three `Loaded`. `A_Restart` unacknowledged, as always. 40 s later:
  `device compare` all three `Loaded`, identical; dump 180 lines
  byte-identical to the pre-dump.
- **[V] Writes were fast again:** under 3 minutes for 1416 octets, against
  12 minutes for 1022 octets at 12:47 the same day (§19.15). The cause of
  the slow morning is still not measured.
- **[I] Same image written**, so this proves the procedure and the
  load-state sequence, not a configuration change. `partial-both` joins
  `crates/knx-app/data/verified_downloads.json`; every download scope of
  this program is now verified on hardware.


<!-- END preserved record-2-026 -->
