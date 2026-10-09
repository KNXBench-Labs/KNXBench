# Research — Commissioning and device download

Application download, download coverage, parameter encoding and write-recovery scopes. Part of [RESEARCH](../RESEARCH.md), which holds the evidence
tags (**[V]** verified here, **[D]** documented, **[A]** assumption), the
section index and the sources. Section numbers are global and stable;
dated entries are newest first. Moved here verbatim from `RESEARCH.md` on
2026-10-04 (AR14D D4); only relative links changed.

## 2026-10-09 — Bathroom fourfold button: complete download and restore

- §19.21: MDT `1.1.14`, program `M-0083_A-0026-15-3591`, mask `0701h`.
  Complete download of 1562 octets, independent readback, main/mirror-light
  telegrams, then restore with all 1570 selected memory/load-state octets
  identical to the pre-dump. Closing restarts unconfirmed. Only the
  `complete` scope gains evidence; partial scopes were not run.

## 2026-10-09 — A second program verified live: the presence detector 1.1.8

- §19.19: `knx device download` wrote the house project's configuration to
  presence detector `1.1.8` (Eibmarkt N000520, `M-006A_A-0001-22-617E-O0079`,
  mask `0701h`): 25 steps, 530 octets read back, three parts `Loaded`, an
  independent compare clean, a functional check on the bus, then a restore
  from the pre-write backup that left the device byte-identical to before.
  The program becomes the second entry of `verified_downloads.json`
  (scope `complete` only).

## 2026-10-07 — Inferences under ADR-0086: the house plans 32 of 35 devices

- ADR-0086 makes the specification, product data and project files the
  evidence of record; working solutions where they are silent ship as named
  inferences, disclosed in readiness, plan and acknowledgement.
- §19.12 follow-ups: two active union members (`union-later-member`), the
  machine-5 task segment (`[D]` Cookbook `02_03_01` §2.3 `AbsCObjSeg`),
  `LdCtrlTaskCtrl1` (`[D]` MP §3.31.2) and machine-5 events after the
  restart (`machine-5-after-restart`).
- §19.18: `Alert` written as urgent from a 17 731-object census of the
  corpus's base images; `ReadOnInit` documented as a System B feature
  (`[D]` Resources NOTE 85) and disclosed on `070nh`; `High` and floats stay
  refused for contradicting product data. Corpus: 1 verified + 89 untested
  of 246 programs (was 1 + 77 this morning).

## 2026-10-01 — Backup directory chains, not only the final directory

- **[D]** Linux [`fsync(2)`](https://man7.org/linux/man-pages/man2/fsync.2.html)
  distinguishes syncing a file's data/metadata from syncing the containing
  directory's entry. The directory must be synced separately. A successful
  call is the OS/storage completion report, not a power-loss experiment.
- **[D]** Rust [`create_dir_all`](https://doc.rust-lang.org/std/fs/fn.create_dir_all.html)
  creates missing parent components and is explicitly non-atomic;
  [`File::sync_all`](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)
  requests content and metadata synchronization. Sources fetched directly
  on 2026-10-01; the configured text-extraction backend refused extraction,
  so the primary HTML was retrieved with the terminal's HTTP client.
- **[V]** Both application backup writers create nested directories, sync the
  new file, verify readback, and sync only the final directory. Missing
  parent directories can therefore acquire entries whose parent sync is
  not explicitly requested by either writer.
- **[A]** Conservatively sync both the absolute supplied path and its resolved
  target directory chains, child-to-parent, after verified file readback,
  deduplicating shared path spellings in traversal order. This applies the
  documented per-directory rule to every possibly new ancestor entry without
  guessing which components existed beforehand or trusting partial mkdir
  progress. Keep symlink and `..` components: canonicalization can erase
  alias-containing directories or intermediate components needed to reopen
  the originally returned path; the resolved target also needs its own
  ancestors. Anchor relative paths to the process CWD; do not synchronize an
  empty relative ancestor. Any sync/path-resolution error must refuse a backup
  receipt. Avoid new dependencies and share the traversal between the writers.
- **[A]** This does not prove filesystem/hardware power-loss behavior, defend
  against concurrent directory replacement, provide process-crash recovery,
  or make plan-scoped backups whole-device recovery evidence. It must not
  open the address-write gates. Offline tests can verify traversal order and
  error propagation; they cannot simulate actual disk durability.
- **[V]** The implemented shared helper covers both directory chains. Four
  initial writer regressions were RED with unchanged leaf-sync behavior;
  the alias-target regression was separately RED with a supplied-path-only
  traversal. Sixteen focused tests pass after the fix. Restored leaf-only and
  canonical-only guard mutations caused eight and two failures respectively.
  Real temporary files/readback/fsync are exercised, with injected parent
  errors; these are request-order/error tests, not a physical crash experiment.
- **[V]** A separate in-session review RED caught accidental empty-path-to-CWD
  normalization. The shared helper now refuses empty paths before any directory
  sync callback, preserving the previous receipt refusal without repairing names.

## 2026-09-29 — Parameter fields across an octet boundary; module instances measured

- **[D]** A numeric field across an octet boundary is written MSB-first on
  into the next octet (*Configuration Procedures* §8.5.4, *Resources*
  §4.18.5.2.5). The 4 programs held up by one such field now plan: **1
  verified, 77 untested**, 168 refused (`parameter-value` 9). Untested.
- **[V]** Module instances were measured and stay refused: the stated
  argument rule does not rebuild the products' base images. §19.11.

## 2026-09-29 — Rename leaves no longer block a download image

- **[V]** `Rename`/`ParameterBlockRename` (326 in the corpus, every one an
  empty leaf retitling a `ParameterBlock`) no longer refuses an image. 4
  more programs plan: **1 verified, 73 untested**, 172 refused
  (`parameter-evaluation` 51, `parameter-value` 13). The remaining
  `image-structure` refusals were re-checked against the PDFs and stay
  refused. §19.10.

## 2026-09-29 — Download coverage re-measured: signed and text values, honest mask refusals

- **[V]** Same 103 packages, same 246 programs: **70** plan (1 verified,
  **69** untested; was 55), 176 refused. Within `MV-0701` 35/40, within
  `MV-0705` 35/141. Reasons: `not-memory-mapped` 65, `parameter-evaluation`
  59, `image-structure` 41, `parameter-value` 9, `unmodelled-step` 2. The
  41 `procedure-style` refusals were all non-`070nh` masks and now say so.
  Details, sources and what stays refused: §19.9. Still the local sample,
  not the market; the counts in the entry below are superseded.

## 2026-09-29 — Commissioning readiness: offline coverage of the installed product corpus

- **[V]** `knx products coverage --product-db <db>` evaluated the **103**
  locally available `.knxprod` packages installed into a fresh database:
  **246** application programs, **55** complete default/no-link memory plans
  (one program with a cited hardware run, 54 without), **191** refused with
  named reasons. Within `MV-0701` and `MV-0705` specifically, **55 of 181**
  planned (28/40 and 27/141 respectively); 126 were refused. Outside those
  masks, no program planned. Reasons across all 246 programs:
  `not-memory-mapped` 24, `procedure-style` 41, `unmodelled-step` 1,
  `parameter-evaluation` 59, `parameter-value` 33, `image-structure` 33.
  This is **not** a figure for all KNX devices: the denominator is the
  installed local sample, not the market. A different corpus or configured
  project can yield different counts. The earlier count of 203 `070n`
  applications was obtained under a different corpus/counting boundary and
  must not be substituted for this 181-program database run.
- **[V]** This evaluation is repeatable via
  `crates/knx-app/tests/download_coverage_corpus.rs` (ignored unless the
  private corpus is present). A plan only says the product's **default**
  image, no group links, and its load procedure are representable. It does
  not prove a user-configured image or hardware response.
- **[V]** `M-0083_A-0027-15-0BAC` has a cited live complete download
  (RESEARCH §19.4) and parameter-only partial download (§19.8) on one MDT
  device. Other plans are `untested` even if they share its mask or vendor;
  group-address partial has no hardware evidence. The implementation ships
  only these two scoped evidence records. See [ADR-0049](../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md).
- **[V]** Pre-write region backup, file read-back, failure-before-mutation,
  restore, and CLI/HTTP support gates passed simulator/corpus tests.
- **[V] Live roundtrip on `1.1.67` (2026-09-29 15:17–15:26, after the user's
  "Hardware go").** Read-only dump before: 180 lines, identical to
  `post-k15` (option C, load states `01 01 01`). (1) `knx device download`
  of the K7 project wrote the backup JSON (1416 octets in 4 regions:
  `4000h`/1, `4003h`/510, `4201h`/511, `4400h`/394, all three machines
  `Loaded`) after the identity checks and **before** step 3 (the first
  write); 0 of the 1416 backed-up octets differ from the independent
  pre-run dump. Download 1416/1416 read back, restart unconfirmed as
  usual; the dump after ~45 s equals `post-k7-cli` (the K7 config).
  (2) `knx device restore <backup.json>` took its own pre-write backup
  (again 0 differing octets against the post-download dump), wrote
  1416/1416; after ~45 s the independent dump is **byte-identical to the
  pre-run dump in all 180 lines**, load states `01 01 01`. Proves on this
  one device and program that the backup is taken before the first write
  and restores the overwritten regions. The failure-before-mutation path
  stays simulator-only (not provoked on hardware). Logs:
  `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-29_{pre-backup-live,
  backup-live-download,post-backup-live-download,backup-live-restore,
  post-backup-live-restore}.txt` and the two JSON files under
  `1.1.67-prewrite/` and `1.1.67-prerestore/` (gitignored).
- **[V] Live roundtrip, partial download (parameters), 2026-09-29
  16:02–16:12.** Pre-dump = option C. (1) `--partial parameters` with the
  option C project: backup (1 region, `4400h`, 394 octets; `partial`
  recorded as parameters only; load state of the application `Loaded`) was
  written after the identity and all-`Loaded` checks (steps 1–6) and before
  step 7; 0 differing octets against the pre-dump; dump after the write
  unchanged; `knx device restore` of it wrote 394/394, dump unchanged
  (180/180). (2) The discriminating case: `--partial parameters` with the K7
  project: backup 394 octets, 0 differing against the device; after the
  write the dump equals `post-k7-cli` (3 dump lines changed, the K7
  parameters). `knx device restore` of that backup (own pre-write backup
  first) wrote 394/394; after ~40 s the dump is **byte-identical to the
  pre-dump in 180/180 lines**, load states `01 01 01`. Restart unconfirmed
  every time, as usual. Logs `…_{pre-backup-partial,backup-partial-*,
  post-backup-partial-*}.txt`, JSON under `1.1.67-prewrite-partial*/` and
  `1.1.67-prerestore-partial*/`.

---

## 19. Application download to a mask `0701h` (BIM M112) device (2026-09-27)

Trigger: a request to configure button 1 of the MDT push button at `1.1.67`
(§8.8.6) as an ON/OFF toggle on group address `2/0/53`. That is an
application download. This section records what the Standard and the product
data do and do not supply for it. Markers as in §4.3.

**Product identity.** The device reports mask `0701h`, manufacturer `0083h`
and `PID_HARDWARE_TYPE` `000000000127` (§8.8.6) **[V]**. In
`MDT_KP_BE_01_Push_Button_V15a.knxprod`, two applications check exactly that
hardware type in `LdCtrlCompareProp PropId="78"`: `A-0023-15-3EC1`
(*Taster 2-fach*, `BE-TA5502.01`) and `A-0027-15-0BAC` (*Taster 2-fach Plus*,
`BE-TA55P2.01`) **[V]**. So the hardware type does not tell the plain device
from the Plus. The type plate (`BE-TA55P2…`) names the Plus, so the matching
application is `A-0027-15-0BAC`, with `ApplicationNumber` 39,
`ApplicationVersion` 21, `MaskVersion` `MV-0701` and `PeiType` 1 **[V]**.
The newer `BE-TA55xx-x2_MDT_KP_V20a.knxprod` (`BE-TA55P2.02`,
`A-0227-20-7DE8`) checks hardware type `0x0239` and mask `MV-0705`, so it
does **not** match this device **[V]**.

**Load procedure (`A-0027-15-0BAC`, verbatim) [V].** The steps are
`LdCtrlConnect`, `LdCtrlCompareProp` (hardware type), `LdCtrlUnload` for
LSM 1, 2 and 3, then one Load / `LdCtrlAbsSegment` / `LdCtrlTaskSegment` /
`LdCtrlLoadCompleted` block for each LSM:

| LSM | segment | `Address` | `Size` | `Access` | `MemType` | `SegFlags` | content |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 (address table) | `AS-4000` | `4000h` | 513 | `FFh` | 3 EEPROM | `80h` | `AddressTable`, MaxEntries 255; 513 data + 513 mask octets |
| 2 (association table) | `AS-4201` | `4201h` | 511 | `FFh` | 3 | `80h` | `AssociationTable`, MaxEntries 255 |
| 3 (application) | `AS-0700` | `0700h` | 152 | `00h` | 2 RAM | `00h` | no data |
| 3 | `AS-0798` stack (`SegType=1`) | `0798h` | 1 | `00h` | 2 | `00h` | no data |
| 3 | `AS-4400` | `4400h` | 394 | `FFh` | 3 | `80h` | `ComObjectTable` (offset 0) and all 66 memory-placed parameters |

The procedure ends with `LdCtrlRestart` and `LdCtrlDisconnect`. The
application has 64 `ComObject`s, 161 `ComObjectRef`s and 81 `choose` blocks.

**How the device takes load events: `DMP_LoadStateMachineWrite_RCo_Mem`.**
MP (`03_05_02` v02.01.02) §3.31.2 states that this procedure *"shall only be
used with device model for mask version 070nh (BIM M112)"* **[D]**. Its rules
**[D]**:

- It uses a connection-oriented session and no Verify Mode.
- Each event is one `A_Memory_Write` of 11 octets (`0Bh`) to the management
  control at `0104h`.
- The client then reads 1 octet of load state back, at most 3 times, from
  `B6EAh` (address table), `B6EBh` (association table), `B6ECh`
  (application) or `B6EDh` (PEI).
- The record is octet 1 *"state machine / event"*, followed by the event data.
  For Unload, Load and LoadComplete the event data is 10 reserved `00h`
  octets.
- For AllocAbsDataSeg / AllocAbsStackSeg the record is `L3`, segment type
  `00h`/`01h`, segment ID `00h`, start `SSSS`, length `EEEE-SSSS+1`, access
  `AA` (bits 0–3 write level, 4–7 read level), memory type `TT`
  (1 zero-page RAM, 2 RAM, 3 EEPROM), memory attributes `MM` (bit 7 =
  checksum control) and one reserved `00h`.
- For AllocAbsTaskSeg the record is `L3 02h 00h SSSS PP MMMM TTTT VV`: the
  PEI type, then the manufacturer, application ID and version.
- TaskPtr (`03h`), TaskCtrl1 (`04h`) and TaskCtrl2 (`05h`) are also defined.

MP never gives numbers for `L1`–`L4`. The page image shows them in italics,
with no legend (checked by rendering p. 135) **[V]**. The numbers are in
*Test Suite Supplement G — Load State Machines Tests* (`08_TSSG` v01.02.01
AS), which describes every property event together with its memory-mapped
twin **[D]**:

- The first octet is `(state machine type << 4) | event`. Type is 1 address
  table, 2 association table, 3 application, 4 PEI, matching
  `DM_LoadStateMachineWrite`'s `stateMachineType`. Event is 0 NoOp, 1 Load,
  2 LoadCompleted, 3 additional load control, 4 Unload. For example, `14h`
  unloads the address table and `22h` completes the association table.
- The state read back is 00 Unloaded, 01 Loaded, 02 Loading or 03 Error.
- It has a worked AllocAbsDataSeg example:
  `23 00 00 42 00 00 10 FF 03 80 00` (start `4200h`, length `0010h`, EEPROM,
  checksum on).
- It has a worked AllocAbsTaskSeg example:
  `23 02 00 42 00 80 00 02 A0 4A 10`.

TSSG itself is not consistent **[V]**. Three occurrences of the data-segment
record carry a 12th octet, which contradicts MP's stated length of `0Bh`.
Its prose annotation also gives the application ID as `0A4Ah`, while the
bytes say `A04Ah`. KNXBench should follow MP's 11-octet layout.

The mapping of `LdCtrlAbsSegment`/`LdCtrlTaskSegment` attributes onto these
records is **[A]**, but it is strongly constrained:

- `LsmIdx` → type nibble.
- `SegType`, `Address`, `Size`, `Access`, `MemType` and `SegFlags` map one
  to one.
- The task segment's manufacturer, application ID and version would be
  `0083h`, `0027h` and `15h`, the same numbers as the application's own id
  `A-0027-15`.

This narrows the gap flagged in §8.6.4 ("absolute-addressing and BCU1/BIM
M112 family … undocumented"). §8.6.4 searched for the `LdCtrl*` *names*,
which appear nowhere in the Standard. The *procedure* those names drive on
mask `070nh` is documented, as described above. The encodings for the other
masks remain open.

**Table formats.** Profiles (`06` v02.01.01) §10.2.7.7 points mask `0701h`
at Resources (`03_05_01` v01.10.01) for the table formats **[D]**:

- §4.16.11 for the Group Address Table format and management-client usage.
- §4.17.9 (*GrOAT – Easy 3*) for the association table.
- §4.19.4 (*Parameter Block Table – Realisation Type 3*) for parameters.

Resources §4.23.3 (*Load State Machine – Realisation Type 2, memory mapped*)
reads *"not specified in this version"* **[D]**. The state and event octets
above therefore come only from MP §3.31.2 and TSSG.

**What an end-to-end download would still need [V]:**

1. ~~A `DMP_LoadStateMachineWrite_RCo_Mem` transport.~~ *Built 2026-09-27,
   simulator only.* `knx_core::commissioning::load_control_memory` builds
   the eleven-octet records and pins them to TSSG's example octets.
   `ManagementSession::write_memory_load_record` reads the state once, sends
   the record as one `A_Memory_Write` to `0104h`, and reads the state back at
   most three times, checking every read against RES Table 94. Two choices
   are not in MP and are recorded here:
   - **Read before the write.** MP's sequence has none, but Table 94 can
     only judge a state relative to the one before it. It is a read, and a
     failed one sends nothing.
   - **Interval between reads.** MP gives none; the property procedure's
     `poll_interval` is reused.

   A lost connection is an error, as MP's *"A_Disconnect.ind ⇒ error"* says;
   unlike the property procedure, nothing re-establishes it. A session that
   knows the mask is `070nh` and is authorised to download or unload does
   not set Verify Mode on connecting, because MP §3.31.2 forbids it; a
   session that learned the mask after connecting refuses the record rather
   than send it with Verify Mode on. The prohibition is read as belonging to
   the load procedure only, not to the device: MP §2.3 address programming
   and restart keep Verify Mode, which is how the address write of
   2026-09-26 succeeded on this `0701h` device. `download.rs`
   does not call any of this yet.
2. ~~Serializers for the Group Address Table (§4.16.11) and the Easy 3
   association table (§4.17.9).~~ *Built 2026-09-27.*
   `knx_core::commissioning::group_tables::build_group_tables` builds both
   tables from `(object, group address, sending)` links:
   - **Address table:** Length (the individual address included), the
     individual address, then the group addresses, sorted and de-duplicated.
   - **Association table:** Current Size, then `TSAP | ASAP` pairs, grouped
     by object with the object's sending association first (§4.17.9.5).
   - **Refusals:** broadcast `0/0/0`, a duplicate link, a second sending
     address on one object, and anything over the product's `MaxEntries` or
     the one count octet (254 group addresses, 255 associations). Nothing is
     truncated.

   It rebuilds the product data's own defaults octet for octet (`AS-4000`
   `03 0000 1900 1901`, `AS-4201` `02 0100 0205`).
   **Byte order, settled `[V]` 2026-09-28:** entries are stored high octet
   first. No clause of Resources states it. It was assumed from indirect
   sources:
   - §4.16.3.4.2's `DMP_MemWrite_LEmi1(0117h, 0118h, PPPPh)`.
   - API §1.2.1's *"Big Endian"* EEPROM pointers.

   A read-back of the real device then confirmed it (§19.1).

#### 19.1 Live read-back of the device at `1.1.67` (2026-09-28, read-only)

`crates/knx-net/tests/live_memory_readonly.rs` read the three segments a
download rewrites. It used `read_only` + `AuthorisationPlan::Skip`, so no
write and no `A_Authorize_Request` was sent.
- **Coverage:** `AS-4000` (513 octets), `AS-4201` (511) and `AS-4400`
  (394), plus the load states: 180 `A_Memory_Read`s of at most 8 octets
  each, all answered. The first attempt read 0 octets on a `u8` overflow in
  the test. The device dropped the connection on it and the test was fixed;
  nothing was written.
- **Backup:** the dump is kept outside the repository, in the gitignored
  `OriginalData/DeviceBackups/`, as the device's configuration before any
  download.

What it established `[V]`:
- **Load states** `B6EA`..`B6ED` = `01 01 01 00`: address table,
  association table and application are *Loaded*; the PEI is *Unloaded*.
- **Group Address Table** at `4000h`: `05 1143 0406 0407 110F 1110`. The
  individual address `1143h` is `1.1.67` read high first, which settles the
  entry byte order. The group addresses are `0/4/6`, `0/4/7`, `2/1/15` and
  `2/1/16`, ascending.
- **Association table** at `4201h`: `04 0300 0401 0112 0212`. That is
  objects 0 → `2/1/15`, 1 → `2/1/16`, and 18 → `0/4/6` and `0/4/7`.
  `build_group_tables` produces exactly this layout.
- **Mask:** `AS-4000`'s `<Mask>` is zero at exactly offsets 1–2, the
  individual address. A segment download must not overwrite the device's
  own address with the product default `0000`.
- **Parameter segment:** `AS-4400` starts with the **group object table**
  (`ComObjectTable`, `AS-4400` offset 0). The per-object configuration and
  type octets there differ from `<Data>` (for example `df`→`4f`/`db`, and a
  type `03`→`00`). They depend on the active `ComObjectRef`s, so the
  segment image needs a group object table encoder, not only parameters.
  79 of 394 octets differ from `<Data>`.
- **The device is configured, not factory-fresh.** Buttons 1/2 are a
  grouped *Shutter* pair: the union at +264 reads `0002`, with objects 0/1
  on `2/1/15` and `2/1/16`. The LED orientation light (object 18) listens
  to `0/4/6` and `0/4/7`. A full download replaces all of this.
3. *Partly built 2026-09-27:* the parameter-segment image. The **bit
   writer** is built: `knx_core::commissioning::parameter_image`.
   - **Placement:** it places a value at `Offset`/`BitOffset`/`SizeInBit`.
     `BitOffset` counts from the octet's MSB to the value's MSB (`[D]`
     *Project Schema23 v01.00.00.pdf* §1.1.3.17 `BitOffset_t`, pp. 29–30). Multi-octet values go high octet
     first (`[V]`: all 20 `Options` elements of mask-`0701h` applications
     in the corpus projects say `ParameterByteOrder="BigEndian"`).
   - **Refusals:** shapes that definition does not settle (unaligned
     across octets, a whole-octet width at a bit offset), fields past the
     segment, values too wide, and overlapping writes. It never lets a
     later union member silently overwrite an earlier one.
   - **Still missing:** choosing *which* parameters are written. That means
     evaluating the `Dynamic` tree and picking the active `Union` member.

   **`[V]` The segment base data is not the parameter defaults.** In
   `A-0027-15-0BAC`, `AS-4400`'s `<Data>` disagrees with the effective
   default in 33 of 66 non-union-alternative parameter locations. Examples:
   `P-5003` is 400 by default but 3000 in the data, and `P-1`/`P-2` are 0 by
   default but 3 in the data. MDT's product
   `ParameterRef`s override several of them, so the data is at most one
   snapshot. Consequence: a download must write **every** active parameter
   over the base data, not just the ones the user changed. Otherwise the
   device runs with values the user never saw. The comparison script and
   its output are not in the repository; they are reproducible from the
   `.knxprod` in a few lines.

   The values the toggle on `2/0/53` needs are all enumerated in the
   product data, not guessed. For button 1:
   - `P-1007` *Function buttons 1/2*: `2` = *Push buttons unique*.
   - `UP-5500` *Function* at `AS-4400`+264: `0` = *Switch*.
   - `UP-5501` *Subfunction* at +266: `1` = *Toggle by push*.

   Each is a 16-bit field; the object is `O-0`.
3. An `AS-4400` image builder. It must evaluate the `choose` tree for the
   chosen parameter values (`knx-productdb::dynamic::evaluate` exists) and
   place each parameter's bits at its `Memory` offset over the segment's
   default data. No such encoder exists.
4. The access-key question. TSSG authorizes before the load, but the key the
   device expects is not in the product data and must not be guessed.
5. A policy decision. `WriteScope::Download` is still refused on hardware
   (§8.8.6's allowlist is `IndividualAddressProgramming` and `Restart` only).
   *Settled 2026-09-28:* the operator authorised the `1.1.67` download, and
   `Download` is on the allowlist for the memory download only (§19.3).

### 19.1 Download data in the product file, and what the PDFs say about it (2026-09-28)

Sources: the source PDFs under `knx-spec-kb/sources/` only, read directly.

**`[D]` What a tool writes.** The KNX Cookbook *Load Controls* (`02_03_01`
v01.00.02, pp. 6–7, Figure 4) describes the tool's side of a download. The
product ships a *"default memory image"*, and the tool *"modifies the
default image according to the ETS project settings, being: 1) group
objects 2) group addresses 3) device parameters"*. The load controls then
drive the management procedures. Configuration Procedures (`03_05_03`
v02.01.01 §3.9.3.2, pp. 71–72) maps `LdCtrlConnect`, `LdCtrlUnload`,
`LdCtrlLoad`, `LdCtrlLoadCompleted`, `LdCtrlRestart` and `LdCtrlCompareProp`
onto `DM_*` procedures for the System B mask.

- `LdCtrlConnect` includes `DM_Authorize` with the *project* key.
- The cookbook's worked ADM1 example (pp. 10–12) pairs an `AbsSegment`
  record for each resource with a `TaskSegment` record, between the
  resource's `Load` and `LoadCompleted`.

**`[D]` The allocation record's second field is a length.** MP (`03_05_02`
v02.01.02 §3.31.2, pp. 135–136) writes it as `EEEE - SSSS + 1`. The
cookbook's ADM1 records carry the *end* address (`4000`…`41FE` for a
`01FF`-octet segment). That is ADM1's storage format, not the memory-mapped
record `load_control_memory` builds, which follows MP. The product's
`LdCtrlAbsSegment/@Size` is the length (`Size="513"` for `AS-4000`, whose
`Data` decodes to 513 octets).

**`[V]` Where the data lives.** The following was measured over the 310
application-program files in the private corpus. All of it is in
`ApplicationProgram/Static`:

- `Code/AbsoluteSegment`: 1,492 segments.
  - 980 have base64 `Data` and 305 have `Mask`; every decoded length
    equals `Size`.
  - The only child elements are `Data` and `Mask`.
  - Other attributes: `MemoryType` (605), `UserMemory` (134), `Name` (4).
- `AddressTable`, `AssociationTable`, `ComObjectTable`: placement through
  `CodeSegment`, `Offset` and `MaxEntries`.
- `LoadProcedures/LoadProcedure`:
  - 303 files have them; 379 procedures in total, 116 of which carry a
    `MergeId`.
  - Steps come in 19 `LdCtrl*` kinds. The only child element any step ever
    has is `OnError` (11 times).
  - 7 files put a `choose` inside a procedure.
- Load procedure styles: `ProductProcedure` 263, `MergedProcedure` 40,
  `DefaultProcedure` 7.

`knx_productdb::code` (ADR-0044) reads all of this back from the stored
blob. It parses all 310 files with 0 errors. 59 contain at least one step
it does not model, which a hardware write must refuse by name.

**`[A]` `Mask` has no definition in any PDF.** No KNX PDF read for this
section defines `AbsoluteSegment/Mask`: *Project Schema23* and Volumes 2, 3
and 8. In `A-0027-15-0BAC` the only `Mask` covers `AS-4000` and marks
exactly octets 1–2, which are `00h` in every other position. Those octets
are where the `[V]` read-back of the real device holds its own individual
address (`11 43` = `1.1.67`). Reading the mask as *"octets the tool must not
overwrite"* fits that evidence, and a download treats it that way. It is
still an assumption and is labelled as one.

**`[V]` Correction: MDT does not declare `ParameterByteOrder`.** §19's claim
that mask-`0701h` applications declare `ParameterByteOrder="BigEndian"` was
measured on *project exports*, where ETS materialises every `Options`
attribute. The MDT product file's own `Static/Options` has exactly one
attribute, `LegacyAllowPartialDownloadIfAp2Mismatch="true"`. Only 26 of 310
product files declare `ParameterByteOrder` at all, all `BigEndian`. So
high-octet-first for MDT is **not** declared by the product. It rests on
the project exports' materialised default and on the device read-back of
`1.1.67` (`AS-4400`+260…267). The first two readings are default values,
the next two are the ones the device's shutter configuration selects:

- `P-5002` (default 50) reads `00 32`;
- `P-5003` (default 400) reads `01 90`, where the base data holds `0B B8`
  (3000);
- the union selector at +264 reads `00 02`;
- +266 reads `00 00`.

A little-endian reading would give 12800, 36865 and 512, none of which is
a value these parameters allow. `parameter_image`'s module documentation
carries the same correction.

### 19.2 Assembling a download image from the product file (2026-09-28)

`knx_productdb::image::build_download_image` produces every segment image
of a program from the product file (ADR-0044), the chosen parameter values
and the group address links. What each rule rests on:

- **`[D]` What changes.** The product's default image, modified in its
  group objects, group addresses and parameters: the Cookbook *Load
  Controls* (`02_03_01` v01.00.02, pp. 6–7). Nothing else is written.
- **`[D]` Bit placement.** `BitOffset` is the distance of the value's most
  significant bit from that of the first octet (*Project Schema23*
  §1.1.3.17, pp. 29–30).
- **`[V]` Union members.** A member lies at the union's `Memory` plus its
  own `@Offset`/`@BitOffset`. None of the PDFs read covers `Union`: Schema23
  describes the project side only (`ComObjectInstanceRef_t` and so on), not
  the product's `Static`.
  - `A-0027-15-0BAC` has six members at `Offset=1`, `BitOffset` 5–7.
  - The rule is used only for unions that start on an octet boundary; the
    corpus has 936 unions that start mid-octet, and those are refused.
- **`[V]`/`[A]` Unmatched `choose`.** A `choose` whose value is legal for
  its type but matched by no `when` activates nothing. This is the
  evaluator's existing `[A]` reading (§4.3). The device confirms it:
  evaluating the MDT tree for the device's values gives ten such `choose`s
  (for example `P-3 = 0`, `P-1007 = 1`), and the result matches the device
  octet for octet. Every other diagnostic refuses the image, and so does a
  value the type does not allow.
- **`[A]` Priority.** Schema23's `ComObjectPriority_t` is `Low`/`High`/
  `Alert` (§1.1.2.4), while *Resources* §4.18.3.1.2.1 has `System`/
  `Urgent`/`Normal`/`Low`. No PDF maps one onto the other.
  - The corpus uses `Low` 270 times, `High` 59 and `Alert` once.
  - An absent priority and `Low` are written as `Low` (`11b`), which is
    what the device holds for MDT's absent priorities.
  - `High` and `Alert` are refused.
- **`[A]` `ReadOnInitFlag`.** It has no bit in the Easy-3 config octet
  (*Resources* §4.18.3.1.2.1), so an enabled one is refused, not dropped.
- **`[A]` `Mask`.** The builder refuses a change to any octet whose mask
  octet is not `FFh`, except the address table's individual-address slot.
  The image still carries the mask, so the writer must skip those octets.
  - Corpus: 5,291 `00h` and 145,896 `FFh` mask octets; no other value.

**`[V]` Acceptance, against the device's read-back.** For the device's own
configuration (five non-default values, four links):

- `AS-4400` is equal in all 394 octets;
- the address table's 11 octets and the association table's 9 are equal.

For option C (button 1 toggles `2/0/53`, button 2 inactive), exactly eight
octets of `AS-4400` differ from the device: objects 1 and 18, the button
1/2 function and subfunction, and three parameters behind them. The address
table becomes `02 1143 1035` and the association table `01 01 00`.

The device keeps stale octets behind both tables (`AS-4000`+11/12,
`AS-4201`+10). These differ from the base data. A download writes the
base data there, which a table's length octet makes irrelevant.

### 19.3 Running a BIM M112 download from the product's procedure (2026-09-28)

The MDT program's `LoadProcedure` (`LoadProcedureStyle="ProductProcedure"`)
has 21 steps:

1. `LdCtrlConnect`, then `LdCtrlCompareProp ObjIdx=0 PropId=78`
   (`PID_HARDWARE_TYPE`) with ten octets of `InlineData`.
2. `LdCtrlUnload` for machines 1, 2 and 3.
3. Per machine: `LdCtrlLoad`, `LdCtrlAbsSegment`, `LdCtrlTaskSegment`,
   `LdCtrlLoadCompleted`. Machine 3 has three `AbsSegment`s: RAM `0700h`
   (152 octets), a stack segment at `0798h`, and EEPROM `4400h`.
4. `LdCtrlRestart`, `LdCtrlDisconnect`.

It has no step that writes segment data. What each part of the plan
(`knx_productdb::download_plan`) rests on:

- **`[D]` Where data goes.** *Configuration Procedures* (`03_05_03`
  v02.01.01) §3.9.2.2.2, pp. 67–68, is the Standard's download of a BIM M112
  (mask 5705h). It writes each table (`DMP_MemWrite_RCoV`) *after* its
  allocation and *before* its task segment. The plan puts each segment's
  data write right after its `LdCtrlAbsSegment`.
- **`[D]` Task segments.** The same procedure gives the tables
  `peitype=00h, appl_id=0000/0000/00`, and the application program the PEI
  type and *"Manufacturer Code, Device Type, Version"*.
- **`[V]`/`[A]` The application identity** comes from the program's
  `PeiType="1"`, `ApplicationNumber="39"` (`0027h`) and
  `ApplicationVersion="21"` (`15h`), plus `M-0083`. `1.1.67` reports
  `00 83 00 27 15`. The attribute-to-field mapping itself is `[A]`.
- **`[A]` `CompareProp`.** `PID_HARDWARE_TYPE` is `PDT_GENERIC_06`
  (*Resources* §4.3.28, p. 78), but the product compares ten octets:
  `00 00 00 00 01 27 00 00 00 00`. No PDF read says how. The rule used is
  that the device's octets start the data and every remaining octet is
  zero. `1.1.67` answers `00 00 00 00 01 27`.
- **`[D]` Order of checks.** CP §3.9.2.2.2 identifies the device
  (`DMP_Identify_RCo2`) before unloading anything. The executor reads
  mask, manufacturer and every `CompareProp` before its first write. It
  refuses a plan that checks later.
- **`[D]` No Verify Mode**, and a read-back of every data write (MP §3.31.2;
  the project's no-write-without-read rule).
- **`[A]` `Mask`.** Masked octets are left out of the writes, not
  rewritten. For MDT this is the individual address at `4001h`–`4002h`.

**`[V]` In the simulator, option C downloads end to end.** The chain is the
stored product file, the image, the 25-step plan, and a simulated
mask-`0701h` device with MDT's identity. Every segment lands octet for
octet. `4001h`–`4002h` is never written, Verify Mode is never set, there
is one restart, and a second run leaves the same memory. The simulator
enforces MP §3.31.2's records and RES Table 94. It does not model
EEPROM timing, checksum control (`SegFlags` bit 7), or what a real
BIM M112 does with the task segment's identity.

### 19.4 First live download attempts on `1.1.67`, and a lost `T_ACK` (2026-09-28)

Two runs of the unchanged plan (`apps/knx-cli/tests/live_memory_download.rs`,
gated by `KNX_DOWNLOAD_ADDRESS` and `KNX_DOWNLOAD_CONFIRM`) both stopped
early. Logs: `OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-28_*`
(gitignored).

- **`[V]` Before the run** a fresh read-only dump was byte-identical to the
  morning's backup; load states `B6EAh` = `01 01 01 00`.
- **`[V]` Run 1** stopped at step 13 (the association table's 511 octets):
  one read-back got `T_ACK` and no answer within 3 s. Afterwards the load
  states read `01 02 00 00`, the address table read `02 1143 1035` (option
  C's one group address, `2/0/53`) and the individual address at
  `4001h`–`4002h` was unchanged.
- **`[V]` Run 2** stopped at step 8 the same way.
- **`[V]` The cause, from a frame trace** (read-only, `diag4-frametrace`):
  1. the device answered a read with `T_DATA_CONNECTED seq 1`;
  2. this client's `T_ACK seq 1` has **no `L_Data.con`** in the trace: it
     never reached the bus (a group telegram from another device arrived
     3 ms after the answer);
  3. the device acknowledged the next request (`seq 2`) at once but did not
     answer it;
  4. it repeated the old answer, unchanged, `seq 1`, 6 s after the first
     transmission and then every 3 s.
- **`[D]` That is TL §5.4.1 exactly** (*Transport Layer* v01.02.03, pp.
  17–22). With its answer unacknowledged the device sits in `OPEN_WAIT`:
  it still receives (`E04` → `A2`, acknowledge), repeats its stored frame
  on each acknowledge time-out (`E17` → `A9`) and keeps every newer answer
  behind it (`A11`, *"Don't change order of T_Data_Connected.req
  events"*). The client has to acknowledge the repetition: `E05`
  (`SeqNo_of_PDU == SeqNoRcv - 1`) → `A3`, *"Send an N_Data_Individual.req
  with T_ACK_PDU … sequence = sequence of received message"*, and nothing
  goes to the user.
- **`[V]` The client's two defects.** It gave up on an acknowledged request
  after one 3 s time-out, before the device's first repetition at 6 s. And
  it had no `SeqNoRcv`, so a repetition would have been taken as the answer
  to a new read of the same address.

The fix (`ManagementSession::exchange_inner`, `receive_numbered`):

- **`[D]`** The session keeps `SeqNoRcv` (reset by `A12` on connect) and
  handles `E04`/`E05`/`E06` as `A2`/`A3`/`A4`. A repetition is
  acknowledged and never matched.
- **`[D]`/`[A]`** An acknowledged request waits up to `MAX_TRANSMISSIONS`
  acknowledge time-outs (12 s) for its answer: the length of the device's
  own repetition ladder (`E17`/`E18`). The request itself is still never
  repeated. Using the ladder as the bound is this project's reading; no
  PDF read states a client-side figure for it.
- **`[D]`** The load-state wait loop keeps one time-out per read. RES
  §4.23.2.4.1: *"The period for reading shall not exceed half the
  TL-timeout, i.e. 3 seconds."*
- The simulator reproduces the device (`lost_ack_for_answer`,
  `answer_repeat_after`). Four new tests; seven mutants of the fix, all
  caught.

**`[V]` Run 3, with the fix (16:55–16:58 CEST, `download-run-3`).** Steps
0–22 completed: every unload, write, read-back and load-state transition.
Only step 23, the final `A_Restart` (Basic Restart), failed. The device sent
no `T_ACK` in four transmissions of 3 s each, the session released the
connection, and the executor reported the download as failed. An
independent read-back straight afterwards, on a fresh connection, found:

- all three segments (`4000h` 513, `4201h` 511, `4400h` 394 octets) equal to
  the option-C image, with **0 differing unmasked octets**;
- the individual address octets at `4001h`–`4002h` still `11 43` (`1.1.67`);
- load states `B6EAh`–`B6EDh` at `01 01 01 00`, all Loaded.

**Update 2026-09-28 (K2), spec reading.** Read directly from the MP
v02.01.02 PDF:

- §3.7.1.1.2, p. 78, Figure 19: between `t0` and `t1` *"The MaS may react
  under the pre-reset conditions, may not react at all, or may already react
  according the post-reset conditions"*. A missing `T_ACK` after a Basic
  Restart is inside that range.
- §3.7.1.1.3, p. 80: *"The Application Layer of the Management Server shall
  not confirm the A_Restart-service if a Basic Restart is called; to obtain
  the same result with an AL-confirmation, the Management Client should
  instead call a Master Reset with Erase Code 00h."*
- §3.7.3, p. 89: the reset of the communication system may mean *"that no
  T_Disconnect –frame is sent on the bus"*.

`[D]` So no clause read requires a `T_ACK` for a Basic Restart, and none
forbids leaving it out. KNXBench reports the case as
`RestartOutcome::Unconfirmed` and never repeats the restart. The confirmed
alternative the Standard names, a Master Reset with Erase Code `00h`, is a
different service. Whether mask `0701h` supports it has not been checked,
so it is not used.

**`[V]` Frame trace of a closing restart (K2, 2026-09-28 19:00 CEST,
`1.1.67_MDT-0701_2026-09-28_restart-trace.txt`).** With the user's go,
`run_memory_download_observed` ran the plan `Connect → Restart →
Disconnect` against `1.1.67`. That is a Download-scoped session on mask
`0701h`, exactly like run 3's, so there is no Verify Mode and no memory
write. The same plan was first run in the simulator to list what it sends:
three reads, one `A_Restart` and the disconnects. The trace:

| t (s) | Frame |
|---|---|
| 0.12–0.42 | `T_Connect`, three reads. Every request is acknowledged (`T_ACK` seq 0–2) and answered: mask `0701h`, manufacturer `0083h` |
| 0.46 | `A_Restart` (Basic Restart, seq 3), positive `L_Data.con` |
| 3.45, 6.45, 9.45 | the same frame repeated by TL clause 4, each with a positive `L_Data.con` |
| 0.42 → 38.5 | **nothing at all from `1.1.67`**: no `T_ACK`, no `T_NAK`, no `T_Disconnect` |
| 3.7–36.5 | group telegrams from `1.1.7`, `1.1.10`, `1.1.25`, `1.1.28`, `1.1.220`, `1.1.251` received throughout, so the receive path was working |
| 12.46, 12.48 | two `T_Disconnect`s from the client: TL's `A6` after the fourth time-out, then MP §3.7.3's explicit `DM_Disconnect` |
| 38.5 | fresh read-only connection: answered within 30 ms, mask `0701h`, `4001h` = `11 43`, `B6EAh` = `01 01 01 00` |

What this shows:

- `[V]` The device does not acknowledge a Basic Restart in any of the four
  transmissions. That makes two observations (run 3 step 23, and this
  trace), so for this device it is the normal case, not a fault.
  `RestartOutcome::Unconfirmed` is the outcome to expect from it.
- `[D]` TL §5.4.1, p. 21: a `T_DATA_CONNECTED` in `OPEN_IDLE`/`OPEN_WAIT`
  gets `A2`/`A3`/`A4` (`T_ACK` or `T_NAK`), and in `CLOSED` it gets `A10` (a
  `T_Disconnect` back to the sender). A Transport Layer that is running
  answers every one of the four transmissions somehow. This one answered
  none of them over 9 s.
- `[A]` So the device's Transport Layer was not running for at least 9 s
  after the first transmission, which is how a device that starts
  restarting before it acknowledges looks. That is strong circumstantial
  evidence that the restart happens. It is not proof: nothing on the bus
  marks a restart from the inside.
- `[V]` Afterwards the device is back, with its address, and all three
  load states still `Loaded`. The restart undid nothing.
- Cosmetic, fixed with it: the error read "no T_ACK for T_ACK for A_Restart".
  The three `send_acknowledged` labels now name only the request.

**`[V]` Master Reset, Erase Code `01h` "Confirmed Restart" (K2, 2026-09-28
19:18 CEST, `1.1.67_MDT-0701_2026-09-28_master-reset-01h-trace.txt`).** MP
§3.7.1.2 Table 4 (p. 81) defines Erase Code `01h` as "the Master Reset as a
confirmed alternative to the unconfirmed Basic Restart", with no Resource
reset. MP §3.7.3 (p. 88) requires the client to verify support first. For
mask `0701h` there is no means to do that: Profiles v02.01.01 §4.2 (p. 37)
marks Master Reset `O` (optional) for BIM M112 mask `0701h`, and the
Management Profile that announces it (Resources §4.1.3 Table 8) belongs to
the E-Mode device descriptor. The user gave the go explicitly for this test
device, and the only way left to find out was to try it once, with the
erase code that resets nothing. The probe first ran in the simulator
(`Connect`, Verify-Mode read/write of `PID_DEVICE_CONTROL`, `A_Restart
type 1 [01 00]`, `Disconnect`; no memory write).

| t (s) | Frame |
|---|---|
| 7.56–7.78 | `T_Connect`; `PID_DEVICE_CONTROL` read `00`, written `04`, echoed `04`: Verify Mode active. All `T_ACK`ed |
| 7.82 | `A_Restart`, restart_type 1, Erase Code `01h`, channel `00h` (seq 2) |
| 10.80, 13.80, 16.81 | the same frame repeated by TL clause 4 |
| 7.78 → 45.8 | **nothing from `1.1.67`**: no `T_ACK`, no `A_Restart_Response`, no `T_Disconnect` |
| 19.81, 19.83 | client `T_Disconnect` ×2 |
| 45.8 | read-only check: mask `0701h`, `4001h` = `11 43`, `B6EAh` = `01 01 01 00`, `4400h` = `40 07 00 07 40 4F 00 07`, identical to before |

- `[V]` `1.1.67` sends no `A_Restart_Response` to a Master Reset. It acts
  exactly as it does for a Basic Restart. MP footnote 11 (p. 88) anticipates
  this: *"Existing implementations may not check bit 0 of octet 7 … may
  only perform a Basic Restart if a Master Reset is called"*. NOTE 10's
  guarantee (the response is sent *before* the reset) therefore does not
  hold for this device.
- `[V]` The device kept its address, application (first 8 octets of `4400h`)
  and load states. Erase Code `01h` really reset nothing.
- `[D]` Verify Mode: this Restart-scoped session set it, as `connect()` does
  for every scope except a memory-mapped load. MP §3.7.1.2.3 does not
  exempt it. On a new connection a client must assume it cleared (MP
  §3.29.1), so it has no lasting effect.
- **Consequence:** a confirmed restart does not exist for this device. The
  inside view of the restart ("did it really restart?") cannot be
  established from the bus. `RestartOutcome::Unconfirmed` stays the final
  answer for mask `0701h` MDT devices, and KNXBench offers no Master Reset
  for them.
- `[V]` Side finding from attempt 1 (19:16, kept as
  `…master-reset-01h-attempt1-lost-disconnect.txt`): the read-only
  pre-check's `T_Disconnect` produced no `L_Data.con` in the trace, and
  `1.1.67` answered the next session's first request with `T_NAK` twice,
  then sent `T_Disconnect` itself after ~6 s. So its old connection was
  still open. A second session started straight after a first one is
  therefore not safe. The probe waited 7 s in attempt 2, and that worked.
  Why the disconnect never showed up is open: `ManagementSession::disconnect`
  discards the send error (`let _ =`).

**`[O]` → `[V]` Open: did the device restart?** *(Answered 2026-09-29 by
the K7 function check below: yes; the new image was active without a
power cycle.)* MP §3.7.1.1.3 says the server does
not confirm a Basic Restart at the Application Layer, and MP §3.7.3
exception (5) tells the client to ignore everything the server sends after
`A_Restart` *"except negative TL-confirmations"*. That clause does not say
whether a missing `T_ACK` counts as one. The run took no frame trace. One
possible reading, **not verified**, is that the device restarts before its
Transport Layer acknowledges. Whether the device now runs the option-C
application has to be checked on the device itself (button 1 → `2/0/53`),
not guessed from this log.

**`[V]` Function check after a power cycle (17:08 CEST,
`monitor-after-powercycle`).** The user disconnected the device from the bus
once and reconnected it, then pressed button 1 repeatedly. A read-only bus
monitor recorded 33 telegrams from `1.1.67`:

- all 33 were `GroupValueWrite` to `2/0/53`;
- the values alternate strictly `1, 0, 1, …`, with no value repeated back to
  back;
- there were **no** telegrams to the old associations (`2/1/15`, `2/1/16`,
  `0/4/6`, `0/4/7`) or to any other group address.

Option C is working: button 1 toggles `2/0/53`. The question above stays
open, because the power cycle restarted the device regardless of whether
the `A_Restart` had already done it. What this check shows is that the
downloaded image is correct and a device restart activates it.

**`[V]` K7 live acceptance, both product paths (2026-09-29, 06:11–06:29 CEST).**
With the user's *"go k7"*, `1.1.67` received two downloads built by the
product path from a project, never from the fixed test image:

1. **CLI, `knx device download`, 06:11:50–06:14:34**
   (`1.1.67_MDT-0701_2026-09-29_k7-cli-download.txt`). The project is a
   new one, `KNXBench 1.1.67 K7 switch-by-push off.knxdb` (gitignored):
   the MDT push button from the product catalog, button 1 in mode
   "Switch by push", value **Off** (`UP-5501 = 0`, `UP-5517 = 0`), object 0
   sending to `2/0/53`. Against the fresh read-only dump taken just before
   (`…pre-k7.txt`), its image differs in exactly **4 octets**
   (`4409h`, `440Ah`, `450Bh`, `4510h`); against option C it therefore
   differs, as K7 requires. 25 steps, **1416 octets written, every one read
   back**, all three load states `Loaded`, restart `NOT confirmed` as
   expected for this device. An independent read-back on a fresh
   connection 2 min later (`…post-k7-cli.txt`) found **0 differing
   unmasked octets** against the K7 image.
2. **Web UI, Download to device tab, 06:25:18–06:28:04**
   (screenshots and request log in the session's scratch, not kept). The
   real front end (Vite dev server against a local `knx-server`), driven
   headless with Playwright: File → Open (.knxdb) with option C, Bus
   monitor → Download to device, device `1.1.67`, gateway, "Show what would
   be written", "Download to 1.1.67", the consent dialog, "Program device".
   The browser sent exactly one `start` with the plan's own id and phrase,
   then only polled status. The panel ended with *"Written to the device:
   yes, 1416 octets, every block read back unchanged"* and *"Restart: NOT
   confirmed"*. The independent read-back 45 s later
   (`…post-k7-web.txt`) found **0 differing octets** against option C.

What this shows and what it does not:

- `[V]` Both product paths (CLI and web) write a project's configuration
  into a real mask `0701h` device, byte-exact, and report the unconfirmed
  restart honestly. The fixed test image is no longer the only hardware
  write path.
- `[V]` The binary carried K8's §105 change: every `T_Connect`,
  `T_Disconnect` and `T_ACK` in both runs went out at system priority
  through the real KNXnet/IP interface, and the device acknowledged every
  data request except the closing restart. The interface accepts the control frames;
  whether it keeps the priority bits on the wire was not traced (no bus
  monitor was running).
- `[V]` **Function check (06:51–06:59 CEST).** The K7 project was
  downloaded a third time (`…k7-check-download.txt`: 1416/1416 read back).
  After the ~38 s restart window, a read-only `knx bus monitor` ran for
  81 s (`…k7-check-monitor.txt`, 06:53:31–06:54:54) while the user pressed
  button 1. It recorded **11 telegrams from `1.1.67`, all
  `GroupValueWrite 0` to `2/0/53`**, and none to any other address. Option C
  toggles (`1, 0, 1, …`, the 2026-09-28 power-cycle monitor), so the K7
  configuration was active. The download's own Basic Restart activated it:
  unlike on 2026-09-28, this time there was **no power cycle**. That answers
  the `[O]` question above for this device: the unacknowledged restart does
  restart it. The monitor printed the small-payload value as `0x00 (6-bit)`
  because it ran without `--project`, so no DPT was resolved.
- `[V]` Afterwards option C was downloaded again (06:55,
  `…k7-check-restore-optionC.txt`, 1416/1416), and an independent read-back
  45 s later (`…post-k7-check.txt`) found **0 differing octets** against it.
  `1.1.67` is back where it started.

**`[V]` K6 live, individual address `1.1.67` → `1.1.68` (2026-09-29,
07:12–07:13 CEST, `…k6-to-1.1.68.txt`).** A read-only scan first found
`1.1.67` occupied (mask `0701h`) and `1.1.68` vacant (`…k6-prescan.txt`).
`knx device program-address 1.1.68` waited for the button, and the user
pressed it in round 37, where exactly one device answered (`1.1.67`).
Steps 1–3 ran. Step 4 connected to `1.1.68` and read the device there;
the settling retry (`be91fe3`) was not needed, and the output shows no
second connect. Only the closing Basic Restart went unacknowledged (four
transmissions, TL released the connection), exactly as in every K7 run.
A read-only scan 40 s later (`…k6-postscan-1.txt`) found `1.1.68` occupied
(mask `0701h`) and `1.1.67` vacant, and the user confirmed the programming
LED was **off**. So the restart did end programming mode: a second
observation that this device restarts without acknowledging.

- `[V]` Bug found: the procedure reported the restart silence as MP §2.3
  "to 4." (`FAILED … address written: yes, but NOT confirmed: the device
  did not answer at the new address`), although step 4 had just read the
  device at that address. Fixed with `AddressRestart` (KNOWN_LIMITATIONS
  §116, status 2026-09-29).
- `[O]` The settling retry itself is still not live-verified: this device
  answered at the new address on the first connect.
- `[V]` **The way back with the fix (`6a71162`), 07:33–07:34 CEST
  (`…k6-back-to-1.1.67-try2.txt`).** The first wait (07:28) expired
  unpressed: `gave up after 90 rounds … nothing was written`, exit 1, which
  is the empty path working as designed. On the second wait the user
  pressed in round 16, and `1.1.68` was found. Result: `finished`, `address
  written: yes, 1.1.68 -> 1.1.67; the device answered at 1.1.67`, `restart:
  NOT confirmed (no T_ACK …)`, exit 0. The scan 40 s later
  (`…k6-postscan-back.txt`) found `1.1.67` occupied (mask `0701h`) and
  `1.1.68` vacant. A memory dump (`…post-k6.txt`) matches the post-K7
  dump line for line (180 of 180 lines), so the two address changes left
  the configuration (option C) untouched.
- `[V]` No `LoadCompleting` stall and no mid-download drop occurred, so
  §101 and §104 had nothing to measure in either run.

---

### 19.5 Commissioning topics in the PDFs that KNXBench does not cover (2026-09-29)

Read with `pdftotext -layout` straight from `knx-spec-kb/sources/` (not
`extracted/`), page numbers from the PDF footers. Sources: Profiles
(`06 Profiles v02.01.01`), MP (`03_05_02`), CP (`03_05_03`), RES (`03_05_01`),
AL (`03_03_07`), TL (`03_03_04`), `Project Schema23 v01.00.00`. Two
Profiles tables were checked as rendered pages, because the text columns
do not line up.

**What the Profiles say about our device class.** `0701h` is **BIM M112**
(Profiles p. 13: *"System 7 · 0700h, 0701h · BIM M112"*), not System 1.
- Table 4.2, p. 37, column `mask 0701h`:
  - Interface Object Handling M.
  - Load State Machine Realisation Type 1 M and Type 2 M.
  - Run State Machine Type 1 M and Type 2 M.
  - Restart: connectionless O, connection-oriented M, Master Reset O.
  - Authorization M, with **16** access levels.
  - The download path uses Realisation Type 1 (memory-mapped load
    controls), and both are mandatory, so that matches.
- Table 4.4, p. 44, column BIM M112:
  - Programming Mode connection-oriented M.
  - KNX Serial Number, client initiated: **M**.
- Profiles p. 42, note 13: *"New implementations of BIM M112 should not use
  mask 0700h or mask 0701h. Implementations of mask 0701h should foresee
  functionality to avoid execution of an incompatible application."* The
  compare-property step (device object PID 78) before the first unload is
  that check, from the device side.
- TL: BIM M112 implements the connection-oriented state machine as
  **Style 3** (Profiles p. 36, TL §5.4.3, p. 24). The acknowledge
  time-out note in IMPLEMENTATION_STATUS cites the Style 1 table. The
  Style 3 cell it relies on is the same: `E18` in `OPEN_WAIT` leads to
  `CLOSED` with `A6`. TL §5.4.4.1 says the styles are *"identical in the
  operation of the TL"* and differ only in error handling.

**Topics not implemented, in order of use for a TP installation of this
class.**

1. **Individual address by serial number** (MP §2.4
   `NM_IndividualAddress_SerialNumber_Read`, §2.5 `..._Write`, pp. 16–17).
   Mandatory for BIM M112 (Profiles Table 4.4). No programming button
   needed. MP §2.5's own note: unlike `NM_IndividualAddress_Write`, it
   *"does not reset the device after assigning the Individual Address"*.
   Prerequisite: the serial number, which `.knxproj` carries as
   `DeviceInstance/@SerialNumber`. It is retained on import (§34), but in
   the corpus only 5 of 75 devices have one, and none is an MDT device.
   Nothing in `knx-net` encodes `A_IndividualAddressSerialNumber_*`.
   **Largest gap for real-world commissioning.** It needs a device whose
   serial number is known, plus user approval for a new live test.
   **Done 2026-09-29 (K12, KL §139), simulator only.** AL Figures 12–14
   (pp. 21–23): read = 6 serial octets; response = 6 serial + 2 domain
   address + 2 reserved, the address being the frame's source; write = 6
   serial + 2 new address + 4 reserved. APCI `3DCh`/`3DDh`/`3DEh`. MP §2.4
   and §2.5 give no response time-out. RES §4.22.1.3: a device with interface
   objects and these services also has `PID_SERIAL_NUMBER` (PID 11), which
   is how to learn a serial number the project lacks. The serial number is
   2 octets of manufacturer code and 4 more (RES §4.22.1.2, DPT 221.001).
2. **`NM_IndividualAddress_Reset`** (MP §2.18, p. 33): write `FFFFh` to
   every device in programming mode, restart at `FFFFh`, repeat until no
   answer. Small, but it writes to hardware.
3. **Partial download** (CP §3.5.3, p. 44). The property path has it in the
   simulator only (ADR-0048, KL §113). For `070nh`, the product's own
   load procedure decides what a partial download is, and the corpus has
   not been read for that yet.
   **Done 2026-09-29 (K15, KL §142), simulator only.** Read directly:
   - CP has no section of its own for mask `070nh`. BIM M112's download is
     §3.9.2.2 (mask 5705h), which `memory_download` already follows, and
     its partial download is §3.9.2.4, pp. 69–70, *"Default Partial
     Download Procedure"*. It is not written out. It is *"generated from
     the complete download procedure by applying the following
     transformations"*: drop the `UNLOAD` of the application and PEI
     programs; without the group-communication part, drop every load
     control and allocation of the two tables; turn application and PEI
     segment allocations into memory writes, *"absolute data or stack
     segments in EEPROM only, all others are simply ignored"*. The extra
     input is the *"Partial Download Type (Parameters and/or Group
     Addresses)"*.
   - Corpus (103 `.knxprod`, 203 application programs of mask `MV-070n`):
     every one has exactly one `LoadProcedure`, none a `MergeId`, and only
     two (MDT `A-0255-21-0ECA`, `A-0054-14-0D63`) carry `AppliesTo`, both
     with the value `full,par`. So the product file does not describe a
     partial download of its own; the CP transformation is the only
     documented one. `Project Schema23` lists `AppliesTo`'s values but no
     default, so KNXBench does not rely on it.
   - The MDT file also carries `Options/@LegacyAllowPartialDownloadIfAp2Mismatch`,
     retained and not interpreted. No PDF read defines it.
   - The transformation does not check that the device carries the
     application; `DMP_Identify_RCo2` checks the hardware. A partial
     download to a device with another application would write parameters
     into memory laid out for something else. KNXBench adds two checks of
     its own before the first write: `PID_PROGRAM_VERSION` of object 3 must
     be the plan's task segment identity (the `1.1.67` answered
     `00 83 00 27 15` in the 2026-09-29 dump), and every part the plan
     loads must be `Loaded`.
4. **Access keys on a download** (AL `A_Authorize`/`A_Key_Write`,
   CP §3.5.2 step 11). BIM M112 has 16 levels. `Project Schema23` p. 38:
   `Installation/@BCUKey`, *"The key used to lock devices supporting
   authentication"*, default `4294967295` (`FFFFFFFFh`, i.e. no key). All
   three corpus projects carry the default. The download path uses
   `AuthorisationPlan::Skip`, and a locked device answers memory accesses
   with `number = 0` (AL p. 111/112), which already surfaces as
   `MemoryRefused` ("unreachable, protected, or an illegal octet count").
   What is missing: taking the key from the project (`BCUKey`) or from the
   operator, and a message that names a key as one possible cause. **No key
   may ever be guessed**; the ruling in `authorisation.rs` stays as it is.
   **Done 2026-09-29 (K11, KL §138).** Project key, `--key-file`, and a
   hint on refusal. Reading the `DM_Authorize2_RCo` diagram (MP p. 76) for
   this showed the key is sent only *"If the free access level is not the
   highest level"*; the code had also sent it after a free level of 0.
   Corrected, with the test rewritten to the diagram.
5. **Master Reset** (Restart type, Profiles Table 4.2: optional for
   `0701h`). Not needed for v1.

**Not relevant to this project's setup:** Domain address procedures (open
media only, Profiles Table 4.4 `C*`; **done in the simulator anyway, K16,
§19.6**), RF/PL/IP configuration, coupler
filter tables (`0912h`/`091Ah`), `NM_Router_Scan`, KNX Data Security
(`DM_SecureSync_*`), Easy Modes (PB/Ctrl), USB interface configuration.

**Blockers this resolves:** none of the open ones. The two parked extras
(interrupted-download recovery, settling retry) were settled in the §7
entry. The Style 3 finding is a documentation correction, not a blocker.

### 19.6 RF domain addresses, from the PDFs (2026-09-29, K16)

Read straight from `knx-spec-kb/sources/`: AL (`03_03_07` v02.01.02)
§3.3.3–§3.3.7, pp. 34–42, and Table 1, p. 13; MP (`03_05_02`) §2.7–§2.14,
pp. 18–27; CP (`03_05_03`) §2.3.1, pp. 17–21; EMI_IMI (`03_06_03`
v01.04.02) §4.1.4.3.2, §4.1.4.3.9, §4.1.5.3.2, §4.1.5.4; DLL General
(`03_03_02`) §2.3. No RF device exists here; everything below is
simulator-tested only (KL §143).

- **APCIs** (AL Table 1, bits written out): `A_DomainAddress_Write`
  `1111100000` (`3E0h`), `_Read` `3E1h`, `_Response` `3E2h`;
  `A_DomainAddressSerialNumber_Read` `1111101100` (`3ECh`), `_Response`
  `3EDh`, `_Write` `3EEh`.
- **Lengths.** A domain address is 2 octets on PL110 and 6 on RF (AL
  Figures 20/21, 23/24, 27/28, 29/30). The serial-number forms put the
  6-octet serial number first. KNX IP uses 4 (Figure 31) and 21 octets
  (Figure 32); not decoded, kept as `Other`.
- **System broadcast.** Every domain-address PDU goes out with
  `T_Data_SystemBroadcast`. DLL §2.3: destination `0000h`, group address
  type, and on the wire cEMI Ctrl1 bit 4 (SB) **clear** — EMI_IMI p. 76:
  *"0: system broadcast, 1: broadcast"*. KNXBench had always sent SB set
  (`0xBC`), which is correct for the plain broadcast. CP §2.3.1.1/§2.3.1.4
  allow the plain broadcast instead when a TP1/RF media coupler forwards
  it, so each procedure takes the destination as a parameter.
- **RF medium information.** Additional-information type `02h`: RF-Info
  (1), serial number or domain address (6), LFN (1); mandatory for RF
  frames. SB clear means the six octets are a serial number, set means a
  domain address (§4.1.4.3.9). All zero / LFN `255` on a request means
  *insert your own*. The RF frame's `L` is void (`00h`) and the NPDU runs
  to the end (§4.1.5.4.1/§4.1.5.4.3). The example on p. 74 has `Len = 7`
  (no LFN), so the decoder takes both.
- **Procedures.** §2.7 read: 3 s window, always waited out, four outcomes
  including duplicates. §2.8 read both: domain read, then individual read.
  §2.9 write, connection-oriented: occupancy probe, `A_DomainAddress_Read`
  with a 1 s window and exactly one answer, write what differs, connect and
  restart. §2.10 write2, the one CP §2.3.1.3 uses for RF: individual read,
  domain write, individual write, connectionless Device Descriptor read at
  the new address (*"only interested in whether it receives a response"*),
  connectionless restart; no occupancy check. §2.12 by serial number:
  write, wait 1 s, verify with `A_IndividualAddressSerialNumber_Read`,
  repeat; *"shall not automatically repeat"* the whole procedure. §2.11 is
  *"not yet specified"*, §2.13 needs Data Security, §2.14 is PL110 only.

### 19.7 RF device configuration, from the PDFs (2026-09-29, K17)

Read straight from `knx-spec-kb/sources/`: CP §3.6–§3.7, pp. 57–60; MP
§2.6, §3.2.2, §3.2.7; AL §3.3.2, §3.4.7 (Figures 59–61, Table 1); RES
§4.1.3 (DD2), §4.3.14 (`PID_OBJECTLINK`), §4.3.16 (`PID_PARAMETER`, pp.
67–68 read as rendered pages because the text layer splits the tables).
Simulator only (KL §144).

- **Bidirectional (CP §3.6).** Identify with `DMP_Connect_RCl(IA, 2)`:
  connectionless `A_DeviceDescriptor_Read` type 2; the answer's type *"may
  be different"*, several answers mean several devices at one address.
  Individualise with MP §2.6 (serial-number write, read-back, DD2 read
  only to see that it answers). Parameters through `PID_PARAMETER` (PID
  65): write = channel (1), parameter (1), value; read = channel,
  parameter; read response = return code, `00h`, value; `FFh` ends at the
  return code. Links through `PID_OBJECTLINK` (PID 63): Flags (s bit 0,
  d bit 1, aet bit 2), `00h`, SN (6), GA (2), GO index (2); return codes
  `00h`/`FFh`/`FEh`/`FDh`/`FCh`.
- **Unidirectional (CP §3.7).** Identification is the device's own
  `A_DeviceDescriptor_InfoReport` on the system broadcast (AL NOTE 5: the
  `A_DeviceDescriptor_Response` APCI). Individual address always `05FFh`.
  Group addresses from `0001h` along the channels' objects, inputs
  included; Example 16 (two channels, 2 + 3 objects) gives `0001h`–`0005h`.
- **Function properties** (AL §3.4.7): APCIs `2C7h` command, `2C8h` state
  read, `2C9h` state response. A property that is not `PDT_Function`
  answers without return code and data.
- **DD2** (RES Figure 2): 14 octets — manufacturer (2), application id (2),
  version (1), Management Profile + reserved (1), four Channel Infos (U3U13:
  count − 1, 13-bit code). Octet 5 values `00h`, `3Fh`, `40h`, `80h`; any
  other *"should not"* be changed by a client. CP Example 16's `Link mode
  00` and `LT_Base 3Fh` are this octet, `3Fh` being the reserved profile.

### 19.8 Live run of the second stage on `1.1.67` (2026-09-29)

User go: *"Freigabe fuer alle Tasks auf der Testhaedware"*. Raw logs:
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-29_{pre-live2,k12-*,k15-*,post-k15}.txt`.

- **Before.** Read-only dump, 180 lines, identical to the last dump after
  K7 (option C still on the device).
- **K12 `[V]`.** `PID_SERIAL_NUMBER` = `0083:7A8213CF` (manufacturer
  `0083h` matches). MP §2.4 broadcast read answers from `1.1.67`. MP §2.5
  write to `1.1.68`: no effect, read-back still from `1.1.67`, scan
  unchanged. `PID_SERVICE_CONTROL` = `0000h`, bit 2 clear, which RES §4.2.8
  says forbids address changes by programming mode *and* serial number;
  the device nonetheless accepts the programming-mode path (K6). Recorded
  as device behaviour (KL §139).
- **K15 `[V]`.** Partial download of the parameters, 11 steps, 394 octets,
  all checks and read-backs passed; restart unacknowledged as usual;
  dump after 40 s identical, load states `01 01 01` (KL §142).
- **Not run, and why.** K13 (MP §2.18 address reset) and K14 (Master
  Reset) stay refused on hardware. K13 resets every device in programming
  mode to `FFFFh` and needs a button press and a re-addressing afterwards;
  K14 on `0701h` cannot pass MP §3.7.3's support check and got no response
  even to Erase Code `01h` before. Opening either scope is a code change to
  the hardware allowlist and asks for its own decision. RF (K16/K17):
  no RF hardware.

### 19.9 The `procedure-style` refusals, and signed and text parameter values (2026-09-29)

Question: can the 41 programs refused as `procedure-style` be planned, and
what else keeps `070nh` programs from planning? Method: the direct PDFs
under `knx-spec-kb/sources/` (no extracted text), the 103 local `.knxprod`
packages, and throwaway scripts that compare each parameter's `Value` with
the octets its product's own base image holds at its `Memory` placement.

- **[V] `procedure-style` was the wrong name for all 41.** Every
  `MergedProcedure`/`DefaultProcedure` program in the corpus has a mask the
  memory planner never handles: `MV-07B0` 27, `MV-0912` 7, `MV-0012` 2,
  `MV-091A` 2, `MV-2920` 2, `MV-0001` 1. None is a `070nh` program. A new
  load procedure style would not have planned one of them; their mask
  would still refuse them. `plan_memory_download` now asks the mask first,
  so they count as `not-memory-mapped` (65, with the 24 `MV-2705`
  `ProductProcedure` programs that already did), which is what an operator
  can act on. `procedure-style` remains for a `070nh` program with another
  style; the corpus has none.
- **[V] What the PDFs say about parameter values.** *Project Schema23*
  §1.1.3.19 (`Value_t`) fixes how a value is **spelled in the XML**: a
  `TypeNumber` as a decimal string, a `TypeFloat` as C#'s `"E15"` notation
  (`-?\d\.\d{15}E[+-]\d{3}`), a `TypeText` as the text itself. A scan of
  every PDF under `sources/` for `signedInt`, `TypeText`,
  `TextParameterEncoding` and `ParameterByteOrder` found only Schema23 and
  the Opternus cookbook's `unsignedInt` example. **No PDF says how a signed,
  float or text parameter is laid out in device memory.**
- **[V] What the products' own base images show** (`070nh` programs only;
  a base image need not hold the default, so only a match is evidence and
  a mismatch against a zero field is none):
  - *Signed `TypeNumber`.* 213 fields (135 of 8 bits, 78 of 16) hold their
    non-negative default high octet first; none holds it low octet first.
    36 16-bit fields hold another non-zero value in either order (e.g. `40`
    for a default of `20`), so the base is not always the default. The one
    negative default sits over a zero field. At or above zero every sign
    representation gives the same bits, so such a value is **written**; a
    negative one is **refused**, because nothing shows its form.
  - *`TypeText`.* 262 fields hold their default as its characters' octets,
    first character first, zero-filled to the field; 8 of them fill the
    field exactly, with no terminator. The six non-ASCII defaults
    (`"Zurück"`) are no exact match (the base holds a leading space), but
    hold `ü` as `FCh`, the ISO-8859-1/-15 octet, not UTF-8's `C3h BCh`.
    Of the `070nh` programs, `MV-0705` ones declare `TextParameterEncoding`
    `iso-8859-1` (12) or `iso-8859-15` (12); `utf-8` appears only on
    `MV-07B0` (3). Text is now
    **written** in the declared one of the two; without a declaration only
    ASCII, which both share; `utf-8` or another name is refused.
  - *`TypeFloat`.* 48 `DPT 9` fields match that encoding, every one at
    zero, which proves nothing about the format. The 15 non-zero ones
    contradict it: a default of `500` sits as `F4h 01h`, the 16-bit
    integer 500 low octet first, not DPT 9's `2E1Ah`. The 16
    `IEEE-754 Single` fields are all zero. Floats stay **refused**, now for
    their type (`image-structure`) instead of as "not an unsigned number"
    (`parameter-value`).
  - *`ParameterByteOrder`.* Declared by 2 `MV-0705` programs, both
    `BigEndian`. Any other declared order is now refused, since
    `parameter_image` writes high octet first.
- **[V] Result** (`crates/knx-app/tests/download_coverage_corpus.rs`,
  release build, 125 s): 246 programs, **1 verified, 69 untested** (was
  54), 176 unsupported: `not-memory-mapped` 65, `parameter-evaluation` 59,
  `image-structure` 41 (19 `TypeFloat`, 10 `ReadOnInitFlag` enabled, 9
  placed by `Property`, 3 priority `High`), `parameter-value` 9 (6 refs of
  one parameter with different values, 3 values outside their
  enumeration), `unmodelled-step` 2 (`LdCtrlTaskCtrl1`). Within `MV-0701`
  35/40 plan, within `MV-0705` 35/141. The 15 new plans are **untested**:
  no hardware has seen a signed or text parameter written by KNXBench.
- **[A] Not concluded.** A negative signed value is very likely two's
  complement (every ETS-era device CPU is), but "likely" is not a source;
  it waits for a product image or a device that shows one. Same for
  floats: the `F4h 01h` example suggests some products store a float
  parameter as a scaled integer, which is a manufacturer choice the
  product data does not declare.

### 19.10 Rename leaves, and the image-structure refusals re-checked (2026-09-29)

Question: which of the 41 `image-structure` refusals and the
`parameter-evaluation` ones can be lifted from a source? Method: direct
PDFs under `knx-spec-kb/sources/`, the 103 local packages, throwaway
scripts.

- **[V] `Rename`/`ParameterBlockRename` retitle, nothing more.** 326 in
  `070nh` programs (270 `ParameterBlockRename`, 56 `Rename`): every one has
  only `Id`, `RefId` and `Text`, no children, sits under a `when`, and its
  `RefId` names a `ParameterBlock`. Neither the Schema23 PDF nor the
  Configuration Procedures PDF mentions either element, so the reading
  rests on that shape. The image holds no titles, so the image builder now
  lets the walk's `UnrecognizedNode` for these two kinds pass. The
  evaluator itself still reports them (the UI does not apply a rename, and
  `rename_button_and_repeat_are_reported_and_a_repeated_module_is_named`
  pins that). A reference *below* one would still be named by
  `RefBelowSkippedNode` and still refuse the image. 8 programs were held
  up only by these: 4 now plan; 4 (`A-008A-25/28`, `A-008B-25/28`) next
  reach a 6-bit field at bit offset 5, which crosses an octet and stays
  refused by the bit writer (§19 item 3).
- **[V] `NoBranchMatched` stays as it is.** The 51 remaining
  `parameter-evaluation` programs are value-dependent diagnostics inside
  **module instances** (e.g. `A-0008-23-8B6E`, `MD-2_P-7_R-9 = 0` with only
  a `when test="1"`). The image builder already accepts a legal uncovered
  value in the program's own tree; inside a module it would still refuse
  the module instance, which it does not image. Module instances are the
  real gap here, not the branch rule.
- **[V] Priority `High` stays refused.** *Resources* §4.18 codes 2
  priority bits in the Type-2 config octet as the transport priorities
  (`system`, `normal`, `urgent`, `low`); the schema's `ComObjectPriority`
  says `Low`/`High`/`Alert`. No PDF maps `High` onto a transport priority,
  and `urgent` vs. `system` is a choice, not a lookup.
- **[V] `ReadOnInitFlag` stays refused.** The Easy-3 config octet in
  *Resources* has no read-on-init bit (§19.2). The products' own base
  descriptors do not help: comparing their config octets with the product
  flags gave no consistent pattern (enabled: 62 descriptors, none matching
  under the assumed layout), so they are not evidence for any placement.
- **[V] Placement by `Property` stays refused.** In the 9 `MV-0705`
  programs it is object index 6, PIDs 31, 58 and 60, written with a
  property write, not a memory write. The download executor writes memory
  only; a property-writing step is a separate, untested capability.
- **[V] The Easy-3 group-object table on `MV-0705`.** *Resources* assigns
  the Easy-3 realisation explicitly to `0701h`; no table row names
  `0705h`. The products agree with its type-octet coding on both masks:
  2349 of 2695 non-zero `MV-0701` descriptors and 9743 of 10694 `MV-0705`
  ones hold the code for their object's single `ObjectSize`; the rest
  differ in ways that look like a base image not holding the default (e.g.
  type `0` for a 4-octet object). This is product evidence, not a PDF
  statement, and the `MV-0705` plans are untested.

### 19.11 Fields across an octet boundary, and why module instances stay refused (2026-09-29)

Two candidates for the next coverage step were checked against the direct
PDFs and the product files: parameter fields across an octet boundary (the
4 programs §19.10 left refused) and module instances (51
`parameter-evaluation` refusals, every first diagnostic inside a
`ModuleDef`).

**Fields across an octet boundary — written.**

- **[D]** *Project Schema23* §1.1.3.17 `BitOffset_t`: *"The bit offset is
  the distance of the most significant bit of the parameter from the most
  significant bit of the first octet in memory"*, 0–7. The schema says
  where the field *starts*, not how it continues.
- **[D]** *Configuration Procedures* `03_05_03` v02.01.01 §8.5.4
  (pp. 197–198, E-Mode parameter blocks): *"The position of the parameters
  inside the parameter block are not restricted to the boundary of the
  parameters itself"*. Its Example 24/25 figures number one two-octet
  block's bit offsets `0…7` through octet 0 and `8…15` through octet 1,
  and place an 8-bit parameter at `8…15`.
- **[D]** *Resources* `03_05_01` v01.10.01 §4.18.5.2.5 `PID_EXT_GRPOBJREFERENCE` (p. 268): *"Bit
  offset shall start from "left" / MSB"*; bit 0 of the third octet of a
  `U16B8` is *"Bit Offset = 23"*. *Resources* §4.19.2.1 (p. 278) shows
  E-Mode parameter blocks numbered the same way.
- So the Standard numbers bits MSB-first on through consecutive octets. A
  `Memory` field of `SizeInBit` bits at `Offset`/`BitOffset` is written as
  that bit string: its most significant bit at `BitOffset` in octet
  `Offset`, the rest following into the next octet's MSB. Whole-octet and
  in-octet fields are the special cases of the same rule, and the
  existing tests (among them the live `A-0027-15-0BAC` image) still pass.
- **Not shown anywhere:** none of these figures has a *parameter* that
  crosses a boundary (Example 24's 4-bit field sits at `2…5`), and no
  device has read one back. The rule is the Standard's numbering applied,
  and the plans it opens are `Untested`.
- **[V] Corpus effect.** 77 crossing union members in the `070nh`
  programs' own `Static` (all MDT `M-0083`; 20 more inside `ModuleDef`s).
  The 4 programs `A-008A-25`, `A-008A-28`, `A-008B-25`, `A-008B-28` were
  refused only at `UP-290` (6 bits at octet 1600 bit 5) and now plan. No
  other program's outcome changed (refusal lists diffed before/after).
- Refused as before: more than 64 bits, zero width, a bit offset above 7,
  a union member offset inside a union that itself starts mid-octet, and
  any overlap.

**Module instances — measured, not written.**

- **[D]** What the PDFs say about arguments: *Project Schema23*
  §1.1.2.38 `ModuleDefArgType_t` (`Numeric`, `Text`, `AllocatorRef`) and
  `Value_t`'s one line on `TypeAllocatorRefId`. `BaseOffset`,
  `BaseNumber`, `Allocates` and `NumericArg` are **not defined in any
  direct PDF**: a full scan of `sources/` for them finds only four schema
  lines on the argument type and the project-side `ModuleInstance`.
- **[V]** The products use them uniformly: every `ModuleDef`'s `Memory`
  (7594 in the `070nh` programs) carries `BaseOffset="…_A-1"`, the
  `ParamOffsBase` argument, and every instantiating `Module` binds it with
  a `NumericArg` (e.g. `A-0009-32-094E`: 12 modules at `72`, `144`, `216`,
  `288`, `Allocates="72"`). The obvious reading is *octet = argument value
  + `Memory/@Offset`*, and object *number = `ObjNumberBase` +
  `ComObject/@Number`*.
- **[V] That reading does not rebuild the base images.** Checked the way
  §19.9 checked value encodings — a field's product default against the
  product's own `Data` at the computed place:
  - program-own parameters: 1046 match, 392 differ (73 %);
  - module parameters at *argument + offset*: 1604 match, 1447 differ
    (53 %); shifted by one octet either way, 19 % and 34 %; read at
    *another instance's* base, **52 %** — no better than chance between
    instances, because most instances share their defaults;
  - only 26 of 56 modular programs have zero contradictions where an
    instance's range is its own; for 13 873 octets two instances'
    computed fields collide.
  - group objects: `ObjNumberBase + Number` matched the GrOT's descriptor
    for 5033 and differed for 360 objects.
- The placement rule for parameters is therefore **not settled** by the
  products, and no PDF states it. Writing module parameters on it would
  risk the octets of another instance or of the program itself. Module
  instances stay refused (`modules` / `parameter-evaluation`), and the
  decisive evidence would be a device read-back of a modular product.
  The group-object side alone would not open a program: its parameters
  are the reason it is modular.

: AppImage interface discovery and line-relative addresses (2026-09-28)


### 19.12 The maintainer's own house, device by device (2026-09-29)

The download-coverage numbers so far are over the 246-program corpus; the
question the maintainer asked is narrower: *which of my devices* can
KNXBench plan a download for. Measured by importing "Unser Zuhause ets
6.3.0 - 2026-09-02.knxproj" into a scratch store, ingesting the same file's
embedded product data into a scratch product database, and running
`knx device download 1.1.<n> --project … --product-db …` (plan only,
nothing sent) for every address. The project has 35 devices: the 32 bus
devices 1.1.1–1.1.32 below, the two IP interfaces 1.1.250 and 1.1.253, and
1.1.220 (a Gira alarm panel, `A-C004-03`), which is on KNXBench's exclusion
list and is refused before any planning ("must never be contacted").

**None of the house's devices is modular.** Its 12 application programs
contain 0 `ModuleDef`s between them, so the module-instance refusal
(§19.11) does not touch this installation.

| Devices | Product | Result |
|---|---|---|
| 1.1.10 | MDT AMS-1216 (`A-0019-16`) | plan |
| 1.1.14–15 | MDT push button 4f (`A-0026-15`) | plan |
| 1.1.16–21, 1.1.32 | MDT push button 8f (`A-0024-15`) | plan |
| 1.1.25–26 | MDT binary input 16f/8f (`A-0030-20`, `A-0031-20`) | plan |
| 1.1.27–31 | MDT dimming actuator AKD-0401 (`A-001B-13`) | plan¹ |
| 1.1.1–9 | Presence detectors (`M-006A_A-0001-22`) | plan³ (machine-5 task segment is `AbsCObjSeg`, not sent) |
| 1.1.11–13 | MDT AMS-1216 (`A-0019-13-B655`) | plan² (inference: later union member at offset 1810) |
| 1.1.22–23 | Gira SmartSensor (`MV-0012`) | refused: not memory-mapped |
| 1.1.24 | Merten blind actuator (`A-5701-10`) | plan³ (`TaskCtrl1`; inference: machine 5 after the restart) |
| 1.1.250, 1.1.253 | EIBMARKT IP interface (`A-0702-10`) | plan³ (`TaskCtrl1`; inference: machine 5 after the restart) |
| 1.1.220 | Gira alarm panel (`A-C004-03`) | excluded: never contacted |

¹ The corpus coverage refuses this program in its *default*
configuration (`UP-5001`'s default `0` is not in its empty enumeration);
the house's own parameter values leave that parameter inactive, so the
device plan does not reach it.

² Refused until 2026-10-07 for two active members of one union; since
ADR-0086 the later member is written as a disclosed inference (follow-up
below).

³ Refused until 2026-10-07 for `LsmIdx 5` (and `LdCtrlTaskCtrl1`); since
then planned as the follow-up on `LsmIdx 5` below describes. The house now
plans 32 of its 35 devices — every one that loads through memory (`knx
device readiness`: 32 untested, 2 unsupported — the `MV-0012` SmartSensors
— 1 excluded).

On 2026-09-30, 17 of the 32 bus devices planned (17 of 35 overall; 32 since ADR-0086); every plan is **Untested** — only 1.1.67 (the test device,
not in this project) is Verified. `knx device readiness` (2026-09-30) reproduces this table from
the project in one command, and `crates/knx-app/tests/house_readiness.rs`
pins it device by device; the union overlap on 1.1.11–13 is reported as
category `parameter-value`. Two findings came out of the run:

1. **1.1.22 and 1.1.24 used to be refused for the wrong reason**, "object
   sends on 2 group addresses". That was an importer bug: every schema ≥21
   `Links` entry was mapped to `Send`. Project Schema23 v01.00.00 states
   "The first group address in the list is always the sending one" (see the
   amendment in §5), and the ETS4 export of the same house agrees on all 543
   comparable senders. Fixed in `knx-etsproj/src/map.rs`; the two devices
   now reach their real, program-level refusals above.
2. **1.1.11–13 are the same product as the plannable 1.1.10 but an older
   program version** whose union at `AS-4400` offset 1810 holds `UP-33`
   (`Factor`, the default member) and `UP-1227` (`Access="None"`). Both are
   written, so the image refuses the overlap. Whether only one member may be
   active, and how the choice is made, is the next item to settle from the
   direct PDFs and the product data — not by picking one.

**Follow-up on 1.1.11–13 (same day).** The overlap is real, not an evaluator
error. In `A-0019-13-B655` the union at `AS-4400` (address `4400h`) offset
1810 holds `UP-33` (`Factor`, `DefaultUnionParameter="1"`, default 230) and
`UP-1227` (`DPT_Switch`, `Access="None"`, `DefaultUnionParameter="0"`,
default 1). `UP-33` is shown in `PB-13` whenever `P-1019`=1 and `P-32`=1;
`UP-1227` is referenced only under `P-40`'s `when test="2"` in the same
block. The house sets `P-1019`=1, `P-32`=1 and `P-40`=2 on all three
devices, so both members are active at once and want different values
(230 vs 1) in the same octet `4B12h`.

- No direct PDF mentions `Union`, `DefaultUnionParameter` or the rule for
  two active members (full-text search of every PDF under
  `knx-spec-kb/sources/`, and of Project Schema23).
- The product data is silent too: the segment's base image holds `00h` at
  1810, the default of neither member.
- MDT's own successor `A-0019-16-CA9D` (1.1.10, plannable) replaces the
  union by a plain `P-33` at offset 1810 and drops `UP-1227` from
  `P-40`'s `when test="2"`. That suggests the hidden member was a mistake,
  but it is a hint, not evidence of what ETS writes for `-13`.

The refusal stays. One read settles it without guessing: 1.1.11–13 were
downloaded by ETS with exactly this configuration, so octet `4B12h` of any
of them holds ETS's choice — `E6h` (230) if `UP-33` wins, `01h` if
`UP-1227` wins, and the house's `UP-33` has no override. That is a
one-octet, read-only `A_Memory_Read` on a device in service and needs the
maintainer's go and gateway; it has not been done.

**Live read on 1.1.11 (2026-09-29): ETS wrote the conditional member.** With
the maintainer's go, one read-only session to 1.1.11 through the house's
gateway (`ManagementSession::read_only`, `AuthorisationPlan::Skip`; log under
`OriginalData/DeviceBackups/1.1.11-probe-4b12/`, not committed):

- mask `0701h`; application `PID_PROGRAM_VERSION` `00 83 00 19 13`
  (MDT, `A-0019-13`), load states `B6EAh`–`B6EDh` = `01 01 01 00`;
- `4B10h`–`4B17h` = `00 00 01 e6 e6 e6 e6 e6`, so **`4B12h` = `01h`**: the
  value of `UP-1227` (`currentFactorOne_0`, 1), not of `UP-33`
  (`currentFactorValue_0`, 230). The neighbouring octets hold 230.

Where the two members sit: `UP-33_R-33` is a direct child of parameter
block `PB-13`; `UP-1227_R-1227` is in the same block, further down, under
`<choose P-40><when test="2">`. With `P-40 = 2` both are visible, and ETS
wrote the second. **[V]** for this one device and configuration.

Why this still does not become a rule: four candidate rules all predict the
same octet here. The member later in `Dynamic`, the non-default member
(`DefaultUnionParameter="0"`), the more deeply nested member, and the member
declared later all point to `UP-1227`. The sample is consistent with each
and discriminates none, and 1.1.12/13 carry the identical configuration. So
`A-0019-13-B655` stays refused for an overlap of two active union members.
The rule is settled only by a device where the candidate rules disagree.
Before its next download, each of the house's three devices keeps its
current octet on the device.

**Online search for the union rule (2026-10-07).** Nothing found states
what ETS writes when two members of one union are active; what was found
says that should not happen:

- `[A]` KNX Association's Manufacturer Tool help, *Define Parameters*
  (`support.knx.org/hc/en-us/articles/360000129320`; the page is behind a
  bot check, read as quoted by the search index): *"To save space, often
  several parameters that are not active at the same time can be put at
  the same location. In this situation, the memory (or property) region
  containing the overlapping parameters must be explicitly declared as
  memory (or property) union."*
- `[A]` *KNX XML Project Schema v1.0 Description* (v1.2, 24.06.2011; read
  as quoted by the search index from copies on yumpu/scribd, not from a
  KNX download), `Parameter_t`: memory parameters "must not overlap … For
  such cases, the Union construct must be used"; `UnionParameter_t`:
  `DefaultUnionParameter` (`xs:boolean`, default `false`) is *"Used during
  image creation"* — nothing more. The ETS4 XSD (namespace `project/11`)
  carries the attribute with no semantics.
- Three independent open-source tools that build download images each pick
  their own rule, none citing ETS behaviour: *bussard*
  (`tmbo/bussard@d309faa9`, `bussard-prod/src/image.rs`) writes the
  `DefaultUnionParameter` member only for a union none of whose members is
  reached and lets reached members, written later, win; *zweidraehte*
  (`ntrnnr/zweidraehte@37459796`) seeds union storage from the default
  member and overlays every visible reference; *koolenex*
  (`jgrahamc/koolenex@2818ee79`) prefers a member with an explicit value,
  then its "default" member, else leaves the group alone, and warns that
  "two members of a <Union> cannot both be live". koolenex reads
  `DefaultUnionParameter="0"` as the marker, the opposite of the boolean.

So two active members are a product-data defect by the authoring rule, not
a case the format defines. One consequence for the candidate rules above:
"the `DefaultUnionParameter` member wins" would have written `UP-33` (230),
and 1.1.11 holds `01h` — the device read rules that one out.

**Resolved as an inference (2026-10-07, ADR-0086).** With the specification,
product data and project files declared enough evidence, the image builder
now writes, of two active members of one union that share bits, the one
later in the parameter tree (`image::UNION_LATER_MEMBER`), and does not
write the other. That reproduces the octet ETS wrote on 1.1.11 and the
"later writer wins" order of the two open-source builders. It is an
inference, not a rule from a source: the image carries it
(`DownloadImage::inferences`), the plan is never `Verified` by evidence of
a run without it, and readiness, `knx device download` and
`/api/device-download/plan` (`support.inferences`) name it. 1.1.11–13 now
plan (Untested). Two active parameters that overlap *outside* a union are
still refused, naming both references.

**Follow-up on 1.1.1–9 and 1.1.24: `LsmIdx 5` on a `0701h` device.** Three
of the house's programs name a fifth load state machine:

- presence detectors `M-006A_A-0001-22-617E` (1.1.1–9):
  `<LdCtrlTaskSegment LsmIdx="5" Address="16628" />` between the
  application program's task segment and its `LoadCompleted`;
- Merten `M-000C_A-5701-10-DCC4` (1.1.24) and the IP interfaces
  `M-006A_A-0702-10-7779` (1.1.250, 1.1.253): after
  `LdCtrlRestart`, `<LdCtrlTaskSegment LsmIdx="5" …/>` and
  `<LdCtrlLoad LsmIdx="5" />` — plus `LdCtrlTaskCtrl1`, refused first.

What the direct PDFs say:

- *Management Procedures* v02.01.02 §3.31.1 lists `stateMachineType` 1–4
  (address table, association table, application program, PEI program),
  and §3.31.2 — the memory-mapped procedure for mask `070nh` — gives a load
  state address for exactly those four (`B6EAh`–`B6EDh`) and "shall
  support only one state machine of each type". There is no fifth.
- *Configuration Procedures* v02.01.01 uses `LsmIdx="5"` only in §3.9.3
  (System B), where `OIDX_APPLICATION_PROGRAM_2 = 5` is an **interface
  object index** loaded through properties (`DMP_LoadStateMachineWrite_RCo_IO`),
  not a memory-mapped machine. Carrying that meaning over to a BIM M112
  would be a guess.

So `LsmIdx 5` on `0701h` has no definition to translate, and all three
programs (12 of the house's devices) stay refused (`DownloadPlanError::Lsm(5)`). What ETS does with it — skip it,
address a manufacturer-specific machine, or something else — can only be
learned from a trace of a real ETS download to one of these devices; none
exists in the corpus. The TaskCtrl1 event itself *is* documented (§3.31.2,
segment type 04h: address + interface-object count) but translating it alone
would still leave 1.1.24 on `LsmIdx 5`.

`[A]` **Outside evidence (2026-10-07), for the post-restart tail only.** The
open-source *bussard* (`tmbo/bussard@d309faa9`, `docs/system7-spec.md` §3)
describes the same tail — `LdCtrlRestart`, `LdCtrlTaskSegment LsmIdx="5"`,
`LdCtrlLoad LsmIdx="5"` — in "converted (pre-ETS4) procedures" (Theben FIX2
`M-0048_A-4947`, mask `0701h`; Jung `M-0004_A-2088-11`) and reports from its
own bus captures that "ETS sends nothing after the restart"; the Jung device
answers a `PID_LOAD_STATE_CONTROL` read of object 5 with count 0. bussard
therefore cuts the procedure at the terminal restart. That is a third
party's capture, not ours, and covers 1.1.24 and 1.1.250/253 (which still
need `LdCtrlTaskCtrl1`), not the presence detectors, whose `LsmIdx 5` task
segment sits *before* `LoadCompleted`. A capture of an ETS download to one
of the house's own devices would make it `[V]`.

**Resolved (2026-10-07, ADR-0086): a documented record, a documented step
and one inference.**

- `[D]` The KNX Cookbook *Load Controls* (`02_03_01` v01.00.02 §2.3)
  documents the presence detectors' step exactly: a task segment of
  machine 5 (`53 02 00 SSSS …`) is `AbsCObjSeg`, which *"announce[s] the CO
  table to a software tool (typically MT) … This record is for
  MT-information only. ETS ignores it and will hence not be transmitted on
  the bus."* Its worked example puts it right after the application's task
  segment at the same address — the presence detectors' `TaskSegment
  LsmIdx 3 @16628` then `TaskSegment LsmIdx 5 @16628`. The plan leaves it
  out; that is the document, not an inference.
- `[D]` `LdCtrlTaskCtrl1` is MP §3.31.2's segment type 4, `L3 04h 00h AAAA
  NN` and five reserved octets, application program only (MP §3.31.1
  table; Cookbook §2.1: "sets the user KNX object table start address and
  the number of user KNX objects"). `load_control_memory::task_control_1`
  builds it; the product's `Address`/`Count` fill `AAAA`/`NN`.
- `[A]` After `LdCtrlRestart` — which "also closes the transport layer
  connection" (Cookbook §2.2) — the converted procedures' `LdCtrlLoad
  LsmIdx="5"` is not sent (`download_plan::MACHINE_5_AFTER_RESTART`): no
  connection is left to send it on, a BIM M112 has no fifth machine, and
  bussard's ETS captures show nothing after the restart. Disclosed as an
  inference. A machine-5 event *before* the restart is still refused
  (`LsmIdx 5 is not loaded here`): nothing documents one.

With these, 1.1.1–9, 1.1.24 and 1.1.250/253 plan (Untested; the last three
name the inference). The product corpus at its defaults (103 packages, 246
programs) now plans 1 verified + 79 untested (was 77); no program is
refused for an unmodelled step any more.

### 19.13 The house read back: the image against what ETS wrote (2026-09-29)

With the maintainer's go ("frage alle ausser 1.1.220 ab"), one read-only
session per device through the house's gateway: identity (mask, manufacturer,
`PID_PROGRAM_VERSION`, load states `B6EAh`–`B6EFh`), then `A_Memory_Read` of
every octet run KNXBench's image would write, compared with the image built
from the ETS 6.3.0 export. No write, no restart; 1.1.220 not contacted. The
probe was a throwaway test (not in the tree); logs are private under
`OriginalData/DeviceBackups/house-readback-2026-09-29/`.

**Reach.** 34 devices addressed. 32 answered and identified as the project
says (programs and masks as in §19.12). 1.1.16 and 1.1.253 rejected the
transport-layer connection twice (KNXBench's error: "the bus rejected the
Transport Layer connection"); nothing is known about them live. 1.1.11–13, 1.1.22/23: identity only (no image). 1.1.24 and
1.1.250: partial compare of a refused plan, not interpreted.

**Compared: 16 plannable devices** (1.1.10, 14, 15, 17–21, 25–32) plus the
nine presence detectors (image builds, plan refused for `LsmIdx 5`). Every
compared octet was read (0 unread). Result by region:

| region | result |
|---|---|
| group object table, octets 0, 1, 3 (RAM pointer, value type) | identical on all 25 devices |
| parameter segments | identical on 11 of 16 plannable devices; 1–3 octets differ on 1.1.15, 18, 20, 29, 31 |
| address + association table | identical (entries resolved to addresses) on 11 of 16; differ on 1.1.18–21, 1.1.32 |
| group object table, octet 2 (flags) | differs on every device — three systematic causes below |

**Project drift, not encoding.** Where tables or parameters differ, KNXBench's
image equals the *project* and the device holds something else: 1.1.19 has
`2/0/25`/`2/1/25` where the project links `2/0/23`/`2/1/23`; 1.1.20, 21 and
32 have `2/0/30`, `2/0/51…53` where the project has `2/0/29`/`2/0/31`; 1.1.29
`452Eh` (`switchOnSetValue_0`) is `33h` on the device and 35 in both the ETS 6
and the ETS 4 export. Only 1.1.18 is marked modified-after-download in the
project; the others carry equal `LastModified`/`LastDownload`, so those stamps
do not prove the device is current. A download of today's project would change
these devices' behaviour, which is what the maintainer asked ETS for, but it
is a change and the device editor should say so.

**Flags: three causes, one of them a KNXBench defect.**

1. **Instance-level flag overrides are not in the image — defect.** 12 active,
   linked objects on 1.1.5, 1.1.20 and 1.1.21 carry `WriteFlag="Enabled"`,
   `UpdateFlag="Enabled"` or `ReadFlag="Enabled"` on their
   `ComObjectInstanceRef`. The device has them; KNXBench's image has the
   product's flags only (`image.rs` builds `ObjectFlags` from the program's
   `ComObject`/`ComObjectRef`). A download would silently drop the user's
   flag, e.g. 1.1.20 object 0 would stop accepting writes. KNOWN_LIMITATIONS
   §145. *Correction and fix, same day:* the 12 linked ones are 1.1.5
   object 0, 1.1.20/1.1.21 objects 0, 5, 10, 15 and 1.1.32 objects 0, 5,
   10, and their overrides are `WriteFlag`/`UpdateFlag`; the `ReadFlag`
   overrides sit on unlinked objects (device `DBh`: read on, communication
   off). The image now applies instance flags; all 12 linked octets match
   the devices (§145, lifted).
2. **Active objects without a link: ETS clears Communication (bit 2).** 173
   of 173 such objects have it clear on the device; KNXBench sets it. The
   object has no association either way, so nothing is sent or received;
   the octet differs but the device behaves the same. Check: product flags,
   then instance overrides, then C cleared if unlinked reproduces the device
   on 450 of 454 active objects; the 4 others (1.1.18) are linked in the
   project to addresses the device does not have (drift above).
   **User decision 2026-09-30: keep.** KNXBench keeps setting C on unlinked
   active objects; this stays a documented, behaviour-neutral difference
   from ETS. Do not reopen without a new user decision.
3. **Inactive objects: C clear on both sides, other bits differ.** For
   `A-0001`, `A-0019-16`, `A-0026`, `A-0030`, `A-0031` the device holds the
   `ComObject`'s own flags with C cleared; KNXBench keeps the base segment's
   octet with C cleared. For `A-0024` the device holds exactly KNXBench's
   octet. No single rule covers both; C is clear either way, so KNXBench keeps
   its documented rule (`group_object_table.rs`).

**Parameters still open (no rule derived).**

- Presence detectors, `4194h`–`4195h` (`brightnessThresholdPIR_0`, 16 bit):
  `0064h` (100) on all nine; the project states no value, the default is
  300, the base segment holds `03E8h`. Refused devices anyway; unexplained.
  *2026-10-09:* the detectors plan since §19.18 and `1.1.8` was downloaded
  live (§19.19); its `0064h` was restored afterwards and stays unexplained.
- 1.1.15 `4593h`/`4595h`: KNXBench writes union member `UP-5677` (`FFh`,
  its default) and subtype `04h`; the device has `00 00`. 1.1.15 `4609h`: the
  device has `06h`, bits 1–2, which no parameter of the program covers.
- 1.1.20 `46DFh`/`46EFh` (`LED8_Funktion`, `LED8_Anzeige`): device `01`/`01`,
  image `00`/`00` although the project states `P-23 = 1` — `P-23_R-23` is
  only active under `P-8 = 1`, and the project states no value for `P-8`.

**What this proves.** For MDT 0701h programs the parameter and table encoding
of KNXBench matches bytes ETS put on real devices wherever project and device
agree. It does not make any device "Verified": no KNXBench download was run,
and defect 1 must be fixed first.

### 19.14 The private telegram capture, decoded offline (K19, 2026-09-30)

- **[V]** The [2026-09-29 inventory](project-format.md#2026-09-29--read-only-inventory-of-local-ets-installation-data)'s ETS `CommunicationLog`
  (`{http://knx.org/xml/telegrams/01}`) holds one `RecordStart`, 71
  `Telegram` and one `RecordStop`. `RecordStart` names the capture mode
  `LinkLayer`, medium `Tp`, connector `IpTunneling`. Every `Telegram` has
  `Timestamp`, `ConnectionName`, `Service`, `FrameFormat` and `RawData`;
  all 71 say `Service="L_Data.ind"` and `FrameFormat="CommonEmi"`, and
  every `RawData` is even-length hex starting `29 00` (cEMI `L_Data.ind`,
  no additional information), 11 to 15 octets. Counts only; values,
  addresses and times stay private.
- **[V]** `knx_net::cemi::decode_l_data` decodes all 71: 71 group
  destinations, 71 `T_Data_Group` (`UnnumberedData`), 55
  `GroupValueWrite`, 8 `GroupValueRead`, 8 `GroupValueResponse`; none
  refused, none left as `Tpci::Unknown` or `ApplicationService::Other`.
  `encode_l_data` gives back every frame octet for octet except Ctrl1 and
  Ctrl2.
- **[V]** Those two octets differ in one field only: 16 telegrams were sent
  at normal priority (Ctrl1 `B4h`) and re-encode at low (`BCh`), because
  `LDataFrame` has no priority field (KNOWN_LIMITATIONS §147). Hop count,
  repeat and every other control bit match the encoder's defaults in this
  capture, which says nothing about other captures.
- **[V]** After `LDataFrame` gained `control` (2026-09-30, KNOWN_LIMITATIONS
  §147 lifted), all 71 re-encode whole, Ctrl1 and Ctrl2 included, and the
  test pins `whole_frame_identical: 71` and an empty
  `control_not_carried`.
- **[I]** 71 telegrams of one installation's everyday traffic exercise only
  group communication; management, point-to-point and extended frames are
  not in it, so this is no evidence for those decoders.
- Test: `crates/knx-net/tests/private_telegram_log.rs`, `#[ignore]`d,
  reads `KNXBENCH_TELEGRAM_LOG` and prints and pins aggregates only;
  4 synthetic tests cover the census itself (5 mutants caught), and the
  decoder's own tests in `cemi.rs` cover the control fields (11 mutants
  caught).

### 19.15 K12 with bit 2 set, an interrupted partial download, and system priority (2026-09-30)

User go "1 alle go" for the open live steps on `1.1.67` (MDT, `0701h`),
gateway `172.18.250.1:3671`. Evidence in
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-30_k12b-*` and
`…_k15b-*` (private; serial number `[REDACTED]` here).

- **[V] Bit 2 can be set and cleared on this device.**
  `knx device service-control 1.1.67 --enable` read `0000h`, wrote
  `0004h`, read it back; a separate session read `0004h` again.
  `--disable` later wrote `0000h` and read it back. ADR-0051's path works
  on hardware.
- **[V] With bit 2 set, the serial-number write still did not take.**
  `address-by-serial 1.1.68`: the device was found by serial number, `1.1.68`
  probed vacant, the write went out, and the read-back by serial number
  answered from `1.1.67`. Scan afterwards: `1.1.67` occupied, `1.1.68`
  vacant. Dumps before and after: 180 lines, byte-identical. So bit 2 was
  not the only reason for 2026-09-29's refusal.
- **[V] KNXBench sent the address broadcasts at the wrong priority.** AL
  v02.01.01 AS §3.2.2–§3.2.5 and §3.3.6/§3.3.7 each say *"The parameter
  priority, implicitly with value 'system', shall be mapped to the
  corresponding parameter of the T_Data_Broadcast.req"* (or
  `T_Data_SystemBroadcast.req`). `FrameControl::default_for` gave every
  request except `T_Connect`/`T_Disconnect`/`T_ACK`/`T_NAK` low priority,
  so `A_IndividualAddressSerialNumber_Write` left as Ctrl1 `BCh` instead of
  `B0h`. Fixed 2026-09-30 (`sent_at_system_priority` in `cemi.rs`), with the
  test first RED on exactly that octet.
- **[V] The priority was not why the write was ignored.** Second run at
  12:40–12:42 with a binary built from `c451fa95` (system priority): scan
  `1.1.67` occupied / `1.1.68` vacant, bit 2 `0000h` → `0004h` (read back),
  `address-by-serial 1.1.68` → again *"still answers from 1.1.67"*,
  `find-serial` → `1.1.67`, scan unchanged, bit 2 back to `0000h` (read
  back twice), dump 180 lines byte-identical to the morning's. Caveat: this
  proves what KNXBench *requested* in the cEMI frame; no bus-monitor trace
  shows the priority the gateway put on the wire.
- **[I] Remaining explanations, none tested:** the MDT firmware does not
  implement `A_IndividualAddressSerialNumber_Write` although it answers
  the Read; it needs a
  restart or programming mode for the bit to take effect; or the gateway
  alters the broadcast. The programming-button path (MP §2.3) works on this
  device and stays the way to re-address it.
- **[V] A partial download interrupted between two table loads leaves the
  association table `Loading`.** `--partial group-addresses` (21 steps,
  1022 octets, backup of 3 regions kept first) was stopped in step 17 of
  21 by the 400 s shell `timeout` the agent had wrapped it in, not by
  KNXBench: memory
  writes took about 6 s each instead of about 1.5 s the day before (cause
  not measured). Everything written was the option-C image the device
  already held. A read-only `device compare` afterwards: all 1416 octets as
  planned, address table and application `Loaded`, association table
  `Loading`. The complete option-C download (25 steps, 1416 octets, each
  read back, backup kept first) repaired it: all three `Loaded`,
  `device compare` identical, and a dump 180 lines byte-identical to the
  morning's. Its `A_Restart` went unacknowledged, as always on this device.
- **[I] Lesson for live runs:** never wrap a download in a wall-clock
  timeout tighter than its worst case; a stopped download is exactly the
  half-loaded state the load-state machine exists to show.
- **[V] The group-address partial download, re-run (user "k15 go").**
  12:46–12:59 with a binary built from `5783b238`, no shell timeout:
  read-only dump before (180 lines, identical to the morning's) and
  `device compare` (all three `Loaded`, 1416 octets as planned); then
  `--partial group-addresses`, 21 steps. Steps 1–6 checked mask, property
  0/78, `PID_PROGRAM_VERSION` and the three load states; the backup (3
  regions, 1022 octets: `4000h`, `4003h`–`4200h`, `4201h`–`43FFh`) was kept
  before step 7 and matches the independent pre-dump in all 1022 octets.
  87 writes, each read back; address and association table `Loaded`; the
  application program was not touched. `A_Restart` unacknowledged, as
  always. 40 s later: `device compare` all three `Loaded`, identical; dump
  180 lines byte-identical to the pre-dump. The same image was written, so
  this proves the procedure and the load-state sequence, not a change of
  configuration. `partial-group-addresses` is now in
  `crates/knx-app/data/verified_downloads.json`; `partial-both` stays
  untested (never run).

### 19.16 K13: `NM_IndividualAddress_Reset` on `1.1.67` (2026-09-30)

User request "K13: Adressen zurücksetzen". Evidence in
`OriginalData/DeviceBackups/1.1.67_MDT-0701_2026-09-30_k13-*` (private).

- **[V] MP §2.18 moves the device to `15.15.255`.** Read-only first: scan
  `1.1.67` occupied, dump 180 lines byte-identical to the K15 post-dump,
  `device compare` all three `Loaded`. Operator pressed the button on
  `1.1.67` only. 15:46:30 `knx device reset-address 1.1.67`: the first
  read found exactly `1.1.67` (the new guard, see KL §140), one round of
  write `FFFFh` + `T_Connect`/Basic Restart/`T_Disconnect` to `FFFFh`, and
  the closing read got no answer. 10 s later: `1.1.67` vacant,
  `15.15.255` occupied, mask `0701h`.
- **[V] The closing read's silence did not mean programming mode ended.**
  The operator reported the LED still on. MP §2.18 sends the restart
  without evaluating anything, and the device did not act on it (its
  addressed Basic Restart goes unacknowledged too, §19.8/§19.15). Why it
  did not answer the closing read is not measured; it did answer the next
  one, 1½ minutes later. The CLI therefore no longer says "nobody answers
  in programming mode any more" but "nobody answered the closing read,
  restart not confirmed", and names the LED and `program-address`.
- **[V] Recovery by MP §2.3.** 15:48:11 `knx device program-address
  1.1.67`: round 1 found `15.15.255` in programming mode, `1.1.67` was
  free, the write landed, the device answered at `1.1.67`; the restart
  again unacknowledged. The operator reported the LED off afterwards. 40 s
  later: `1.1.67` occupied, `15.15.255` vacant, `device compare` all three
  `Loaded` and 1416 octets as planned, dump 180 lines byte-identical to the
  pre-dump. The individual address is not in the dumped memory, so the
  application survived the round trip untouched.
- **[I] Not observed:** the frames themselves (no bus monitor ran), and
  what the device would do with a second pressed device on the line; the
  guard refuses that case before writing and is covered in the simulator.

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

### 19.18 Object priority, ReadOnInit and floats under ADR-0086 (2026-10-07)

The three image-structure refusals of §19.10, re-examined with the
specification, the product data and the projects as evidence.

- **`Priority="Alert"` → urgent (`10b`), an inference.** A census of the
  corpus's `070nh` programs (198 with a `ComObjectTable` in a segment with
  `Data`, 17 731 objects) compared each `ComObject`'s `Priority` with the
  priority bits of its descriptor's config octet in the product's own base
  image: `Alert` carries `10b` in 284 of 285 objects (one `11b`); objects
  without a priority carry `11b` in 15 221 of 15 477, `Low` in 1 853 of
  1 935. *Resources* §4.18.3.1.2.1 names `10b` urgent. The image builder
  writes `Alert` as urgent and discloses `image::ALERT_IS_URGENT`.
- **`Priority="High"` stays refused.** The same census gives `00b` 17
  times, `10b` 4 times and `11b` 13 times, and never `01b` (normal): the
  product data contradicts itself, which ADR-0086 decision 4 refuses.
- **`ReadOnInitFlag="Enabled"` is documented as unrepresentable, and the
  loss is disclosed.** `[D]` *Resources* §4.18.6.2.4.1.3, NOTE 85: *"'Value
  Read on Initialisation' is a new feature introduced with System B."* The
  System B descriptor has it as bit 13; the `070nh` (Easy-3/Type-2) config
  octet has the segment selector in that position and no read-on-init bit
  (§19.2). So the image is written exactly as with the flag disabled, and
  `image::READ_ON_INIT_NOT_ON_070N` tells the user the device will not read
  the object after a reset. The project's own `ComObjectInstanceRef`
  `ReadOnInitFlag` now reaches the image request (`FlagOverrides::
  read_on_init`) and decides, over the product's, whether that is said.
  **[V] Corpus effect:** the 10 programs this refused now plan; the corpus
  at its defaults plans 1 verified + 89 untested (was 79), `image-structure`
  41 → 31. No `Alert` object was the first refusal of a program, so that
  inference changes no count today.
- **Floats stay refused.** The product declares the encoding (`TypeFloat
  @Encoding`: `DPT 9` or `IEEE-754 Single`), but the base images contradict
  a DPT 9 reading where they hold a non-zero default (`500` as `F4h 01h`, a
  little-endian integer, §19.9); 57 non-zero `DPT 9` and 16 non-zero
  `IEEE-754 Single` defaults match neither encoding. With
  contradicting sources and no working solution shown, ADR-0086 keeps the
  refusal.

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

### 19.21 Bathroom fourfold button 1.1.14: complete download and restore (2026-10-09)

With the maintainer's go for the proposed test on `1.1.14`, KNXBench's
existing CLI downloaded the house project's configuration to the MDT
fourfold button (mask `0701h`, manufacturer `0083h`, program
`M-0083_A-0026-15-3591`). Source revision `b3ee55a0`, isolated project and
product database imported from the same ETS 6.3 project. No access key;
no changes to the executor or protocol. **[V]**

- **Read-only baseline:** the complete address segment `4000h`–`4200h`,
  association segment `4201h`–`43FFh`, application segment
  `4400h`–`461Bh`, and six load-state octets `B6EAh`–`B6EFh`: 1570 octets.
  All 1562 configuration octets comparable to the 2026-09-29 reading
  match; physical address is `1.1.14`; the three affected parts are Loaded.
- **Fresh compare:** 57 of 1562 planned octets differ, each a group-object
  descriptor config octet. Two are the documented C-bit difference on
  active unlinked objects; 55 are inactive-object config differences
  (§19.13). No parameter, active association or group-address difference.
  This is not proof of a newly changed light configuration: the run tests
  the load sequence with the existing parameter values and links.
- **Download:** 11:10:47–11:14:04 CEST, 25 steps, 1562 memory octets,
  every write read back, three parts Loaded. The pre-write backup is
  identical to the separate baseline at all 1562 covered octets. Basic
  Restart did not receive T_ACK: `RestartOutcome::Unconfirmed`, not a
  confirmed restart. No separate restart or power cycle was performed.
- **Independent post-download readback:** after the restart wait, all 1570
  selected octets equal the expected image and retained physical-address
  and load-state bytes. A fresh CLI compare is clean (exit 0).
- **Functional check:** during the user-operated check, `1.1.14` sent
  `DPST-1-1 on/off` to the main light (`1/0/11`) and mirror light
  (`2/0/43`) between 11:25:48 and 11:25:51 CEST; the user replied
  "Erledigt". The monitor also captured three user-originated blind
  up/down/stop telegrams on the project's configured groups. These extra
  telegrams do not establish physical blind movement; no additional
  software-driven group write was made. No telegram from this device
  addressed an unconfigured group in the retained monitor interval.
- **Restore:** same 25-step product procedure, its own pre-restore backup,
  11:27:39–11:30:49 CEST, 1562 octets read back, three parts Loaded,
  Basic Restart again unconfirmed. After the wait, a separate dump
  equals the pre-dump at all **1570** selected octets (0 differences);
  before/after-restore CLI comparisons are byte-identical. This proves
  restoration of these configuration regions and selected load-state
  bytes, not all device memory, properties or volatile operating state.
- **Evidence scope:** the third shipped program entry, `complete` only.
  House readiness: 11 verified / 21 untested / 2 unsupported / 1 excluded.
  `1.1.15` inherits the program grade, but was not contacted or downloaded;
  its unexplained configuration differences in §19.13 are not resolved.
  UI and partial downloads were not exercised in this run.
- **Private artefacts:** `OriginalData/DeviceBackups/2026-10-09_button-114-live/`
  holds independent dumps, compare logs, download/restore plans and runs,
  both backups, timestamped monitor, activity history and the read-only
  probe source. No raw project configuration is committed.

## 22. Serial-address write recovery scope (2026-09-30)

**[V]** `serial_number_write` (MP §2.5) can observe the former address by
serial number and the proposed address's occupancy before broadcasting; the
current CLI/HTTP paths did not persist that state before send. A later
readback/HTTP response is volatile and cannot recover an interrupted write.
The new shared application precondition refuses confirmed CLI/HTTP writes
before opening a tunnel (ADR-0057); read-only lookup and the protocol
simulator are unaffected.

**[D]** The KNX Association's Architecture v3.0 §2.6 says network identifiers
and formats do not necessarily describe device-internal storage
([specification PDF](https://pahl.de/download/dissertation/ds2os.lab/files/03_01_01_architecture_v3.0.pdf),
p. 9). The Association's [Device Reader documentation](https://support.knx.org/hc/en-us/articles/115001822070-Device-Reader)
requires the memory range and address space and potentially a supplied
access key for reading a device's memory. **[I]** Neither source identifies the complete
manufacturer-specific storage side effects of an MP §2.5 address write; a
saved old address or an arbitrarily chosen memory region is therefore not
proof of a full affected-storage backup. No access key may be guessed.

**[A]** Do not mistake this fail-closed availability change for a verified
recovery implementation or an authorization for another live test. Research
per-device/mask scope and prove durable pre-send persistence/readback plus a
recovery path before considering reopening, then add phase-aware activity
without claiming a Telegram's send is a verified device effect.

## 23. K13 reset recovery scope (2026-09-30)

**[V]** The public `knx device reset-address` previously connected after a
valid address list and confirmation phrase without performing an automatic
pre-write storage backup. MP §2.18's first broadcast read identifies devices
in programming mode, not their internal storage. ADR-0058 now blocks the
confirmed CLI before tunnel opening; an offline test with a bound local UDP
socket witnesses zero datagrams. Plan-only and direct simulator procedures
remain available.

**[V]** RESEARCH §19.16 records one authorized `1.1.67` live reset followed
by address recovery and an unchanged application-memory dump. The operator
had separately captured a persistent backup; the CLI had not verified it or
bound it to every storage area a reset could change. The device's programming
LED stayed on despite the unevaluated restart.

**[I]** Neither that single-device observation nor the general address-reset
procedure establishes the complete manufacturer-specific affected-storage
scope for every target or every simultaneously pressed device. ADR-0057's
Architecture and Device Reader references (§22) caution against treating a
network address or an arbitrary memory range as a whole-device backup. A
safe reopening needs an identified device/mask, proven storage scope,
pre-send durable per-device backup/readback and a separate recovery plan. No
access key may be guessed; no new hardware go is implied.

## 24. K6 button-address programming recovery scope (2026-10-01)

**[V]** The public CLI and HTTP button-based address programming paths
previously passed a valid new-address phrase and could open a tunnel. Neither
created a durable pre-send backup/readback for the actual device selected by
pressing its programming button. The generic download backup is plan-scoped
and cannot automatically prove the storage effects of MP §2.3. ADR-0059
now rejects confirmed public calls before tunnel or lock acquisition; an
offline CLI UDP listener and HTTP simulator witness no datagram/connector call.
Plan/phrase reads and direct simulated protocol sessions are unaffected.

**[V]** RESEARCH §19 and KNOWN_LIMITATIONS §116 describe a user-authorized
`1.1.67 → 1.1.68 → 1.1.67` run with readback and unchanged application
dump. That verifies the observed run, not an automatic full-storage backup
for arbitrary future button-selected devices, nor a successful restart
acknowledgment.

**[I]** MP §2.3's one-device button count establishes a protocol recipient,
not the recipient's product identity, affected-storage scope or durable
recovery record. A safe reopening requires those facts and a persisted,
read-back, device-bound pre-send backup plus an abort/restore plan. ADR-0057's
Architecture/Device Reader sources (§22) warn that network identifiers and
arbitrary read ranges are insufficient as full manufacturer-specific memory
evidence. No access key may be guessed; no new hardware permission is implied.

**[V] Public-entry audit, 2026-10-01 (offline).** The CLI and server's
confirmed K6 and serial-address routes, plus the K13 CLI, invoke their
respective pre-tunnel recovery gates. CLI device download and restore both
call `device_download::execute`, whose `run_memory_download_with_backup`
reads the plan's overwritten regions and touched load states before the
first mutation; `write_backup` fsyncs the file and directory and compares a
readback before allowing that write. This is *plan-scoped*, not a complete
device image, and does not establish a manufacturer-independent rollback.
The Debug-gated service-control CLI/HTTP action saves and rereads the exact
property element before writing bit 2; that receipt does not cover any
other storage. K14's erasing operation remains hardware-blocked. Group-value
sends (`knx bus write`, `POST /api/bus/write`) belong to `goal.md` (§5),
not a commissioning write gate; their receiver effects remain unverified.
No write route was reopened or hardware contacted in this audit.

**[V] Web projection boundary.** The still-locked Program address panel
fetches a valid phrase, opens its consent dialog, then on a confirmed start
displays the server's `412` message as an alert. It does not claim a
successful write, but still offers a now-unavailable action and has no
focused UI regression for the durable-recovery refusal. The Web lock owner
must decide the affordance and add its test; the server gate is the safety
boundary in the meantime. No Web source was changed here.

**[V] Official diagnostic-source cross-check, 2026-10-01 (offline).** KNX's
[Individual address](https://support.knx.org/hc/en-us/articles/360018775719-Individual-address)
documentation describes programming-mode response counts and a line scan of
addresses and mask versions; neither is a complete product/application or
storage-scope proof. [Device Info](https://support.knx.org/hc/en-us/articles/360018777979-Device-Info)
can report manufacturer, order number, serial, firmware and application, but
its available fields depend on mask and some group-communication information
cannot be read from some devices. The Association's
[Device Reader](https://support.knx.org/hc/en-us/articles/115001822070-Device-Reader)
documents a caller-selected memory range and address space. Its property
export leaves array properties larger than 64 bytes blank if they were not
loaded on demand. These are diagnostic facilities, not a documented complete
pre-write image of the button-selected device. The cited pages do not specify
which persistent areas MP §2.3 changes for our MDT 0701h target.

**[V] MDT manual supplied by the operator, 2026-10-01.** The public
[Taster 55 / Plus 55 / Plus TS 55 technical manual, version 1.3](https://www.mdt.de/fileadmin/user_upload/BE-TA55xx-02_MDT_TM_V13_DE.pdf)
(07/2025) covers `BE-TA55Px.x2`, explicitly including `BE-TA55P2.02` and
`BE-TA55P2.G2`. That is the *right product family*, unlike the AMI actuator
manual supplied earlier. It does not establish that the live `1.1.67` is
this `.x2` generation: the operator previously named `BE-TA55P2.G1`, the
local product-file comparison was only consistent with `.01`, and
`PID_ORDER_INFO` was not read (RESEARCH §8.8.5). The manual covers user
functions, commissioning and ETS parameters; no complete affected-storage
inventory or recovery image for MP §2.3 is established from it. The user
explicitly said non-public manufacturer documents are **not a blocker to
continuing the project**. This is not an authorization for a live write or
a substitute for durable, read-back pre-write recovery evidence.

**[V] Read-only order-identity feasibility, 2026-10-01 (offline).** KNX
Association's [Device Info description](https://support.knx.org/hc/en-us/articles/360018777979-Device-Info)
qualifies its order number: only when the manufacturer supplies it in a
readable/decodable form; otherwise the result is hexadecimal. Weinzierl's
[BAOS user guide, §5.2.5](https://weinzierl.de/images/download/documents/baos/weinzierl_knx_baos_users_guide.pdf)
lists `PID_ORDER_INFO` as property 15 of its own Device Object with a
generic-ten type; that is an example of property access, **not** proof that
MDT `1.1.67` exposes the same bytes, how to decode them, or that `.G1` and
`.G2` can be distinguished by them. The existing KNXBench
`ManagementSession::read_property` would surface a refused property as an
error; there is currently no public read-only `PID_ORDER_INFO` CLI route.
Even a successful raw read could narrow identity only after an independently
verified MDT mapping; the project product reference describes intent, not
the hardware. No property read, gateway contact, key use or live operation
occurred in this source/code check. A future explicitly scoped read-only
preflight must use the named device and gateway, the one-tunnel exclusion,
an operator/project-supplied key if actually needed (never guessed), and
report raw bytes or refusal without presenting an inferred model as fact.
This does not supply affected-storage coverage or lift the K6 write gate.

**[V] Alternate K6 candidate presence, 2026-10-01 16:55 CEST (read-only
hardware).** After the operator reported `1.1.67` unavailable and requested
another pushbutton, the project-labelled `1.1.32` was selected as a *candidate*
within the pre-approved `1.1.24`–`1.1.32` read range. A production CLI
`knx bus scan` dry run named exactly one candidate (`1.1.32`). Immediately
before the live command, no *local* KNX process or UDP socket for the gateway
was observed. One tunnel to the previously confirmed gateway probed only
`1.1.32`; the reviewed CLI path sends `T_Connect`, DD0 read and
`T_Disconnect` (no independent wire capture). The CLI reported
`occupied (mask 0x0701)` in 199 ms, summary `1 probed / 1 occupied`, exit 0.
No access key, property read, programming button, write or other address was
used. This is **current address occupancy and mask only**: it does not prove
the ETS project label `BE-TA55P8.01`, manufacturer, order number, serial,
physical accessibility or a durable recovery image. The older twofold
`.G2` manual cannot be transferred to this eightfold candidate. A write to
`1.1.32` needs its own device-specific recovery proof and fresh go; the prior
`1.1.67` authorization does not follow the candidate.

**[V] Separate bounded identity probe and offline cross-check, 2026-10-01.**
With exactly `KNX_IDENTIFY_ADDRESS=1.1.32` and the IP-only discovered tunnelling
gateway, the explicitly ignored `knx-net` `live_identify` test used
`ManagementSession::read_only` and `AuthorisationPlan::Skip`; it passed **1/1**
in 0.61s, disconnected cleanly, and again read mask `0701h`. Device Object
`PID_MANUFACTURER_ID` and `PID_HARDWARE_TYPE` each returned octets;
`PID_PROGRAM_VERSION` answered **no elements** (a refusal, not an empty
version). No key, write, other address or programming-mode broadcast was
involved. The two-octet manufacturer value agrees with MDT's `M-0083`
master-data entry documented in §8.8.5. A bounded in-memory inspection of
the local `MDT_KP_BE_01_Push_Button_V15a.knxprod` found that the six-octet
hardware type differs from the historical twofold target but occurs as a
**prefix** in two of eight Device Object (`ObjIdx=0`, `PropId=78`)
`LdCtrlCompareProp` records: `A-0020-15-7F81` and `A-0024-15-6E79`. Each
record has **ten** octets; a six-octet prefix match is neither a successful
full comparison nor proof of the installed product/order/application or
affected storage. The project names the latter program, which is evidence of
intent, not the installed image. Raw endpoint and property bytes were not
copied into Git. The operator replied “go” to the candidate-suitability
question; an exact write plan, complete durable recovery and operation-specific
write confirmation remain absent. See the scoped `.ai` read-only log.

**[I] K6 recovery input still missing.** Before implementing a backup-based
reopening, obtain the actual target's product/application identification and
authoritative affected-storage mapping, including non-memory state and
readability/access restrictions. Prove exact per-device coverage and persist
and read back all pre-write bytes/properties before any send; a Device Reader
file with blank/unread fields or a generic memory range cannot qualify. The
older *source-only* check did not contact a gateway; the later read-only probes
above did, but grant no write authorization. No access key may be guessed.

---
