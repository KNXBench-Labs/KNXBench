# Bounded commissioning support

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-007)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

## Service-specific memory budgets — 26 October 2026 source

Ordinary memory and user memory share a service-aware data-budget helper;
user memory reserves its extra header octet. A standard frame therefore carries
12 ordinary or 11 user-memory data octets. Hardware-kind/default-unknown routes
are held conservatively to that frame after existing property handling;
simulator modeled capacity remains unchanged
([contract](../contracts/memory-transfer-budget.md)). Actual interface/coupler
capacity, extended-path authority and physical throughput remain unmeasured.
Original semantic RED and independent initial/correction reviews passed;
all 16 selected maintained integrated stages passed, published as `1e6229cea3`
([record](../status/2026-10-26-memory-transfer-budget.md)). KL-007 remains open;
no new family, write authority or whole-recovery capability follows.

## Regular MergeId fragment admission — 25 October 2026 source

The regular planner conservatively refuses a declared `MergeId` fragment beside
its one main procedure ([contract](../contracts/regular-merge-fragment-admission.md)).
The prior positive unit contract is intentionally narrowed; fragment meaning
under `ProductProcedure` is not invented. Read-only comparison, source-freshness
and restore preparation sharing the planner inherit the refusal. Exact stored
source and the fragment-free plan stay unchanged. Focused tests, independent
review and all 14 selected maintained stages passed; published as `be2687796`
([acceptance](../status/2026-10-25-regular-merge-fragment-admission.md)). No family,
hardware or whole-recovery claim follows; KL-007 and PRG-01 remain open.

## System-2 prestate observations — 25 October 2026 source

After a field report (GitHub issue #30: a UP 258D12 refused with "current
resource pointer differs from original source allocation shape" before any
write), the owner ruled that a prestate differing from the project/source
shape must not refuse the experimental System-2 download. Mask, manufacturer
and hardware type stay hard pre-write checks. Serial number, current program
version, non-Loaded resources and a differing PID_TABLE_REFERENCE are now read
and recorded as observations ([contract](../contracts/alpha16-system2-configured-download.md#actual-state-backup-and-recovery-limits)).
After any such difference except the serial, original-state recovery is
unavailable for that operation; recovery replay stays bound to the serial read
at acquisition. Final memory, Loaded and program-version verification stay hard;
a final pointer difference is recorded only. The owner also makes the
read-back after each allocation/task record an observation as well, records a
refused PID 7 read as `refused` in every phase, and stops zero/all-FF serials
from blocking (recovery is unavailable for them). An allocation record a device
silently ignores is therefore caught only by the final memory, Loaded and
program-version verification. A refused or short prestate memory read is
recorded and makes recovery unavailable instead of refusing. Accepted
consequence: a device that refuses every memory read was refused before any
write; it is now written and then fails final verification with
mutationMayHaveStarted true and no recovery. The UI lists such allocations and
never calls their zero placeholders a backup. Final pointer
observations are journaled before the closing VerifyMode write, and download
and restore observations are kept separately. The status DTO reports whether a
failed operation could have written anything. Software and simulator tests only; whether the
relaxed admission downloads the reporter's device correctly is unverified.

## Regular download receipts — 25 October 2026 source

Run-scoped preparation and settlement receipts are display evidence in regular
history ([contract](../contracts/download-settlement-receipt.md)). Applied parts,
computed-but-not-applied parts and the later read-only check remain distinct.
Independent reviews and all 15 integrated stages passed on `3431aabbc`
([acceptance](../status/2026-10-25-download-settlement-receipt.md)); final documentation
integration preserves runtime bytes. A missing receipt is not evidence of unchanged
project state; project and history persistence are not atomic. CLI/System-2,
physical-exemplar identity, device baselines and whole-device recovery remain outside
this increment. KL-007 stays open.

## Regular download prewrite stop — 25 October 2026 source

Server shutdown requests a run-bound stop before tunnel/spawn; accepted stop and
first mutation commitment are atomically ordered. Before commitment no mutation
is sent; afterwards execution and cleanup continue unchanged
([contract](../contracts/regular-download-prewrite-stop.md)). Independent reviews
and all 14 integrated stages passed; published as `6c9ca2d7d`
([acceptance](../status/2026-10-25-regular-prewrite-stop.md)). No public stop route,
UI/CLI interrupt, System-2 stop, effect journal or whole-device recovery is added.
Actual HTTP-start execution and physical device outcomes remain unverified. KL-007
and the forced-exit shutdown boundary remain open.

## Duplicate image-start admission — 25 October 2026 source

The regular planner refuses two data-bearing image segments at an allocated start,
instead of choosing the first payload; legitimate payload-free RAM/stack allocations
remain valid ([contract](../contracts/duplicate-image-segment-start.md)). Read-only
preparation sharing this planner inherits the refusal. Independent review and all
14 integrated stages passed; published as `d73a2413b`
([acceptance](../status/2026-10-25-duplicate-segment-start.md)). Synthetic source
ambiguity and selected corpus regression prove no real-product fault, new family
support, different-start overlap policy or hardware outcome. KL-007 remains open.

## Parameter number-bound admission — 25 October 2026 source

Declared TypeNumber bounds must be readable by the existing integer parser before
an image can be built; unreadable bounds no longer disappear. Unsigned negative
bounds are refused, not converted to zero. Raw source, encoding and experimental
retained-source admission are unchanged
([contract](../contracts/parameter-number-bound-admission.md)). Published as
`87d71d4aa` after independent review, correction re-review and all 14 integrated
stages ([acceptance](../status/2026-10-25-parameter-number-bounds.md)). The selected
97-case corpus regression is not an exhaustive bound census; other real-product
spellings and hardware remain unverified. KL-007 stays open.

## Memory response service pairing — 25 October 2026 source

A response must match both the requested memory service and address before it
can confirm a read or active-Verify write; a wrong-service empty response cannot
become the requested service's refusal. Read-back without active Verify inherits
the rule ([contract](../contracts/memory-response-service-pairing.md)). Independent
reviews and all 14 integrated package stages passed; published as `53400189e`
([acceptance](../status/2026-10-25-memory-response-pairing.md)). Synthetic
nonconforming-peer coverage does not prove real-device faults, aliasing or hardware
compatibility. Retry and admission policies are unchanged; KL-007 remains open.

## Property load-control response — 25 October 2026 source

The state in a property load-control write response is validated before polling;
observed LoadCompleting seeds the existing conditional wait during device silence
([contract](../contracts/load-control-response-state.md)). Invalid responses fail
before a later poll can hide them. No mutation is re-sent, deadlines and profile
restrictions remain, and the separate memory-mapped mechanism is unchanged.
Independent review and all 14 integrated package stages passed; published as
`e4f5e7c6f` ([acceptance](../status/2026-10-25-load-control-response.md)). KL-007 and hardware acceptance remain open.

## Region-bound memory reads — 25 October 2026 source

Backup, comparison and probe reads retain one service per region; user-memory
reads respect their 15-octet wire count even under a larger negotiated budget
([contract](../contracts/memory-region-read-service.md)). Synthetic real-codec
regressions, independent review and all 14 integrated package stages passed;
published as `a4a4ebcf1` ([acceptance](../status/2026-10-25-memory-region-reads.md)). This changes no family, path-budget or recovery admission and proves
no physical address-space relationship. KL-007 remains open.

## Source freshness at settlement — 25 October 2026 source

A completed regular download credits written memory parts only if the current
catalogue still plans the original request identically at the selected scope.
Changed or unprovable plans leave those parts pending, including their undo/redo
claims ([contract](../contracts/download-settlement-source-freshness.md)).
The guard observes settlement only: subsequent catalogue changes and flags for
parts the run did not write are not monitored. It is not device-byte, runtime,
exemplar or recovery evidence. Independent reviews and focused software tests
passed. Published as `fa4c18213` after all 14 selected integrated package stages
([acceptance](../status/2026-10-25-download-source-freshness.md)). KL-007 stays open.

## Post-download check — 25 October 2026 source

Regular memory downloads now perform one fresh read-only follow-up after the
existing transfer/restart wait, preserving reach, identity and expected/observed
load states in CLI, HTTP, web reports and durable history. Software acceptance
and publication as `8b5edad7f` are complete
([contract](../contracts/post-download-check.md),
[acceptance](../status/2026-10-25-post-download-check.md)). Runtime remains unknown,
physical-exemplar identity is not proved, and restore is unchanged. This does
not broaden device-family admission or close this limitation. No hardware
acceptance is inferred.

## Partial application identity width — 25 October 2026 source

The existing partial-download object 3/PID 13 precondition now requires exactly
five response octets, before backup or mutation; product comparison padding does
not define response width. Complete downloads gain no unsolicited identity check
([contract](../contracts/partial-identity-comparison.md)). Independent review,
focused simulator regressions and integrated acceptance passed; published as
`02c437110` ([acceptance](../status/2026-10-25-programming-scope-and-identity.md)).
Other object indices retain their generic comparison rule; this does not close
general target identity, family support or hardware acceptance.

## Backup prior-application identity — 25 October 2026 source

A backed-up download records the application (object 3/PID 13, five octets) the device answered
before the first write; backup format 2 stores it, format 1 reads as not recorded. A restore is
derived only when that evidence equals the plan's single application identity, or, for a legacy
partial backup, behind the plan's own executed identity precondition
([contract](../contracts/backup-application-identity.md)). Older complete backups no longer
restore, and format 1-only builds cannot read format 2. This is a necessary guard only: it does
not prove allocation compatibility, the physical exemplar, the final identity or whole-device
recovery, and it changes no admission. Published as `08db292f3` after independent
reviews and all 16 selected integrated package stages
([software acceptance](../status/2026-10-25-backup-application-identity.md));
this limitation and hardware acceptance remain open.

## Hardware identity width follow-up — 25 October 2026 source

The regular memory executor now refuses Device Object PID 78 responses whose
length is not six octets, before backup acquisition/persistence or mutation.
Valid padded product comparison declarations remain supported; diagnostic
collector/archive exact-length guards remain in place
([contract](../contracts/hardware-identity-comparison.md)). Focused simulator
regressions and the maintained integrated package gate passed; source published
as `07fe2e67f` ([delivery](../status/2026-10-25-hardware-identity-width.md)). This neither resolves general
serial/exemplar identity nor changes device-family admission or this ledger row.

## Current experimental System-2 boundary — 23 October 2026

The separate consent-gated App/Net/CLI route defined by
[ADR-0152](../adr/0152-gated-hardware-system2-download.md) supersedes historical
blanket-refusal descriptions for that exact experimental workflow only. Ordinary
memory-download admission remains unchanged. Physical effects/recovery and full
ETS compatibility are not verified; target-specific confirmation acknowledges
best-effort risk, not missing semantic evidence as fact.

Project-configured source preparation is independently delivered offline and
cannot be admitted to hardware by editing its simulator marker. The adopted
[Alpha 16 plan](../goals/alpha16-release-plan-2026-10-23.md) requires a separate
configured hardware service and actual UI integration; these remain implementation
work until combined acceptance. No new device operation or support certification
is implied by this prose reconciliation. Earlier dated evidence below remains
historical rather than a duplicate implementation queue.

## System-2 inactive-policy follow-up (20 October 2026 source)

An explicit `DownloadInvisibleParameters` option now refuses offline System2
image assembly by name; retaining the option without interpreting it no longer
returns an apparent active-only success. This is a conservative integrity
boundary, not implementation of the inactive/default/legacy patch rules.
[The source/test matrix](../research/2026-10-20-system2-download-integrity.md)
separates verified descriptor/association rules from unresolved tool policy.
System1/Easy3 policies, module mapping, complete-image translation, full-state
recovery and public mask admission stay unchanged. No new device support.

## Explicit parameter-bit follow-up (16 October 2026)

[Direct single-octet fields](../contracts/offline-explicit-parameter-bits.md) now
replace differing template defaults without guessing byte order. Latest private
initial candidates/four independent comparisons are historical, not complete
source acceptance. LegacyPatchAlways refusal closes the strict System1/System2
source entries, including inactive declarations. The broader trial Easy3 guard
was withdrawn rather than weakening existing positive corpus tests. Easy3 keeps
its existing inference policy; this neither defines the directive there nor
adds any source/firmware completeness claim. Configured13-source refusals are
7FixupList/2legacy-patch/3inferred-default/1Union. No executable plan,
whole-mutation recovery, public admission or firmware verification is added.
The P5/P6 counts below remain historical, not current outcome totals.

## System-300 offline property images (16 October 2026 source)

[Pure S-mode property-table images](../contracts/offline-system300-property-tables.md)
separate element0 length from the actual data elements using shared Type6 grammar.
Local codec tests are not source/format/object discovery, complete application
loading, property-service addressability, durable recovery or hardware support.
Empty address/association tables retain their acceptance/default-mapping meaning.
No public mask is admitted; KL-007 remains open at its existing scope.

## Exact UP258 source boundary (16 October 2026)

[Fresh vertical-path recheck](../research/2026-10-16-up258-vertical-path-audit.md)
pins the current first LegacyPatchAlways refusal in defaults and two configured
requests. All 21 source positions are literal regression oracles, not a complete
source-to-wire plan. The six patch declarations, absent effective byte-order
rule and whole-mutation reconstruction gap remain unresolved. No new recovery
receipt or public write route is admitted.

The [exact-application decision](../research/2026-10-16-up258-production-decision.md)
keeps issue #22 programming refused: multibyte source encoding, XML SegFlags/
AllocExtraByte/resource transforms and reconstructible complete prior allocator/
property/task state are unresolved. Published `6d26a1e3` corrects the pure property
checksum-disable polarity (bit7=0), not these mappings. The [prepared reporter
request](../history/REMOVED_DOCS.md#reporter-request-removal-23-october-2026) is not a sent request or
permission for any device operation. No new complete image, recovery or device
support is claimed.

## Shared source-fixup refusal (16 October 2026 source)

The [shared image guard](../contracts/source-fixup-integrity.md) now also covers
Easy3/System-2 in published P6 (`39c88aba`). It prevents partial-artifact success but
does not interpret fixups. Even matching source/master target words are only
correlation, not a general no-op exemption or permission for a hardware write.

## Offline System-1 source-image boundary (16 October 2026 source)

[Bounded Type-1 assembly](../contracts/offline-download-system1-image.md) is
software infrastructure, not complete real-product support. All thirteen final
real-source cases refuse: FixupList7, source-default bits5, mid-octet Union1.
An apparent intermediate image ignored FixupList and is withdrawn. Source
FixupList semantics, ambiguous parameter placement/encoding, default-procedure
translation and complete durable recovery remain open. No new hardware admission.

## Offline System-B table boundary (16 October 2026 source)

[The independent Format-1 object-table codec](../contracts/offline-systemb-object-table.md)
is a pure byte prerequisite, not a product image, executable plan or recovery
implementation. Explicit initial-read selection and extended types do not grant
new mask admission. Three measured real-source AP1 results remain partial and
non-executable; relative images, source scopes/substitutions and complete durable
property/memory/setup/allocation recovery remain open. No new hardware support.

P4b adds a [source-only relative segment view](../contracts/offline-relative-source-segments.md),
verified against three original application sources. Relative source extraction
is not relative-image assembly. The existing absolute image builders now refuse
relative/mixed sources explicitly, rather than omitting relative data; source
scope/substitution, generated table mapping, encoding and recovery remain open.

P4c adds [pure System-B link-table grammar](../contracts/offline-systemb-link-tables.md),
not format discovery/product placement or download admission. Its narrow/wide
indices and first-match sender policy must not be replaced by an Easy-3 or
legacy table. Original57B0merge examples do not resolve all selected07B0modern
source substitutions. Relative image assembly and complete recovery remain open.

P4d adds [complete explicit object tables](../contracts/offline-systemb-object-table.md#p4d-explicit-complete-construction-16-october-2026)
without a retained source template; missing descriptors are refused, not padded.
This does not resolve product-number/ASAP mapping, generated table location,
source substitution or durable recovery. No hardware admission is widened.

## Normalized simulation boundary (16 October 2026 source)

[The P3a normalized property-plan simulator](../contracts/offline-property-plan-simulation.md)
executes a validated synthetic sequence using the existing management session.
This is not the missing ProductDB source translator: Lsm/resource admission,
modern XML SegFlags, real-source16-bit encoding and restart remain unresolved or
unimplemented. A mutation inventory is not a durable backup. Complete direct and
indirect property/memory/setup recovery and hardware validation remain open;
public0025 programming stays refused.

## Offline source-image boundary (15 October 2026 source)

[System2 source-image assembly](../contracts/offline-download-system2-image.md)
now has an explicit inference-bearing library path and indexed legacy association
codec. This is not full System2 support: the real Siemens witness reaches an
uncorroborated16-bit parameter encoding and remains refused. No public download,
property/memory recovery or hardware verification promotion follows. Successful
synthetic assembly and a pinned real-source refusal are different evidence.

## Offline codec prerequisite (15 October 2026 source)

[Pure legacy table/property codecs and System2 tool metadata](../contracts/offline-download-codecs.md)
are implemented with offline regression evidence. They do not build a complete
System2 image or executable plan, define all property/memory recovery, or change
public mask admission/field verification. Broader commissioning remains open.

## Session device comparison (P4 source after Alpha 11)

Tags are volatile observations, not monitored device state. Relevant project
edits invalidate them through P3 revisions. Decoding needs verified table
profiles and actual active image writes; incomplete, overwritten, unverified
signed-negative or unsupported-encoding fields remain Unknown with a reason,
beside all raw ranges. No new image/device admission or universal ETS
compatibility is claimed. Cancel drains the current read. The inherited global
achievement toast can transiently overlap lower mobile modal content; local
scrolling keeps values and close controls reachable.
[Comparison contract](../contracts/device-comparison.md).

## Universal reporting boundary (15 October 2026 source)

The [universal support reporter](../contracts/universal-device-support.md)
collects structured offline evidence even for wholly unknown devices and offers
only reviewed, separately consented read-only diagnostics. It does not unlock a
download, assign Verified status, provide arbitrary memory/property exploration
or implement System-2 image/table/LSM/recovery contracts. Mask0025 status and
mask0012 bit0 recipes are source/spec/simulator evidence, not hardware support;
0020/0021/0701 and other families stay descriptor-only here. Source projection is
bounded and rights-gated; original firmware/binaries/full XML/spec pages are not
attachments. Unknown redistribution rights remain a reason to withhold content,
including email. A whole-session timeout can yield an explicitly unavailable
acquisition. The reporter's8MiB source and cumulative declared Text-buffer
assessment budgets can leave support unknown despite some available catalogue
facts; they do not impose a new device-programming limit or guarantee whole-process
resource use. Safe bounded context remains available when assessment is omitted. [ADR0131](../adr/0131-universal-device-support.md).

<!-- BEGIN retained catalogue record -->

<a id="7-commissioning-and-device-download-are-required-but-blocked"></a>
## 7. Commissioning: a verified `070nh` memory path, not general device support

**Current public address-write boundary:** confirmed K6/serial/K13 address
operations fail before a tunnel until action- and device-specific durable
recovery exists (ADRs 0057–0059). The earlier address-change/reset experiments
below are dated evidence, not current permission or a working public write.
Memory downloads retain their separately evidenced, bounded backup scope.

**Scope (ADR-0048/0049, verified 2026-09-29/30).** `knx device download`
and the Web download tab, plus individual-address programming, are real
product commands. They require the identified target, a plan, a
device-specific confirmation phrase and exclusive gateway access. Complete
and all three partial download scopes were exercised on one MDT `1.1.67`
(`0701h`, `A-0027-15-0BAC`) with read-back; an address change and return,
and an address reset with guarded recovery, were also exercised there.
On 2026-10-09 a complete download, a functional check and a restore from
the pre-write backup ran on the Eibmarkt presence detector `1.1.8` (`0701h`,
`M-006A_A-0001-22-617E-O0079`, RESEARCH §19.19); partial scopes did not run
there. Also on 2026-10-09, MDT fourfold button `1.1.14`
(`M-0083_A-0026-15-3591`) completed a full download, light-function
telegram check and restore (RESEARCH §19.21); its partial scopes remain
untested. A successful byte read-back does **not** confirm the closing Basic
Restart: none of these three devices acknowledged it; the result is explicitly
`RestartOutcome::Unconfirmed`. Other programs, application versions and
masks have not acquired this evidence. The property-based `Downloader`
remains simulator-only. Full traces and the chronological corrections are
in [RESEARCH §19](../RESEARCH.md) and the
[implementation log](../IMPLEMENTATION_STATUS.md), not an open task list here.

**What still limits a download.** The product-database coverage probe could
plan 55 of 181 `0701h`/`0705h` application programs at their defaults from
103 packages; a plan is not a live verification. Other masks and unsupported
procedures are refused by name. `Mask` semantics and parts of the
BIM-M112 memory-procedure interpretation remain inference, identified as
`[A]` in RESEARCH §19.1–§19.3. `LdCtrlMerge`/`MergedProcedure`, unsupported
procedure steps, property-placed/unaligned parameter values, unsupported
parameter encodings and types, ambiguous object priority/ReadOnInit and
other unproved image shapes are refused rather than guessed.
`PID_GROUP_RESPONSER_TABLE`, `A_Key_Write`, some master resets and RF writes
have separate boundaries below (§110, §112, §141, §143–§144).

**Inferences (ADR-0086, 2026-10-07).** Where the specification, product
data and project files are silent but a working solution exists, a download
applies it as a named inference and shows it in readiness, the plan and the
acknowledgement; such a plan is always Untested and never counts as Verified.
Current inferences: `union-later-member` (two active members of one union:
the later one is written) and `machine-5-after-restart` (a machine-5 event
after the final restart is not sent), both RESEARCH §19.12;
`alert-is-urgent` (§19.18). `read-on-init-not-on-070n` discloses a loss
instead: a `070nh` group object cannot read its value after a reset
(Resources NOTE 85), so an enabled `ReadOnInitFlag` is not carried out.
None of these has been exercised by a KNXBench download on hardware.
`Priority="High"` and `TypeFloat` parameters stay refused: the product data
contradicts every reading (§19.18).

**Backup is bounded.** Before a permitted memory download's first mutation,
the executor persists the overwritten memory regions and affected load
states. A complete and parameters-only backup/restore roundtrip ran on
`1.1.67`; failing-backup refusal and group-address-partial restore remain
simulator-only. This is not a full image and cannot recover property values,
keys outside those regions, or arbitrary non-Loaded states. See the
commissioning-readiness section above and [ADR-0049](../adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md).
Since 2026-10-01, a restore also refuses missing, duplicate or extraneous
load-state machine records relative to its download plan, even when the
saved memory-region lengths still match. This is offline-tested validation
of the existing bounded backup, not evidence of complete address-write
recovery or support for previously untested devices.
Returned executor errors now close a still-open device management connection
best-effort (2026-10-01, simulator-tested), preserving the original failure
and existing backup rather than automatically restoring or retrying. This
does not prove remote receipt of disconnect and does not cover a dropped
future, process termination or a power loss during the run.
Server polling now recognises a finished worker with no terminal result as
failed, preserving its progress and backup path. `written:partially` after a
mutation step starts is a conservative unknown outcome, not proof that any
telegram was sent or that device recovery succeeded. An already witnessed
device result survives a later cleanup panic, but does not prove disconnect.
The tunnel reservation remains held while cleanup runs, even if a `join`
wait is cancelled. These offline-tested checks last only for this server
process; there is no new cancel route, automatic recovery or crash journal.

**Live safety and compatibility remain narrow.** `1.1.220` is an excluded
alarm panel. No prior approval carries over to a new target, write scope or
experiment. The gateway allows one tunnel; a refused plan sends nothing.
No general ETS commissioning parity, every-vendor download claim, automatic
rollback or RF hardware support is established by one device.
<!-- END retained catalogue record -->
