# 19.12 The maintainer's own house, device by device (2026-09-29)

[Research index](../commissioning.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

## Dated note: declared ETS download plug-ins (2026-10-24)

The preserved table below is unchanged. Since KL-006's 2026-10-24 boundary
([dossier](../../limitations/KL-006-devices-dependent-on-vendor-plug-ins.md)),
a program declaring `Static/Extension/@EtsDownloadPlugin` is refused for every
download scope. In this house that moves the **1.1.24** row from *plan³* to
*refused: `vendor-download-plugin`*. The **1.1.22–23** rows declare a plug-in
too but keep their grade and reason (*refused: not memory-mapped*): the kind
check comes first, by fixed precedence. Readiness now reads 31 untested,
3 unsupported and 1 excluded of 35 devices
(`crates/knx-app/tests/it/house_readiness.rs`).

<a id="1912-the-maintainers-own-house-device-by-device-2026-09-29"></a>
<!-- BEGIN preserved record-2-021 -->
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


<!-- END preserved record-2-021 -->
