# System-2 archive mirrors and tool-only table references — issue #22

Date: 2026-10-14. Research-only continuation after owner pause/resume.
Baseline: published `46c50485`; no production code, mask admission or hardware
operation changed. The [prerequisite contract](../contracts/system2-download-prerequisites.md)
still governs support. `[V]` means directly inspected bytes/source, `[D]` a read
primary document, `[A]` an inference requiring bounded implementation validation.

## 1. Recovered primary artifacts

[V] SourceForge still serves the author's BCU SDK 0.0.5 release. The successful
gzip download is **byte-identical** to the recovered
Wayback copy, rather than merely an HTML page returned with status 200.[1]

| Artifact | Retrieved bytes | SHA-256 |
| --- | ---: | --- |
| `bcusdk_0.0.5.tar.gz` | 913348 | `014cdaafc36f223c584b39ae5d698dd037a8e15aba4e78a2e531b51ff1331304` |
| `sdkdoc-0.0.5.pdf` | 1024437 | `f23df34706bdaed33a0840cd59958e291f6fb791f60fbb089d7d81a99563fada` |
| `Bcu2Help_v12.chm` | 1164422 | `cad215a8018ead73cd3e12bf0fc843366dcd5b79bbb4e8233735160978faaf53` |

[V] The PDF and CHM were recovered from Wayback's binary replay, not the current
TU Wien redirect. The CHM's SHA-1/Base32 matches archive digest
`DCIOIQATYERJP7BCIYYQK5JGJIOVIZ4S`. Its extracted HTML and referenced diagram were
inspected; the PDF is retained as recovered documentation, not as proof of an
algorithm that was not read.[2], [3]

[V] A second direct download also recovered the SDK PDF from SourceForge;
its PDF magic and bytes match the Wayback PDF exactly.[4]

[V] `common/loadimage.cpp` in the SDK admits `0x0012`, `0x0020`, and `0x0021`, not
`0x0025` (lines 93, 136, 465). Its absolute-allocation and TaskCtrl2 builders are
useful independent source witnesses, **not a ready-to-use Siemens driver**.
The SDK was read, not built, installed or connected to a bus.[1]

## 2. The strongest new finding: tools also have control records

[D] In `Bcu2Help_v12.chm`,
`Reference/Load_Control_Macros_Reference/AbsCObjSeg.htm` defines `%AbsCObjSeg` as an
S19 record telling the engineering tool where the communication-object table is.
It expressly says that record is not transmitted over the bus. The example in
`System_Architecture/Load_Procedure/ExampleLoadControls.htm` places it after
Restart. Conversely, `TaskCtrlRec2.htm` describes the pointers announced to the
system. These are different roles; the XML attribute name `LsmIdx` alone does not
prove that every occurrence names an on-device interface object.[2]

[V] The exact public Siemens application `M-0001_A-9205-04-9270` declares
`LdCtrlTaskSegment LsmIdx="5" Address="1024"`. Both its ComObjectTable and
TaskCtrl2 also refer to `0x0400`. The retained master defines interface objects
0–3 for this family; no object 5 is declared there. These are source observations,
not a direct XML-to-S19 normative mapping.

[V] A second, bounded, network-isolated XML scan inspected **2 prior-candidate
packages / 191 direct application XML members**. It found **44 distinct
application/mask/procedure witnesses**, each with exactly the two attributes
`Address,LsmIdx`, no children, and equality to both table pointers:

| Mask | Distinct witnesses |
| --- | ---: |
| `MV-0020` | 3 |
| `MV-0021` | 8 |
| `MV-0025` | 33 |

[V] In 20 witnesses the entry occurs after Disconnect, at the end; another 16
place it after LoadCompleted and before Restart or Disconnect. Eight place it
between TaskCtrl2 and LoadCompleted. The scanner read 261932531 bytes, refused
30 members above its 8 MiB/ratio limits, and had a 512 MiB aggregate budget.
Thus **44 is a scoped result, not all programs in the corpus**. The earlier
broader-limit exploratory result of 47 is not promoted into this count. IDs and
manufacturer-source bytes stay in private evidence, not Git.

[A] These independent witnesses strongly support interpreting the exact
System-2 `TaskSegment/5` shape as **tool-side communication-table metadata**,
not a property write to an invented remote object 5. They do not establish a
normative mapping for every ETS XML construct. Never drop every `LsmIdx=5` step:
a real Load/Unload/WriteProp on object 5, additional attributes, nested content,
pointer disagreement or another family must remain preserved and refused.

## 3. Facts that narrow implementation work, not support claims

- [D] `03_05_01 Resources v01.10.01`, §4.3.28, printed/PDF pp. 78–79 defines
  PID_HARDWARE_TYPE as one six-octet element. [V] Siemens' comparison literal is
  ten octets with a zero tail; the retained master also specifies length six.
  `crates/knx-core/src/commissioning/memory_download.rs::property_matches` already checks
  equality plus zero padding, but accepts variable nonempty response lengths.
  A System-2 adapter still needs a **six-octet response-width guard**, not a new
  guessed padding algorithm or unchecked reuse of the helper.
- [D] `03_05_02 Management Procedures v02.01.02`, §3.31.3.4, printed/PDF
  pp. 139–141 defines ten-octet property records, absolute allocation subtype 0,
  task-segment subtype 2 and TaskCtrl2 subtype 5. Per-profile/resource admission
  remains separate from encoding.
- [V] Siemens' base ComObjectTable begins `64 00`, followed by bytes consistent
  with a two-octet header and three-octet descriptors. [A] That would mean 100
  descriptor slots and a one-octet RAM-flags pointer. [D] The historical help's
  ApplicationProgramObject table and Resources §4.18.4 pp. 265–266 describe the
  older structure. [V] Our `0701` writer has a
  three-octet header and **four-octet** descriptors. [A] A separate, source-bound
  System-2 image adapter is indicated; do not reuse the writer or treat a matching
  prefix as complete descriptor/activation proof.
- [V] Siemens declares table capacities 160/200 and memory at `0x3C00`, whereas
  the historical documentation discusses older limits and the retained master
  has a generic 880-byte EEPROM declaration. Do not impose those historical
  limits or treat that declaration as verified hardware capacity for this board.
- [V] All four exact Siemens absolute-allocation records have `SegFlags=0`.
  Do not invent an enabled CRC bit or unconditional extra checksum octet from
  the master feature name `AllocExtraByte`. Its effects require an explicit rule.

## 4. Recovery remains the hard blocker

[D] The help's `LoadProcedure.htm` says allocation depends on existing memory
control blocks and access rights. The diagram
`$images/load_ctrl_behavior_BCU2.gif`, inspected visually, shows application
Unload clearing pointers and allocation ownership as well as load/run state.
A memory-byte dump plus one load-state byte cannot be advertised as a complete
restoration recipe. This historical diagram establishes a side-effect warning,
not the complete private internals of a 0025 device.[2]

[D] Resources §4.23.2.3, printed/PDF p. 294 also defines optional intermediate
Unloading/LoadCompleting states. The state value 5 and the XML `LsmIdx=5` are
unrelated concepts; a planner must not confuse them. Finding the generic
PID_MCB_TABLE definition (§4.2.27 pp. 39–40) does not prove that this System-2
device exposes that property, or that it serializes its complete allocator.

## 5. Next bounded implementation slices

1. **Offline metadata projection:** represent the exact TaskSegment/5 witness as
   retained tool metadata, resolve its table reference, keep source order, and
   assert zero wire steps for it. Add malformed/extra/nested/mismatched-pointer
   controls and prove unrelated object-5 operations still refuse. This is the
   next manageable slice; it must not grant mask admission.
2. **Source-bound image adapter:** preserve the complete original descriptor
   slots, data pointers, selector bits, masks and inactive slots while generating
   parameter/GA/association updates. Validate bounds and byte-level roundtrips
   against the actual Siemens package, not just an invented generic fixture.
3. **Property-control plan:** combine verified target/object mapping, six-octet
   hardware identity, source allocations and preserved data writes, with complete
   order and failure-path tests. No 070nh memory-record conversion.
4. **Durable recovery contract:** enumerate direct and indirect mutations,
   establish readable pre-write evidence and a reconstruction method, bind it to
   device/application identity, and fault-test it before executable admission.
   If evidence is insufficient, keep refusing. Firmware confirmation and a
   separately authorised hardware evaluation follow, never precede, this gate.

No new runtime test, simulator download, live read or programming success is
claimed by this research-only delivery. Existing public refusal stays in force.

## Retained evidence

Licensed KNX page extracts, retrieved binaries, scanner/limits, per-witness rows
and artifact hashes are retained in local private evidence. Public Siemens
source provenance is recorded in the prerequisite contract. No corpus/project
or full manual text is added to the repository.

## Sources

[1] https://downloads.sourceforge.net/project/bcusdk/bcusdk/bcusdk_0.0.5.tar.gz — BCU SDK 0.0.5 — successfully recovered SourceForge archive
[2] https://web.archive.org/web/20221210121239id_/https://www.auto.tuwien.ac.at/~mkoegler/eib/doc/Bcu2Help_v12.chm — BCU2 Help v12 — successfully recovered historical CHM
[3] https://web.archive.org/web/20160418110523id_/http://www.auto.tuwien.ac.at:80/~mkoegler/eib/sdkdoc-0.0.5.pdf — BCU SDK 0.0.5 documentation — recovered historical PDF
[4] https://downloads.sourceforge.net/project/bcusdk/sdkdoc/sdkdoc-0.0.5.pdf — BCU SDK 0.0.5 PDF — successfully recovered SourceForge mirror
