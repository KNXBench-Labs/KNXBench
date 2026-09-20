# RESEARCH.md — Session 0: Technical Research

Status: **complete for Session 0 scope**
Date: 2026-09-02
Method: primary-source inspection of a real ETS4 project plus a live KNX installation, supplemented by public documentation. No application code written in this session.

Every statement below is tagged:

* **[V]** — verified in this repository against real data or installed source code. Reproducible.
* **[D]** — documented by a public, citable source, not verified here.
* **[A]** — assumption or inference. Must be validated before it drives an irreversible design decision.

---

## 1. Evidence base

| Artifact | What it is | Notes |
| --- | --- | --- |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | Real ETS 4.1.8 project, 1.7 MB packed / 22 MB unpacked, 38 archive entries | Not password protected. 36 devices, 514 group addresses, 4 manufacturers. |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` | The **same** installation, re-exported unchanged from ETS 6.3.7959.0, 1.7 MB packed / 22 MB unpacked, 48 archive entries | Not password protected. Schema 23. Same 4 manufacturers, same 514 group addresses. Gives a same-content, cross-schema diff instead of a second independent sample — see §2.4/§3.3. |
| `KV v2.5 - demo.knxproj` | KNX Association/manufacturer demo project (ETS 5.7), 111 KB packed | Not password protected. Schema 21. A **genuinely independent second installation** — different devices (4), different group addresses (13), different manufacturer (`M-00FA`) — not a re-export of the reference project. Session 7 evidence (2026-09-06); see §2.5/§3.4. Sourced from `OriginalData/DemoProjects/` (gitignored there; committed to the repo root as a test fixture instead, same as the other two `.knxproj` files). |
| `project_dump.json`, `group_addresses.json`, `devices.json` | `xknxproject` 3.10.0 output of the ETS4 project | Used as a *reference implementation baseline*, not as ground truth. |
| `bus_traffic.jsonl` | 280 live telegrams captured from the real bus | Via `monitor_bus.py`, KNXnet/IP tunnelling to gateway `192.0.2.1`. |
| `.venv/` | `xknx` 3.20.0 (MIT), `xknxproject` 3.10.0 (GPL-2.0-only) | Source read directly for format details. |
| KNX Standard v3.0.0, full text | 180 documents (all 10 volumes + Application Notes), local at `/home/knxbench/knx-ai/extracted_clean/` on the dev machine | Licensed KNX Association material — machine-local, not committed to this repo, not portable. Query it via the `knx-spec` Claude Code skill (`~/.claude/skills/knx-spec/`) rather than grepping by hand. Primary source for protocol/DPT correctness questions; cite as `<filename> §<section>`, never quote verbatim into `docs/`. |

All container- and project-level numbers quoted below are reproducible with:

```bash
python3 tools/inspect_knxproj.py "Unser Zuhause ets4 - 2025-12-15.knxproj"
python3 tools/inspect_knxproj.py "Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj"
```

(stdlib only — deliberately no `xknxproject`, see §10.)

This gives one complete, non-trivial, real-world reference installation, exported twice: once from ETS 4.1.8 (schema 11) and once — unchanged, by the same user — from ETS 6.3.7959.0 (schema 23). It is still a **single installation**: the two exports let us diff schema 11 against schema 23 with confidence (same devices, same group addresses, same manufacturers), but they say nothing about schema 12, 13, 14, 20 or 22, and — until Session 7 — nothing about a second, differently-structured project. Sections marked as such must be re-verified against other ETS5/ETS6 projects before being treated as general.

Session 7 (2026-09-06) added a second, genuinely independent sample for schema 21 (§2.5/§3.4) — still leaves schema 12, 13, 14, 20 and 22 unsampled.

---

## 2. `.knxproj` container format

### 2.1 Structure [V]

`.knxproj` is a plain ZIP archive. Observed layout:

```text
knx_master.xml                      # KNX master data (DPTs, manufacturers, mask versions, media)
<M-xxxx>.signature                  # one per referenced manufacturer
<P-xxxx>.signature                  # one for the project
<M-xxxx>/Catalog.xml                # manufacturer product catalog tree
<M-xxxx>/Hardware.xml               # hardware, products, hardware→program mapping
<M-xxxx>/Baggages.xml               # optional, references binary baggage
<M-xxxx>/<M-xxxx>_A-<app>.xml       # application programs (one file each)
<M-xxxx>/Baggages/*.dll             # opaque manufacturer binaries
<P-xxxx>/Project.xml                # project metadata only
<P-xxxx>/0.xml                      # installation 0: topology, buildings, group addresses
<P-xxxx>/BinaryData/<guid>.dat      # opaque per-device blobs
<P-xxxx>/ExtraData/*.azp, *.rbg     # opaque legacy (ETS3-era) plugin data
```

The project identifier is recovered from the `P-*.signature` filename [V]. Signature files contain a base64 blob (RSA signature over the entry content) [V]; the signing scheme is documented by KNX for product data and keyrings [D] and is **not** required to read a project.

Note the filename case difference: ETS4 writes `Project.xml`, ETS5/6 write `project.xml` [V — encoded in `xknxproject/zip/extractor.py`].

### 2.2 Schema versioning [V for 11 and 23, D otherwise]

The schema version is the trailing integer of the default XML namespace, e.g. `http://knx.org/xml/project/11`. It must be read from the file, never assumed.

| Namespace version | ETS generation |
| --- | --- |
| 11 | ETS 4.1 / 4.2 **[V]** |
| 12 | ETS 4 **[D]** |
| 13, 14 | ETS 5 up to 5.6 **[D]** |
| 20 | ETS 5.7 **[D]** |
| 21, 22 | ETS 6.x, early **[D]** |
| 23 | ETS 6.3.7959.0 **[V]** |

`xknxproject` reads the namespace from `knx_master.xml` (first or second line, differing between ETS 4.1 and later) [V]. Reading it from `P-*/0.xml` is equally valid for this sample and avoids a dependency on master-data placement [A].

Official XSDs are **not** published on the public KNX website; they ship with the Manufacturer Tool (MT5/MT6) or via the KNX GitLab account [D]. We therefore cannot validate against an authoritative schema without KNX membership. Consequence: our importer must be **tolerant and inventorying** rather than schema-validating — see §9.

### 2.3 Password protection [D/V for the ETS6 key derivation, A for ZipCrypto — see below]

When protected, the archive contains a nested `<P-xxxx>.zip`.

* Schema < 21 (ETS4/ETS5): standard ZipCrypto, password used as UTF-8
  bytes. **Demoted from [V] to [A] on 2026-09-13.** This section's heading
  used to read "[V — read from `xknxproject` source]" as a single
  section-level marker covering both bullets; splitting the ETS6 bullet
  out to `[D]/[V]` below leaves this one where it always actually stood —
  read from `xknxproject`'s source, never independently verified, and
  never checked against a real protected project. Nothing about the
  ZipCrypto claim itself changed; only its marker got honest.
* Schema >= 21 (ETS6): AES ZIP, password derived as

  ```text
  base64( PBKDF2-HMAC-SHA256(
      password = utf-16-le(user_password),
      salt     = b"21.project.ets.knx.org",
      iterations = 65536,
      dklen    = 32 ) )
  ```

  **[D]/[V], corrected 2026-09-13.** This formula is not an `xknxproject`
  implementation detail — it is the KNX Standard's own specification:
  *The KNX Standard v3.0.0*, *Project Schema23 v01.00.00*, clause 4.2.4
  "Password protection", p.64/64, which also publishes three test
  vectors. `crates/knx-secure::derive_knxproj_zip_password` implements
  this derivation and its tests assert byte-exact agreement with all
  three vectors, the third recovered from a broken PDF text layer by
  rendering and reading the glyphs directly (see the module's own
  comments for the recovery method) — `xknxproject`'s matching
  implementation is now corroborating evidence, not the primary source.
  The formula above was already correct when this section was first
  written from `xknxproject`'s source; what was missing was knowing that
  an independent, citable specification existed to verify it against.

  This closes the *key derivation* only. Our sample is still unprotected,
  so *container decryption* — actually opening the nested, AES-encrypted
  `<P-xxxx>.zip` with the derived password — remains **unverified in
  practice**. Must be tested against a real ETS6 protected project before
  we claim support for opening one.

### 2.4 Container differences, ETS4 (schema 11) vs. ETS6 (schema 23) [V]

Same installation, exported by ETS4 then by ETS6.3.7959.0, diffed archive entry by archive entry:

* **48 entries vs. 38.** ETS6 adds a top-level `P-0512.info` file (new, uninspected — opaque for now) and explicit ZIP directory entries (`M-0008/`, `P-0512/`, …); ETS4's archive has none. Neither affects extraction.
* **`P-0512/Project.xml` → `P-0512/project.xml`.** Filename case changes, confirming the `xknxproject` behaviour already noted in §2.1 — now verified directly rather than read from `xknxproject` source.
* **`ExtraData/*.azp`/`*.rbg` are renumbered** (`20001.azp` → `393255.azp`, etc.). The numbers are ETS-internal and carry no meaning across exports; do not use them as stable identifiers.
* **Application-program filenames carry a different hash suffix for every manufacturer even though the app number is unchanged**, e.g. `M-0083_H-4-1_HP-0019-16-ECA7` (ETS4) vs. `M-0083_H-4-1_HP-0019-16-CA9D` (ETS6.3.7959.0) — same product, same application `0019-16`, different trailing hash. **Consequence: this hash is not a stable cross-export identifier and must not be used as a product-database key.** Match on manufacturer + application number + version instead.
* Manufacturer/project ID (`P-0512`), archive size, group address count (514) and manufacturer set (`M-0008`, `M-000C`, `M-006A`, `M-0083`) are unchanged.

### 2.5 A second independent sample: schema 21 (KV demo project) [V]

Unlike §2.4, this is not a re-export of the reference installation — a different project entirely (`P-03DE`, 4 devices, 13 group addresses, manufacturer `M-00FA`). It closes part of the "single-sample bias" gap (KNOWN_LIMITATIONS §1): schema 21 evidence no longer rests on inference from documentation, only full parsing support still does (see §3.4's closing note).

Container shape matches schema 23's, not schema 11's: `project.xml` lowercase (§2.4), directory entries present, `P-03DE.info` present. Content-model comparison against schema 11 continues in §3.4, after the schema-11 baseline it compares against has been established.

---

## 3. Project content model (`0.xml`)

Full element inventory of the real project, with observed counts and the complete attribute set actually used [V]:

| Element | n | Attributes observed |
| --- | --- | --- |
| `Project` | 1 | `Id` |
| `Installation` | 1 | `InstallationId`, `Name`, `BCUKey`, `DefaultLine`, `IPRoutingMulticastAddress`, `SplitType`, `CompletionStatus` |
| `Area` | 1 | `Id`, `Name`, `Address`, `CompletionStatus` |
| `Line` | 1 | `Id`, `Name`, `Address`, `MediumTypeRefId`, `DomainAddress`, `DomainAddressIsChecked`, `IPRoutingMulticastAddress`, `MulticastTTL`, `CompletionStatus` |
| `BusAccess` | 1 | `Name`, `Edi`, `Parameter` |
| `DeviceInstance` | 36 | `Id`, `Name`, `Description`, `Address`, `ProductRefId`, `Hardware2ProgramRefId`, `LastModified`, `LastDownload`, `CompletionStatus`, `IndividualAddressLoaded`, `ApplicationProgramLoaded`, `ParametersLoaded`, `CommunicationPartLoaded`, `MediumConfigLoaded`, `IsCommunicationObjectVisibilityCalculated`, `Broken` |
| `ParameterInstanceRef` | 1390 | `RefId`, `Value` |
| `ComObjectInstanceRef` | 907 | `RefId`, `IsActive`, `DatapointType`, `Description`, `Text`, `ReadFlag`, `WriteFlag`, `TransmitFlag`, `UpdateFlag`, `CommunicationFlag` |
| `Send` / `Receive` | 569 / 27 | `GroupAddressRefId` |
| `GroupRange` | 35 | `Id`, `Name`, `RangeStart`, `RangeEnd` |
| `GroupAddress` | 514 | `Id`, `Name`, `Address`, `Central`, `Unfiltered` |
| `BuildingPart` | 22 | `Id`, `Name`, `Number`, `Type`, `DefaultLine`, `CompletionStatus` |
| `DeviceInstanceRef` | 29 | `RefId` |
| `BinaryData` | 6 | `Id`, `Name` |

### 3.1 Findings that constrain the domain model

**Topology is not the only device container.** `Topology` contains `Area → Line → DeviceInstance`, *and* a sibling `UnassignedDevices` element. Our project has exactly one unassigned device (`Rolladenaktor J.1`, no `Address`) [V]. A model that assumes every device sits on a line will lose it.

**A device's identity is a triple, not a name.** `ProductRefId` (hardware product) and `Hardware2ProgramRefId` (which application program version is loaded) are separate references, and both are needed to resolve parameters and communication objects. The individual address is *configuration*, not identity — it is absent for unassigned devices.

**"Loaded" state is first-class.** `IndividualAddressLoaded`, `ApplicationProgramLoaded`, `ParametersLoaded`, `CommunicationPartLoaded`, `MediumConfigLoaded`, `LastDownload`, plus `CompletionStatus` (`Undefined` / `Editing` / `FinishedDesign` / `Accepted`, observed: 25× `FinishedDesign`, 11× `Undefined`) describe the delta between the planned project and the physical installation. This is engineering-critical state and must be in the core model, not treated as import metadata.

**Group address links are directional.** `Connectors` holds `Send` and `Receive` children. 569 sending links vs. 27 receiving links in this project. Direction is semantically meaningful (which object writes the GA vs. which listens) and must not be flattened into an undirected "association" set.

**Building structure is a tree with typed nodes.** `BuildingPart/@Type` observed: `Building` (1), `Floor` (3), `Room` (14), `Corridor` (2), `DistributionBoard` (1), `BuildingPart` (1). `BuildingPart` nests recursively and carries `DefaultLine`. Devices are attached by `DeviceInstanceRef`, i.e. **building placement is a many-to-one reference, independent of topology**. Two orthogonal hierarchies over the same device set.

**`Functions` exist in the schema but not in this project** — `xknxproject` reports an empty `functions` map [V]. ETS5+ projects use them. Model design must not assume they are absent [A].

**`BusAccess` stores the commissioning interface** as a free-form connection string:
`Name='GATEWAY-NAME';IpAddr='192.0.2.1';Port='3671';NAT='on';` plus an `Edi` GUID identifying the ETS interface driver [V]. This is ETS-tool-specific configuration. It should be preserved verbatim for round-trip and *not* mapped into our own connection model.

### 3.2 The override chain (most important single finding)

A communication object's effective properties are resolved through **three layers**:

```text
ApplicationProgram/.../ComObject            (defaults: Name, Text, FunctionText,
                                             ObjectSize, Priority, DatapointType, flags)
        ↓ overridden by
ApplicationProgram/.../ComObjectRef         (per-variant override: Text, FunctionText,
                                             ObjectSize, DatapointType, flags, Tag)
        ↓ overridden by
0.xml  ComObjectInstanceRef                 (per-device-instance override: Text,
                                             Description, DatapointType, all 5 flags, IsActive)
```

Frequency of instance-level overrides in this project [V]: `DatapointType` 758×, `Description` 691×, `Text` 121×, `ReadFlag` 39×, `UpdateFlag` 30×, `TransmitFlag` 27×, `WriteFlag` 18×, `CommunicationFlag` 8×.

**Amendment, measured during Session 3 implementation: the 758 `DatapointType` occurrences are not all alike.** 497 of the 758 are the empty string (`DatapointType=""`); only 261 carry an actual value. The empty string is ETS's way of saying "the program's datapoint type is deliberately cleared here" — it is not the same as the attribute being absent (149 of the 907 instances carry no `DatapointType` attribute at all: 497 + 261 + 149 = 907). A model that collapses "present and empty" into "absent" cannot write the file back the way it read it. See [ADR-0010](adr/0010-per-attribute-override-representation.md).

758 of 907 instances carry a `DatapointType` attribute at all (497 empty, 261 valued), leaving 149 with none. **An import that reads only the application program produces wrong data for the large majority of objects.** The core model must represent each layer separately (or store resolved values *plus* the layer they came from), otherwise export cannot reconstruct the original file and edits cannot be attributed.

`ComObjectInstanceRef/@RefId` is a compound key of the form
`M-006A_A-0001-22-26C0-O0079_O-0_R-10001` — application program id + `_O-<ComObject number>` + `_R-<ComObjectRef id>`. It simultaneously identifies the base object and the variant. **This is schema-11-specific — see §3.3.**

**The communication-object number is the `O-<digits>` segment immediately preceding the final `R-<digits>` segment, not the first `O-` token in the string.** The application program identifier that precedes it can itself contain an `O`-looking token — for example `M-006A_A-0001-22-26C0-O0079_O-0_R-10001`, where the *program* id already contains `-O0079` and the true object number is the `0` in the trailing `_O-0_R-10001`. Verified against all 907 `ComObjectInstanceRef/@RefId` values in the reference project during Session 3 implementation (`knx-etsproj`'s `com_object_number`): 0 mismatches, 0 parse failures.

**`ParameterInstanceRef/@RefId` references two distinct parameter kinds**, measured during Session 3 implementation: 1174 plain parameters (`..._P-<n>_R-<n>`) and 216 union parameters (`..._UP-<n>_R-<n>`), 1390 total. A parser that matches only the `_P-` form silently drops the 216 union ones — 15.5% of the parameter values in this project.

### 3.3 Content-model differences, ETS4 (schema 11) vs. ETS6 (schema 23) [V]

Same diff as §2.4, one level down, on `0.xml` itself. These are real, load-bearing format changes, not noise:

* **`RefId` is no longer self-contained.** Schema 11 writes the full compound key (`M-0083_A-0019-16-ECA7_O-59_R-149`); schema 23 writes only the local part (`O-59_R-149`). The application-program identity must now be recovered from the owning `DeviceInstance/@Hardware2ProgramRefId`, not from the reference string itself. A parser that treats `RefId` as globally unique and self-describing (reasonable for schema 11) breaks silently on schema 23.
* **Group address links move from child elements to an attribute, and gain a device-wide index.** Schema 11: `ComObjectInstanceRef` nests `Connectors/Send` and `Connectors/Receive` elements, each with `@GroupAddressRefId` (fully qualified, `P-0512-0_GA-373`). Schema 23: the link is a `Links` attribute directly on `ComObjectInstanceRef` (short id, `GA-373`; presumably space-separated for multiple links, unverified here since every observed instance has exactly one), **and** every `DeviceInstance` gains a `GroupObjectTree/@GroupObjectInstances` attribute — a space-separated list of every communication-object `RefId` on that device, including ones with no override and no link. Send vs. Receive direction is no longer encoded positionally; it must come from the application program's `ComObject/@ReadFlag`/`WriteFlag` defaults (or the instance-level flag overrides), which schema 23 still carries as before.
* **`ComObjectInstanceRef` elements are omitted when they would carry no information.** Count drops from 907 to 691 for the same 35 devices: objects with neither an override nor a link are no longer written as elements at all — they exist only as an id in `GroupObjectTree/@GroupObjectInstances`. **An importer that equates "device's communication objects" with "device's `ComObjectInstanceRef` elements" will undercount by ~24% on schema 23.** `GroupObjectTree` must be read as the authoritative object list; `ComObjectInstanceRefs` as the overrides/links on top of it.
* **Booleans switch from `"1"`/`"0"` to `"true"`/`"false"`** across the board (`ApplicationProgramLoaded`, `IndividualAddressLoaded`, `Central`, `Unfiltered`, …). Verified on `GroupAddress/@Central` and `@Unfiltered`: the values are unchanged (2 group addresses carry both flags in both exports), only the literal spelling changed. A parser that string-matches `"1"` instead of parsing as boolean will misread every flag on schema 23 as false.
* **Amendment, found during Session 3 implementation (Task 14): a third boolean spelling exists *within* schema 11 itself, not just across the 11→23 boundary.** `ComObjectInstanceRef`'s five flag attributes — `ReadFlag`, `WriteFlag`, `TransmitFlag`, `UpdateFlag`, `CommunicationFlag` — write `"Enabled"`/`"Disabled"` on schema 11, not `"1"`/`"0"`. Verified against the full reference project: every one of these five attributes is exclusively `"Enabled"` or `"Disabled"`, while every *other* schema-11 boolean attribute checked (`IsActive`, `Central`, `Unfiltered`, `Broken`, the four `*Loaded` flags, `IsCommunicationObjectVisibilityCalculated`, `DomainAddressIsChecked`) is exclusively `"1"`/`"0"` as expected. A boolean converter scoped to "schema version" rather than "schema version and attribute" will misread every communication-object flag on schema 11. Not yet verified whether schema 23 keeps `"Enabled"`/`"Disabled"` for these same five attributes or switches them to `"true"`/`"false"` like everything else — check against the ETS6 reference project before relying on either assumption.
* **A `Security` child element appears on every `DeviceInstance`** (`SequenceNumber`, `SequenceNumberTimestamp`), present even for devices with no KNX Secure configuration (empty/absent values). New in schema 23; absent in schema 11. See §9 for KNX Secure generally.
* **A `Puid` attribute appears on nearly every element** (`Area`, `Line`, `Segment`, `GroupRange`, `GroupAddress`, `Space`, `DeviceInstance`, …) — a small integer, project-scoped, stable-looking persistent id distinct from the existing `Id` (GUID-shaped string) and `Address`. Purpose not yet determined; carried through unopened for now.
* **Topology gains an explicit `Segment` level and synthetic scaffolding.** Schema 11: `Area → Line → DeviceInstance` directly, `Line` itself carries `MediumTypeRefId`. Schema 23: `Area → Line → Segment → DeviceInstance`, with `MediumTypeRefId` moved onto `Segment`. ETS6 also writes topology nodes that do not correspond to any device: an extra `Line` named "Main line" (`Address="0"`) inside the existing area, and a second `Area` named "Backbone area" containing a "Backbone line" — both empty, both IP-backbone scaffolding that ETS6 always emits regardless of project content. **Do not interpret these as real topology added by the user.**
* **`BuildingPart` is renamed `Space`, `Buildings` is renamed `Locations`.** Same tree shape, same `@Type` values, same 22 nodes (14 Room, 3 Floor, 2 Corridor, 1 Building, 1 DistributionBoard, 1 BuildingPart) — confirmed by count, not just by name.
* **One orphaned device silently disappears.** The schema-11 export has 36 `DeviceInstance` elements: 35 addressed devices plus one duplicate-looking entry (`Rolladenaktor J.1`, no `Address`, same `ProductRefId`/`Hardware2ProgramRefId` as the real addressed device) sitting in `UnassignedDevices` — almost certainly a leftover from a hardware swap that ETS4 never garbage-collected. The schema-23 export has exactly 35 `DeviceInstance` elements and no `UnassignedDevices` section at all: the orphan is gone, the 35 real devices are unchanged (same names, addresses, product refs). **Re-exporting a project from a newer ETS version can silently drop orphaned/unassigned device stubs.** This is arguably correct behaviour, but it means byte-for-byte roundtrip cannot be a goal across ETS versions, and an ETS4→ETS6 migration report should call this out explicitly rather than treat it as data loss.

### 3.4 Content-model differences, schema 11 vs. schema 21 (KV demo) [V]

Measured by walking both `0.xml`/`project.xml` trees and diffing element paths + attribute sets (`ElementTree`, namespace-stripped). Headline finding: **every structural delta §3.3 attributed to schema 23 is already present at schema 21** — schema 23 did not introduce them, it inherited them from whatever schema first introduced ETS5's project model. This sample cannot say which of 12/13/14/20 was first; it only proves 21 already has them.

* `Segment` level between `Line` and `DeviceInstance` — present, same shape as §3.3's schema-23 description.
* `GroupObjectTree` — present per `DeviceInstance`, alongside `ComObjectInstanceRefs` (both coexist, as in schema 23), **but its internal shape differs from schema 23's.** Schema 23 (§3.3): a flat `GroupObjectTree/@GroupObjectInstances` attribute. Schema 21 (this sample): `GroupObjectTree/Nodes/Node[@Type='Channel']/@GroupObjectInstances` — one `Node` per module channel, each listing that channel's live communication-object `RefId`s space-separated. Both shapes serve the same role: verified on all 4 KV devices that every `ComObjectInstanceRef/@RefId` is a member of `GroupObjectTree`'s id set with zero exceptions, while `GroupObjectTree` carries substantially more ids (e.g. one device: 5 `ComObjectInstanceRef`s against 26 `GroupObjectTree` ids) — the same undercount §3.3 measured for schema 23 (24%), now confirmed on independent data and worse for module-heavy devices. See [ADR-0014](adr/0014-group-object-tree-authoritative-source.md).
* **`ComObjectInstanceRef` carries almost no overrides for module-based devices.** Measured across every `ComObjectInstanceRef` in the sample: only `RefId`, `ChannelId`, `Links` ever appear — never `DatapointType`, `Text`, `Description`, or a flag override. `ChannelId` (new, not in schema 11 or §3.3's schema-23 diff) points at the `GroupObjectTree/Nodes/Node` grouping it belongs to. DPT/Text for these objects must be resolved through the module chain (below), not the instance layer.
* **`ModuleInstance` → `ModuleDef` resolution chain**, measured against the KV project's `M-00FA` application programs: `DeviceInstance/ModuleInstances/ModuleInstance/@RefId` (local id, e.g. `MD-2_M-1`) resolves via the owning `DeviceInstance/@Hardware2ProgramRefId` to `ApplicationProgram/ModuleDefs/ModuleDef/@Id` (e.g. `M-00FA_A-2504-10-C071_MD-2`) — the same short-id-recovered-via-owning-element pattern §3.3 already established for schema-23 `ComObjectInstanceRef/@RefId`, now shown to apply to `ModuleInstance/@RefId` too. `ModuleDef` internally repeats the existing `ApplicationProgram` shape one level deeper: its own `Static/ComObjects` + `ComObjectRefs` (identical attribute set to the top-level ones already modelled by `knx-productdb`), plus a `Dynamic/Channel/ParameterBlock/choose/when` tree (the same grammar covered by §4.3: its `when/@test` value grammar is now documented from the KNX Standard, while the surrounding structural grammar remains corpus-observed only) that picks which `ComObjectRef`s are active for given argument values. `ModuleInstance/Arguments/Argument/@RefId` supplies those argument values per instance (e.g. `argCH=1` — which channel number this repetition represents). Import does not need to evaluate `choose`/`when` itself: `GroupObjectTree` already carries ETS's own evaluation of it (previous bullet), so the active-object set is read, not recomputed.
* `Puid` — present (`Area`, `Line`, `GroupRange`, `GroupAddress`, …). **Corrects `known.rs`'s existing test comment, which called `Puid` "schema 23 only" — it is at least schema-21-and-up.**
* `Locations`/`Space` — present, replacing `Buildings`/`BuildingPart` already at schema 21.
* `GroupAddress` gained a `DatapointType` attribute directly (not present at schema 11) — potentially resolves the ambiguous space-separated `DatapointType` list problem (§4.2/KNOWN_LIMITATIONS §12) for schema ≥ 21 specifically, since the group address itself states its type rather than requiring inference from a linked communication object. Unverified whether it is ever ambiguous/multi-valued here — this sample's 13 group addresses all carry single, unambiguous values.
* `ProjectInformation` gains `Comment`, `Guid`, `LastUsedPuid`, `ProjectType`; loses `CompletionStatus`, `Hide16BitGroupsFromLegacyPlugins`, `ProjectId`, `ProjectTracingLevel` (relative to schema 11 — not cross-checked against schema 23 here).
* `ProjectTraces`/`ProjectTrace` (new) — an audit-log-shaped element, not seen in schema 11 or documented in §3.3's schema-23 diff. Purpose not investigated.
* **New, not seen in §3.3 at all:** `ModuleInstances`/`Arguments` per `DeviceInstance` — modular application programs, where a device's configuration is composed from reusable modules with their own argument sets rather than one monolithic application program. This is a real domain-model gap (absent from [DATA_MODEL.md](DATA_MODEL.md)), not just a parser detail.
* `Security` element per `DeviceInstance` — present, matching §3.3.
* `Line`/`Area` lose `CompletionStatus` (and `Line` loses `Name`) relative to schema 11 in this sample — possibly because this demo project was never "downloaded" to real devices, not necessarily a schema-wide change; needs a second schema-21+ sample with real completion-status history to confirm.

**Session 7 (2026-09-06) amendment, brainstorming pass following cycle 1:** the domain-model decision this section called for is now made — [ADR-0013](adr/0013-module-instance-representation.md) (`ModuleInstance` as a first-class entity) and [ADR-0014](adr/0014-group-object-tree-authoritative-source.md) (`GroupObjectTree` as the authoritative object-list rule, schema-version-generic). Design: [`docs/superpowers/specs/2026-09-06-schema-21-23-import-support-design.md`](superpowers/specs/2026-09-06-schema-21-23-import-support-design.md). Building `known_schema(21)`/`(23)` and wiring the mapper is the implementation that design hands off to — tracked in [ROADMAP.md](ROADMAP.md) and [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) §1.

---

## 4. Manufacturer data model

Verified against `M-006A` (small) and `M-0083` (2.4 MB application program) [V].

```text
Manufacturer (M-xxxx)
├── Catalog
│   └── CatalogSection (recursive)
│       └── CatalogItem  → ProductRefId, Hardware2ProgramRefId
├── Hardware
│   └── Hardware  (SerialNumber, VersionNumber, BusCurrent, IsCoupler,
│       │          IsPowerSupply, IsIPEnabled, HasIndividualAddress,
│       │          OriginalManufacturer, …)
│       ├── Products
│       │   └── Product (OrderNumber, IsRailMounted, WidthInMillimeter,
│       │                DefaultLanguage, Hash, RegistrationInfo)
│       └── Hardware2Programs
│           └── Hardware2Program (MediumTypes, Hash)
│               ├── ApplicationProgramRef → ApplicationProgram Id
│               └── RegistrationInfo (RegistrationNumber, RegistrationStatus,
│                                     RegistrationSignature)
├── ApplicationPrograms
│   └── ApplicationProgram (ApplicationNumber, ApplicationVersion, MaskVersion,
│       │                   ProgramType, PeiType, LoadProcedureStyle, …)
│       ├── Static
│       │   ├── Code → AbsoluteSegment (Address, Size, Data, Mask)
│       │   ├── ParameterTypes → ParameterType
│       │   │     (TypeNumber | TypeRestriction+Enumeration | TypeText | TypeNone | …)
│       │   ├── Parameters → Parameter / Union → Memory (CodeSegment, Offset, BitOffset)
│       │   ├── ParameterRefs → ParameterRef
│       │   ├── ComObjectTable → ComObject
│       │   ├── ComObjectRefs → ComObjectRef
│       │   ├── AddressTable / AssociationTable (CodeSegment, Offset, MaxEntries)
│       │   ├── LoadProcedures → LoadProcedure → LdCtrl*
│       │   └── Options (≈25 Legacy* compatibility flags)
│       └── Dynamic
│           └── Channel | choose | Module | ChannelIndependentBlock (§4.3)
│               └── ParameterBlock → choose/when → ParameterRefRef / ComObjectRefRef / …
└── Languages → Language → TranslationUnit → TranslationElement → Translation
```

### 4.1 Findings

**The `Dynamic` tree is a conditional UI/visibility program, not a flat list.** `choose`/`when` nodes keyed on `ParamRefId` decide which parameters and which communication objects are visible and active for a given parameter configuration. Scale in one real device: 1211 `Parameter`, 2236 `ParameterRef`, 767 `ComObjectRef`, 526 `choose`, 1282 `when` [V]. Rendering a device editor faithfully means **evaluating this tree**, which is the single largest piece of work in an ETS alternative. `test` expressions on `when` were the subject of the Session 4 R3 spike (2026-09-11) — see §4.3.

**Parameter values live in memory layout, not in a property bag.** `Parameter/Memory` gives `CodeSegment`, `Offset`, `BitOffset`; `Union` packs several parameters into shared bits (104 `Union` elements in one program). Parameter *values* are stored per device in `0.xml` as `ParameterInstanceRef/@Value` (1390 in our project). Correct interpretation requires the `ParameterType` from the application program. This is also exactly what a device download must serialize into `AbsoluteSegment` memory images.

**Translations are a side table, not inline text.** 5919 `Translation` elements in one application program. Every visible string may be language-dependent, resolved by `RefId` + `AttributeName`. The domain model needs a language-aware string resolution layer from the start; retrofitting it later is expensive.

**`Options` carries ~25 `Legacy*` behavior flags** (`LegacyNoPartialDownload`, `ParameterByteOrder`, `TextParameterEncoding`, …). These change programming semantics per application. They are opaque to us for now; **preserve verbatim** and treat as a hard blocker signal for any download implementation.

**Application programs are large.** Single file up to 5.7 MB in this project; total unpacked 22 MB for 12 distinct application programs. `xknxproject` requires `lxml` for exactly this reason [D]. Streaming/indexed parsing and a persistent product-database cache are a requirement, not an optimization.

### 4.2 `ComObjectRef/@DatapointType` can be a space-separated list [V]

Found while implementing `DptRef` parsing (Session 2). Direct raw-XML
inspection of `M-0083/M-0083_A-0019-13-A892.xml` shows a `ComObjectRef`
whose `@DatapointType` attribute holds two space-separated DPT references
rather than one, e.g. `"DPST-9-21 DPST-9-21"`. `knx_master.xml`'s own
schema documentation does not call this out, and `xknxproject` does not
appear to special-case it either. Read as a list of *acceptable*
alternative datapoint types for that communication object, not as a typo.

`knx-core`'s `DptRef` parses a single `DPST-<main>-<sub>` string and does
not yet handle this case (DATA_MODEL §9). Resolving which alternative
applies — and whether it ever varies within one list — needs more samples
than this project provides, so it is deferred to `knx-productdb`
(Session 4), which owns DPT compatibility resolution generally.

### 4.3 The `when/@test` grammar and `Dynamic`-tree evaluation semantics — R3 spike (Session 4, 2026-09-11)

**Risk R3 (§11) is answered.** At the time of this spike, parameter
interpretation itself did not exist yet — no evaluator, no editor. A
headless evaluator over the stored tree was built afterward (T18 slice 1,
2026-09-11); no editor exists still. See
[KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md). What follows is the
research; the implementation is future work (T18,
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)). Full spike report:
`.ai/logs/2026-09-11_claude_r3_dynamic_grammar.md`; this section is the
durable summary that survives outside that log.

Corpus: 34 `ApplicationProgram` elements across 7 archives — 4 `.knxprod`
product databases and 3 `.knxproj` demo/reference projects, all under
`OriginalData/` — 22630 `when` elements and 12149 `choose` elements,
independently cross-checked against raw `grep -o` counts on the source
XML, not just parser output.

Three confidence levels are kept apart throughout, as in the rest of this
document: **[D]** what the Standard states, **[V]** what the corpus shows
(reproducible, but from a 34-application-program, 4-manufacturer sample),
**[A]** inference beyond both.

**[D] The Standard specifies the `@test` value grammar.**
`Project Schema23 v01.00.00.md` §1.1.3.18, simpleType `Condition_t` — the
type of `When_t/@test` — is normative (re-read directly against
`Project Schema23 v01.00.00.json#/tables/82` for this section, not merely
quoted from the spike report). It gives three alternatives: a single
number (`number`); a space-separated list of numbers
(`number (⎵number)*`); and a comparison expression (`op number`, where
`op` is one of `= != > < >= <=`, with `<`/`>` written `&lt;`/`&gt;` in
XML attributes). It states explicitly that **the controlling parameter
must be of type `TypeNumber` or `TypeRestriction`**, and that for
`TypeRestriction` the comparison uses the matching `Enumeration/@Value`,
never the `Parameter`'s own raw value. §1.1.3.19 (`Value_t`), immediately
following, separately documents how the numeric literals themselves are
encoded (e.g. `TypeFloat` as C#'s `"E15"` scientific notation) — relevant
to parsing `@test` literals correctly, but a distinct simpleType.

The Standard does **not** define the surrounding structural grammar —
`Dynamic`, `Channel`, `ParameterBlock`, `choose`, `When_t` itself, or
`ChannelIndependentBlock` (below) — anywhere in this repository's KNX
Standard v3.0.0 extraction. `Project Schema23` is the only
project/application-program schema document present there; it documents
the shared simpleTypes and the *Project*-instance schema (installations,
topology, group addresses, …), not `ApplicationProgram`'s own
complexTypes. Everything below the `@test` value grammar is therefore
corpus-observed, or drawn from the KNX Association's Manufacturer Tool
(MT4) cookbook (`02 Volume 2 Cookbook/02_04_01 Manufacturer Tool
v01.00.01.md`, tooling documentation, not schema documentation) — never
Standard-normative. This refines §4.1's "single largest piece of work"
framing: the *value* grammar of `@test` turned out smaller and
better-specified than feared; the *structural* grammar around it remains
genuinely open.

**[V] Four `@test` shapes, zero unparsed residue.** Every one of the
22630 `when` elements in the corpus classifies cleanly into one of:

| Shape | Count | Note |
| --- | --- | --- |
| `SINGLE_INTEGER` | 19138 | e.g. `test="3"` |
| `DEFAULT_ATTR(true)` | 3417 | `<when default="true">`, no `@test` at all — not part of `Condition_t`, see below |
| `SPACE_LIST_OF_INTEGERS` | 62 | e.g. `test="1 2"`; observed lists are length 2-3 only; **[V]** all 62, like `OP_NUMBER` below, occur in `prod3` alone (independently confirmed by T18 slice 1's corpus test, 2026-09-11) |
| `OP_NUMBER(>)` | 13 | of the six operators `Condition_t` allows, only `>` was ever observed, all 13 in one `prod3` application program |

**[V] The `choose` → `ParameterRef` → `Parameter` → `ParameterType`
resolution chain is unambiguous and 100% resolvable.** All 12149
`choose` elements resolve `@ParamRefId` successfully (0 dangling
references anywhere in the corpus, including a cross-check against
`ParameterRefRef`/`ComObjectRefRef`). Controlling-parameter type
distribution: `TypeRestriction` 11464 (94.3%), `TypeNone` 604 (5.0%),
`TypeNumber` 81 (0.7%). For `TypeRestriction`, 19132 of the 19138
`SINGLE_INTEGER` `when`s independently match a real `Enumeration/@Value`
of the resolved type; the small remainder are the `TypeNumber`-controlled
ones, which have no enumeration to match against.

**[A, corpus-consistent] `@default="true"` is the fallback branch
selector.** It never co-occurs with `@test` on the same `when` (0/22630).
No `choose` has more than one default `when` (0/12149); no `choose` has
two `when` children with an identical `@test` value (0 duplicates in
12149 `choose`); 3417 `choose` elements mix ordinary `test`-`when`
siblings with one trailing default `when` — the overwhelmingly normal
case, not an edge case. This is strong, consistent evidence for "first
(and only) matching test wins, default covers the rest" — but it remains
an **inference** from consistency: the Standard is silent on `@default`
altogether, and no source consulted states the matching algorithm itself
(e.g. whether two simultaneously-true `@test`s on sibling `when`s would
be a validation error was never observed, but its absence could equally
be an artifact of these particular sample programs).

**Three findings that matter for T18's design, in order of how much they
should shape it:**

1. **`TypeNone`-controlled `choose` contradicts `Condition_t`'s own
   stated constraint.** 604 of 12149 `choose` elements are controlled by
   a `TypeNone` parameter — neither `TypeNumber` nor `TypeRestriction`.
   All 604, with no exception, have exactly one `when default="true"`
   child (604/604; confirmed against a concrete example,
   `M-0008_A-C004-03-7AB2-O000A_PT-dummy`, a `ParameterType` whose sole
   child is `TypeNone`, referenced by a `Parameter` with `@Access="None"`
   and empty `@Value`). This is a real ETS/MT4 tooling idiom — an
   always-true, single-branch "dummy" wrapper used to group a fixed block
   of content structurally, with no actual conditional gating — that a
   literal reading of the Standard's text has no defined behaviour for.
   **[A]** An evaluator can safely treat it as "always take the sole
   default branch", but this is inferred from 604/604 consistency, not
   documented anywhere consulted.
2. **"No branch matches" is a common, reachable state, not a corner
   case.** Of the 8732 `choose` elements with no default `when` at all,
   5570 (63.8%) are `TypeRestriction`-controlled and have at least one
   legal enumeration value covered by no `@test` — a real parameter value
   for which no `when` branch matches. Neither the Standard nor the MT4
   cookbook states what that means structurally. The natural reading
   ("nothing under this `choose` is active"), consistent with the
   cookbook's own "comparable to if/then" framing, is a plausible **[A]**
   inference, not a stated rule. An evaluator must pick a policy; this is
   too common in this corpus to defer as a corner case.
3. **`ChannelIndependentBlock` was discovered mid-spike** and appears in
   no prior documentation in this repository and no schema extraction
   available here: `<ChannelIndependentBlock>` wraps `ParameterBlock`,
   `choose`, and `Module` children directly under `Dynamic`, outside any
   `Channel`. Observed 5 times total (`kv25` ×1, `prod3` ×3, `prod4` ×1),
   never in the `ez4`/`ez630` samples, no attributes ever seen on it. Its
   late discovery, in only a 34-application-program corpus, is itself
   evidence that the when-child vocabulary below should be read as an
   **observed superset, not a closed grammar**.

**[V] `when`-child vocabulary, by count, across the whole corpus:**
`ParameterRefRef` 33468 (by far the most common), `ComObjectRefRef`
12368, `choose` 7597 (nested — 62.5% of all 12149 `choose` elements are
themselves a `when` child), `ParameterBlock` 2403, `ParameterSeparator`
117, `Module` 89, `Channel` 15, `Assign` 3. The last four appear only in
scheme-20/21 samples (`prod3`, `prod4`, `kv25`) — never as when-children
in the scheme-11/23 (`ez4`/`ez630`) samples — but `ez4`/`ez630` are only
2 of the corpus's 7 archives by element count, so this may be a
sample-size artifact rather than a genuine schema-version cutoff; §6 of
the full spike report names this explicitly as unresolved.

This does refine one existing claim in this repository, precisely: §4.1
above and [DATA_MODEL.md](DATA_MODEL.md) describe "Module-based objects"
as **schema ≥21** — that claim is about *project*-level
`ModuleInstance`/`ModuleDef` composition (`0.xml`'s `DeviceInstance` side)
and was, and remains, observed only in the schema-21 KV project; this
spike sampled no schema-20 *project*, so that claim is untouched. But at
the *application-program* level, `ModuleDef` and the extended `Dynamic`
when-child vocabulary above (`Module`, `Channel`, `Assign`,
`ParameterSeparator`) are already present at scheme **20**
(`prod3`, an MDT product database) — one scheme lower than the only place
this repository had previously observed them. Read "schema ≥21" as
accurate for project-level module composition, and "scheme ≥20" for the
application-program `Dynamic` vocabulary, until a schema-20 *project*
sample closes the gap either way.

**Nesting and evaluation order [V/A].** Maximum observed nesting depth is
10 (`M-000C_A-5701-...`, identical in `ez4` and `ez630`); all depths 1-10
occur, with depth 3 (2074) and depths 6-7 (1920, 1944) the most common.
Of the 7597 nested `choose` elements, 5578 (73.4%) have a controlling
`ParamRefId` that is also a `ParameterRefRef` sibling within the very
same enclosing `when` — "reveal parameter P, then immediately branch on
P's own value" is the dominant nested idiom. **[A]** This is consistent
with single-pass, top-down evaluation being sufficient (a parameter's
controlling relevance never needs a later/forward value in this corpus),
but it is demonstrated only by absence of counter-examples in this
specific sample, not proven in general.

**Other gating mechanisms, checked and inconclusive or absent [V].**
`Access="None"` on `Parameter` and its correlation with a `Memory` child
came back essentially 50/50 (4976 vs. 4878 parameters) — no discernible
rule found. A `Visible` attribute, speculated about as a possible gating
mechanism, was searched for across the entire corpus and never found (0
occurrences on any element) — informative, but its absence in this
sample does not prove it can never appear in unsampled manufacturer data.

**Corpus evidence table:**

| Archive | Format | Schema | AP count | `choose` | `when` | Max depth | Notable |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `ez4` (demo project) | `.knxproj` | 11 | 12 | 4985 | 9670 | 10 | Baseline; 7 distinct application programs |
| `ez630` (demo project) | `.knxproj` | 23 | 12 | 4985 | 9670 | 10 | Byte-identical `choose`/`when` counts to `ez4` for the same 7 underlying application programs |
| `kv25` (demo project) | `.knxproj` | 21 | 4 | 19 | 51 | 3 | Smallest sample; `ChannelIndependentBlock`/`Module`/`Channel` as when-children |
| `prod1` (product DB) | `.knxprod` | 11 | 1 | 1646 | 2252 | 8 | |
| `prod2` (product DB) | `.knxprod` | 11 | 1 | 5 | 5 | 3 | Minimal application program |
| `prod3` (product DB) | `.knxprod` | 20 | 3 | 509 | 982 | 4 | Only archive with the `OP_NUMBER` (`>`) shape and `Assign` when-children |
| `prod4` (product DB) | `.knxprod` | 20 | 1 | 0 | 0 | — | "Dummy_Applikation_Secure" KNX-Secure stub, genuinely empty Static tree — confirmed as a real minimal AP, not a parsing gap |
| **Total** | | 11/20/21/23 | 34 | **12149** | **22630** | 10 | Independently verified against raw `grep` counts on the source XML |

**Sharpest remaining unknowns**, each with what would resolve it (full
detail in the spike report §6):

1. The `Dynamic`/`Channel`/`ParameterBlock`/`choose`/`When_t`/
   `ChannelIndependentBlock` complexType grammar is not backed by any
   normative schema document available to this research — only the
   `Condition_t` value grammar is. Resolvable by obtaining the actual
   `ApplicationProgram.xsd` (or equivalent) from KNX Association, which
   is not part of the current `knx-spec-kb` extraction, or by an
   order-of-magnitude larger, more-manufacturer corpus.
2. The no-match evaluation rule for a no-default `choose` (finding 2
   above) is inferred, not documented. Resolvable by the missing
   complexType schema, if it turns out to specify one, or by direct
   behavioral observation of ETS itself rendering such a dialog — not
   attempted in this read-only, ETS-free spike.
3. `Access`/`Visible` as gating mechanisms independent of `choose`/`when`
   remain open (the `Memory`-child correlation is inconclusive; `Visible`
   was never observed at all). Resolvable by the EEPROM/memory-mapping
   part of the KNX Standard not covered by this spike, or a much larger
   corpus.

**Advisory for T18** (research input, not a design decision made here):
the `@test`/`@default` value grammar and the resolution chain are solid
and simple enough to build a Dynamic-tree evaluator against now, without
further research blocking it. `TypeNone`-controlled `choose` needs its
own explicit code path rather than being forced through generic
`TypeNumber`/`TypeRestriction` comparison logic. The no-default/no-match
case needs an explicit, even conservative (e.g. "hide everything"),
policy decision before shipping, since it is common, not rare. And the
parser should preserve or loudly flag unrecognized `Dynamic`/when-child
element kinds rather than silently drop them (consistent with this
project's existing tolerant-parser posture, ADR-0011) — this spike found
one previously-undocumented construct (`ChannelIndependentBlock`)
partway through itself, which is a reasonable signal that a larger
manufacturer corpus would find more.

---

### 4.4 `Module`/`ModuleDef` expansion semantics — R4 spike (Session 4, 2026-09-11)

T18 slice 1 (§4.3, 2026-09-11) deliberately does not follow a `Module`
node: it evaluates to a `ModuleNotExpanded` diagnostic and its subtree is
not entered. This spike answers how following it — slice 2, which did not
exist yet at spike time — would actually have to work, before any design
is written. Read-only research; no production code changed. **Slice 2
shipped later the same day** ([design](superpowers/specs/2026-09-11-module-expansion-design.md)
D12-D19, [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md)'s T18 slice
2 entry): `Diagnostic::ModuleNotExpanded` no longer exists in the crate,
replaced by `ModuleDefNotFound`/`NestedModuleNotExpanded`. This section
is left exactly as the spike produced it — a record of what was known
before slice 2 was designed, not a live status report; read it as
history alongside §4.3. Full spike report:
`/home/knxbench/.claude/jobs/8098e9e6/tmp/r4-findings.md` (this section is
the durable summary that survives outside that file).

Corpus: the same `OriginalData/` archives as §4.3, re-extracted to
`/home/knxbench/.claude/jobs/8098e9e6/tmp/r4/`. Standard source:
`Project Schema23 v01.00.00.{md,json}` — cross-checked both, as §4.3 also
notes; the `.json` twin's table-grid jumbling around wide tables is a
rowspan/colspan artifact in the source document, not a conversion loss.
Same three-level confidence marking as the rest of this document:
**[D]** the Standard states it, **[V]** the corpus shows it (counts and
method given), **[A]** an inference beyond both, stating what it rests on
and what would falsify it.

**Q1 — how does a `Module` name its `ModuleDef`? [V]** Via `@RefId`,
whose value is the `ModuleDef`'s **full id**, byte-for-byte, never a short
id recovered through an owning element (unlike `ComObjectInstanceRef`'s
schema-23 `RELIDREF` form, §3.3). Example (`prod3`,
`M-0083_A-0317-31-7DC6.xml`):

```xml
<ModuleDef Id="M-0083_A-0317-31-7DC6_MD-1" Name="ModuleDefSwitch">
...
<Module Id="M-0083_A-0317-31-7DC6_MD-1_M-10" RefId="M-0083_A-0317-31-7DC6_MD-1" Name="Channel A">
```

**[V] Never crosses an `ApplicationProgram` boundary.** Every `Module`
element in every module-bearing AP file in the corpus — 102 across 7
files (3 `prod3`, 4 `kv25`) — was checked by comparing its `@RefId`
prefix against its own file's `ApplicationProgram/@Id` prefix: **0 of 102**
cross-boundary. **[A]** A same-`program_id` lookup is therefore a
reasonable design for slice 2's resolver, but it is an inference from a
102-sample corpus — the Standard does not define `ModuleDef`/`Module` as
AP-scoped complexTypes at all (next finding) — falsifiable by a single
sample whose `Module/@RefId` prefix differs from its own AP id.

**Q2 — a `ModuleDef`'s structure. [V], with a stated Standard gap.**
`Project Schema23` documents the *Project*-instance schema and shared
simpleTypes only, the same gap §4.3 already found for `Dynamic`/`choose`.
Grepping both the `.md` and `.json` twin for `ModuleDef`, `Module` and
`ChannelIndependentBlock` as complexType headings finds nothing beyond
`ModuleInstance_t` (project-side, §1.2.5.18) and `ModuleDefArgType_t`
(§1.1.2.38, below) — **there is no [D]-strength Standard definition of
`ModuleDef`'s or `Module`'s own complexType in this extraction.**
Everything about their structure below is [V], not [D].

Observed: `ModuleDef -> ['Id', 'Name']`. `Module -> ['Id',
'InternalDescription', 'Name', 'RefId']`. A `ModuleDef` owns exactly one
`Arguments`, one `Static` and one `Dynamic` child in every sample examined.
`ModuleDef/Static` contains its own `Parameters`/`ComObjects`/
`ParameterRefs`/`ComObjectRefs`, plus constructs not seen in this corpus's
AP-level `Static`: `LParameters`, `RParameters`, `ParameterCalculations`,
`Union`, `Memory`, `LRTransformation`, `RLTransformation` — these look like
a scaled/repeated-instance memory-layout mechanism but were **not**
researched further, flagged not resolved. `ModuleDef/Dynamic` mirrors the
top-level `Dynamic` vocabulary exactly (`Channel`, `ParameterBlock`,
`choose`/`when`, `ComObjectRefRef`, `ParameterRefRef`, `ParameterSeparator`
all observed inside it).

Argument declaration: element `Argument`, child of `Arguments`, attributes
`Id`, `Name`, `Type` (only `Text` seen; absent means numeric — the
implicit default), `Allocates` (bit/byte count). Example (`prod3`,
`M-0083_A-0317-31-7DC6_MD-1`):

```xml
<Arguments>
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-1" Name="ParamOffsBase" Allocates="132" />
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-2" Name="ObjNumberBase" Allocates="20" />
  <Argument Id="M-0083_A-0317-31-7DC6_MD-1_A-3" Name="ChNo" Type="Text" />
</Arguments>
```

**[D]** `Argument/@Type` corresponds to the Standard's `ModuleDefArgType_t`
simpleType (§1.1.2.38), whose facets are `Numeric`, `Text`,
`AllocatorRef`. Only `Numeric` (default) and `Text` are attested in the
corpus; `AllocatorRef` has **zero corpus occurrences** (sharpest unknown
#2, below). `@DefaultValue`/`@AllocatesPerSlot`, named as possibilities in
the spike brief, were **not observed** on any `Argument` — absence, not
proof of nonexistence elsewhere.

**Q3 — how do argument values reach the module's `Dynamic` tree? [V]**
`Module` carries argument values as children — `NumericArg` (`RefId`,
`Value`) and `TextArg` (`Id`, `RefId`, `Value`) — bound 1:1 and
exhaustively to the `ModuleDef`'s declared `Argument`s in every sample
checked (`prod3` `M-0083_A-0317-31-7DC6`: each of its 4 `ModuleDef`s
declares 3 arguments, 2 numeric and 1 text, and the AP holds 44 `Module`
elements in total → 88 `NumericArg` + 44 `TextArg` = 132 bind elements,
matching the raw count. Restricted to `MD-1` alone: 12 `Module`s, 24
`NumericArg`, 12 `TextArg`).
Example:

```xml
<Module Id="M-0083_A-0317-31-7DC6_MD-1_M-10" RefId="M-0083_A-0317-31-7DC6_MD-1" Name="Channel A">
  <NumericArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-1" Value="32" />
  <NumericArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-2" Value="0" />
  <TextArg RefId="M-0083_A-0317-31-7DC6_MD-1_A-3" Id="M-0083_A-0317-31-7DC6_MD-1_M-10_A-3" Value="A" />
</Module>
```

**Central finding: `choose` does not branch on `Argument`, only on
`ParameterRef`.** Every `choose/@ParamRefId` inside `ModuleDef/Dynamic`
(57 distinct values, `prod3` `M-0083_A-0317-31-7DC6_MD-1`) was checked
against that `ModuleDef`'s declared `Argument/@Id` set (3) and
`ParameterRef/@Id` set (208): **57/57** match a `ParameterRef`, **0/57**
match an `Argument`. The same 57/57, 0/57 split reproduces in the archive's
other two application programs' `MD-1` (`A-0318`, `A-0319`).
`ModuleDef/Dynamic`'s
`choose` mechanism is **structurally identical** to the top-level tree's
(§4.3): it branches on the current value of a `ParameterRef` declared in
the module's own `Static`. An argument's role, evidenced separately by
`Memory/@BaseOffset` literally holding the argument's own id (e.g.
`BaseOffset="M-0083_A-0317-31-7DC6_MD-1_A-1"`), is **memory-offset
placement** for numeric arguments and **text-template substitution** for
text arguments (`{{ChNo}}`-style placeholders observed in `Channel/@Text`,
e.g. `Text="Channel {{ChNo}}: {{0}}"` — seen, not exhaustively catalogued;
flagged plausible, not fully verified).

**Reconciliation with T18 slice 1's existing test.** `dynamic_tree.rs`'s
zero-dangling-`choose/@ParamRefId` assertion carries no `module_def_id`
filter — it checks every `choose` row in `dynamic_node` against
`parameter_ref`, and `ModuleDef`-scoped `ParameterRef` rows are already
ingested under the owning AP's `program_id` by slice 1's parser. Since
this spike shows a `ModuleDef`-internal `choose` always targets a
`ModuleDef`-internal `ParameterRef`, never an `Argument`, and those rows
already exist in the table the test checks — **there is no gap, and the
existing test needs no change once Module expansion ships.**

**Q4 — what is repetition? [V]/[D] mixed.** At the AP level, repetition is
purely **N sibling `Module` elements**, each with a distinct `@Id` and the
same `@RefId` — there is **no repeat-count attribute anywhere at the AP
level**, on neither `ModuleDef` nor `Module`. `prod3`
`M-0083_A-0317-31-7DC6_MD-1` is instantiated by exactly 12 separate
`Module` elements, no numeric repeat attribute among them. **Multiplicity
is confirmed within a single AP's own `Dynamic` tree, not only at
`DeviceInstance` level:** every `ModuleDef` in every module-bearing AP file
is instantiated multiple times purely within that file's own tree — e.g.
`prod3`'s `A-0317` file: `MD-1`×12, `MD-2`×12, `MD-3`×12, `MD-4`×8 (44
total), all before any project-side repetition is applied at all.

**`ModuleInstance/@RepeatIndex` is a project-side concept.** **[D]** The
Standard defines `ModuleInstance_t` (§1.2.5.18) with a `RepeatIndex`
attribute described in the abstract as XmlOrder×repeat-counter
information. **[V]** Real KV values for `MD-2`'s 8 `ModuleInstance`s
(`kv25/P-03DE/0.xml`): `"6x1"`, `"10x1"`, `"14x1"`, `"32x1"`, `"36x1"`,
`"40x1"`, `"44x1"`, `"48x1"` — genuinely the two-component `"NxM"` form
[`ModuleInstance::repeat_index`](../crates/knx-core/src/module.rs)
already anticipates as an opaque string (ADR-0013), confirmed directly
against the raw XML for this write-up. In this sample the second
component is always `1`; whether and how it varies is unconfirmed (see
sharpest unknown #1). The first component does not decode into an obvious
formula across the 8 values (differences 4, 4, 18, 4, 4, 4, 4 — not
constant), so the concrete encoding rule stays open. What is established:
`RepeatIndex` is `ModuleInstance_t`-only (project-side), a separate,
simpler mechanism from the AP-level `Module` multiplicity above (sibling
elements, no counter), which any `DeviceInstance`-level repetition layers
on top of.

**Q5 — how do a `ModuleDef`'s internal ids relate to instance-level ids?
[V], 35/35 (100%), two element types, two independent verification
scripts.** Rule: a project-side, module-instance-scoped ref id splices
`_M-<m>_MI-<k>_` into the middle of the `ModuleDef`'s own declared local
ref id, immediately after the `MD-<n>` segment:

```
<owning-AP-id>_MD-<n>_M-<m>_MI-<k>_<local-ref-suffix>
```

where `<owning-AP-id>_MD-<n>_<local-ref-suffix>` is exactly the
`ModuleDef`'s own declared `ParameterRef`/`ComObjectRef` id. Verified on
the KV project: `ParameterInstanceRef` (full AP-prefixed form) 9/9,
`ComObjectInstanceRef` (short, unprefixed form) 26/26 — both independently
re-derived for this write-up directly against
`kv25/P-03DE/0.xml`, matching the spike's counts.

**A real inconsistency worth flagging on its own: the two instance-ref
element types spell the same rule differently in the same file.**
`ParameterInstanceRef/@RefId` uses the full AP-prefixed id form
(`M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1`); `ComObjectInstanceRef/@RefId`
in the same file uses the short, unprefixed form (`MD-2_M-1_MI-1_O-2-0_R-4`
— no leading AP id). A parser/design for slice 2 must handle these as two
distinct id-spelling conventions per element type, not one uniform
spelling. **This is not a hypothetical edge case:** the mangled id
`MD-1_M-2_MI-1_O-2-3_R-4` appears twice in `kv25/P-03DE/0.xml`, once under
`DeviceInstance Id="P-03DE-0_DI-2"` and once under `DeviceInstance
Id="P-03DE-0_DI-3"` — two different physical devices each independently
instantiating the same `ModuleDef`'s `Module MD-1_M-2` as their own first
`ModuleInstance`. The mangled id is scoped to (unique within) its own
`DeviceInstance`, not globally unique across a project — expected
behaviour, not a bug, and directly relevant to Q8(c) below.

**Q6 — can modules nest or recurse? [V] zero in the corpus; [D] a related
but distinct project-side concept exists.** A regex scan for `<Module\b`
(the instantiation element) inside every `ModuleDef/Dynamic` block across
all 7 module-bearing files found **0 occurrences**; `ModuleDef`'s own
child-element vocabulary (Q2) contains no `SubModuleDef`. **[D]** The
Standard *does* name a one-level-deeper nesting concept, but only on the
**project-instance side**: `ModuleInstance_t/@Id`'s documented grammar
(§1.2.5.18) provides for `SubModuleDef`/`SubModule`/`SubModuleInstance`
segments beyond the plain `MD-<n>_M-<m>_MI-<k>` case this corpus exercises.
This spike found **no equivalent AP-side Standard text** for whether a
`ModuleDef` itself can declare a `SubModuleDef` — a genuine open question
(sharpest unknown #3), not an artifact of the corpus being small. **[A]**
Given (a) zero corpus nesting, (b) no Standard-documented AP-side
`SubModuleDef` concept, and (c) the project-side concept is bounded to one
extra level, not unbounded recursion — a defensible slice 2 design
position is: implement Module expansion **one level only**, and treat a
`Module` node encountered *inside* an already-expanded `ModuleDef`'s own
`Dynamic` tree as an error diagnostic rather than recursing. This rests on
the corpus never exhibiting nesting and the Standard never documenting an
AP-side recursive form; it would be falsified by a single corpus file (or
future Standard revision) showing a `Module` inside a `ModuleDef/Dynamic`
block.

**Q7 — distribution. [V]**

| File | ModuleDefs | Module nodes | Args/ModuleDef | NumericArg binds | TextArg binds |
| --- | --- | --- | --- | --- | --- |
| `prod3` `M-0083_A-0317-31-7DC6.xml` | 4 | 44 | 3 each | 88 | 44 |
| `prod3` `M-0083_A-0318-31-DB39.xml` | 4 | 28 | 3 each | 56 | 28 |
| `prod3` `M-0083_A-0319-31-587B.xml` | 4 | 14 | 3 each | 28 | 14 |
| `kv25` `M-00FA_A-2500-10-51CB.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2502-10-8698.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2504-10-C071.xml` | 1 | 8 | 3 | 24 | 0 |
| `kv25` `M-00FA_A-2507-10-0DE5.xml` | 1 | 8 | 3 | 24 | 0 |

Depth: every `ModuleDef/Dynamic` observed is a flat `Channel`/
`ParameterBlock`/`choose`/`when` tree, 3-4 levels, similar shape to the
AP's own top-level `Dynamic`. **Only `prod3` (MDT) and `kv25` (the KV demo
project) exercise `Module`/`ModuleDef` anywhere in the available corpus** —
confirmed zero in `prod1` (4 AP files), `prod2` (Weinzierl 730, 3 AP
files), `prod4` (Dummy_Secure, 3 AP files), and in three further files
scanned directly from `OriginalData/` without extraction: both `Unser
Zuhause` exports and `Weinzierl_730_KNX_IP_Interface_ETS4.knxprod`.
**The sample is narrow: two manufacturers, no independent third source to
cross-validate structural assumptions against.** Any acceptance test slice
2 writes will need its module-bearing fixtures from just these two.

**Q8 — what would change for slice 1's existing evaluator?**

- **(a) No change needed — [V].** `Module/@RefId` is already captured into
  `dynamic_node.ref_id` by the existing generic parser handling.
  `load_tree`, `resolve_control_kind` and `resolve_values` in
  `evaluate.rs` are scoped by `program_id` only, with no `module_def_id`
  filter — once a `ModuleDef`'s own tree rows exist (already true as of
  slice 1), these three functions need no modification to work against a
  `ModuleDef`'s own tree.
- **(b) Real gap, an addition not a behaviour change — [V].**
  `NumericArg`/`TextArg` fall through to generic `UNMODELLED` handling
  today — no entry in `parse.rs`'s `spec_for` table, so `@Value` lives
  only in the free-text `extra` column. Q3's finding — argument values
  drive memory-offset placement and text substitution, not `choose` —
  suggests activation-set computation may not strictly need parsed
  argument values at all; but reporting *which* values were bound, or any
  future memory-layout work, needs structured columns. **No `argument` or
  `module_def` table exists anywhere in `migration.rs` today.**
- **(c) Forced behaviour change, not an addition — [V], a required design
  constraint for slice 2.** Slice 1's `Activation` dedup-by-first-occurrence
  keys on the raw `ref_id` string. Q5 already establishes that a single
  `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are **reused
  verbatim** by every sibling `Module` instantiating it — differentiation
  only exists in the mangled, instance-scoped id, a *project*-side
  (`ModuleInstance`) construct that does not exist at the AP level at all.
  Confirmed directly in `prod3`: `M-0083_A-0317-31-7DC6_MD-1` declares its
  41 `ComObjectRef` ids exactly once, and **12 sibling `Module` elements
  reference it** — walking the `ModuleDef` once per `Module` emits each of
  those 41 ids twelve times over. The KV project shows the same ids
  surviving into a real project: local suffix `O-2-1_R-2` appears under 2
  distinct `ModuleDef`+`Module`+`ModuleInstance` triples (`MD-1_M-1_MI-1_…`
  and `MD-1_M-2_MI-1_…`), and `O-2-0_R-1` under the same 2 — each triple
  then reused by 2 different `DeviceInstance`s (`P-03DE-0_DI-2`,
  `P-03DE-0_DI-3`, different application programs). Two distinct collision
  mechanisms, and only the first one — sibling `Module` instantiation
  within a single application program — is what this finding rests on;
  `DeviceInstance`-level reuse is a project-side concern that
  `knx-productdb` never sees. **If slice 2 walks a `ModuleDef`'s tree once
  per instantiating `Module` sibling and reuses the existing flat
  `HashSet<String>` dedup keyed on raw ModuleDef-local `ref_id`, it will
  incorrectly collapse distinct per-instantiation activations into one**
  (e.g. "Channel A" and "Channel B" instantiating the same `ModuleDef`
  would wrongly report only one activated `ComObjectRef` where two really
  exist). Design implication: any Module-expansion activation must be
  qualified by the instantiating `Module`'s own id before dedup. This is
  marked [V] for the id-collision evidence and **[A]** for the
  consequence-for-slice-2's-code claim, since slice 2 does not exist yet —
  falsified if slice 2's design already qualifies activations this way
  before dedup, which is exactly the fix this finding recommends.
  **Not falsified: slice 2 shipped 2026-09-11 with exactly this
  qualification** (design D14/D18, `ModuleScope::module_node` folded into
  the dedup key as `(Option<module_node>, ref_id)` before any activation
  is recorded). The `[A]` marker stays as written — this is still a
  record of what the spike could infer before slice 2 existed, not
  promoted to `[D]` — but the prediction held.
- **(d) Scope boundary — [A].** Slice 1 (and the slice 2 this spike feeds)
  operates at the `(program_id, module_def_id)` AP level only.
  `ModuleInstance`/`RepeatIndex`/project-side id mangling (Q4, Q5) stay
  outside `knx-productdb`'s scope per [ADR-0014](adr/0014-group-object-tree-authoritative-source.md):
  import never needs to evaluate `Dynamic`/`choose` itself, because
  `GroupObjectTree` already carries ETS's resolved answer for schema ≥21.
  An AP-level Module-expansion evaluator only ever needs to reason about
  `ModuleDef`+`Module`, never `ModuleInstance` — falsified if a future task
  needs `knx-productdb` itself to resolve project-side `ModuleInstance`s, a
  larger scope than T18 slice 2 as currently described.
- **(e) No-match/`TypeNone`/document-order handling — [A], unresearched
  beyond the above.** No evidence of a difference from the top-level
  tree's behaviour was found, but this spike did not specifically
  stress-test `TypeNone`/no-match handling *inside* a `ModuleDef` tree
  against the evaluator's existing code paths. Treat as "no evidence of a
  difference," not "confirmed identical."

**Three sharpest remaining unknowns.**

1. **`RepeatIndex`'s concrete multi-value encoding.** The two-component
   `"NxM"` shape is now confirmed real (Q4), correcting this spike's own
   earlier working notes, which had misread the KV sample as plain
   integers. What the second component (`1` throughout this sample) means
   or when it varies, and what formula produces the first component
   (differences 4, 4, 18, 4, 4, 4, 4 — no obvious pattern), remain open.
   Settled by either a normative worked example in a Standard section not
   yet located, or a hand-built multi-repeat-level fixture.
2. **`AllocatorRef` argument type is completely undemonstrated.**
   `ModuleDefArgType_t` names it as a legal facet [D]; zero `Argument` in
   the corpus uses it, and no corresponding `Value_t` usage was found
   either. Settled by a corpus sample that uses it (none in the available
   `OriginalData/`) or a normative worked example beyond the bare
   enum-facet listing.
3. **Whether an AP-side `ModuleDef` can itself declare a `SubModuleDef`.**
   No Standard text defines `ModuleDef` as an AP-side complexType at all
   (Q2's gap), so there is no [D]-strength answer independent of the
   corpus, and the corpus has zero nesting examples to fall back on.
   Settled by locating an AP-side complexType definition in a Standard
   document not yet checked (an "Application Program Schema" document, if
   one exists under a different filename in the extraction, was not
   specifically searched for), or a corpus sample that exercises nesting.

**Ready-to-design advisory: ready, with named constraints.** The core
mechanism — how a `Module` names, binds arguments to, and should expand
into its `ModuleDef`'s own `Dynamic` tree — is solidly evidenced (Q1, Q2,
Q3, Q5, all [V]-backed with cross-checked counts, several at or near 100%
verification). A slice 2 design can proceed on: resolving `Module/@RefId`
as a full id, same-`program_id` lookup only (Q1); reusing `load_tree`/
`resolve_control_kind`/`resolve_values` unmodified against
`(program_id, module_def_id=<the ModuleDef's id>)` (Q8a); treating a
`ModuleDef`'s internal `choose` exactly like the top-level tree's, no
argument special-casing (Q3). It **must** design an explicit
per-Module-instantiation qualification for activation identity before
walking a `ModuleDef`'s tree once per instantiating sibling, to avoid the
dedup-collision failure mode in Q8c — the one required design decision,
not an optional refinement. It **should** explicitly decide and document a
nesting policy (one-level, reject-if-nested, Q6) even though the corpus
never exercises it, since "recurse until termination" is not defensible on
its own, and no cycle guard exists today. What it does **not** yet support
a design for: parsed (structured) argument *values* beyond activation-set
computation (Q8b — no `argument`/`module_def` tables exist, and whether
slice 2 needs them is an undecided scope question) and the `AllocatorRef`
argument type (unattested, unknown #2). If slice 2's scope is "expand
`Module` nodes to compute the correct active `ParameterRef`/`ComObjectRef`
set," this evidence is sufficient. If its scope also includes reproducing
memory-offset/text-template argument substitution, the
`LParameters`/`RParameters`/`ParameterCalculations`/`Union`/`Memory`
mechanism flagged in Q2 needs its own research first.

**Addendum, T18 slice 3 design revision (2026-09-11) — the project-side id
shape, re-measured against all three demo projects. [V] throughout, corpus
observation, not Standard text.** Q5 above already established the
splicing rule from the KV project alone; this addendum re-derives it
independently against all three `OriginalData/DemoProjects/` archives —
each one a zip, unpacked read-only to a scratch directory outside the
repository, never into `OriginalData/` — for the parameter-editor design
that consumes it
(`docs/superpowers/specs/2026-09-11-parameter-editor-design.md`,
decisions D21-D25). Recorded here, not only in that dated spec, because a
`docs/superpowers/` spec is a session artefact and this is a durable
format fact — see `.ai/logs/2026-09-11_claude_r5_commissioning_research.md`
for the same ruling applied elsewhere.

An application program declares `<ParameterRef Id="…_MD-2_P-1_R-1">` with
no instantiation segment, while its `Dynamic` declares `<Module
Id="…_MD-2_M-4" RefId="…_MD-2">`; a project then stores
`<ParameterInstanceRef RefId="…_MD-2_M-4_MI-1_P-1_R-1" Value="17"/>` — that
is `Module/@Id` + `_MI-<k>` + `_P-n_R-m`, the same shape Q5 names, spelled
out again here in the parameter editor's own terms.

Counts, reproduced with `grep -o '<ParameterInstanceRef RefId="[^"]*"'
<project>/0.xml` against each `.knxproj` zip's extracted `0.xml`, and with
`grep -rc '<ParameterRef '`/`grep -o '<ParameterRef Id="[^"]*"'` against
every `M-*/M-*_A-*.xml` application-program file embedded in the same
archive (not the project's own `0.xml`) for the declared-id side:

| Project | `ParameterInstanceRef` rows | Module-qualified (`_M-\d+_MI-\d+_`) | Verbatim match to a declared `ParameterRef` id | Union-typed (`_UP-n_R-n`) among them |
| --- | --- | --- | --- | --- |
| KV v2.5 demo | 9 | 9 | 0 | 0 |
| Unser Zuhause ETS 6.3.0 | 1343 | 0 | 1343 (all) | 208 |
| Unser Zuhause ETS 4 | 1390 | 0 | 1390 (all) | 216 |

Union-typed rows (`_UP-n_R-n`) match verbatim like any other row — they
are already counted inside the "verbatim match" column, not a separate
population.

Stripping `_M-\d+_MI-\d+_` → `_` from each of KV's 9 stored ids recovers a
declared `ParameterRef` id for 9 of 9 (verified by direct string
comparison against the declared-id set extracted from
`M-00FA_A-2504-10-C071.xml`/`A-2502-10-8698.xml`/`A-2500-10-51CB.xml`/
`A-2507-10-0DE5.xml`); no declared `ParameterRef` id in any of the three
projects contains that pattern (0/24 KV, 0/18843 for each Unser Zuhause
project's shared declared-id space — the two Unser Zuhause projects embed
the same four manufacturer catalogs, `M-0008`/`M-000C`/`M-006A`/`M-0083`,
so their declared-id counts and content are identical), so the
decomposition has no observed false-positive risk on this corpus.

`_MI-` is `1` in every occurrence found anywhere in the corpus (`grep -oE
'_MI-[0-9]+_'` against all three projects' `0.xml`, deduplicated); what an
index above `1` would mean is **unattested**, stated as such rather than
guessed at.

KV stores five *different* values — 17, 33, 49, 32, 48, for `Module`
instantiations `M-4`/`M-5`/`M-6`/`M-2`/`M-3` respectively — for the one
declared `ParameterRef` `M-00FA_A-2504-10-C071_MD-2_P-1_R-1` across five
`Module` instantiations of `ModuleDef` `M-00FA_A-2504-10-C071_MD-2`.
Per-channel parameter values are real data in our own corpus, not a
hypothetical this design had to imagine storage for.

This addendum only restates the corpus shape; it does not restate the
design decisions built on it — see D21-D25 in the design spec above for
what the parameter editor does with it.

**Addendum, T18 slice 4 design revision (2026-09-12) — the `MI-`/
`@RepeatIndex` relationship, measured, still open. [V] each reading on
its own, [A] the connection between them.** The two facts above were
measured separately (this paragraph's own is that they were never
compared on the same element before). On the very same KV v2.5
`ModuleInstance` element:

```xml
<ModuleInstance Id="MD-2_M-2_MI-1" RefId="MD-2_M-2" RepeatIndex="10x1">
```

the `MI-` token is the literal digit `1`, and `@RepeatIndex` is the
string `"10x1"` — two different strings on the same element, so the
embedded `MI-<k>` is **not** `@RepeatIndex`'s own string. But `MI-<k>`'s
digit is `1` in all 32 `ModuleInstance` elements of this project (line
936 above), and `@RepeatIndex`'s second ("repeat counter") component is
`1` in every sample checked (Q4 above) — so `MI-<k>` is *consistent with*
being that second component specifically, not the attribute's string as
a whole. Nothing in the extraction states this correlation normatively,
and a corpus where both digits happened to agree while being fed by two
genuinely unrelated counters would look identical to this one. **Sharpest
unknown #1 stays open** — this measurement narrows what a future answer
could look like without supplying one. T18 slice 4 (design D35-D43,
specifically D40) is built so that it does not need the answer either
way: a device with two `ModuleInstance`s sharing one `RefId` is refused,
not resolved by guessing which repeat-counter value is "right".

**Addendum (goal.md T18, task 11, 2026-09-14) — Q6 re-measured, D15
superseded.** Q6's "one level only" evaluator policy is superseded by
`docs/superpowers/specs/2026-09-11-module-expansion-design.md`'s D44/D45
addendum: bounded recursive expansion (`MAX_MODULE_NESTING_DEPTH = 16`,
**[A]**) plus ancestor-chain cycle detection, replacing the flat
"refuse-if-nested" policy. Re-measured against the currently installed
corpus, two independent ways, both agreeing on zero:

1. **[V]** A fresh scratch Python scan
   (`xml.etree.ElementTree`, run outside the repo) over every extracted
   application-program XML file, counting `Module` elements found inside a
   `ModuleDef` element's own subtree:
   ```
   646704-04_ETS4_2012_47_DE_EN/M-000C/M-000C_A-5703-10-085F.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   Dummy_Applikation_Secure/M-0008/M-0008_A-9021-21-0CC4-O000A.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0317-31-7DC6.xml: ModuleDef=4 Module=44 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0318-31-DB39.xml: ModuleDef=4 Module=28 nested_Module_inside_ModuleDef=0
   MDT_KP_AMI_AMS_03_Switch_Actuator_V31a/M-0083/M-0083_A-0319-31-587B.xml: ModuleDef=4 Module=14 nested_Module_inside_ModuleDef=0
   Weinzierl_730_KNX_IP_Interface_ETS4/M-00C5/M-00C5_A-0702-10-1B22.xml: ModuleDef=0 Module=0 nested_Module_inside_ModuleDef=0
   TOTAL nested Module elements across all 6 application-program files: 0
   ```
   (`kv25`, referenced in Q6/Q7 above, is not present under this machine's
   `OriginalData/ProductDatabases/` — the four archive files installed here
   (`prod1`/`prod2`/`prod3`/`prod4`, four distinct packages) are the ones
   this ran against; see the Rust corpus test below for the exact archive
   list this measurement actually ran against.)
2. **[V]**
   `crates/knx-productdb/tests/dynamic_tree.rs`'s
   `corpus_nested_module_measurement_task_11` installs every `.knxprod`
   file under `OriginalData/ProductDatabases/` into a fresh database and
   runs `SELECT COUNT(*) FROM dynamic_node WHERE kind = 'Module' AND
   module_def_id != ''` — a `Module` row stored under a non-empty
   `module_def_id`, i.e. found inside a `ModuleDef`'s own tree rather than
   the program's top-level tree. Result: **0**, out of 86 total stored
   `Module` rows (`kind = 'Module'`, any `module_def_id`) across all 5
   installed archive files (four distinct packages — the two
   `Weinzierl_730_KNX_IP_Interface_ETS4` files are byte-identical, so
   `install_package` skips storing the second one's members a second
   time). This test now asserts `total_module_rows == 86` as well as the
   nesting count, so a corpus change that moves either number fails loudly
   instead of only changing an `eprintln!`.

**[D]** A fresh `pdftotext -layout` extraction of `Project Schema23
v01.00.00.pdf` for this task (independent of the extraction Q2/Q6 used)
confirms the same absence again: grepping the extraction's numbered
`complexType`/`element`/`simpleType` headings for "module" finds only
`1.1.2.38 simpleType ModuleDefArgType_t` and the `1.2.5.16`-`1.2.5.20`
`ModuleInstance_t`/`Arguments` family (project-instance side, §4.4 Q6's
`SubModuleDef` grammar) — no AP-side `ModuleDef`/`Module` complexType
definition exists anywhere in this extraction. The bound and the cycle
policy in D44/D45 are therefore inference (**[A]**), not derived from the
Standard — but not from a blank slate either: the only Standard text that
touches module nesting at all is the project-side `ModuleInstance_t/@Id`
grammar §4.4 Q6 already records as **[D]**, and it documents exactly one
extra level (a `SubModule` segment), not unbounded recursion. That text
is project-side, not AP-side, so it does not settle `16`; it is the one
documented neighbour `16` is chosen deliberately far above, not a source
this addendum had nothing to derive from.

**Conclusion: zero products in the installed database actually nest
modules**, before and after this task. The bounded-recursion capability
this task adds is exercised, in this corpus, only by synthetic unit
tests — a documented, not hidden, gap between capability and corpus
evidence.

**Addendum (goal.md T18, task 12, 2026-09-14) — module *arguments*
measured, and `AllocatorRef` searched for and not found.** Q6 recorded the
`Module`/`ModuleDef` structure; this pass counted what the bindings inside
it actually say, over every `ApplicationProgram` member of every archive
installed under `OriginalData/`:

| Where | `Module` | `NumericArg` | `TextArg` | `AllocatorRef` | `Argument` decls |
|---|---|---|---|---|---|
| `ProductDatabases/` | 86 | 172 | 86 | **0** | 36 |
| `DemoProjects/` | 32 | 96 | 0 | **0** | 12 |

All **[V]**, this run. Every one of the 118 `Module` elements carries at
least one binding — arguments are not a corner of the format, they are how
a modular product is written. The 36 product-database declarations are all
MDT `M-0083` (12× `ParamOffsBase` `Allocates="132"`, 12× `ObjNumberBase`
`Allocates="20"`, 12× `ChNo` `Type="Text"`); the 12 demo-project ones are
all KV25 `M-00FA` (`argCH`/`argObj`/`argPar`). **No declaration anywhere
spells `Type="Numeric"` explicitly** — numeric is the absent case, which is
why the reader treats a missing `@Type` as numeric rather than as unknown.

Where an argument is actually *consumed*, corpus-wide **[V]**:
`Memory/@BaseOffset` → a numeric argument id, **705**;
`ComObject/@BaseNumber` → a numeric argument id, **157**; `{{Name}}`
placeholders in text, **978**, of which **978 resolve to an
`Argument/@Name` declared by the enclosing `ModuleDef` and 0 do not** (775
of them reached through `TranslationElement/@RefId`, 203 direct). A
separate, larger family of **948** purely numeric `{{<digits>}}`
placeholders resolves to nothing in any file and belongs to
`TextParameterRefId`, not to this mechanism. Only **24** placeholders sit
where the `Dynamic` evaluator can reach them — inside a `ModuleDef`'s own
stored tree — and those 24 are what T18 task 12 interprets.

**`AllocatorRef` is unattested, and these are the bases that were
searched.** **[D]** `Project Schema23 v01.00.00` §1.1.2.38
`ModuleDefArgType_t` names the facet (`Numeric`, `Text`, `AllocatorRef`)
and `Value_t` describes it in one line — *"TypeAllocatorRefId — A module
allocator refId as string"* — with no rule for what an allocator does.
Beyond that: `OriginalData/` in full (`.knxprod` and `.knxproj`, element
and attribute spellings, every readable archive member) → **0**, with the
only 3 unreadable members being the encrypted contents of the single
`.vd2`, a format already out of scope;
`knx_spec_kb_programming.sqlite` (2,207 facts over 27 programming PDFs with
figures) → **0**; `knx_spec_kb_full179_clean.sqlite` (16,536 facts over 177
PDFs, text only) → **0** — both searched across `content`, `title`,
`keywords` and `evidenceText`. The only `Allocator` hits in either base are
"heat cost allocator", in DPT documents. It therefore stays unimplemented
and is *reported* (`UnsupportedModuleArgumentKind`) rather than guessed at.
`crates/knx-productdb/tests/dynamic_tree.rs`'s
`corpus_argument_measurement_task_12` re-measures the corpus half of this
on every test run, so the zero above is an assertion, not a memory.


---

## 5. `knx_master.xml`

Content of the ETS4 master data file shipped inside our project [V]:

* `DatapointTypes`: 46 main types, 289 subtypes — re-verified
  (`unzip -p "OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj"
  knx_master.xml | grep -o '<DatapointType ' | wc -l`, same command with
  `DatapointSubtype` for the second figure). This is a property of
  *this one* master-data file, not a fixed constant: the repository
  carries seven other `knx_master.xml` copies (one per `.knxprod`/
  `.knxproj` under `OriginalData/`), and their main-type counts spread
  from 29 to 63 depending on ETS/master-data vintage — the two other
  demo projects alone give 57 and 63. "46 main types, 289 subtypes"
  describes the ETS4 vintage bundled in our primary reference project,
  not KNX main types in general.
* `MediumTypes`: `MT-0` = TP, `MT-1` = PL, `MT-5` = IP. (RF absent in this ETS4 master file.)
* `Manufacturers`: 447 entries, `M-0001` Siemens, `M-0002` ABB, …
* `MaskVersions`: 29 entries, e.g. `MV-0010` mask `16` "1.0" `ManagementModel=Bcu1`, `MV-0020` mask `32` "2.0" `Bcu2`.
* `Languages` / translations for the above.

Each `MaskVersion` carries a `HawkConfigurationData` block containing `Resources` (611 `Resource` entries with `ResourceType`, `Location`, `AccessRights`), `MemorySegments` (47), `Procedures` (74 `Procedure` with `LdCtrlConnect`, `LdCtrlLoad`, `LdCtrlWriteMem`, `LdCtrlWriteProp`, `LdCtrlMerge`, `LdCtrlRestart`, …), `InterfaceObjects` (32) with `Property` definitions, `Features`, and `DownwardCompatibleMasks` [V].

**This is the device-programming rulebook in machine-readable form.** Combined with the application program's own `LoadProcedures` and `AbsoluteSegment` data, the load procedure for a device is *data-driven*, not hardcoded per manufacturer. That makes commissioning technically approachable in principle — see §8.3 for what still blocks it.

`knx_master.xml` is shipped **inside every `.knxproj`** [V]. We therefore always have the master data matching the project we import, and do not need to bundle our own copy to read a project. Whether that copy may be extracted and reused as a general database is a licensing question (§10).

---

## 6. Group addresses and DPT resolution

* Style: `ThreeLevel` for this project; ETS also supports Free / TwoLevel [D]. Raw address is an integer; `raw_address=1` renders as `0/0/1` [V].
* `GroupRange` nests two levels deep with `RangeStart`/`RangeEnd`; middle groups are `GroupRange` inside `GroupRange` [V].
* `GroupAddress/@Central` and `@Unfiltered` are filter-table hints for line couplers — 2 addresses each in our project [V]. Rarely used, easily lost, must be preserved.

### 6.1 DPT coverage is genuinely incomplete [V]

Of 514 group addresses:

| | count |
| --- | --- |
| no DPT set at all | 194 (38%) |
| DPT 1.x (1.001 / 1.008 / 1.* unspecified) | 207 |
| DPT 14.019 | 52 |
| DPT 5.001 | 41 |
| DPT 3.007 | 20 |

Also: 110 of 514 group addresses (21%) have **no linked communication object** at all — orphan addresses that exist only as documentation. All 570 communication objects, by contrast, have at least one group address link.

Implications:

1. A DPT-less group address is normal, not an error. The model must allow it. Where a GA has no DPT but its linked communication objects do, the DPT can be **inferred** — but inference must be stored as inferred, never written back as if the user had set it.
2. Orphan group addresses must survive import and export unchanged. They are the user's plan.
3. Conflict case: multiple linked objects declaring different DPTs. Must be detected and reported, not silently resolved. (Not present in this sample; still needs a rule.)

`data_secure` is `false` for all 514 addresses here — no Data Secure in this installation [V].

---

## 7. Opaque and unsupported data

Data present in the file that we can preserve but should not interpret [V]:

| Item | Content | Handling |
| --- | --- | --- |
| `<M>/Baggages/*.dll` | Windows PE binaries (`econEts3.dll` 641 KB, `FastDownload.dll` 160 KB) — ETS plug-ins | Preserve as opaque bytes. Never execute. Report as unsupported. |
| `<P>/BinaryData/<guid>.dat` | 8-byte header + `<BlobInfo>` + CSV payload (`BlobFile=SmartSensor29899.blob`) | Preserve verbatim, keyed by `BinaryData/@Id` from `DeviceInstance`. |
| `<P>/ExtraData/*.rbg`, `*.azp` | ISO-8859 CSV, CRLF — legacy ETS3-era plugin data. Byte-identical payload to the corresponding `.dat` minus header. | Preserve verbatim. |
| `*.signature` | RSA signatures over manufacturer/project data | Preserve. Cannot be regenerated without KNX signing keys → **any export we write will be unsigned**. |
| `Options/Legacy*` flags | Per-application compatibility switches | Preserve. Surface as capability blockers. |
| `RegistrationInfo` / `Hash` attributes | Certification metadata | Preserve verbatim. |

**These plug-in binaries are why "full ETS compatibility" is not achievable and must never be claimed.** For devices whose configuration lives partly inside a vendor DLL, no independent tool can reproduce ETS behavior.

### 7.1 Measured data loss in `xknxproject` [V]

`xknxproject` is an excellent *reader* but is explicitly lossy, and its output shape is a reasonable sanity baseline rather than an import target. Confirmed gaps against our sample:

* **Parameter values dropped entirely.** 1390 `ParameterInstanceRef` in the file; the output model exposes none. Parameters are read only to substitute text placeholders in names (`xknxproject/util.py`, `models.py`).
* **The unassigned device is dropped.** 36 `DeviceInstance` in the XML, 35 in `project_dump.json`.
* Not represented: `BusAccess`, `BinaryData`/`ExtraData`, `CompletionStatus`, `LastModified`/`LastDownload`, the five `*Loaded` flags, `Broken`, `BCUKey`, `SplitType`, `GroupAddress/@Central`/`@Unfiltered`, `BuildingPart/@DefaultLine`, `Send`/`Receive` directionality.
* No export path at all.

Conclusion: **we must write our own parser.** `xknxproject` remains valuable as a cross-check oracle during development.

---

## 8. KNXnet/IP and bus access

### 8.1 Verified live [V]

Tunnelling to a real gateway at `192.0.2.1:3671` works from Linux via `xknx`, and 280 telegrams were captured in a 300 s window. Observed APCI: `GroupValueWrite` (majority), `GroupValueRead` (30), `GroupValueResponse` (30). Payload forms: `DPTBinary` (small values) and `DPTArray` (2/3/4-byte). All 280 telegrams resolved to a named group address from the project — the project model and the live bus agree.

This confirms end to end: project data → group address semantics → live telegram decoding, on Linux, without ETS.

**Session 6, Cycle 1 (2026-09):** An own Rust implementation of the
tunnelling connect/heartbeat/receive/disconnect lifecycle and cEMI `L_Data`
decode, built directly from the KNX Association specification (Core v01.06.02
AS, Tunnelling v01.07.01 AS, EMI_IMI v01.04.02 AS — not from reading
`xknx`'s implementation). This cycle delivered `crates/knx-net`'s five codec
modules with unit tests, the `TunnelClient` state machine, an `#[ignore]`d
live-gateway integration test (the first of its kind in this repository —
a pattern for future hardware-dependent tests), and the `knx bus monitor`
CLI subcommand. Live-gateway verification against a real KNXnet/IP gateway
is the next step, to be run by the user via `cargo test -p knx-net --
--ignored` and `knx bus monitor` against their own gateway in an environment
with LAN access.

**Session 6, Cycle 3 (2026-09):** `discover()` implemented — a
`SEARCH_REQUEST` multicast to `224.0.23.12:3671` (Core v01.06.02 AS §4.2),
collecting `SEARCH_RESPONSE`s for the full 10s `SEARCH_TIMEOUT` and
parsing both the Device Info and Supported Service Families DIBs
(§7.5.4.2/§7.5.4.3). `crates/knx-net` gained `core::dib` and `discovery`,
plus a `knx bus discover` CLI subcommand. Live verification against the
reference gateway — confirming it answers and advertises tunnelling
support — is the next step, to be run by the user via
`cargo test -p knx-net -- --ignored` and `knx bus discover` on their own
LAN.

### 8.2 Standards position [D]

* KNXnet/IP is publicly standardized as **ISO 22510:2019** (EN ISO 22510:2020), covering Overview, Core, Device Management, Tunnelling, Routing, Remote Diagnosis, Secured Communication, plus cEMI and coupler resources. Purchasable, not free.
* The full KNX Specification (v2.1/v3.0 — TP1, PL110, RF, application interworking) is available to KNX Association members via MyKNX, or purchasable from the KNX shop.
* The publicly standardized subset is precisely why independent stacks (Calimero, knxd, xknx) target KNXnet/IP.

### 8.3 Commissioning / device download — scope decision

Technically, the ingredients are present and machine-readable: mask-version `Procedures` and `Resources` in `knx_master.xml`, `LoadProcedures` + `AbsoluteSegment` + `AddressTable`/`AssociationTable`/`ComObjectTable` offsets in each application program, and A_Memory/A_PropertyValue management services over the bus.

It is nevertheless **not started**, and four things block it:

1. Writing wrong memory images to a real device bricks it. This needs hardware we can afford to destroy.
2. **Revised, 2026-09-11 (R5 spike, §8.4).** The generic complete/partial download load procedure *is* documented in the KNX Standard — `03_05_03 Configuration Procedures` §3.5.2/§3.5.3, CRC-driven via `PID_MCB`, and "Differential Download" is a formally defined Glossary term. What remains undocumented outside ETS/manufacturer tooling, and absent from both KNX specification databases searched, is the product-specific `Legacy*` compatibility-flag matrix that decides whether and how a given application program participates in that procedure. See §8.4 Q3/Q4 for the exact citations and the distinction between "the procedure" (documented) and "this product's flags for the procedure" (not).
3. Vendor `Baggages` DLLs participate in download for some devices.
4. KNX Secure devices require the key material handling of §9.

**Ruling, 2026-09-11.** Asked whether commissioning is permanently out of scope, the user said no: it must work too, but the work waits until the KNX specification database is finished. The four blockers above are unchanged — they are why it has not started, not a reason it never will. See [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked), [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) row **E1** (which stays open), and backlog task **T30**.

Recommendation: build toward *read/diagnose/monitor* first (Session 6), and treat programming as a separate, later, explicitly-flagged research effort. Nothing in the architecture should preclude it — hence keeping `LoadProcedures`, `Memory`, `AbsoluteSegment` and mask data in the model rather than discarding them at import.

---

### 8.4 Commissioning / device download procedures — R5 spike (Session 6, 2026-09-11)

**This spike documents. It does not verify, and it does not implement.** Every
procedure below is *documented from the KNX Standard*, never *verified on
hardware* — no device was touched, no bus was contacted, and nothing here
moves [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
one line closer to working. A documented procedure is a documented
procedure; ETS parity and hardware behaviour remain unestablished. Full
spike report: `/home/knxbench/.claude/jobs/8098e9e6/tmp/r5-report.md`; this
section is the durable summary that survives outside that file.

**Reusable pointer — where this evidence lives and how to get more of it.**
Two complementary SQLite databases, same schema (`facts` table plus an FTS5
`factsSearch` over `title`/`content`/`keywords`, `tokenize='unicode61'`;
every fact carries `sourcePdf`, `sourceMarkdown`, `chunkStart`/`chunkEnd`,
`evidenceText`, `contentSha256`):

| Database | Scope | Facts | PDFs | Has figures? | Use for |
| --- | --- | --- | --- | --- | --- |
| `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_programming.sqlite` | Commissioning/programming-relevant subset | 2207 | 27 | **Yes** — `figures`/`figuresSearch` (`section`, `summary`, `steps`, `labels`), figure `summary`/`steps`/`labels` are vision-model-generated (qwen2.5vl:7b) descriptions, mark as **[V]** with that caveat, never **[D]** | Anything involving a diagram (e.g. the Load State Machine figure, Q2 below), and as the first stop for any commissioning question — narrower but richer (figures, captions, OCR-recovered footnotes) |
| `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/knx_spec_kb_full179_clean.sqlite` | Full KNX Standard v3.0.0 corpus | 16536 | 177 | **No** — no `figures` table at all | A second pass when the programming database runs thin — broader but text-only. Two of the 179 source PDFs produced no usable facts. `knx_spec_kb_full179_unfiltered_reference.sqlite` exists for comparison only — do not query it, do not cite it |

The two are **complementary, not ranked** — a gap in the programming
database's coverage is a much weaker claim than a gap in both. Query either
with `sqlite3`, e.g.
`sqlite3 -json "$DB" "SELECT f.* FROM facts f JOIN factsSearch fs ON f.factId=fs.factId WHERE factsSearch MATCH '...' "`.
Every **[D]** claim below carries a quoted citation naming its source. Most
quote `evidenceText` from a database fact row and cite `sourcePdf` directly.
A substantial number — marked **[D, corpus]** at the point they occur, and
roughly as many as the fact-row citations — quote the
extracted Markdown/PDF text directly, because exhaustive FTS search against
both databases turned up no matching fact row for that specific sentence,
even though the surrounding passage is present and correctly attributed.
This is a known property of the extraction pipeline (it chunks/samples the
document rather than indexing every sentence as its own fact), not a gap in
this spike's searching, and not a licence to under-cite: every **[D, corpus]**
claim still names the exact source file. The CLI
described in `knowledge_base/KB_PROMPT.md` (`scripts/05_knowledge_base_v1.py
--query`/`--query-figures`) works against either database via `-o`. The
extracted Markdown corpus at
`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`
is where `sourceMarkdown` points, and is the right place to read a normative
section in full context once the database has located it.

**The extraction is lossy, and tables are where it loses most.** Read a
normative *table* from the source PDF under the sibling `sources/…` subtree,
not from the Markdown, and record which of the two you read. Two measured
examples, both paid for in this repository's own work. `03_07_02 Datapoint
Types` §3.11's Day column truncates mid-enumeration at `7 =`, which is how an
earlier draft of `KNOWN_LIMITATIONS.md` §61 came to call a documented day
code undocumented. `03_03_07 Application Layer`'s Table 1 — the APCI code
table, the one thing anyone opens that file for — survives extraction three
separate times (Markdown lines 272, 451 and 453), each time mangled
differently: the ten bit columns collapse into single cells, rows merge into
their neighbours, the row for `1 0 1 1 0 0 1 0 0 1` has lost its service name
altogether, and `A_FunctionPropertyState_Response-PDU` floats in a cell of its
own with no bits beside it. No APCI value can be read out of that safely.
Prose sections fare far better — the caveat is about tables and figures, not
about the corpus as a whole. **[V]** 2026-09-13, two files inspected.

Confidence markers, as elsewhere in this document: **[D]** the Standard
states it (quoted `evidenceText` + named PDF); **[D, corpus]** the same, but
quoted directly from the extracted Markdown/PDF because no database fact row
matches that sentence (see "Reusable pointer" above); **[V]** verified by
observation (here: the corpus itself, or a measured query count); **[A]**
inference beyond both. Markers are never promoted.

**Note on this pass.** §8.4 was drafted against the programming database
first; a coordinator message mid-spike surfaced the second, broader
database and asked for a fold-in pass. Q7 and Q9 changed materially as a
result (the Master Reset *triggering* procedure, and the dual-database
verification counts); Q1, Q2, Q3, Q5, Q6 kept their original programming-database
citations because the second pass found nothing that contradicted or
usefully extended them; Q4 and Q8 gained one additional citation each. See
"which questions changed" at the end of this section.

---

**Q1 — Individual-address programming.** **[D, corpus]** Two Network
Management Procedures carry this, both in `03_05_02 Management Procedures
v02.01.02 AS.pdf` §2.2/§2.3 (and §2.9 for the Domain-and-IA variant used on
RF/PL110 media): `NM_IndividualAddress_Read` and `NM_IndividualAddress_Write`.
All four quotes below are taken from the extracted Markdown/PDF directly —
exhaustive FTS search of both databases found no matching fact row for any
of these sentences, despite the surrounding procedure text being present and
correctly attributed to this PDF.

`NM_IndividualAddress_Read` — *"This Network Management Procedure shall be
used to read out the Individual Addresses of all the devices that are in
Programming Mode."* Used Application Layer service: `A_IndividualAddress_Read`
(a system broadcast; every device in Programming Mode answers). Detection of
multiple devices is by **response counting during a fixed time-out window**,
stated explicitly: *"The Management Client shall always wait until the
time-out has elapsed. It shall collect all responses IAn during this
time-out."* — *"If no A_IndividualAddress_Response-PDU is received, no
device is in Programming Mode. If one A_IndividualAddress_Response-PDU is
received, exactly one device is in Programming Mode. If more than one
response is received, several devices are in Programming Mode. If two or
more responses with the same Individual Address are received, there is more
than one device with the same Individual Addresses."*

`NM_IndividualAddress_Write` — *"This Network Management Procedure shall be
used to write the Individual Address of one single device that is in
Programming Mode. The procedure shall wait until exactly one device is in
Programming Mode. It shall check that no other device has the same
Individual Address. The procedure shall check if the programming is
successful and shall deactivate the Programming Mode by executing a restart
of the device."* Used Application Layer services, in the order the section
lists them: `A_IndividualAddress_Read`, `A_IndividualAddress_Write`,
`A_DeviceDescriptor_Read`, `A_Restart`, `A_Connect`. So: (1) re-run the
read/detect-conflict step above before writing, as the procedure explicitly
requires waiting for exactly one responder; (2) `A_IndividualAddress_Write`
assigns the new address; (3) verification afterwards is **`A_DeviceDescriptor_Read`**
(confirms a device now answers at the new address — the standard way to
address-and-probe a specific IA) — the procedure text does not spell out a
second `A_IndividualAddress_Read` as the verification step, `A_DeviceDescriptor_Read`
is; (4) `A_Restart` deactivates Programming Mode, matching the "shall
deactivate the Programming Mode by executing a restart" sentence above; (5)
`A_Connect` establishes the Transport-Layer connection the subsequent
Configuration Procedure (Q3) runs over.

**[V, with the figure-provenance caveat above]** `figuresSearch` in the
programming database returns two candidate sequence diagrams in this PDF —
figureId ending `:2` (page 14) and `:14` (page 24). Page 14 falls inside
§2.3's page range and is the more likely `NM_IndividualAddress_Write`
diagram; page 24 falls later and more likely illustrates §2.9
`NM_DomainAndIndividualAddress_Write`. This spike did **not** resolve the
mapping past "more likely" — the figure `summary`/`steps` fields are
vision-model text describing an image, not a caption extracted from the
PDF, and asserting a definite figure-to-procedure mapping from that alone
would overstate the evidence.

---

**Q2 — The Load State Machine.** **[D]** `03_05_01 Resources v01.10.01
AS.pdf` §4.23 is normative and complete. States: `Unloaded` (0), `Loaded`
(1), `Loading` (2), `Error` (3), `Unloading` (4, optional), `LoadCompleting`
(5, optional). Events (values of `PID_LOAD_STATE_CONTROL`, PID = 5): `No
Operation` (00h), `Start Loading` (01h), `Load Completed` (02h), `Additional
Load Controls` (03h), `Unload` (04h). A loadable part that fails reports it
by transitioning to `Error` (3); the reason is then readable via
`PID_ERROR_CODE` (PID = 28, `PDT_ENUM8`, `DPT_ErrorClass_System` 20.011) —
see Q5 for the full enum.

**[D] "More than one Load State Machine is possible in one device."** Each loadable part
(Application Program 1, Application Program 2, Group Object Table, Address
Table, Association Table, …) is its own Interface Object with its own LSM,
addressed independently by `object_index` (`PID_OBJECT_INDEX`, PID = 29) —
there is no single global load state. For a downloader this means: track
load state **per Interface Object**, not per device; the order in which the
LSMs of different Interface Objects must be driven relative to each other
(e.g. Group Address Table before Association Table) is a **Profile-level**
dependency, not stated once in the base Resources document — the base
document defines the single-LSM state/event machine, cross-LSM ordering
lives in `06 Profiles v02.01.01.pdf` and the specific Configuration
Procedure being run (Q3/Q4 answer the ordering for the standard case).

**[D] Realisation Type 2 exists and is documented, but in the Profiles
volume, not Resources.** `06 Profiles v02.01.01.pdf` documents "Programming
Mode - Realisation Type 2" (§4.26.3) and separately states *"The Load - and
Run State Machines - Realisation Type 2 are not allowed for the Coupler
Model 2.0 and the derived masks"* (`06_02_42 mask 2920h v01.01.01.pdf`) — a
mask-specific restriction, not a Resources-document gap. This spike's first
pass (programming database only) had flagged Realisation Type 2 as an open
question; the broader database's second pass resolved it as documented
elsewhere in the same corpus, not missing.

**[V, figure-provenance caveat]** `figuresSearch` for "load state machine"
returns the state diagram at `03_05_01 Resources...:44` (page 297) and a
second, `03_05_01 Resources...:43` (page 295) immediately preceding it, plus
a Management Procedures figure (page 143) and a Load Controls cookbook
figure (`02_03_01 Load Controls...:3`, page 5) — the latter is explicitly
non-normative (a cookbook), useful for cross-checking the textual state
table above, not as a citation source in its own right.

---

**Q3 — The complete download procedure.** **[D]** `03_05_03 Configuration
Procedures v02.01.01 AS.pdf` §3.5.2 "Load procedure for complete download"
gives a fully ordered, numbered sequence per loadable segment. For
Application Program 1, the sequence includes (quoting the fact titles and
content captured, in the document's own order): unload dependent tables
first — *"Set AssociationTable.LoadState = Unloaded"* — then *"Set
Application Program 1 to the LoadState 'Loading'"* (`MaC:
ApplicationProgram_1.LoadControl = Load`, `MaS:
ApplicationProgram_1.LoadState = Loading`), then *"Allocate the required
memory size"*, then write the data via the memory services (Q6), then a
CRC check step — *"Compare CRC checksum: MaC:
PropertyRead(ID_ApplicationProgram_1, PID_MCB), MaS:
PropertyResponse(ID_ApplicationProgram_1, PID_MCB, Data). The current CRC
shall be responded and shall be compared with the stored CRC. If the CRC
matches, then MaC shall use differential download algorithm. Load data via
direct memory access..."* — then
`LoadControl = Load Completed` to leave `Loading` for `Loaded`. The same
document specifies the Group Address Table, Association Table and Group
Object Table loads with the equivalent load/allocate/write/complete
pattern, each preceded by unloading whichever table depends on it. §3.5.4
"Load Procedure for unload" is the mirror sequence (Q7). Preconditions:
the Management Client must already hold an established connection
(`A_Connect`, from Q1's tail) and the target Interface Object's LSM must be
in a state from which `Start Loading` is legal (not already `Loading`) —
what makes it fail: an `A_Restart` mid-sequence, a device response placing
the LSM in `Error` (Q2), or (Q6) a memory write that the device legally
refuses.

---

**Q4 — Partial download.** **[D] The generic partial-download procedure is
documented — the old §8.3 claim was wrong on this point.**
§3.5.3 "Load procedure for partial download" in the same PDF specifies five
variants (one per segment kind) built on the same CRC-based precondition
check as complete download: read `PID_MCB` (Memory Control Block), compare
the reported CRC against the CRC the Management Client already holds for
that segment from a prior download, and only reload the subsegments whose
CRC differs. `PID_MCB_TABLE` (PID = 27) is the property that carries this —
*"This optional Property shall divide the segment into multiple
subsegments with access rights definable for each subsegment and carrying
the checksum for the subsegments. The use case for this subsegmentation is
to separate code and parameter sections."* (`03_05_01 Resources` §4.20.3.3.7
/ §4.21.2.8, `PID_MCB_TABLE` general spec at §4.2.27).

The optimisation this implements has a formal name and a formal definition,
found via the second-pass database: `03_01_02 Glossary v01.05.03 AS.pdf`
defines **"Differential Download"** — *"Optimisation of the Configuration
Procedure in S-Mode, in which only the data is downloaded that is assumed
to differ between the current contents and the intended contents after
download. NOTE 3 To this purpose, the Management Client may for instance
hold a memory image of a preceding download, which it compares with a new
memory image (new parameters, links…) to decide on which data to write in
the device."* This is a Glossary-grade, Standard-normative definition, not
an inference.

**What is not documented, precisely.** The `Legacy*` option matrix — the
manufacturer/product-data compatibility flags (e.g. something functionally
equivalent to `LegacyNoPartialDownload`) that decide whether *a specific
application program* is allowed to participate in partial download at all,
or must always fall back to a complete download — is **absent from both
databases**. A `Legacy` FTS query, run against both this spike and verified
independently in the second pass, returns exactly **one hit in each
database, and both are unrelated**: the programming database's hit is
*"Legacy implementations need this command. Newer implementations however
might ignore this command"* (`03_03_07 Application Layer`, about optional
backward-compatible command handling, not a per-product flag matrix); the
full database's hit is *"Fast Repeaters shall have two working modes, KNX
RF Ready (legacy compatible KNX RF 1.1) or KNX RF Multi"* (`03_02_05
Communication Medium RF`, RF hardware-generation compatibility, also
unrelated). Neither database has ever indexed the ETS/`.knxprod` vocabulary
this spike was looking for. **Precise correction of the old §8.3 wording**:
"partial-download rules are undocumented" is false (§3.5.3 documents them);
"the `Legacy*` matrix is undocumented" remains true, and is a product-data
construct outside the Standard's scope, not a gap in this research corpus.

---

**Q5 — Resources and property IDs.** **[D]** From `03_05_01 Resources
v01.10.01 AS.pdf` Table 89/90/91 (Application Program / Application Program
1 / Application Program 2 Interface Objects) and §4.2.x per-property
clauses, plus `AN194` for `PID_MANUFACTURER_ID`:

| PID | Property | Type | Role in a download |
| --- | --- | --- | --- |
| 1 | `PID_OBJECT_TYPE` | `PDT_UNSIGNED_INT` | Identifies the Interface Object (e.g. Application Program Object = 0003h) |
| 2 | `PID_OBJECT_NAME` | `PDT_UNSIGNED_CHAR[]` | Name of the application program |
| 5 | `PID_LOAD_STATE_CONTROL` | `PDT_CONTROL` | Drives the LSM (Q2) — write an event, read the resulting state |
| 6 | `PID_RUN_STATE_CONTROL` | `PDT_CONTROL` | Drives the Run State Machine (§4.24), independent of the LSM |
| 7 | `PID_TABLE_REFERENCE` | `PDT_UNSIGNED_LONG` | Pointer/base address the segment is downloaded to; set to the allocated address on success, 0 when Unloaded or on allocation failure |
| 12 | `PID_MANUFACTURER_ID` | `PDT_UNSIGNED_INT` (Device Object, constant) | Manufacturer identity check step in a download procedure |
| 13 | `PID_PROGRAM_VERSION` | `PDT_GENERIC_05` | Version of the application program being (or already) loaded |
| 16 | `PID_PEI_TYPE` | `PDT_UNSIGNED_CHAR` | Required physical-external-interface type |
| 27 | `PID_MCB_TABLE` | `PDT_GENERIC_08[]` | CRC/subsegmentation table, the partial-download precondition check (Q4) |
| 28 | `PID_ERROR_CODE` | `PDT_ENUM8` (`DPT_ErrorClass_System` 20.011) | Reason for LSM state `Error` |
| 29 | `PID_OBJECT_INDEX` | — | Addresses *which* Interface Object/LSM a given access targets (Q2) |
| 30 | `PID_DOWNLOAD_COUNTER` | — | Counts downloads; affected by Master Reset Erase Codes (Q7) |

Each row is precision a Rust implementation would encode as a `const` or
enum discriminant directly; no row above is inferred.

---

**Q6 — Memory services.** **[D, corpus]** `03_03_07 Application Layer v02.01.01
AS.pdf` §3.5.4 `A_Memory_Write-service` / the preceding `A_Memory_Read-service`
clause (quoted from the extracted Markdown directly — no matching database
fact row was found for this sentence in either database): *"The A_Memory_Write.req primitive shall be applied by the user of
Application Layer, to write between 1 octet and 63 octets in the address
space of the remote communication controller. The parameter memory_address
shall specify the 16 bit start address..."* — 1-63 octets per call, 16-bit
addressing, always relative to the target Interface Object's allocated base
(`PID_TABLE_REFERENCE`, Q5). `A_Memory_Read` is symmetric, with the same
16-bit addressing and a length field.

**Verify Mode.** **[D, corpus]** *"The service shall be a confirmed service
if Verify Mode is active, otherwise it shall be an acknowledged service."*
This exact sentence recurs verbatim across several other service clauses in
the same PDF — the one database fact row that captures it is titled for
`A_MemoryBit_Write`, not `A_Memory_Write`, so citing that row here would
misattribute the sentence to the wrong service; quoted from the Markdown
directly instead. With Verify Mode **inactive**, the remote application
process does not respond at the Application Layer at all (only the
Transport Layer confirms delivery). With Verify Mode **active**: **[D,
corpus]** *"the remote application process shall respond
to the A_Memory_Write.ind primitive with an A_Memory_Write.res primitive
containing the requested number of octets of the associated memory area.
The value of the associated memory area shall be explicitly read back after
writing to it."* (also no matching fact row). Verify Mode itself is controlled via `PID_DEV_CONTROL`
(PID = 14, per the Configuration Procedures "Set Verify Mode" fact) and
defaults to disabled — **[D]** *"The value of Verify Mode Control shall per default
be 0 ('disabled')"* (`03_05_01 Resources`, database fact row); a Profiles-volume footnote adds
**[D]** *"If Verify Mode is not implemented, it shall always be off."* (database fact row)

**What a device may legally refuse, and how.** **[D, corpus]** *"If data are
to be written to a protected area from any logical address that is not
associated to physical memory then the service indication shall be
ignored. ... If only a part of the addressed memory is protected or does
not exist, then the complete write operation shall fail."* Quoted from the
Markdown directly: the database captured only a near-duplicate instance of
this same clause worded for the Filter Table write operation ("addressed
Filter Table"), not this Memory-write-specific instance, even though the
sentence recurs verbatim elsewhere in the same document too. Refusal is
**silent** (the indication is dropped, there is no explicit NAK APDU)
rather than an error response, which matters for a Rust implementation's
timeout/retry design. Length is also a hard refusal ground: **[D, corpus]**
*"the remote Application Layer shall
ignore the A_Memory_Write.ind if the value of the parameter 'number' is
greater than Maximum APDU Length - 3"* (no matching fact row; read is `- 3` too; a related
extended-addressing variant elsewhere in the same document uses `- 4`),
and if `number` does not match the actually-received octet count. If Verify
Mode is active and the write failed, **[D, corpus]** *"the field number of the
A_Memory_Response-PDU shall be zero and there shall be no field data to
indicate an error."* (no matching fact row)

---

**Q7 — Unload and reset.** **[D] Unload** — `03_05_03 Configuration
Procedures` §3.5.4 "Load Procedure for unload" mirrors §3.5.2/§3.5.3: drive
the target Interface Object's `PID_LOAD_STATE_CONTROL` with the `Unload`
event (04h, Q2), from whichever state it is currently in, ending in
`Unloaded`; dependent tables (Association Table depending on Application
Program 1, etc.) are unloaded first, mirroring the load order.

**[D] Reset — materially improved by the second-pass database.** The first
pass found `AN194 Master Reset of Resources` alone, which documents a
per-Resource *effect* table for each Erase Code (`not influenced` /
`recalculate` / `KNX default` / `implementation default` / `runtime` /
`not applicable`) but explicitly defers the *triggering procedure itself*
to **[D, corpus]** *"[01] clause 3.7.1.2 'Master Reset'"* (quoted from
`AN194 v02 Master Reset of Resources AS.md` directly; no matching fact row
in either database) without identifying which document `[01]` is. The second pass located it: **it is in the same PDF
already used for Q1**, `03_05_02 Management Procedures v02.01.02 AS.pdf`
§3.7 `DM_Restart`, §3.7.1.1 "Basic Restart" and §3.7.1.2 "Master Reset" —
missed on the first pass because the earlier search terms did not reach
that far into the document. This closes a gap this spike had originally
planned to report as unresolved.

§3.7.1.1 Basic Restart — **[D, corpus]** *"To perform a Basic Restart the
Management Server shall switch off Programming Mode, clear runtime errors,
reset all access levels, ... switch off safe state, ... reset its KNX
communication system, close all KNX Transport Layer connections, close all
KNXnet/IP connections ..., close all KNX Secure Sessions, close all KNX TCP
connections, apply changed configuration Parameters at the latest 30 s
after completing the restart."* Quoted from `03_05_02 Management Procedures
v02.01.02 AS.md` directly; no matching database fact row exists for this
bulleted list in either database. The middle ellipsis above elides one
further list item, "send an appropriate LM_Reset.ind message through the
EMI interface." Identified by a cleared `A_Restart-PDU` `restart_type`
field, not confirmed at the Application Layer (unconfirmed service).

§3.7.1.2 Master Reset — *"To perform a Master Reset, the Management Server
shall reset its configuration data according the following, if supported
and as requested by the Management Client"* — clears Group Address Table /
Group Object Association Table link information, resets application
parameters to default, resets the application to the default application,
resets the IA to the medium-dependent default, then executes a Basic
Restart. Identified by `A_Restart-PDU` with `restart_type = 1` plus an
`erase_code` field (`03_03_07 Application Layer` Figure 40 gives the exact
octet layout: octets 6-9, `Restart Type` / `Erase Code` / `Channel Number`
fields). **Erase Code table (§3.7.1.2.3.1, Table 4)** — which a tool may
issue:

| Erase Code | Name | Effect |
| --- | --- | --- |
| 01h | Confirmed Restart | No Resource reset; a confirmed alternative to the unconfirmed Basic Restart |
| 02h | Factory Reset | Ex-factory state, implementation-dependent which Resources reset (IA included) |
| 03h | ResetIA | IA reset to the medium-specific default |
| 04h | ResetAP | Application Program Memory reset to the default application |
| 05h | ResetParam | Application Parameter Memory reset to default value(s) |
| 06h | ResetLinks | Group Object link information (Group Address Table, Group Object Association Table) reset |
| 07h | Factory Reset without IA | As 02h, but the Individual Address is not reset |
| 08h | Erase persistently stored application data | Application-specific; device documentation should list what this erases |
| 00h, 09h-FFh | reserved | Management Client shall not use; Management Server responds `Error Code = Unsupported Erase Code` |

The Management Server confirms with an `A_Restart_Response-PDU` carrying an
Error Code (`00h` No Error, `01h` Access denied, `02h` Unsupported Erase
Code, `03h` Invalid Channel Number) and a Process Time the client must wait
out before assuming the Master Reset failed. `AN194`'s per-Resource effect
tables (Device Object properties like `PID_MANUFACTURER_ID` are
`not influenced` by every Erase Code, for example) remain the right source
for "what does Erase Code X do to Property Y specifically" — §3.7.1.2
answers "how do I trigger it and what does the base spec guarantee",
`AN194` answers "what happens to this particular Resource".

---

**Q8 — KNX Secure's effect on Q1-Q7.** **[D, structural claim only — not an
implementation, per the standing T19 deferral].** The procedures in Q1-Q7
are not replaced by KNX Secure; they are **wrapped**. `03_03_07 Application
Layer` describes the Secure Application Layer (S-AL) intercepting
`T_Data_Individual.ind`/`T_Data_Connected.ind` before Application Layer
processing (Figure 118): every relevant APDU (`A_IndividualAddress_Write`,
`A_Memory_Write`, `A_Restart`, …) becomes the payload of an `S-A_Data` frame,
authenticated and (for confidentiality) encrypted with a symmetric key and
protected by a monotonically-increasing sequence number the receiver
enforces (*"SeqNrlocal = ... (next valid SeqNr accepted for Tool Key)"*) —
this is the mechanism that would need implementing before *any* of Q1-Q7
could run against a Secure device, not a change to their step order.

**What the tool needs in hand.** Two keys matter for commissioning
specifically: the **FDSK** (Factory Default Setup Key) — *"shall be a
default Tool Key for the authentication and confidentiality to be used by
the MaC when the KNX secure device is firstly configured fresh from
factory"* — used for the very first secure exchange with a factory-fresh
device, and the **Tool Key** proper — *"shall be used to store the security
information for the central MaC in KNX S-Mode (ETS®) and KNX Ctrl-Mode"* —
which the commissioning tool assigns per device and which replaces the
FDSK from then on. Without the FDSK (printed on the device, per §9's
existing account) a factory-fresh Secure device's first commissioning
exchange cannot be authenticated at all; without the per-device Tool Key
material afterwards, none of Q1/Q6/Q7's services can be re-run against an
already-secured device. This sizes, but does not lift, the existing T19
deferral — nothing here is new information that argues for lifting it, and
the project has no sample key material to test against regardless.

---

**Q9 — What this database does not answer.** Measured, both databases,
2026-09-11:

| Query | Programming DB (2207 facts / 27 PDFs) | Full DB (16536 facts / 177 PDFs) |
| --- | --- | --- |
| `Baggage` | 0 | 0 |
| `knxproj` | 0 | 1 — `Project Schema23 v01.00.00.pdf`, the *.knxproj file-extension row of the ETS project XML schema, not a commissioning document |
| `"Building Part"` | 0 | 1 — same PDF, `GroupAddressRef ... List of functions in this building part` — ETS project-tree vocabulary, not a Standard commissioning term |
| `"functional block"` | 1 | 206 — almost entirely Volume 7 "Application Descriptions" (HVAC FB *, System Clock, Common Sensors, …) and the Interworking Model/Glossary/Datapoint Types documents: this is the KNX **interworking-model** term for a datapoint grouping, a different, older concept from ETS5's UI "Functions" feature, and unrelated to the download procedures answered above |
| `Legacy` | 1 (unrelated, Q4) | 1 (unrelated, Q4) |
| `Function` | 42 | 239 |

**Confirmed gaps, with what would close each:**

1. **The `Legacy*` compatibility-flag matrix and vendor-specific download
   sequences (Q4).** Absent from both databases; this is `.knxprod`
   manufacturer product-data vocabulary, not KNX Standard vocabulary — no
   amount of further Standard-corpus searching will find it. Closed only by
   a `.knxprod` sample corpus and its schema, which is a different research
   effort from this one.
2. **`Baggage`/vendor-DLL participation in download.** Confirmed 0 hits in
   both databases — this is a known ETS/manufacturer-tooling mechanism (see
   RESEARCH §7, risk R5) with no counterpart term in the Standard corpus at
   all. Closed only by manufacturer documentation for a specific vendor's
   `Baggage`, or observed ETS behaviour.
3. **The ETS *project file* side (`knxproj`, "Building Part") is a
   different document family, present only in the broader corpus.** The
   full database's one hit each for `knxproj`/`"Building Part"` is the same
   `Project Schema23 v01.00.00.pdf` already used elsewhere in this
   repository for the *.knxproj importer/exporter work (§2-§7 of this
   document) — it documents the *project interchange format*, not device
   commissioning, and this spike did not need it. Not a gap in the
   commissioning research; a reminder that "0 hits" in the narrower
   database can mean "wrong document family," not "undocumented."
4. **The `Dynamic`/`choose`/`when` structural grammar gap from §4.3
   remains a gap here too, for a related reason.** Not re-tested in this
   spike (out of scope — Q1-Q9 concern commissioning, not parameter
   editing), noted only because both gaps share the same root cause: the
   KNX Association's schema/tooling documents (`ApplicationProgram.xsd`
   equivalent, `.knxprod` compatibility-flag schema) are consistently the
   material missing from both databases, which extract the *specification*
   corpus, not KNX Association *tooling* artifacts.
5. **AN194's own cross-reference gap was real on first read, and closed on
   the second (Q7).** Recorded here as a worked example of exactly the
   failure mode Q9 asks to guard against: a document that looks like it
   defers to something absent may simply defer to something not yet
   searched for correctly.

**Which questions changed as a result of the second-database fold-in:**
Q7 changed materially (Master Reset's triggering procedure, previously
reported as an unresolved cross-reference, is now cited in full). Q9
changed materially (the dual-database counts above are the answer, not a
single-database approximation of it). Q4 and Q8 each gained one additional
citation (the Glossary's "Differential Download" definition; nothing new
for Q8 beyond confirming no Secure-specific commissioning term was missed).
Q1, Q2, Q3, Q5, Q6 are unchanged from the programming-database-only draft —
the second pass was run against each and found nothing that contradicted or
usefully extended the existing citations.

---

**Sharpest remaining unknowns**, each with what would resolve it:

1. **The exact figure-to-procedure mapping for the `NM_IndividualAddress_Write`
   sequence diagram (Q1)** is asserted as "more likely," not established —
   resolvable by opening the source PDF at the two candidate pages directly
   (`03_05_02 Management Procedures` pages 14 and 24) rather than relying on
   `figuresSearch`'s vision-model summaries.
2. **Cross-LSM ordering dependencies beyond the standard segment set (Q2)**
   are stated to live in device Profiles, plural — this spike read the one
   Profiles-volume example that came up in search (Coupler Model 2.0's
   Realisation Type 2 restriction) but did not attempt an exhaustive survey
   of `06 Profiles`' 251 facts for every documented ordering constraint.
   Resolvable by a dedicated Profiles-volume spike, should a specific
   device class become the implementation target.
3. **The `Legacy*` matrix and `Baggage` mechanism (Q4, Q9)** are confirmed
   absent from the KNX Standard corpus, full stop — not resolvable by more
   database queries against either database used here. Resolvable only by
   `.knxprod` product-data samples and, for `Baggage` specifically,
   manufacturer-supplied documentation or observed ETS behaviour.
   **Followed up 2026-09-13 in §8.6**, which did exactly that against the
   product corpus: the flag *names* are still absent from the Standard, but
   the matrix itself, its value domains and the download sequence it modifies
   all turn out to be reconstructible from ingested product data, and the
   `Baggage` DLL is not on the critical path. Read §8.6 before relying on the
   paragraph above.
4. **Whether `A_Memory_Write`'s "- 4" extended-addressing variant (Q6)**
   changes any addressing constraint relevant to a download (this spike
   found the reference in passing but did not chase the extended-memory
   service's own clause to the same depth as the base service). Resolvable
   by reading `03_03_07 Application Layer`'s extended memory-services
   clause (AN177 "Extended Memory services", noted as integrated in this
   document's own revision history) in full.

**Advisory** (research input, not a design decision made here): the
procedures in Q1, Q3, Q6 and Q7 are documented precisely enough — services,
order, field encodings — that a Rust implementation of the *protocol steps*
could be written against this section's citations without further research
blocking it. What would still be missing before such an implementation
could safely run against a real, arbitrary device is exactly what §8.3's
unrevised blockers 1, 3 and 4 already say: hardware to test against safely,
the vendor-DLL/`Baggage` mechanism for devices that need it, and Secure key
material. This spike does not change that calculus — it removes one
blocker's factual basis (blocker 2, now split above) without removing any
of the other three, and without turning "documented" into "verified"
anywhere.

---

### 8.5 Line-scan / bus-side device discovery — T17 spike (2026-09-12), shipped (2026-09-13)

**This spike documents a procedure. Nothing described here is implemented,
and the measurements in this section were taken by the controller against
their own installation, not by this repository's code.**
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md) row **T17** guessed that a line
scan would be built on "individual-address serial-number read services".
That guess was wrong, and this spike exists to correct it and to name the
service the Standard actually defines for this purpose. It extends §8.4's
Q1 finding rather than contradicting it: §8.4 already established that
`NM_IndividualAddress_Write` verifies a freshly-written address by opening
a connection and issuing `A_DeviceDescriptor_Read` — "the standard way to
address-and-probe a specific IA." T17's procedure is the same technique
turned around: instead of confirming an address you just wrote, you probe
an address whose occupant is unknown.

**Status (2026-09-13): implemented.** The disclaimer directly above was
accurate when it was written and is kept as written, not edited into a
retroactive lie — but it stopped being true on 2026-09-13. T17 shipped:
`ScanPlan` (`crates/knx-core/src/scan.rs`, exclusions dropped while the
candidate range is built, with an assertion); `Tpci` encode/decode and
the device-descriptor APCIs (`crates/knx-net/src/cemi.rs`); `ProbePolicy`,
`ProbeOutcome`, `probe_address`, `scan_line`
(`crates/knx-net/src/scan.rs`); and `knx bus scan`
(`apps/knx-cli/src/scan.rs`, `apps/knx-cli/src/main.rs`). See
[GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)'s **T17** entry for what
shipped measured against what this section specified. Finding 4, appended
at the end of this subsection, adds what a live run of the shipped binary
against real hardware confirmed and narrowed; everything else below this
point is the spike's own pre-implementation procedure and reasoning,
corrected in place where it was wrong (see Finding 1) rather than
silently left to mislead a future reader.

The disclaimer's second clause — that the measurements in this section
were taken by the controller against their own installation, not by this
repository's code — is likewise only half true from here on: it still
describes every pre-implementation measurement below (the 2026-09-12
`xknx` run and Findings 1-3), but not Finding 4, whose measurements are
this repository's own shipped binary against real hardware.

**The correct procedure.** **[D]** `03_05_02 Management Procedures
v02.01.02 AS.md` §2.19 defines `NM_IndividualAddress_Check`, with a note
naming its colloquial alias directly: *"NOTE This procedure has also been
named NM_IndividualAddress_Scan."* Its purpose, quoted: *"This Network
Management Procedure shall be used by a network Management Client to check
whether a given Individual Address is occupied on the network or not."*
The sequence per candidate address `IA_test`, all **[D]** to the same
clause unless noted: (1) `T_Connect.req` (via `A_Connect`) addressed to
`IA_test`; (2) if a connection is accepted, `A_DeviceDescriptor_Read.req`
with `descriptor_type = 0` sent as `T_Data_Connected`
(`03_03_07 Application Layer v02.01.01 AS.md` §2.2, Table 1); (3) `A_Disconnect`
to close the connection, best-effort, no confirmation required
(`03_03_04 Transport Layer v01.02.03 AS.md` §3.8). Presence is decided at
step (1)/(2), not by anything the application layer says — see below.

**Why the two services the Gap Analysis and this spike both checked are the
wrong tools.** Both were worth ruling out explicitly rather than assuming;
both are wrong for this exact reason:

* `A_IndividualAddress_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.3. Sent as a system
  broadcast, addressless, and answered only by a device currently in
  *programming mode* (its programming button held/pressed). This is the
  commissioning-time "which device is in programming mode" check behind
  `NM_IndividualAddress_Read`/`_Write` (§8.4 Q1), not a way to ask "is
  address X occupied" for an arbitrary, already-commissioned address.
* `A_IndividualAddressSerialNumber_Read`/`_Response` — **[D]**
  `03_03_07 Application Layer v02.01.01 AS.md` §3.2.4. This is the
  Gap Analysis's guess, and it runs the wrong direction: it is addressed by
  a 6-octet serial number you must already know, sent as a system
  broadcast, and answered by whichever device holds that serial number
  with its current individual address and domain address. It answers "what
  address does this known device have," never "is this address occupied."
  It cannot be repurposed for a line scan without already knowing the
  serial number of every device you are trying to discover — which defeats
  the point of scanning.

**Absent vs. occupied: what the Standard resolves, and the one case it
does not.** **[D]** Presence is signalled at the Data Link Layer, not the
application layer: a device recognizing its own individual address as
destination must acknowledge at Layer 2, independent of whether any higher
layer does anything with the frame. `NM_IndividualAddress_Check` §2.19
states the mechanism outright: *"If a device that occupies IA_test is
present on the network, and does support Transport Layer connections, it
shall have no other reaction on the bus than the Layer-2 acknowledge that
initiates the above A_Connect.Lcon."* Three outcomes follow, all **[D]**:

* **No device present**: no ACK, no NAK, no BUSY at all; the sender times
  out the acknowledge window and retries per its own retry budget, then
  concludes not occupied.
* **Device present and busy with another connection**: it does not fall
  silent. Per `03_03_04 Transport Layer v01.02.03 AS.md` §3.7, *"If the
  remote Transport Layer receives … a T_CONNECT_REQ_PDU and does not allow
  for building up a new connection, the frame shall not be passed to the
  remote Transport Layer user. Instead, the remote Transport Layer shall
  send a T_DISCONNECT_REQ_PDU."* `NM_IndividualAddress_Check`'s own
  possible-reaction list treats "Disconnect received, no
  DeviceDescriptor response" as **occupied**, never absent — the
  present-but-busy safeguard the Standard requires is built into the named
  procedure, not left to the implementer to get right.
* **Device present but unable to manage even a BUSY response within its
  own timing budget**: `03_02_02 Communication Medium TP1 v01.03.03 AS.md`
  §2.4.2 requires a device to send BUSY only if it "will again be able to
  process Frames that starts 100 ms after the reception," and explicitly
  forbids sending BUSY otherwise. A device that cannot meet that 100 ms
  budget is, by the Standard's own text, indistinguishable at Layer 2 from
  an absent one. This is a documented limit of the mechanism itself, not a
  gap in this spike's reading, and no scanner built on
  `NM_IndividualAddress_Check` alone can resolve it.

One further limitation this subsection used to carry is resolved here
rather than left hanging: `03_03_04 Transport Layer v01.02.03 AS.md`
§5.5.1.5 ("Connect from the local User to a non-existing Device") and
§5.5.1.9 ("Connection timeout") are both figure-only in this corpus's
Markdown extraction — the sequence diagrams did not survive extraction
and no surrounding prose substitutes for them. But whether the Transport
Layer adds its own independent timeout logic on top of the Data Link
Layer's ACK/retry cycle does not actually hinge on those two blank
figures: clause 4, "Parameters of Transport Layer"
(`03_03_04 Transport Layer v01.02.03 AS.md:665-688`, PDF page 16 of 38)
answers it directly, **[D]**. It fixes connection timeout at 6 s,
acknowledgement timeout at 3 s and max_rep_count at 3 as Transport Layer
parameters, not Data Link Layer ones, and clause 5's "Local variables of
Transport Layer" table (`03_03_04 Transport Layer v01.02.03 AS.md:696-705`,
PDF page 17 of 38, specifically lines 702-703) names two independent
local timers, `connection_timeout_timer` and `acknowledgment_timeout_timer`,
started and stopped by separate actions in the clause's Actions table
(page 19 of 38 onward). The Transport Layer keeps its own clock; it is
not a pure pass-through of Layer 2's ACK/retry cycle. §3.7's prose had it
right; it just did not need the hedge.

**Bus-load budget.** This subsection originally carried an **[A]**
extrapolation for the absent-address case and the whole-line duration,
scaled from nine occupied-address samples — no vacant address was
available to probe at the time. The controller has since run a full line
scan, and the measurement below replaces both **[A]** figures with **[V]**
ones. The arithmetic estimate is kept alongside it, not deleted, because
it is still useful evidence of what the Standard's documented constants
alone predict — see Finding 1 for why that prediction and the measurement
disagree by two orders of magnitude, and why the disagreement is not an
error:

* **[A]**, built from **[D]** inputs (TP1 9600 bit/s per
  `03_02_02 Communication Medium TP1 v01.03.03 AS.md:151`; minimum
  `L_Data_Standard` frame 8 octets; 13-bit-time character slots; 50-bit-time
  idle; 15-bit-time-plus-30µs acknowledge timeout; All TP1 Profiles'
  optional `nak_retry = busy_retry = 3` per
  `06 Profiles v02.01.01.md:775`; all same clauses as §8.4's style of
  citation): one minimal message cycle is roughly 17.6 ms; an absent
  address costs roughly 70 ms (retries exhausted, no ACK ever); an
  occupied, responsive address costs a similar order of magnitude, roughly
  60-100 ms, dominated by the round trip for the actual
  `A_DeviceDescriptor_Response`. This models the KNX medium's own retry
  budget only — it does not, and cannot, model a KNXnet/IP client's own
  connection-timeout policy, which is what the measurement below actually
  hit.
* **[V]**, full scan of area 1 / line 1, one installation, one gateway,
  2026-09-12, via `NM_IndividualAddress_Check` (`xknx`'s
  `nm_individual_address_check`, the same §2.19 procedure cited above)
  over a KNXnet/IP tunnelling connection, sequential, 200 ms pause between
  probes. **254 addresses probed** — the whole line except one address
  excluded by construction (see the exclusion-list paragraph below) —
  with **zero probe errors**: 35 reported occupied, 219 reported vacant.
  Occupied-probe round trips ranged 13.6-6016.5 ms (median 121.1 ms);
  vacant-probe round trips ranged 6275.9-6323.8 ms (median 6279.9 ms).
  Summed probe duration: **1 385.75 s ≈ 23.1 minutes**. One installation,
  one gateway, one client implementation — this stays project-local
  **[V]**, never promoted to a Standard-normative figure, and none of
  these numbers may be written as one either.

  **Finding 1 — the absent-address cost is set by the Standard, not
  invented by a client.** The vacant-probe figures above are the tell:
  **~6.28 s** median, with under 50 ms of spread across 219 samples, is
  far too tight to be bus retry behaviour and far too slow to be a KNX
  medium timing at all. It is a Transport Layer timeout expiring on
  schedule, and the Standard names that schedule directly: **[D]**
  `03_03_04 Transport Layer v01.02.03 AS`, clause 4 "Parameters of
  Transport Layer" (`03_03_04 Transport Layer v01.02.03 AS.md:665-688`,
  PDF page 16 of 38) fixes connection timeout at 6 s, acknowledgement
  timeout at 3 s and max_rep_count at 3; clause 5's Local Variables table
  (`:696-705`, PDF page 17 of 38, specifically lines 702-703) names the
  two local timers — `connection_timeout_timer` and
  `acknowledgment_timeout_timer` — that implement them. The measured
  ~6.28 s tracks the 6 s connection timeout plus transport/IP overhead
  almost exactly, not the ~70 ms the **[A]** bus arithmetic above
  predicted, because the two figures measure different layers: the
  arithmetic modelled TP1's own retry budget; the measurement is the
  Transport Layer's connection-teardown clock running out.

  An earlier draft of this finding attributed the ~6 s figure to a
  client library's own policy constant — `xknx`'s
  `MANAGAMENT_CONNECTION_TIMEOUT`
  (`xknx/management/management.py:35-36`, the copy installed in this
  project's `.venv`) — as if KNXBench were free to pick something
  different for no particular reason. That was wrong and is corrected
  here: `xknx` did not invent 6 s, it implemented the Standard's own
  clause-4 parameter. Any conforming implementation of
  `NM_IndividualAddress_Check`, KNXBench's own included, pays ~6 s
  wherever it waits out a conforming Transport Layer connection timeout.

  The consequence is the design conclusion this whole subsection was
  building toward, corrected: **the scan's duration is dominated by the
  Standard's own Transport Layer connection timeout**, not a policy
  constant invented independently of it. That figure is documented, not
  chosen freely — clause 4 fixes it at 6 s for every conforming
  Transport Layer implementation. The TP1 arithmetic above was not wrong
  about the bus; it modelled a different layer, the medium's own retry
  budget, which is why it landed roughly two orders of magnitude below
  the measured figure. Implementing T17 still meant picking a
  `response_timeout` value for `ProbePolicy`
  (`crates/knx-net/src/scan.rs`), and that choice still governs whether a
  full-line scan finishes in minutes or drags on needlessly — too short,
  and a slow-but-present device is reported vacant (see the fast-preset
  paragraph below); too long, and a mostly-vacant line takes longer than
  it needs to clear. But the choice was made against a documented
  anchor, not blind: `ProbePolicy::default()` sets `response_timeout` to
  6000 ms, matching clause 4's connection timeout exactly, because there
  is no principled reason to wait past the point the Standard itself
  says the far end has given up. A shorter timeout remains a supported,
  explicit option (`bus scan --timeout-ms`) for lines known to run
  entirely fast, present devices — a bet on the installation, not a
  correction to the Standard's own number.

  **Why the fast preset is not the default.** The **[V]** measurement
  above is the reason a short `--timeout-ms` is not shipped as the
  default: occupied-address round trips in the very same run ranged
  **13.6-6016.5 ms**. A 1000 ms timeout — plausible-looking, since most
  occupied addresses answered in tens of milliseconds — would have
  reported the slowest observed present device, at 6016.5 ms, as vacant.
  Repeating a too-short question does not make it a better question:
  shortening the timeout does not distinguish a slow device from an
  absent one, it only moves the point at which the scan starts lying
  about which is which. `bus scan`'s default stays anchored to the
  Standard's own connection timeout for this reason; a faster preset is
  a deliberate, explicit trade a specific installation's operator can
  choose to make (`--timeout-ms`), not KNXBench's default guess.

  The earlier **[A]** 26-57 second full-line extrapolation this
  subsection carried is corrected here rather than silently replaced: it
  scaled from nine addresses that were *all occupied*, reasoning that a
  line looks like those nine. A real line is mostly *vacant*, and vacant
  is the expensive case here, not the cheap one — the earlier
  extrapolation scaled the wrong sample, in the direction that made the
  answer look better than it is. The measured figure, **23.1 minutes**,
  is dominated almost entirely by 219 vacant-address timeouts at ~6.28 s
  each.

  An unthrottled full-line scan is therefore not a multi-second burst as
  first estimated — it is a multi-minute, sustained run of
  connection-oriented traffic on a live line, for as long as the chosen
  timeout policy makes it. It still competes directly with whatever else
  needs that line's bandwidth for that whole window, including devices
  where a delayed response matters, which is exactly why pacing and an
  exclusion list are not optional polish. Any real implementation needs
  an explicit pacing/timeout policy, and an installation-specific
  exclusion list honoured **by construction** — addresses to skip
  enumerated out of the scan range itself, never filtered out afterwards
  — belonging in the domain layer, not the UI, consistent with this
  repository's standing rule against UI workarounds for domain-layer
  problems. This scan exercised exactly that rule for real: one address
  on this installation must never be read (a hazard on any read while a
  condition holds) and was dropped while the address range was built,
  with an assertion that refuses to start if the address survives into
  the list, rather than filtered out of the results afterward. That is
  the shape T17's own exclusion list needs, not a UI checkbox someone can
  forget to tick.

**Finding 2 — a scan sees its own tunnel, and the gateway's.** **[V]**
Three of the 35 addresses this scan reported occupied were not devices on
the twisted pair at all — they were KNXnet/IP tunnelling endpoints
answering from the gateway itself, and their timing gives them away. Two
answered in **13.6 ms** and **14.0 ms**, roughly an order of magnitude
faster than the 100-150 ms a real device on the bus typically needs,
because the answer never left the IP side of the gateway. The third took
**6016.5 ms** and was still reported occupied — the present-but-busy
outcome documented above (`A_Disconnect` received, no
`A_DeviceDescriptor_Response`), not silence.

One of those three was the scanner's own tunnelling connection: asking
the gateway what individual address it had assigned that connection
returned the same address the scan had just reported occupied. A line
scan detects itself, unless it is told not to.

Both facts are implementation requirements for T17, not just
observations. A scan must know its own tunnelling connection's assigned
individual address and exclude or clearly mark it rather than reporting
it as a device — this needs no heuristic, since the gateway hands that
address over during connection setup. A scan should also not present
*other* tunnelling endpoints as bus devices. The sub-20 ms response time
is a usable heuristic for spotting those, but it stays **[A]**: three
samples on one gateway with one client implementation. A genuinely fast
device, or a slower IP path on a different gateway, could break it —
nothing in the Standard promises this gap.


**What a scan does not learn.** **[D]** `A_DeviceDescriptor_Read` with
`descriptor_type = 0` returns DD0, the Mask Version — *"Identification of
an implementation, for operation like download, memory_write … In
particular, the Mask Version is read through a dedicated Application Layer
service by the S-Mode Management Client (ETS) to conclude on the
Configuration Profile of the device and on possible further discovery and
configuration steps."* (`03_01_02 Glossary v01.05.03 AS.md:236`). This
identifies the implementation family/coupler-medium class a device
belongs to, not which product it is, its manufacturer, its application
program, or its serial number — **[D]**
`06_02_01 Coupler Model 2.0 v01.01.01 AS.md` §1.5.2 makes the same point
from the coupler side: many different coupler products deliberately share
one Mask Version. Product/manufacturer identity needs a separate,
additional connection-oriented read after the scan step — e.g.
`A_PropertyValue_Read` on the Device Object (`object_index = 0`),
`PID_SERIAL_NUMBER` (PID 11) — **[D]**
`03_05_03 Configuration Procedures v02.01.01 AS.md:4797` and
`03_06_03 EMI_IMI v01.04.02 AS.md:5074`. This is not part of
`NM_IndividualAddress_Check` itself, and whether a given mask version even
supports Property services (versus only Memory-based access, as some
older masks do) is unverified by this spike — flagged, not resolved. In
short: a scan as specified by §2.19 alone can mark an address
occupied/absent and, where the device answers, its Mask Version. It cannot
by itself populate a topology view with product identity; T16's
device-catalog work would need the extra `A_PropertyValue_Read` step per
occupied address, as a real, visible scope distinction.

**Finding 3 — the bus and the project disagree, and that is the whole
point of E2.** **[V]** On this installation, the reference project's
device list and the bus scan's results did not match: addresses answered
that the project's own records did not account for, and an address the
project lists did not answer at all. Only the counts are recorded here,
deliberately: this repository is public-facing, and no addresses, device
names, or manufacturer inventory belonging to this installation are
written into it.

This mismatch is not a scan defect; it is the reason **E2**
([GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) is a gap worth closing at
all. An ETS project file is a *plan*; the bus is the *installation*. They
drift apart in ordinary use — devices get added by hand, replaced,
re-addressed, or removed without the project file being updated to
match. A scan that only ever confirmed what the project already claims
would be visible but pointless busywork; the value is entirely in where
the two disagree. The two directions of disagreement mean different
things, and belong in a report as two distinct findings rather than one
"mismatch" bucket: an address the scan finds occupied but the project
does not know about is an **undocumented device**; an address the
project lists that the scan finds vacant is either a **removed device, a
failed one, or one whose individual address changed** — three
possibilities a bus scan alone cannot distinguish between, and which
would need a second signal (serial number, product identity read after
the scan — see above) to tell apart.

**Finding 4 — the shipped `bus scan` against real hardware, 2026-09-13.**
**[V]** The controller ran the finished binary against the same
installation's line, distinct from and later than the 2026-09-12
measurement Finding 1 corrects above (that earlier run predates this
implementation and was produced with `xknx`, not KNXBench). No address is
named here for the same reason as Finding 3: this repository is
public-facing.

A dry run (`bus scan --dry-run`) against the full line produced 254
scannable candidates, and 255 when one existing exclusion was lifted —
consistent with `ScanPlan`'s exclusion-by-construction guarantee
(`crates/knx-core/src/scan.rs`) dropping exactly one address, not zero
and not more than one. The candidate list's first entry was device 1, not
device 0 — consistent with device 0 never being a valid candidate (the
line coupler's own address, `crates/knx-core/src/scan.rs`). A dry run
against an unroutable gateway address returned in 1 ms with exit code 0:
a dry run never opens a connection, so an unreachable gateway cannot fail
it.

Two live runs followed, both with `ProbePolicy::default()`
(`response_timeout` 6000 ms, `vacant_confirmations` 1,
`inter_probe_pause` 100 ms):

* **Run 1**, nine consecutive occupied addresses: every probe returned
  `Occupied`, round trips 84-202 ms, elapsed 1810 ms for the nine, and
  every one of the nine reported the same Mask Version, `0x0701`. That
  last fact is recorded and not generalized: nine addresses on one
  installation sharing a Mask Version says nothing about any other
  installation's device mix, and is not evidence that `0x0701` is common
  or typical.
* **Run 2**, five consecutive vacant addresses: every probe returned
  `Vacant`, each one costing 6006 ms, for an elapsed total of 30431 ms
  against a predicted 30430 ms (five × 6006 ms) — the arithmetic and the
  measurement agree to within 1 ms, which is the expected shape for a
  policy-timeout-dominated cost, not bus jitter. Zero probes in this run
  returned `OccupiedBusy` or `OccupiedSilent`. That absence is a fact
  about these five addresses on this one gateway and this one sample,
  nothing more — it does not show that `OccupiedSilent` cannot occur, or
  that it is rare; the evidence-honesty rule against promoting one
  outcome into another binds this null result exactly as it binds every
  other measurement in this section.

Both runs are consistent with the 2026-09-12 measurement's per-address
costs (occupied fast, vacant ~6 s) and with Finding 1's corrected
explanation of where the ~6 s comes from. Neither run is a substitute for
a full-line measurement repeated on this implementation; none has been
done, and this section does not claim one.

**What exists and what shipped, 2026-09-13.** The KNXnet/IP tunnelling
transport, cEMI `L_Data` encode/decode, point-to-point addressing, and a
raw-APCI escape hatch already existed in `crates/knx-net` before T17 and
are directly reused: `TunnelClient` (`crates/knx-net/src/client.rs`) for
the transport; `decode_l_data`/`encode_l_data`
(`crates/knx-net/src/cemi.rs`) for the frame codec; `IndividualAddress`
(`crates/knx-core/src/address.rs:9-50`) and `Destination::Individual`
(`crates/knx-net/src/cemi.rs`) for point-to-point addressing; and
`ApplicationService::Other` (`crates/knx-net/src/cemi.rs`) for the
raw-APCI escape hatch. `client.rs` and `cemi.rs` are cited by name, not
by line: T17 added code to both files, so any single line range picked
now would drift again as soon as either file changes further. What did
not exist as of
this section's original writing, and now does: `Tpci` encode/decode
(`Connect`, `Disconnect`, numbered `Data_Connected`, `Ack`, `Nak`) and the
device-descriptor Application Layer services
(`crates/knx-net/src/cemi.rs`); a `ProbePolicy`/`ProbeOutcome` probe of
one address implementing `NM_IndividualAddress_Check`'s
Connect → `A_DeviceDescriptor_Read` → Disconnect sequence over exactly
that Tpci support, plus `scan_line` for a range
(`crates/knx-net/src/scan.rs`); `ScanPlan`'s exclusion-by-construction
range builder (`crates/knx-core/src/scan.rs`); and the `knx bus scan` CLI
surface, dry-run and live (`apps/knx-cli/src/scan.rs`,
`apps/knx-cli/src/main.rs`). What this closes: the KNX **bus-level**
Transport Layer connection lifecycle needed to run one
`NM_IndividualAddress_Check` probe now exists and is exercised by tests
and by the live runs above. What it does not close, and was never in
scope for T17: a general-purpose, long-lived Transport Layer connection
manager for other connection-oriented services (memory writes, program
downloads); those would reuse the same `Tpci` variants but need their
own sequencing and lifetime, not this scan's single-probe-then-disconnect
shape. Also unbuilt, and explicitly out of scope: scanning across
couplers or more than one line per invocation, and any concurrency
between probes — see `KNOWN_LIMITATIONS.md` for the durable record of
what a scan still cannot do. Connectionless, unnumbered `T_Data_Group`
traffic — the ordinary group-communication path this crate already
supported before T17 — is untouched by any of this and keeps working the
same way it always did; what changed is additive, a second, connected
mode used only by the scan.

---

### 8.6 Is the `Legacy*` matrix reconstructible, or does it need a vendor DLL? — T30 spike (2026-09-13)

**This spike documents. It does not implement, and it touched no bus.** No
socket was opened, no device was read, no device was programmed, and no
`download` function exists anywhere in this section. The product corpus was
copied to a scratch directory outside the repository, measured there, and
deleted; `OriginalData/` was read and never written. Everything below is
either **[D]** (documented in the KNX Standard, with volume and section),
**[D, corpus]** (quoted from the extracted Standard text or from the KNX
Association's *Project Schema Documentation*, because no indexed fact row
carries the sentence), **[V]** (verified locally, with the command), or
**[A]** (assumption, and labelled as one).

#### 8.6.1 The answer

**No, it does not require a vendor DLL. Yes, the matrix is reconstructible
from the product data we already ingest** — for its inventory, its shape, its
value domains, its defaults, and the ordered download sequence it modifies.
There is exactly one bounded exception, and it is about *meaning*, not about
*availability*: the per-flag semantics are undocumented. We can enumerate
every flag and read every value; we cannot cite what ETS does when a given
flag is set.

Three measurements carry that answer:

1. Every `Legacy*` flag is a plain XML attribute on
   `ApplicationProgram/Static/Options` (plus one on `Parameter` and one on
   `MaskVersion/HawkConfigurationData`) inside files the importer already
   opens and already stores byte-for-byte **[V]**.
2. Every ordered download step is declarative data — either the product's own
   `<LoadProcedures>`, or the mask's default `<Procedure>` in
   `knx_master.xml`, or the two spliced together at numbered merge points. In
   the corpus the mask-default procedure for System B reproduces the KNX
   Standard's normative step table *exactly*, which makes the sequence
   checkable rather than guessable (§8.6.4) **[V]** **[D]**.
3. The DLL hook exists but is not load-bearing for the sequence. Five of 35
   application programs declare an `EtsDownloadPlugin` GUID; in every one of
   them the ordered bus sequence is still fully determined by declarative data
   **[V]**. And the Project Schema Documentation says the plugin's other
   historical job has been retired: *"Used to provide richer error messages to
   the ETS user if something fails during download. A plugin is no longer
   required fot [sic] this information."* — **[D, corpus]** *Project Schema
   Documentation* (KNX Association, v01.00.00, 01.03.2024, schema 2.3)
   §1.1.2.12 `LdCtrlErrorCause_t`.

The residual unknown is small, bounded and nameable, and it is written down in
§8.6.7 rather than papered over.

#### 8.6.2 Corpus and method

Eight containers — five `.knxprod` and three `.knxproj`, spanning master-data
schema 11, 20, 21 and 23 — were copied out of `OriginalData/ProductDatabases/`
and `OriginalData/DemoProjects/` into a scratch directory, unzipped there, and
measured. They contain 35 `ApplicationProgram` elements **[V]**:

```
# $S is a scratch directory outside the repository; ex/ holds the extractions
grep -rl '<ApplicationProgram ' "$S/ex" --include='*.xml' | wc -l          # 35
grep -roh '<ApplicationProgram [^>]*LoadProcedureStyle="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'LoadProcedureStyle="[^"]*"' | sort | uniq -c
grep -roh '<ApplicationProgram [^>]*MaskVersion="[^"]*"' "$S/ex" \
  --include='*.xml' | grep -oE 'MaskVersion="[^"]*"' | sort | uniq -c
```

| Dimension | Distribution |
| --- | --- |
| `LoadProcedureStyle` | `ProductProcedure` 28, `MergedProcedure` 4, `DefaultProcedure` 3 |
| `MaskVersion` | `MV-0701` 25, `MV-07B0` 4, `MV-0705` 3, `MV-0012` 2, `MV-0001` 1 |

`LoadProcedureStyle_t` has exactly those three values — **[D, corpus]**
*Project Schema Documentation* §1.1.2.11: *"ETS supports three different
mechanism to specify a device load procedure"*, facets `DefaultProcedure`,
`ProductProcedure`, `MergedProcedure`. The mask decides which management model
applies; `knx_master.xml` states it per mask as
`MaskVersion/@ManagementModel`, and for the masks present here it is
`MV-0001` → `None`, `MV-0012` → `Bcu1`, `MV-0701`/`MV-0705` → `BimM112`,
`MV-07B0` (and `MV-17B0`, `MV-57B0`) → `SystemB` **[V]**.

Single-corpus caveat, same as §1: five manufacturers is not the market. Value
*domains* below are what these files contain, not what the schema permits.

#### 8.6.3 Q1 — What `Legacy*` attributes actually occur

Fourteen distinct `Legacy*`-prefixed attribute names occur, 12 of them on
`Options` **[V]**:

```
grep -rohE '\bLegacy[A-Za-z0-9_]*' "$S/ex" --include='*.xml' | sort | uniq -c | sort -rn
```

| Element / attribute | n | files | Observed values |
| --- | --- | --- | --- |
| `Options/@LegacyAllowPartialDownloadIfAp2Mismatch` | 30 | 30 | `1` ×22, `true` ×6, `0` ×2 |
| `Options/@LegacyAlwaysReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotCheckManufacturerId` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotReportPropertyWriteErrors` | 24 | 24 | `0` ×24 |
| `Options/@LegacyDoNotSupportUndoDelete` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyKeepObjectTableGaps` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNeverReloadAppIfCoVisibilityChanged` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoBackgroundDownload` | 24 | 24 | `0` ×22, `1` ×2 |
| `Options/@LegacyNoMemoryVerifyMode` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoOptimisticWrite` | 24 | 24 | `0` ×24 |
| `Options/@LegacyNoPartialDownload` | 24 | 24 | `0` ×24 |
| `Options/@LegacyProxyCommunicationObjects` | 24 | 24 | `0` ×24 |
| `HawkConfigurationData/@LegacyVersion` | 29 | 8 | `1` ×29 |
| `Parameter/@LegacyPatchAlways` | 7 | 7 | `1` ×6, `true` ×1 |
| `ApplicationProgram/@CreatedFromLegacySchemaVersion` | 27 | 27 | `0` ×24, `true` ×3 |
| `ProjectInformation/@Hide16BitGroupsFromLegacyPlugins` | 2 | 2 | `1` ×1, `true` ×1 |

`Options` carries 30 distinct attributes across 35 elements; 13 are `Legacy*`
and five more are download-relevant without the prefix **[V]**:
`DownloadInvisibleParameters` (`Background` ×15),
`PartialDownloadOnlyVisibleParameters` (`0` ×24),
`PreferPartialDownloadIfApplicationLoaded` (`0` ×24),
`SetObjectTableLengthAlwaysToOne` (`0` ×24),
`LineCoupler0912NewProgrammingStyle` (`0` ×24). The rest are comparison and
text-encoding switches (`DeviceCompareAllowCompatibleManufacturerId`,
`DeviceInfoIgnoreLoadedState`, `DeviceInfoIgnoreRunState`, `Comparable`,
`Reconstructable`, `NotLoadable`, `ParameterByteOrder`,
`TextParameterEncoding`, `TextParameterEncodingSelector`,
`TextParameterZeroTerminate`, `EasyCtrlModeModeStyleEmptyGroupComTables`,
`SupportsExtendedMemoryServices`, `SupportsExtendedPropertyServices`).

**The crucial availability result: which schema writes the full set.** A
standalone `.knxprod` emits only non-default attributes; a *project* export
materialises them all. So the defaults are directly observable, from files we
already have **[V]**:

| Master-data schema | `Options` elements | Distinct attributes | of which `Legacy*` |
| --- | --- | --- | --- |
| 11 | 15 | 26 | 12 |
| 20 | 4 | 7 | 1 |
| 21 | 4 | 2 | 0 |
| 23 | 12 | 25 | 12 |

Measured by grouping each `Options` element by its file's
`xmlns="http://knx.org/xml/project/<n>"`. Two boolean spellings occur —
`0`/`1` in schema 11 and 23, `false`/`true` in schema 20 and 21 — which is not
cosmetic; see §8.6.5.

Every `Legacy*` value seen is boolean. No enumerations, no numeric ranges, no
free text. The matrix is 12 booleans wide on `Options`, plus one boolean per
parameter (`LegacyPatchAlways`) — 13 per application program in total — and
one per mask (`HawkConfigurationData/@LegacyVersion`).

#### 8.6.4 Q3 and Q4 — What the Standard specifies, and where it stops

**On the flags themselves the Standard is silent, and that is a finding.**
Searching the whole extracted KNX Standard v3.0.0 corpus for each name returns
zero files for all 19 of them **[V]**:

```
cd "/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0"
for n in LegacyNoPartialDownload LegacyNoOptimisticWrite LegacyNoMemoryVerifyMode \
         LegacyNoBackgroundDownload LegacyKeepObjectTableGaps LegacyProxyCommunicationObjects \
         LegacyDoNotCheckManufacturerId LegacyAllowPartialDownloadIfAp2Mismatch \
         LegacyPatchAlways LegacyDoNotSupportUndoDelete LegacyDoNotReportPropertyWriteErrors \
         LegacyAlwaysReloadAppIfCoVisibilityChanged LegacyNeverReloadAppIfCoVisibilityChanged \
         LineCoupler0912NewProgrammingStyle PreferPartialDownloadIfApplicationLoaded \
         SetObjectTableLengthAlwaysToOne DownloadInvisibleParameters \
         PartialDownloadOnlyVisibleParameters HawkConfigurationData; do
  printf '%-46s %s\n' "$n" "$(grep -rl "$n" . --include='*.md' | wc -l)"
done   # every line: 0
```

What the Standard *does* do is acknowledge the category. In the System 300
complete download sequence, step 10 is *"optional"* and carries footnote 6:
*"This is an option for the Management Client. For the common tool ETS®, this
can be controlled via a flag in the database entry for the product."* —
**[D, corpus]** `03_05_03 Configuration Procedures v02.01.01 AS`
§3.4.1.2.1, footnote 6 (read from the source PDF, not the Markdown). That is
the whole of it: the Standard says such flags exist and live in the product
database, and declines to name them. Any claim about what an individual
`Legacy*` flag does is therefore **[A]**, and this section makes none.

**On the sequence the Standard is specific, and it matches the data.** The
declarative vocabulary is `LdCtrl*`. Across the corpus, product-supplied
`<LoadProcedures>` use 13 kinds in 629 instances **[V]**:

```
grep -rohE '<LdCtrl[A-Za-z0-9_]*' "$S/ex" --include='*.xml' \
  --exclude='knx_master.xml' | sort | uniq -c | sort -rn
```

`LdCtrlAbsSegment` 155, `LdCtrlTaskSegment` 95, `LdCtrlLoad` 91,
`LdCtrlUnload` 84, `LdCtrlLoadCompleted` 84, `LdCtrlRestart` 28,
`LdCtrlConnect` 28, `LdCtrlDisconnect` 26, `LdCtrlCompareProp` 19,
`LdCtrlTaskCtrl1` 7, `LdCtrlWriteRelMem` 4, `LdCtrlRelSegment` 4,
`LdCtrlMasterReset` 4.

A single schema-23 `knx_master.xml` carries 25 kinds in 1179 instances,
spread over per-mask `<Procedures><Procedure ProcedureType="Load|Unload"
ProcedureSubType="all|ap1|par|grp|par,grp|cfg" Access="remote local2">`
elements — the twelve extra kinds are `LdCtrlWriteMem`, `LdCtrlMerge`,
`LdCtrlWriteProp`, `LdCtrlSetControlVariable`, `LdCtrlLoadImageProp`,
`LdCtrlMapError`, `LdCtrlDelay`, `LdCtrlClearLCFilterTable`,
`LdCtrlLoadImageMem`, `LdCtrlCompareMem`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl2`
**[V]**. `ProcedureType_t` is `{Load, Unload}` and `LdCtrlProcType_t` (the
`AppliesTo` attribute) is `{full, par, grp, full,par, full,grp, par,grp, all,
auto}` — **[D, corpus]** *Project Schema Documentation* §1.1.2.14 and
§1.1.2.10.

**12 of those 25 kinds are named in the Standard; 13 are not.** Documented in
`03_05_03` §3.9.3.2 "Load Control implementation", which gives each one's
implementation as a Management Procedure from `03_05_02`: `LdCtrlConnect`,
`LdCtrlDisconnect`, `LdCtrlRestart`, `LdCtrlUnload`, `LdCtrlLoad`,
`LdCtrlLoadCompleted`, `LdCtrlRelSegment`, `LdCtrlWriteRelMem`,
`LdCtrlWriteProp`, `LdCtrlCompareProp`, `LdCtrlLoadImageProp`, `LdCtrlMerge`
— **[D]**. Two further kinds are documented but absent from this corpus
(`LdCtrlReadProp`, `LdCtrlLoadImageRelMem`). Absent from the entire extracted
Standard corpus: `LdCtrlWriteMem`, `LdCtrlCompareMem`, `LdCtrlLoadImageMem`,
`LdCtrlAbsSegment`, `LdCtrlTaskSegment`, `LdCtrlTaskPtr`, `LdCtrlTaskCtrl1`,
`LdCtrlTaskCtrl2`, `LdCtrlMapError`, `LdCtrlDelay`,
`LdCtrlClearLCFilterTable`, `LdCtrlMasterReset` (12 names, 0 files each), and
`LdCtrlSetControlVariable`, which appears in exactly one file — the *Project
Schema Documentation*, and only as the host of an enumeration **[V]**. That
undocumented group is the absolute-addressing and BCU1/BIM M112 family. Each
of them plainly *names* a documented Management Procedure —
`LdCtrlWriteMem`/`DM_MemWrite` (`03_05_02` §3.16),
`LdCtrlCompareMem`/`DM_MemVerify` (§3.17), `LdCtrlLoadImageMem`/`DM_MemRead`
(§3.18), `LdCtrlMasterReset`/Master Reset (§3.7.1.2),
`LdCtrlDelay`/`DM_Delay` (§3.8) — but that correspondence is inference
**[A]**, not documentation, and the argument encodings are the part inference
cannot supply.

Sample mappings, verbatim from `03_05_03` §3.9.3.2 **[D]**:
`LdCtrlConnect` → `DM_Connect(flags=0)` + `DM_Authorize(flags=0,
key=project_key)`, with the remark that the load control *"will be ignored if
a connection is already established"*; `LdCtrlUnload`/`LdCtrlLoad`/
`LdCtrlLoadCompleted` → `DM_LoadStateMachineWrite(event=…)` plus *"Invalidate
cached base pointer"*; `LdCtrlRelSegment` →
`DM_LoadStateMachineWrite(event=AllocRelSegment)`; `LdCtrlWriteRelMem` →
*"If base pointer not yet determined"* `DM_InterfaceObjectRead(PID_TABLE_-
REFERENCE)` then `DM_MemWrite(...)`/`DM_UserMemWrite(...)` *"depending on
address"*; `LdCtrlWriteProp`/`LdCtrlReadProp`/`LdCtrlCompareProp` →
`DM_InterfaceObjectWrite/Read/Verify`. The Object Indexes are fixed constants
— `OIDX_ADDRESS_TABLE = 1`, `OIDX_ASSOCIATION_TABLE = 2`,
`OIDX_GROUPOBJECT_TABLE = 3`, `OIDX_APPLICATION_PROGRAM_1 = 4`,
`OIDX_APPLICATION_PROGRAM_2 = 5` — and *"ETS will access these Interface
Objects at these Indexes without checking the Interface Object Type"*
(§3.9.3.3) **[D]**.

**The cross-validation, which is the strongest single result here.** The
Standard's §3.9.3.4 "Complete Download" is a 34-row table of load controls.
Two of its rows are annotated *"Only for Mas[k] Version 17B0h"* (row 23,
`LdCtrlWriteProp Object Index="1" PID="53"`, and row 33,
`LdCtrlWriteProp Object Index="0" PID="73"`). In `knx_master.xml`, the
`MV-17B0` `Load`/`all` procedure is **34 steps and reproduces that table row
for row, including both 17B0h-only rows**; the `MV-07B0` and `MV-57B0`
procedures are **the same list with exactly those two rows removed — 32
steps** **[V]**. The `ap1` variant is 29 steps: it drops every `LsmIdx="5"`
step that the table marks *"Not executed if only AP 1 shall be loaded"*, and
brackets the one remaining AP2 unload with
`<LdCtrlMapError OriginalError="3221498632" MappedError="0"/>` … `MappedError=
"3221498632"/>` — precisely where the table's Remarks column says *"Any Errors
here are ignored if only AP 1 shall be loaded"* **[V]** **[D]**. Two
independently produced artefacts, the normative table and the shipped master
data, agree to the row. That is what makes the sequence reconstructible rather
than reverse-engineered.

**Merge points are the documented extension seam.** `03_05_03` §3.9.3.1
defines MergeIDs 1–7 for Mask 57B0h with optional/mandatory status: 1 (O)
pre-download checks, 2 (M) AP1 segment allocation, 3 (M) AP2 segment
allocation, 4 (M) write AP1 application data and parameters, 5 (M) the same
for AP2, 6 (O) post-load property writes, 7 (O) MCB LoadImage records for
differential download **[D]**. The `MergedProcedure` programs in the corpus
supply exactly fragments 2 and 4 and nothing else **[V]**:

```xml
<LoadProcedure MergeId="2">
  <LdCtrlRelSegment LsmIdx="4" Size="256" Mode="0" Fill="0"/>
  <LdCtrlMasterReset EraseCode="4" ChannelNumber="0"/>
</LoadProcedure>
<LoadProcedure MergeId="4">
  <LdCtrlWriteRelMem ObjIdx="4" Offset="0" Size="256" Verify="false"/>
</LoadProcedure>
```

Merge point 2 is where the Standard expects *"the necessary segment
allocation"* on `OIDX_APPLICATION_PROGRAM_1`, and merge point 4 *"the load
controls necessary to write the Application Program data including parameters
for Application Program 1"* — the fragments do exactly that **[D]** **[V]**.
`EraseCode="4"` is `ResetAP`, *"Application Program Memory shall be reset to
the default application when an A_Restart-PDU is received"*, Channel Number
fixed `00h` — **[D]** `03_05_02 Management Procedures v02.01.02 AS`
§3.7.1.2.3.1, Table 4 (read from the source PDF). AP2 fragments 3 and 5 are
absent because these products have no AP2, which the Standard's own remarks
allow: *"If no AP2 is present in the product database entry, errors accessing
the AP2 Interface Object are ignored."* **[D, corpus]** §3.9.3.4.

`LdCtrlSetControlVariable`'s variable set is `{EnableSegmentWrite,
EnableVerifyOnWriteDirect, EnableOptimisticWrite, EnableMemoryAutoVerify}`;
`LdCtrlMemAddrSpace_t` is `{Standard, User, LcSlave, LcFilter}`;
`LdCtrlErrorCause_t` is `{ResourceNotFound, CompareMismatch}` — **[D, corpus]**
*Project Schema Documentation* §1.1.2.8, §1.1.2.9, §1.1.2.12. Three of those
four control variables are near-homonyms of `Legacy*` flags
(`LegacyNoOptimisticWrite`, `LegacyNoMemoryVerifyMode`), which strongly
suggests the flags gate the control variables. Suggests. Not established
**[A]**.

**Pre-download verification is normative and belongs to the tool, not the
sequence.** `03_05_03` §3.9.3.4 requires ETS to read Device Descriptor Type 0
and the Manufacturer Identifier from the installed device and compare them
against the project, and for System B to compare `PID_ORDER_INFO` (identical),
`PID_VERSION` (identical or higher) and `PID_HARDWARE_TYPE` (identical), *"The
comparison is against the product data (parameter value in Application
Program); if no such parameter exists the comparison shall be skipped."*
**[D, corpus]**. In the corpus, all 19 product-side `LdCtrlCompareProp`
instances are `ObjIdx="0" PropId="78"` with 10 distinct `InlineData` payloads
— PID 78 is `PID_HARDWARE_TYPE`, `PDT_GENERIC_06` **[D]** (`03_07_03
Standardized Identifier Tables v01.04.01 AS`). The payloads are
device-identifying and are deliberately not reproduced here; only their shape
is: 20 hex characters, i.e. 10 octets, against a property the Standard defines
as 6 octets. That width mismatch is unexplained by anything found and is
listed as open in §8.6.7.

#### 8.6.5 Q2 — What `knx-productdb` does with all of this today

Traced by reading, then measured by ingesting each `.knxprod` into a scratch
database with the release binary (`cargo build --release -p knx-cli`, then
`./target/release/knx products ingest <file> --product-db "$S/probe.db"`, exit
status 0 each time) **[V]**.

*Stored.* `application_program` has columns `id, manufacturer_id, name,
application_number, application_version, program_type, mask_version, pei_type,
load_procedure_style, default_language, hash, linkable, original_manufacturer,
source_sha256` — the attribute list at
[`crates/knx-productdb/src/parse/program.rs:36`](../crates/knx-productdb/src/parse/program.rs)
(`PROGRAM_ATTRS`). So `load_procedure_style` *is* captured: six programs
ingested, `ProductProcedure` ×5 and `DefaultProcedure` ×1 **[V]**. That is the
one field of the download model we already have.

*Dropped, silently.* The element dispatch in the same file ends in a bare
`_ => {}` at `program.rs:425`, and `Dynamic` is explicitly skipped at
`program.rs:122`. `Options`, `LoadProcedures`, every `LdCtrl*`,
`AbsoluteSegment`, `Extension`/`Baggage` and `Parameter/@LegacyPatchAlways`
therefore never reach a table — and, worse for a project whose first rule is
"never silently discard information", they are not recorded as
`UnknownConstruct` either. `ingest_unknown` holds 123 rows for these six
programs; only three of them touch any of this, and all three are the
attribute `CreatedFromLegacySchemaVersion`. The only unknown *elements*
reported at all are `/Package` (a manufacturer's `Baggages.xml`) and three
`ParameterType/TypeTime` **[V]**:

```
sqlite3 "$S/probe.db" "select kind, count(*) from ingest_unknown group by 1;"
sqlite3 "$S/probe.db" "select kind,name,count(*) from ingest_unknown
  where name like '%Legacy%' or name like '%Option%'
     or name like '%LdCtrl%' or name like '%LoadProc%' group by 1,2;"
```

*Preserved anyway.* `source_file` holds 24 blobs totalling 12 066 503 bytes
for the same ingest — every `Options` attribute and every `LdCtrl*` element is
still there, byte for byte, exactly as
[`crates/knx-productdb/src/blob.rs`](../crates/knx-productdb/src/blob.rs)
promises ("a vendor's `Legacy*` option … is still here, byte for byte"). The
data is not lost. It is merely not queryable, which is the difference between
an archive and a database.

*One measured defect, described and deliberately not fixed here* (this task
may not edit `crates/knx-productdb/**`) — **fixed on 2026-09-13, after this
spike, in the commit that cites this paragraph.** `bool_flag` at
[`crates/knx-productdb/src/parse/mod.rs:34`](../crates/knx-productdb/src/parse/mod.rs)
accepted only `"1"` and `"0"` and mapped anything else to `None`. Schema 20 and
21 spell booleans `true`/`false`, and schema 11 is mixed. Consequence:
`linkable` was `NULL` for all six ingested programs, not because the attribute
was missing but because its spelling was not anticipated **[V]**
(`sqlite3 "$S/probe.db" "select count(*) from application_program where
linkable is null;"` → 6). The same helper would have swallowed six of the 30
`LegacyAllowPartialDownloadIfAp2Mismatch` values for exactly the same reason
if the flags were wired up naively. It now accepts all four of `xs:boolean`'s
canonical spellings — `"1"`, `"0"`, `"true"`, `"false"` — and still returns
`None` for anything else, `"True"` and `"yes"` included, because a lexical
form the datatype does not define is not a boolean this parser may guess at.

*The minimal parsing addition, described and stopped at, as the task
requires.* Three tables would make the matrix queryable without touching the
domain model: `application_program_option(program_id, name, value)` as a plain
key/value strip of `Static/Options` (30 attribute names today, all boolean or
short enum — a narrow typed table would go stale on the next schema bump);
`load_procedure(program_id, merge_id, procedure_index)` and
`load_procedure_step(program_id, procedure_index, step_index, control,
attributes_json)` for `<LoadProcedures>`. Plus routing everything still
unmatched in the `program.rs:425` arm into `UnknownCollector`, so the next
unknown construct is reported rather than inferred from a blob three years
later. No further design work belongs in a research spike.

#### 8.6.6 Q5 — A safe first slice

Two slices, in this order. The first cannot touch a bus at all; the second
only reads.

**Slice 0 — the offline dry-run resolver. No hardware, no socket, testable
today.** Given a device and its application program, resolve the effective
ordered load procedure and print it, without executing anything:

* `DefaultProcedure` → take the mask's `<Procedure ProcedureType="Load"
  ProcedureSubType="…">` from `knx_master.xml` verbatim.
* `ProductProcedure` → take the program's own `<LoadProcedures>`.
* `MergedProcedure` → take the mask default and splice each
  `<LdCtrlMerge MergeId="n"/>` with the program's `<LoadProcedure
  MergeId="n">` fragment; flag a missing mandatory merge point (2, 3, 4, 5 per
  §3.9.3.1) as an error rather than silently emitting a shorter list.

This is verifiable without a device in two ways that both already exist: the
`MV-17B0` resolution must equal the 34 rows of `03_05_03` §3.9.3.4 and the
`MV-07B0`/`MV-57B0` resolution must equal the same list minus its two
17B0h-only rows (§8.6.4), and all 35 corpus application programs must resolve
without an unresolved merge point or an unknown `LdCtrl*` kind. A golden-file
test over the corpus costs nothing and catches every parsing regression. It
also produces the exact artefact a human needs before any bus work is
authorised: a printed, reviewable step list.

**Slice 1 — read-only device inspection, over the bus, never writing.** Every
one of these is a read; none changes a device's state, and none of them is a
step of a download:

| Procedure | `03_05_02` | Reads |
| --- | --- | --- |
| `DM_Identify_R` / `DM_Identify_RCo2` | §3.4.2 / §3.4.3 | Device Descriptor Type 0 (mask version), `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78) |
| `DM_LoadStateMachineRead` | §3.33 | Load State per Interface Object |
| `DM_InterfaceObjectRead` / `DM_InterfaceObjectScan` | §3.27 / §3.28 | `PID_TABLE_REFERENCE` (7), `PID_PROGRAM_VERSION` (13), `PID_MCB_TABLE` (27), `PID_ERROR_CODE` (28), `PID_DOWNLOAD_COUNTER` (30) |
| `DM_MemRead` / `DM_UserMemRead` | §3.18 / §3.21 | Memory, read only |

Together those answer "what is actually on this device, and does it match what
the project thinks" — which is the pre-download verification step of
§3.9.3.4 **[D]**, and is useful on its own as a diagnostic feature long before
anything is ever written. It is also the natural consumer of the T17
line-scan's connection handling (§8.5).

**Hardware this still needs:** a KNXnet/IP interface and at least one
non-critical device on a line that can be isolated from anything that matters,
plus a spare device of a mask we can afford to lose before Slice 2 (which is
not specified here) ever writes a byte. Configuration for any such test uses a
placeholder address such as `192.0.2.1` in committed text. Slice 0 needs
nothing. The rule that address `1.1.220` is never read and never written is
not relaxed by any of this, and neither slice has a reason to go near it.

#### 8.6.7 What this spike did not settle, and what would settle it

1. **Per-flag semantics.** Undocumented everywhere searched (§8.6.4). Closed
   by: the MT6 XSD `KNX-Project-Schema-v23.xsd`, which the *Project Schema
   Documentation* §2 says ships with Manufacturer Tool 6 and may be
   distributed by KNX members, with updates published via `gitlab.knx.org`;
   and failing that, differential testing of ETS against a product whose flags
   are flipped. Note what the same document §1.1 already concedes: it
   deliberately does not document the manufacturer side.
2. **What an `EtsDownloadPlugin` actually does.** Five of 35 programs declare
   one, under two distinct GUIDs. Three of those five pair it with
   `RequiresExternalSoftware="0"` and a `Baggage` DLL reference *and* a
   complete `ProductProcedure` sequence of 25 `LdCtrl*` elements; the other
   two pair it with `RequiresExternalSoftware="1"`, no `Baggage` child, and a
   `DefaultProcedure` BCU1 program containing zero `LdCtrl*` elements — for
   which the mask default in `knx_master.xml` supplies the sequence **[V]**.
   So in this corpus the plugin is never the *source* of the step list. What
   it contributes instead — parameter transformation, image generation, error
   text — is not inferable from the data. Closed by: manufacturer
   documentation, or observing ETS with the plugin disabled.
3. **The `Baggage` DLLs themselves.** Two distinct manufacturer-scoped DLLs
   occur (five copies), `PE32 executable for MS Windows 4.00 (DLL), Intel
   i386`, file versions `1.0.8.0` and `0.3.1.1` timestamped 2005-02-25 and
   2004-03-25 — ETS3-era binaries. Schema 11 declares them
   `InstallOnImport` false (spelled `false` and `0`); schema 23 drops that
   attribute and carries a `FileIntegrity` checksum instead **[V]**. Nothing was executed and nothing was disassembled.
4. **The 10-octet `LdCtrlCompareProp` payload against a 6-octet
   `PID_HARDWARE_TYPE`** (§8.6.4). Closed by: the XSD's definition of
   `InlineData`, or `03_05_01 Resources` on the property's on-wire framing.
5. **Everything about real hardware.** Nothing here was verified against a
   device, because nothing here was allowed near one. Documented is not
   verified, and the thing on the other end of the bus is somebody's wall.

**Consequence for T30.** The two blockers that were stated as research gaps —
the `Legacy*` matrix and vendor-DLL-driven sequences — are no longer research
gaps of the kind that stop work. The matrix is enumerable from data we hold,
the sequence is documented and cross-checkable, and the DLL is not on the
critical path. What remains is engineering (a parsing addition, §8.6.5 — its
`bool_flag` prerequisite was done on 2026-09-13), one bounded unknown (per-flag
semantics, which Slice 0 does not need), and hardware. See
[KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked).

### 8.7 Commissioning and download — the specification pass, T30 phase 1 (2026-09-13)

§8.4 asked nine questions and answered them from indexed fact rows. §8.6 asked
whether the product-data half needed a vendor DLL and concluded no. This pass
did something different and narrower: it read the source PDFs clause by clause
to produce an **implementable** specification —
[`docs/superpowers/specs/2026-09-13-commissioning-download-design.md`](superpowers/specs/2026-09-13-commissioning-download-design.md).
No code was written and no bus was contacted. What follows is only what this
pass *established or corrected*; the specification document carries the full
citations.

**Eight** source PDFs were quoted: `03_03_07 Application Layer v02.01.01 AS`,
`03_05_01 Resources v01.10.01 AS`, `03_05_02 Management Procedures v02.01.02
AS`, `03_05_03 Configuration Procedures v02.01.01 AS`, `03_03_04 Transport
Layer v01.02.03 AS` (the last via §8.5's already-recorded clause 4 figures), and
— added in the same day's fix round, §8.7.10 to §8.7.13 —
`03_07_02 Datapoint Types v02.02.01 AS`, `AN194 v02 Master Reset of Resources
AS` and `06 Profiles v02.01.01`.

#### 8.7.1 Method, and why it changes what is citable

`pdftotext -layout` on the **source PDFs** produces clean, row-by-row readable
tables where the Markdown extraction §8.4 assessed produces mush. This matters
for one specific claim. §8.4 says of `03_03_07`'s Table 1, the APCI code table,
*"No APCI value can be read out of that safely."* **That is true of the
Markdown extraction and not of the PDF.** `pdftotext -layout` renders Table 1
one row per line with its ten bit columns intact. Spot-checked against this
project's own shipped code: `A_DeviceDescriptor_Read` reads as `1100000000` =
`0x300`, which is exactly what `crates/knx-net/src/cemi.rs` encodes as
`0x300 | descriptor_type`. **[V]** 2026-09-13.

The practical rule for anyone extending this research: query the knowledge bases
to *find* the clause, then read the clause out of a `-layout` extraction of the
PDF. The bases point; the PDF quotes. §8.4's "reusable pointer" still holds —
this is an addition to it, not a replacement.

One extraction artefact, recorded so nobody files it as a spec gap: in
`03_03_07` the heading of §3.5.4 renders as `..  _ emory_rite-service` while the
clause body extracts intact. The body is what was quoted. **[V]**

#### 8.7.2 The Load State Machine transition table exists and is complete

§8.4 Q2 had the states and the events but not the transitions. `03_05_01
Resources` **Table 94 – Load State Machine transition table** (PDF page 296)
gives all 6 states × 5 load events plus a `Device Restart` row, with the legend
*"I: intermediate state / M: mandatory / O: optional / R: recommended"*. It is
reproduced in full in the design spec §5.4. **[D]**

Four facts fall out of it that the implementation is built on:

- **`Error` is a trap.** Every load event except `Unload (04h)` leaves the state
  at `Error`. A retry loop that re-sends `Start Loading` after an error never
  terminates.
- **`Start Loading` from `Loaded` goes straight to `Loading`**, and `03_05_01`
  Table 92 says *"Only in this state [Loaded] the associated data shall be
  considered as valid; in all other states the data shall be considered as
  invalid"*. So beginning a download invalidates a working configuration before
  any payload is sent. There is no stage-then-commit semantics anywhere in the
  procedures read.
- **`Device Restart` while `Loading` is `R: Loading` / `O: Error`**, and
  `Device Restart` from `Loaded` is *"Loaded (Error in case of error detection at
  start-up)"*. Combined with §4.23.1's *"The state of the Load State Machine
  shall be stored in non-volatile memory"*, this answers the interrupted-download
  question outright: an interrupted download leaves a persistently invalid part,
  and power-cycling does not fix it.
- **A device may legitimately not answer at all** in state `LoadCompleting` —
  Table 94's footnote: *"a device may be offline during state LoadCompleting and
  therefore not react to load events from a MaC"*. Treating that silence as
  failure and retrying drives a correct device into `Error` (`R: Error` from
  `LoadCompleting`).

#### 8.7.3 Correction to §8.4: `PID_OBJECT_INDEX` is not the LSM selector

§8.4 states each LSM is *"addressed independently by `object_index`
(`PID_OBJECT_INDEX`, PID = 29)"*, and its PID table row 29 reads *"Addresses
which Interface Object/LSM a given access targets"*. That conflates two
different things.

`03_05_01 Resources` §4.2.29: `PID_OBJECT_INDEX` is **read-only** and *"shall
contain the local Object Index of the Interface Object in which it is located"*
— a property a device reports about itself. **[D]** The thing that selects
which Interface Object a property access targets is the **`object_index` field
of `A_PropertyValue_Read`/`_Write`** (`03_03_07` §3.4.4), not PID 29. The
earlier rows are left in place above rather than rewritten, with this
correction as the authority; the design spec §3.2 states it for implementers.

#### 8.7.4 Correction to §8.6: the Additional Load Control encodings are documented

§8.6.4 concluded that *"the argument encodings are the part inference cannot
supply"*. That is too strong. `03_05_02 Management Procedures` §3.31.3
specifies `DM_LoadStateMachineWrite_RCo_IO` as **exactly 10 octets** written to
`PID_LOAD_STATE_CONTROL` (property_id 5, start_index `01h`, nr_of_elem `01h`) —
`01h`/`02h`/`04h` + 9 × `00h` for Start Loading / Load Completed / Unload, all
zeros for No Operation, and `03h` + subtype + 8 octets for Additional Load
Controls — with field tables for subtypes `00h` AllocAbsDataSeg, `01h`
AllocAbsStackSeg, `02h` AllocAbsTaskSeg, `03h` TaskPtr, `04h` TaskCtrl1, `05h`
TaskCtrl2, `0Ah` Relative Allocation and `0Bh` Data Relative Allocation. **[D]**

What remains undocumented is narrower than §8.6 stated, and worse in one
respect. The **`LdCtrl*` element names** still return zero hits across the
extracted Standard corpus, so the mapping from the 13 unmatched product-data
kinds to these subtypes is the gap — not the payload layouts. And the hazard is
sharper than an error message: **[D]** §3.31.3 on subtype `0Ah`, *"If the
requested number of octets is not supported by the Management Server (device)
then the Load State Machine of the loadable part shall change to error."* Per
§8.7.2, `Error` is escapable only by `Unload`, and **[D]** Table 93 says unload
makes the data *"undefined"*. A guessed subtype destroys a configuration
silently rather than failing loudly.

#### 8.7.5 The write chunk size is 12 octets, not 63

`03_03_07` §3.5.3/§3.5.4 specify `A_Memory_Read`/`_Write` for *"between 1 and 63
octets"*. `03_05_02` §3.16 `DM_MemWrite` adds the constraint that actually
governs: *"If the Management Server does not support the L_Data_Extended frame
format, then this maximal size shall be 12 octets."* **[D]**

So the service permits 63 and a large real-device population permits 12. How a
client discovers whether a device supports `L_Data_Extended` is **not stated in
§3.16** — the clause states the consequence, not the discovery mechanism. The
first draft of this subsection concluded from that that the discovery mechanism
was undocumented. **That was wrong, and the fix round of the same day corrected
it: it is documented, in `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH`.** See §8.7.13.
The default of 12 stands anyway, because it is what the Standard prescribes when
the property is absent and because the failure mode of guessing high is a
partially written memory region that the device may report as successfully
`Loaded`.

Two adjacent facts from the same clause, both design-relevant.

First, **corrected 2026-09-14**: this paragraph used to read *"the read-back is
normative, not a precaution this project invented"*, citing `03_03_07` §3.5.4's
*"The value of the associated memory area shall be explicitly read back after
writing to it."* Quoted with its context, that sentence sits **inside the
active-Verify-Mode paragraph** and constrains the **device**, which is why
`A_Memory_Write.res` carries *"data: the octet(s) read back or no data"* rather
than an echo of the request. The preceding sentence is **[D]** *"With inactive
Verify Mode the remote application process shall not respond."* — so with Verify
Mode off, §3.5.4 obliges nobody to read anything back, least of all the client.
The design spec's read-back rule is therefore **[A] this project's design rule**,
kept because a device that silently drops a write is indistinguishable from one
that took it, and recorded that way in spec §6.2 and §13 **R6**. The rule did not
change; its authority did.

Second, `03_05_02` §3.16's non-verify procedure inserts a *"delay for programming
the memory in the device"* which is **named and never quantified** anywhere
searched — see `GAP-T30-07` in §8.7.15, including the reference chase that failed
to close it. Undocumented.

#### 8.7.6 Verify Mode dies with the connection

`03_05_01` §4.2.14 Table 11: `PID_DEVICE_CONTROL` (PID 14, `PDT_BITSET8`, DPT
21.002) bit 2 is Verify Mode On; §4.2.14.7.3/§4.2.14.7.4: the default is 0, the
client must set it actively, and it is **automatically disabled when the
Transport Layer connection closes**. **[D]**

That collides with §4.23.2.4.1's requirement to re-establish a broken TL
connection periodically during a long load transition: a download that
reconnects mid-flight silently loses verification unless bit 2 is re-asserted
per connection. This is the sort of defect that produces no symptom until it
produces a corrupted device.

#### 8.7.7 Programming mode may switch itself off, and can be switched remotely

`03_05_01` §4.26 specifies an **optional** autonomous inactivation: *"if the
Programming Mode becomes enabled, by any means, the MaS (device) shall start a
time-out timer of 4 minutes … If the timer expires, the MaS shall autonomously
and automatically disable its Programming Mode."* **[D]** Optional means a
client may neither rely on it nor assume its absence, so programming mode must
be re-verified immediately before a broadcast address write rather than once at
the start of a wizard.

`03_05_02` §3.13 `DM_ProgMode_Switch(flags, mode)` switches it remotely —
*"mode 0: switch Programming Mode off, 1: switch Programming Mode on"*,
*"flags — All bits are reserved. These shall be set to 0."* **[D]** Procedure
`DMP_ProgModeSwitch_RCo` implements it as a read-modify-write of one octet at
memory address `60h` (*"the state of the Programming Mode is located at memory
address 60h"*), where *"bit 0 has to be set according to the mode. The parity
(bit 7) has to be calculated."*

**Correction, 2026-09-14.** This section previously ended "The parity
**computation** is not specified beyond that sentence … Undocumented." That was
wrong, and it was wrong while citing the clause that contradicts it. `03_05_01`
§4.26.3.1 specifies the rule, and it is a rule about *change*, not about
absolute value: **[D]** *"if the value of prog_mode is changed from "0" to "1" or
from "1" to "0" then the variable p_parity shall be inverted."* §4.26.3.4.1
repeats it as a client obligation: **[D]** *"The variable p_parity shall be
inverted, if the value of prog_mode is changed."* Location is prose as well as
figure: **[D]** §4.26.3.2 *"The location of curr_prog_mode shall be the memory
address 0060h."*

So the whole octet is `old XOR 0b1000_0001` — invert bit 0, invert bit 7, carry
bits 1 to 6 through untouched, which matters because §4.26.3.1 calls them
*"the shared bits"* of a Resource that *"may share its storage location with
other data on the same memory location inside the device"*, and adds that
*"These other data may be different depending of the mask version of the
device."* (The earlier revision of this paragraph rendered that as
*"can be shared with other functionality"*, which is not a sentence RES
§4.26.3.1 contains; corrected in fix round 3 against `pdftotext -layout` of the
PDF.) Inverting rather than recomputing
means the odd-versus-even convention never has to be known, so the one residual
unknown is not on the critical path. The write must be skipped entirely when
bit 0 already matches the requested mode — §4.26.3.4.2 and §4.26.3.4.3 guard the
procedure with `if prog_mode = 0` / `if prog_mode = 1`, and a no-op toggle would
flip the parity against an unchanged `prog_mode`, whose reaction is **[D]**
§4.26.3.3 *"manufacturer specific"* (footnote 96: *"Typically the system is
restarted if p_parity is invalid."*).

**Second correction, 2026-09-14 (fix round 3, narrowed in round 4): none of the
above is *established* for System B, and it is not excluded either.** `03_05_01` §4.26.3 carries a scoping header between its title and
§4.26.3.1, and the previous revision used the clause without reading it:
**[D]** *"Used by: − Ctrl-Mode fixed DMA − Ctrl-Mode reloc DMA − masks 0012h
0020h, 0021h, 0701h in E-Mode"*. System B is absent — and so is every mask in the
`07B0h`/`17B0h`/`57B0h` family the download spec targets. Absence from that list
is weak evidence on its own, because the list is not exhaustive: `06 Profiles`
§5.5.1.1 profiles S-Mode **couplers** onto *"§4.26.3 “Programming Mode –
Realisation Type 2”"* and couplers are not in the list either. The decisive
evidence is positive and points the other way, in `06 Profiles` §4.4.1.1
(Device Individualisation → Programming Mode → connection oriented), which splits
into two lettered alternatives each naming its profiles: **[D]**
*"a) Realisation Type 1 - Property based • System B • Mask 57B0h"* and
**[D]** *"b) Realisation Type 2 – Memory mapped • System 1 • System 2 • BCU 1
• BCU 2 • BIM M112"*. Both columns covering the target — `System B` and
`Mask 57B0h` — are Realisation Type 1, i.e. `PID_PROGMODE` (`03_05_01` §4.3.5).

That assignment reaches exactly as far as §4.4.1.1's own title, *"connection
oriented"*, which round 4 added after a review caught the over-correction. The
clause next door goes the other way: **[D]** `06 Profiles` §4.4.1.2 *"Programming
Mode – connectionless"* lists, under *"Programming Mode Control"* — where, unlike §4.4.1.1 a) and b), no
*"• via bus:"* sub-bullet appears; the only bullet is *"• via HMI: device selection
and indication of Programming Mode"*, and the two service references follow it
directly — both *"§3.13.2 “DMP_ProgModeSwitch_RCo”"* and *"§4.26.3 “Programming
Mode – Realisation Type 2”"*. The second is reached twice over: **[D]** MP §3.13.2
itself says *"The Programming Mode shall be realised as “Programming Mode –
Realisation Type 2” as specified in [05]."*, which needs no layout interpretation
at all. And §4.4's feature table gives row *"1.b Connectionless"* the value `O`
in the `System B` and `Mask 57B0h` columns **[D]** (`-` in the System 1, System 2,
BCU 1, BCU 2, BIM M112 and Mask 5705h columns; `O` for System 300 and `M` for RF
Bidirectional). So a System B device may optionally implement the connectionless
path, and that path is profiled onto the very clause that describes `0060h`.

Consequences, recorded so no later pass re-derives them: the `60h` read-modify-write
above is correct *for the profiles its source clauses name*; for a System B device
it is **unestablished** — not assigned on the mandatory connection-oriented path,
optionally assigned on the connectionless one — so what the octet at `0060h` holds
on such a device is **[A]** unknown, which is a gap in *interpretation*, not a
proof of absence. The design spec's phase-3 permission for
`A_Memory_Read(60h, 1)` inside `1.1.24`–`1.1.32` is unchanged, because a one-octet
read is harmless; what changed is that the result must be recorded as a raw octet
of unknown meaning rather than as programming-mode state (spec §3.4, §4.4, §15).
This is not a new gap — the phase-3 read establishes the device's own Profile
before anything interprets the octet — and the gap count stays at six. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf` §4.26.3 and of
`06 Profiles v02.01.01.pdf` §4.4 (feature table), §4.4.1.1, §4.4.1.2 and §5.5.1.1.

The design spec still forbids writing to `60h` on real hardware — but now as an
explicit project risk decision (spec §13 **R11**, restated as "writing to `60h`
at all on a real device"), not as a consequence of not knowing how. Knowing how
removed an accidental protection, so the refusal had to be made deliberate:
spec §4.4 keeps the write inside the mutation API, §15 keeps it a non-goal in
every phase, and phase 2 exercises it against the simulator only. Source:
`pdftotext -layout` of `03_05_01 Resources v01.10.01 AS.pdf`, §4.26.3.1 to
§4.26.3.4.3.

#### 8.7.8 Smaller findings

- **LSM Realisation Type 2 is not specified at all.** **[D]** `03_05_01`
  §4.23.3: *"This Realisation Type is not specified in this version of this
  document."* Not a gap in our search — a gap in the Standard.
- **`PID_ERROR_CODE` must be read before unloading.** **[D]** `03_05_01`
  §4.2.28: *"When the load state changes from 'Error' to a different state then
  the Error Code shall be set to '0' (no error)."* Unloading to recover destroys
  the only evidence of what failed.
- **Allocation outside `Loading` is silently ignored.** **[D]** `03_05_03`
  §3.5.1.2 Table 4: *"An allocation (if necessary) shall occur if a segment is
  in the state Loading. In all other states the memory allocation shall be
  ignored."* Combined with `PID_REFERENCE` reading zero on failed allocation,
  zero has two distinct meanings.
- **The 64 kB / 1 MB service split is decided on `base + length`.** `03_05_03`
  §3.5.1.4 Table 5 gives the limits (`A_Memory_Write` ≤ 64 kB,
  `A_UserMemory_Write` ≤ 1 MB, `PID_TABLE_REFERENCE` 20 bits) and §3.5.2's step
  list gives the rule: *"if BaseAddress plus allocated memory is lower than
  FFFFh"* → `MemoryWrite`, else `UserMemoryWrite`. **[D]** Switching on the base
  address alone writes a segment's tail through the wrong service.
- **A failed partial allocation escalates to a larger download.** `03_05_03`
  §3.5.3: when the base address reads zero the procedure does not abort, it
  *"⇒ Continue at Nr. 07"* — unload all following segments and reload them,
  because *"Due to the ascending order of the memory segments it can be
  necessary to rearrange all or a subset of the segments when modifying one
  segment."* **[D]** The *"differential download algorithm"* that a matching CRC
  enables is **named and not specified**. Undocumented.
- **Unload ends with an address erase.** `03_05_03` §3.5.4 step 07: *"Set
  SerialNumber_IndividualAddress_Write(FFFFh) via broadcast"*. **[D]** Makes a
  device unaddressable by individual address. Documented here so it is
  recognised, explicitly not implemented.
- **Master Reset Erase Codes are fully enumerated** in `03_05_02` Table 4 —
  `01h` Confirmed Restart through `08h` Erase persistently stored application
  data, with `00h` reserved and `09h`–`FFh` rejected with *"Unsupported Erase
  Code"*. **[D]** `01h` is a confirmed no-op restart and is the restart a client
  should prefer over the unconfirmed `restart_type = 0`. `02h` Factory Reset's
  effects are *"implementation dependent"*, which means this application can
  never tell a user what a factory reset will do to their device.
- **A restart may or may not drop the connection.** **[D]** `03_03_07` §3.4.2.2:
  *"The Management Server (device) may or may not break down the Transport Layer
  connection."* Client-side session state after a restart is unknown by
  specification, not by accident.
- **Cross-reference drift.** `03_05_01` Table 93 points at *"clause 3.27
  'DM_LoadStateMachineWrite' in [09]"*, but in the current `03_05_02` that
  procedure is **§3.31** and §3.27 is `DM_InterfaceObjectRead`. Follow the name.

#### 8.7.9 What this pass deliberately did not read — closed the same day

The first version of this subsection listed four documents as readable but
unread. A reviewer's ruling, which is worth recording because it is a good rule:
*a gap that says "undocumented in the Standard" is a finding; a gap that says
"documented, but I did not read it" is unfinished work.* All four were then read.
What each contributed is §8.7.10 to §8.7.14 below, and the design spec's §10,
§5.6, §8.2 and its per-mask notes. Nothing in the T30 phase 1 material now rests
on an unread document.

#### 8.7.10 The authorisation model, in full

`03_03_07` §3.5.7 `A_Authorize_Request-service` specifies the whole model and it
is small enough to state completely. **[D]**

- A key is *"four octets long and of data type unsigned32"*. Access levels are
  *"(unsigned8) between 0 (maximum level, i.e., maximum access rights) and 3
  (minimum level …) or 0 … and 15 (minimum level …)"* — **lower is more
  powerful**, and both ranges exist.
- Lifetime: *"A current access level shall be valid until the connection is
  released or a new key is indicated with the A_Authorize_Request service."* So
  authorisation is **per connection**, not per operation, and every reconnect
  must re-authorise — the same structural trap as Verify Mode in §8.7.6, and for
  the same reason.
- Not authorising is **not** an error: *"if the communication partner does not
  authorize itself, the Remote Management shall select the maximum access level
  protected with FFFFFFFFh as the current access level."*
- Authorising with a wrong key is **worse than not authorising**: *"if the
  communication partner authorizes itself with an invalid key, the Remote
  Management shall select the minimal access level (this is level 3 or level
  15)"*. There is no negative response. A guessed key therefore silently reduces
  the access the client would have had by doing nothing — which is why the design
  spec forbids sending any key the user did not supply.

`03_05_02` §3.5 `DM_Authorize (flags, keys)` wraps it, requires `DM_Connect`
first, runs *"only when it is required by the Management Server"*, and says the
support question is answerable up front: *"Whether or not a Management Server
supports authorisation can directly be retrieved from the Device Descriptor Type
0 (mask version)."* Its `_RCo` sequence guards the exchange with
*"if authorization is required (key != FFFF FFFFH)"*, making `FFFFFFFFh` the
client-side "no key configured" sentinel.

§3.5.2 `DM_Authorize2_RCo` handles the asymmetry above: it *"does not presume
that the device has been locked with the key that is provided to the procedure.
Therefore, it authorizes subsequently with the key FFFFFFFFh and with the key
client_key and continues with the key that gives the maximal access rights."* Its
error handling is absolute — *"Failure of any of the contained Application Layer
Services shall lead to failure of the entire Configuration Procedure."* — which is
the safe direction: step 03 failing means the destructive step 05 never runs.

**Scoping, corrected 2026-09-14.** This paragraph used to call §3.5.2 *"the
procedure a real client should run"*. It is not unconditionally that. The clause
carries a *Use • Profiles* row naming **System 2** and **BIM M112**, while the
download this project specifies is `03_05_03` §3.5.2 **System B**; §3.5.1
`DMP_Authorize_RCo` carries no profile restriction. Claiming §3.5.2 as the
default would be asserting cross-profile applicability that no clause states, so
the design spec §10.4 rules the other way: **MP §3.5.1 is the default for System B
and §3.5.2's two-key comparison is a defensive extension, off by default**, with a
regression test asserting that the default issues exactly one
`A_Authorize_Request` per connection and never the `FFFFFFFFh` probe.

Key writing is `03_05_02` §3.6 `DM_SetKey (flags, keys, level)` over `03_03_07`
§3.5.8 `A_Key_Write`: *"Every device shall be able to handle exactly one key per
access level. The number of access levels supported by a device is Profile
dependent."*, `FFFFFFFFh` invalidates a level's key, and you cannot grant what
you do not hold — *"The current access level shall be less or equal to the access
level indicated … otherwise the remote application process shall return FFh"*.
Not implemented in this project, and the design spec says why: a key write can
lock the client out of its own reach.

How insufficient access **presents on the wire** is the part that matters for
error reporting, and it splits by service family: classic memory and property
services fail silently (`03_03_07` §3.5.4: *"the service indication shall be
ignored"*), while the extended services carry `FCh E_ACCESS_DENIED` in their
return-code tables (`03_03_07` §3.4.5.5, §3.4.8.3, §3.4.9.1, §3.4.9.2). **[D]**
So an unauthorised classic write is indistinguishable from a lost frame, and the
level has to be established before writing rather than diagnosed after.

#### 8.7.11 `DPT_ErrorClass_System` 20.011, enumerated from its own PDF

`03_07_02 Datapoint Types v02.02.01 AS`, DPT ID **20.011**,
`field1 = ErrorClass_System`, `Range: [0 to 18]`, encoding `N8`: 0 no fault,
1 general device fault, 2 communication fault, 3 configuration fault,
4 hardware fault, 5 software fault, 6 insufficient non volatile memory,
7 insufficient volatile memory, 8 memory allocation command with size 0
received, 9 CRC-error, 10 watchdog reset detected, 11 invalid opcode detected,
12 general protection fault, 13 maximal table length exceeded, 14 undefined load
command received, 15 Group Address Table is not sorted, 16 invalid connection
number (TSAP), 17 invalid Group Object number (ASAP), 18 Group Object Type
exceeds (PID_MAX_APDU_LENGTH – 2), and *"19 to 255 : reserved, shall not be
used"*. **[D]** Verbatim in the design spec §5.6, which is the copy the DPT
main-type-20 codec should use.

Two things fall out of it. **Values 6, 8, 9, 13 and 14 are download
diagnostics** — they say the *client's* request was wrong, which is what resolves
§9.2's "state is `Error` and I do not know why". And **value 15 is a validity
rule for the download itself**: `03_05_01` §4.16.3 requires *"The Group Addresses
shall be sorted in ascending order with increasing memory locations."*, so a
client that writes an unsorted Group Address Table manufactures a runtime error
report. For contrast, the neighbouring `20.012 DPT_ErrorClass_HVAC` has
`Range: [0 to 4]`; the reserved boundary differs per sub-type and cannot be
shared across main type 20.

#### 8.7.12 `AN194`: what a reset does to which resource

`AN194 v02 Master Reset of Resources AS` tabulates, per Interface Object and PID,
the effect of each reset flavour, in columns `-` Local Reset, `02h`, `07h`, `01h`
Confirmed Restart, `none` Basic Restart, and Power Cycle. **Six columns, of which
exactly three are Erase Codes** (`02h`, `07h`, `01h`); Local Reset, Basic Restart
and Power Cycle are not Erase Codes at all. "All six columns" is therefore a
correct paraphrase of a full row and "all six Erase Codes" is not — there are
only three of those. Its vocabulary has to
be read first, because "default" means four things: *"Not influenced"* (the
client may rely on the value not changing), *"recalculate"* (*"The MaC shall read
the value before using it."*), *"KNX default"* (a value *"not specified in the
KNX Specifications"* that the client may nevertheless assume), and
*"implementation default"* (*"If the value is given in the product description
then the MaC may rely on this; otherwise, the MaC has to read the value."*).
**[D, corpus]**

The rows that matter here, from §2.3.2.2 (Device Object) and §2.3.2.3 (Address
Table Object):

- `PID_LOAD_STATE_CONTROL` and `PID_ERROR_CODE` are *"implementation default"*
  after a local reset, `02h` or `07h`, and **`"not influenced"` by Confirmed
  Restart, Basic Restart and Power Cycle**. That is an independent confirmation
  of §8.7.2's conclusion from a different document: an interrupted download is
  not repaired by power-cycling the device.
- `PID_DEVICE_CONTROL` (Verify Mode) and `PID_PROG_MODE` are *"KNX default"* in
  **all six** columns — so Verify Mode does not survive a power cycle, and
  programming mode is off after any restart, which is a second cause for
  §8.7.7's "it may be off when you get there".
- `PID_TABLE_REFERENCE` is *"recalculate"* — but **not uniformly, and the
  difference is the interesting part.** Read per Interface Object rather than as
  one row (correction, 2026-09-14; the earlier single bullet flattened five
  objects into one claim):
  - §2.3.2.3 Address Table Object and §2.3.2.10 Object Type 9 Group Object Table:
    *"recalculate"* in the `-`, `02h` and `07h` columns — and that set is one
    reset kind plus two Erase Codes, not three Erase Codes, because `-` is
    AN194's *"Local Reset to default state"* and carries no Erase Code — and
    *"not influenced"* in the `01h` (Confirmed Restart), Basic Restart and Power
    Cycle columns.
  - §2.3.2.4 Association Table Object, §2.3.2.5 Application Program Object and
    §2.3.2.6 Application Program 2 Object: *"recalculate"* in **all six**
    columns, Local Reset, Erase Codes and restarts alike.

  The design rule that survives both shapes is the stronger one: re-read
  `PID_TABLE_REFERENCE` after **any** restart, not merely after a reset, because
  the Local Reset column is *"recalculate"* in every one of the five objects. A
  cached base address is invalid in all cases; only the *reason* differs.
- `PID_RUN_STATE_CONTROL` is *"recalculate"* in §2.3.2.5 and §2.3.2.6 in **all
  six** columns — Local Reset, `02h`, `07h`, `01h` Confirmed Restart, Basic
  Restart and Power Cycle. (Statement *extended* in fix round 3: the rows read say
  all six, and the earlier wording named only Local Reset plus the Erase Codes,
  which was true but narrower than the evidence.) So the run state of an
  application program is never inferable after a reset **or after any restart**.
- `PID_MAX_APDU_LENGTH` is marked CONSTANT and *"not influenced"* everywhere —
  the one value in this area that may legitimately be cached per device.
- `PID_DOWNLOAD_COUNTER` is *"recalculate"*, so it is not a "has this device been
  touched" fingerprint across a reset.

Three limits AN194 sets on any client, all quoted: *"The interpretation of this
term is implementation specific"* (Power Cycle — so no test plan step may be
"power-cycle it"); *"The below defined Erase Codes “ex-factory” shall thus be
understood as “set back a default state” rather than “set back to the delivery
state”."*; and *"There are no requirements on whether or not an application may be
running in the MaS after a Master Reset"*. The last one is the sharpest: a device
that reports `Loaded` after a factory reset may be conformant, so device state
must always be read and never inferred from an operation performed.

**What was not read**, stated so a later reader can tell "AN194 says nothing"
apart from "nobody looked": six of AN194's twelve Interface Object sections were
transcribed (§2.3.2.2 Device, §2.3.2.3 Address Table, §2.3.2.4 Association Table,
§2.3.2.5 Application Program, §2.3.2.6 Application Program 2, plus the
§2.3.2.10 Group Object Table rows named above). The remaining six Interface
Object types in the document — §2.3.2.7 Router, §2.3.2.8 LTE Address Routing
Table, §2.3.2.9 cEMI Server, §2.3.2.11 KNXnet/IP Parameter, §2.3.2.13 Security
Interface and §2.3.2.14 RF Medium — were not transcribed, and neither was
§2.3.2.12 *"Data Security"*, which is a three-Resource table rather than an
object type. (Counts and the object list corrected in fix round 3: the prose said
five while the list named six, "fourteen" was a miscount of a thirteen-clause
range, and the earlier "among them" list invented a Polling Master and a File
Server object that AN194 does not tabulate.) They were skipped
because the download procedure this pass specifies does not
touch them. Three PIDs were checked across every object that was read and agree
everywhere: 5 `PID_LOAD_STATE_CONTROL`, 28 `PID_ERROR_CODE` and 27
`PID_MCB_TABLE`.

#### 8.7.13 Correction: the `L_Data_Extended` discovery mechanism **is** documented

§8.7.5 first recorded it as undocumented. It is `03_05_01` §4.3.7
`PID_MAX_APDU_LENGTH` (PID 56, `PDT_UNSIGNED_INT`), whose §4.3.7.2.1 states:
*"A Management Client supporting the L_Data_Extended-frame has to check this
value before starting download. If the PID_MAX_APDU_LENGTH is not present in the
Device Object, then the Management Client shall manage the device with
L_Data_Standard-frames with an APDU-length of maximal 15 octets."*, with the
value *"in the range between 15 and 254"* and 255 reserved as an ESCape Code.
**[D]**

That also explains where the 12 comes from: 15-octet APDU minus the 3 octets of
`A_Memory_Write` overhead (`03_03_07` §3.5.4 ignores a request with
`number > Maximum APDU Length – 3`) is exactly 12. The constant is arithmetic,
not folklore.

Two adjacent traps from the same clause: the property must be read from the
**Device Object**, because *"If PID_MAX_APDU_LENGTH is solely in the Router
Object …, then the device shall only support L_Data_Extended-frames for Routing
and only L_Data_Standard-frames for Management."*; and `PID_EXT_FRAMEFORMAT` is
**not** the mechanism — `03_05_01` §4.16.7.2.5: *"This Property is reserved for
LTE-Mode devices."*

#### 8.7.14 `06 Profiles`: no cross-LSM ordering, and five constraints that do exist

The question §8.7.9 originally deferred was whether `06 Profiles v02.01.01`
states an ordering requirement between the Load State Machines of different
loadable parts. **That document does not — and, corrected 2026-09-14, that is a
weaker result than it first looked, because the Standard explicitly delegates the
question to per-device Profiles.** `03_05_01` §4.23.2.4.1: **[D]** *"If a device
contains more than one Load State Machine, dependencies between these Load State
Machines have to be defined in the Profiles (see [17]) of these devices."* RES's
reference `[17]` is Volume 6, "Profiles". So a rule is *expected* to exist; it is
just expected to live per device.

What was searched, so the negative is auditable rather than asserted:
`pdftotext -layout` of `06 Profiles v02.01.01.pdf`, swept for ordering and
sortedness language, plus keyword queries against both knowledge bases
(`knx_spec_kb_programming.sqlite` and `knx_spec_kb_full179_clean.sqlite`) which
returned `[]`. The one grep hit was about sorting table *contents*, not load
order. **What was not read:** the per-mask Profile documents where such a rule
would actually live — `06_01_33` (mask `2705h`), `06_01_35` (mask `27B0h`),
`06_02_42` (mask `2920h`), `06_03_31` (mask `2311h`). Absence of a rule in the
general Profiles document is therefore not absence of a rule for any given
device, and the design spec's phase 2 never reorders loadable parts on the
strength of this negative. The spec's use of `03_05_03` §3.5.2's one concrete
System B order stands, and so does its refusal to generalise that order to other
masks — the absence of a general rule is not permission to invent one.

What `06 Profiles` does constrain, and the design spec now records:

- **One Table 94 cell is narrowed per mask.** §5.3 footnote a: *"The Load State
  Machine transition table allows an optional transition from state “Loaded” to
  “Error” in case of an event “Load Completed”. This is not allowed for mask
  version 0912h Couplers. Mask 0912h shall stay in state “Loaded” in case of an
  error."* **[D, corpus]** So the permitted-outcome set for a cell with
  alternatives is narrowed by mask, never widened.
- **Which Load Controls a device must support is per mask.** Annex A.2.4.1,
  *"Table 7 – Required Load Controls"*: absolute allocation and the four record
  subtypes are `M` across the System 2 and System 300 columns and `n/a` for mask
  `57B0h`; `0Ah` Relative Allocation is `M` for mask `0300h`; `0Bh` Data Relative
  Allocation is `M` for `07B0h`, `17B0h` and `57B0h`. Footnote 66: *"The
  Additional Load Controls are only required if Additional Data shall be
  downloaded."* And the NOTE that decides how to use the table: *"Table 7
  specifies globally for the device which Load Controls shall be supported. Which
  Load Controls shall be supported for the management of a specific Resource is
  specified in the (Realisation Type of) the Resource in [11]."* There is
  therefore **no fallback between allocation styles** — pick by mask or decline.
- **Authorisation is mandatory for some profiles and optional for others**, and
  the number of access levels is 4 or 16 depending on the profile. Row-level
  facts only (see the extraction caveat below).
- **A device without protected areas grants level 0 to anything.** Footnotes 10
  and 18: *"The support of the A_Authorize- and the A_Keywrite-service does not
  imply that the device itself has access protected areas. If this is not the
  case, a device shall always allow – regardless of the attributed keys – access
  to the highest level (0), including when receiving an illegal key"*. So a
  successful authorisation proves nothing about enforcement.
- **`"If Verify Mode is not implemented, it shall always be off."`** (footnotes 8
  and 16). A device may conformantly report bit 2 clear after the client sets it,
  which means §8.7.6's re-assertion rule needs a read-back that decides which
  write path runs rather than an error.
- **LSM Realisation Type 1 is not universally mandatory** — the row contains both
  `M` and `O` values — so a device without `PID_LOAD_STATE_CONTROL` is not
  necessarily faulty.
- **Access levels in Annex A are recommendations.** Annex A.1.2.1: levels are
  noted *"read access level"/"write access level"*, and Table 3's legend defines
  them as *"recommended default"* values. The `PID_LOAD_STATE_CONTROL` row's
  values across masks include `3/3`, `3/1`, `3/0`, `15/2` and `15/1` — i.e. there
  exist devices where reading the load state is permitted at the minimum level
  but **writing a load event needs level 2, 1 or 0**. That is a silent failure
  mode: pre-flight reads succeed and the first `Start Loading` write is ignored
  with no negative response.

**Extraction caveat, recorded because it limits what may be cited:** in the
`-layout` extraction of `06 Profiles`, the mask-version columns of the feature
tables in §4.2 and §5.3 are misaligned against their headers. Per-mask
attribution from those two tables is therefore **not** citable and neither this
section nor the design spec makes any. Annex A's Table 7 does survive extraction
and is cited per mask; footnotes have no columns to misalign and are cited
verbatim.

#### 8.7.15 The genuinely undocumented set

After both passes and the 2026-09-14 fix round, **six** items. Each carries the
stable identifier defined in the design spec §12; those ids are canonical across
this file, the spec and `docs/KNOWN_LIMITATIONS.md` §7, and are never reused or
renumbered. The ordinals this list used before are given in the spec's mapping
table, because all three files had renumbered independently and "item 5" used to
mean different things in different files.

1. `GAP-T30-01` — per-`Legacy*`-flag semantics (§8.6; zero hits across the
   extracted Standard corpus);
2. `GAP-T30-02` — the `LdCtrl*`-name to load-control-subtype mapping for 13 of 25
   kinds (§8.6);
3. `GAP-T30-03` — what an `EtsDownloadPlugin`/`Baggage` DLL does (compiled code;
   not documentable from either knowledge base);
4. `GAP-T30-04` — the *"differential download algorithm"* named in `03_05_03`
   §3.5.3, p. 46 — **narrowed 2026-09-19**: the trigger, goal, required
   client-side state and consistency argument are documented across three
   more sources (`03_01_02 Glossary` p. 9, RES §4.2.27.1.2 p. 39, `Project
   Schema23` `DeviceInstance` attributes p. 44); only the diffing/
   chunk-selection strategy is genuinely absent, and the Glossary's own *"may
   for instance"* marks that absence as implementation-defined — a licence,
   not a gap (design spec §12 item 4;
   `docs/spec-audits/2026-09-19-cp-3_5_3-partial-download.md` Q1);
5. `GAP-T30-07` — the unquantified *"delay for programming the memory in the
   device"* (`03_05_02` §3.16 and four sibling procedures). The footnote does name
   a reference — *"The delay time depends on the Management Server and on the
   amount of written octets (see [08])."* — and `[08]` is Chapter 3/7/2 Datapoint
   Types, which was followed: it yields only `DPT_Time_Delay` 20.013, an
   enumeration of 26 coarse delay *labels* for a Use FB, not a formula and not a
   per-device value. The gap survives being chased to its own citation;
6. `GAP-T30-08` — LSM Realisation Type 2, a gap the Standard states itself
   (`03_05_01` §4.23.3).

Plus one that is **narrowed rather than open or closed**, and is therefore counted
separately instead of being folded into the six: `GAP-T30-09`, cross-Load-State-
Machine ordering. `03_05_01` §4.23.2.4.1 delegates it to per-device Profiles
(§8.7.14), the general Profiles document states no rule, and the per-mask Profile
documents where a rule would live were not read.

Two items that used to be on this list are gone, for two different reasons, both
recorded so neither looks like an oversight:

- `GAP-T30-05` — `L_Data_Extended` discovery — **withdrawn 2026-09-13**: it is
  documented, at `03_05_01` §4.3.7 `PID_MAX_APDU_LENGTH` (§8.7.13).
- `GAP-T30-06` — the `60h` programming-mode parity computation — **reclassified
  2026-09-14**: also documented, at `03_05_01` §4.26.3.1 and §4.26.3.4.1
  (§8.7.7). Writing to `60h` on real hardware is still forbidden, but as an
  explicit project risk decision (spec §13 **R11**) rather than as a documentation
  gap. The only residual unknown — whether a given device uses odd or even parity
  — is not a gap either, because the derivation inverts the bit instead of
  computing it and therefore never needs the convention.

Each item was searched for in both KNX specification knowledge bases
(`knx_spec_kb_programming`, 27 programming PDFs with figures; the 177-PDF
text-only base) and, where relevant, in the extracted Standard corpus and §8.6's
product corpus. That naming convention is the point: it is what lets a later
reader tell "the Standard is silent" from "nobody looked".

---

### 8.8 Commissioning, phase 3 — read-only verification against the real installation (2026-09-14)

§8.7 specified. This ran it, against the real hardware behind the gateway
`192.0.2.1:3671` (redacted, as in §8.1) that R-SAFE-2 (spec §2.2) approves for
**active reads only** on nine named individual addresses, `1.1.24`–`1.1.32`.
`1.1.220`, the alarm panel R-SAFE-1 (spec §2.1) makes structurally
unreachable, was never read, never written, and never named as a literal in
any address collection the new test builds — its own first assertion checks
that `1.1.220` is still in `EXCLUDED_INDIVIDUAL_ADDRESSES` before a single
frame is sent, redundant with `ContactableAddress::new`'s own refusal and
`ScanPlan::verify`'s. No write of any kind reached the wire in this pass:
every session used `ManagementSession::read_only` (no `WriteAuthorisation`,
so no write path exists to call) and `AuthorisationPlan::Skip` (so
`authorise()` returns without sending `A_Authorize_Request` at all — spec
§10.2's "free level, unknown value" path, not a tested one this time but a
skipped one). Bus contact, one line per address, per `CLAUDE.md`'s rule
against silently discarding information, applied here as the test's own
module doc practice (`crates/knx-net/tests/live_commissioning_readonly.rs`:
"prints one block per address and never summarises a non-answer away"):
`1.1.24`–`1.1.32`, each via `A_DeviceDescriptor_Read` (mask version),
`A_PropertyValue_Read` (`PID_MANUFACTURER_ID`, `PID_HARDWARE_TYPE`,
`PID_PROGRAM_VERSION`, `PID_LOAD_STATE_CONTROL` ×3 objects) over a
connection-oriented session, and again via the scan probe's own
`A_DeviceDescriptor_Read`. Nothing else.

#### 8.8.1 Method

Two read-only probes ran, sequentially, minutes apart, 2026-09-14:

1. **`crates/knx-net/tests/live_commissioning_readonly.rs`**, new, `#[ignore]`d,
   this task's own deliverable, run as
   `KNX_GATEWAY=<redacted>:3671 cargo test -p knx-net --test
   live_commissioning_readonly -- --ignored --nocapture --test-threads=1`.
   One `ManagementSession::read_only` per address, built fresh each time from
   the address literal (never a range — see the source's own doc comment),
   each doing `connect()` then, on the Device object (index 0): Device
   Descriptor Type 0, `PID_MANUFACTURER_ID` (12), `PID_HARDWARE_TYPE` (78),
   `PID_PROGRAM_VERSION` (13); then `PID_LOAD_STATE_CONTROL` (5) on the
   Address Table (index 1), Association Table (index 2) and Application
   Program (index 3) objects; then `disconnect()`. Every read or its error is
   printed, never summarised away. Took 505.41s for the nine addresses,
   `test result: ok. 1 passed; 0 failed`.
2. **The already-shipped `knx bus scan`** (§8.5), run immediately after as an
   independent cross-check: `knx bus scan --gateway <redacted>:3671 --line 1.1
   --range 1.1.24-1.1.32`, `ProbePolicy::default()` (6000 ms response timeout,
   1 confirmation, 100 ms inter-probe pause). `ScanPlan`'s exclusion-by-
   construction still reported `1.1.220` excluded (1) even though the range
   given never reaches it — the guard runs regardless of whether it would have
   mattered. Took 7717 ms total for the nine addresses.

Both probes reuse the connection-oriented `NM_IndividualAddress_Check`
sequence (`03_05_02` §2.19: `T_Connect` → `A_DeviceDescriptor_Read(0)`) over
the same underlying transport (`ManagementTransport`/`ScanTransport` share one
blanket impl, `crates/knx-net/src/management.rs`) — the difference between
them is explained in §8.8.3.

#### 8.8.2 Raw observations, by address

**`1.1.24`** — occupied, both methods agree:

| Read | Raw payload / outcome |
| --- | --- |
| `A_DeviceDescriptor_Read(0)` | `07 01h` (Mask Version `0701h`) |
| `A_PropertyValue_Read`(obj 0, PID 12, 1 elem @1) | `00 0Ch` |
| `A_PropertyValue_Read`(obj 0, PID 78, 1 elem @1) | refused, `nr_of_elem = 0` (AL §3.4.4.2's own refusal shape) |
| `A_PropertyValue_Read`(obj 0, PID 13, 1 elem @1) | refused, `nr_of_elem = 0`, same shape |
| `A_PropertyValue_Read`(obj 1, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 2, PID 5, 1 elem @1) | `01h` (`Loaded`) |
| `A_PropertyValue_Read`(obj 3, PID 5, 1 elem @1) | `01h` (`Loaded`) |

The scan probe's independent `A_DeviceDescriptor_Read(0)` on `1.1.24` agrees:
`Occupied`, mask `0701h`, 202 ms.

**`1.1.25`, `1.1.26`, `1.1.27`, `1.1.28`, `1.1.30`, `1.1.31`, `1.1.32`** —
occupied per the scan probe: `Occupied`, mask `0701h` each, round trips
86–107 ms. But every one of the seven `ManagementSession` reads listed above
for `1.1.24` returned, for every one of these seven addresses, the identical
outcome: *"no answer to A_DeviceDescriptor_Response / A_PropertyValue_Response
after 3 attempt(s) of 3s; this is a lost frame, a protected or absent target,
or a device that is not listening, and the wire does not distinguish them"* —
the session's own honestly-worded time-out, TL clause 4's `max_rep_count = 3`
exhausted every time. §8.8.3 reconciles this.

**`1.1.29`** — vacant by both methods, and this is the one address where they
agree on absence rather than disagree on presence: the scan probe got no
`L_Data.con` at all for its `T_Connect` within the full 6006 ms window (the
Standard's own presence signal, `03_05_02` §2.19); the `ManagementSession`
read got the same worded time-out as the seven false negatives above — from
that method alone, `1.1.29` is indistinguishable from them.

#### 8.8.3 Finding — `ManagementSession`'s connect-then-read split misses present devices the scan probe sees

This is the headline result of the phase, and it is a finding against the
**verification method**, not against the nine devices: seven real, present,
answering devices looked absent to the one API this project has for reading
a device's properties.

The mechanical difference between the two code paths, read from the source
rather than inferred: `probe_once` (`crates/knx-net/src/scan.rs`) sends
`T_Connect` and the `A_DeviceDescriptor_Read(0)` `T_Data_Connected` frame as
one back-to-back pair, with no gap and no separate retry of the data frame —
if nothing answers, the *next* attempt (`probe_address`'s own retry loop)
sends a **fresh** `T_Connect` again. `ManagementSession::connect()` sends
`T_Connect` once, and every read after it (`exchange`/`exchange_inner`,
`crates/knx-net/src/commissioning.rs`) retries only the **data** frame, up to
`MAX_REP_COUNT = 3` times, against the one `T_Connect` already sent — TL
clause 4 read literally: a repeat is of the unacknowledged request, and
`T_Connect` is not repeated by that clause. `ManagementSession` never
inspects whether its own `T_Connect` actually got a positive `L_Data.con` at
all; the scan probe does, and a negative or absent one is exactly what
`deadline_verdict` in `scan.rs` treats as inconclusive rather than as
`Occupied`.

The two application-layer requests are otherwise identical (same descriptor
type, same object indices, same property ids, same sequence number 0 on the
first exchange after `connect()`), and both this test and the scan run went
through the same tunnelling connection and the same gateway seconds apart —
so a generic "the bus was busy" or "the gateway was overloaded" explanation
does not fit seven-for-seven identical outcomes on one side and seven-for-
seven identical successes on the other. The most likely mechanical
explanation, stated as a hypothesis and not confirmed further by additional
bus contact (out of this task's read-only-verification scope): whatever
window these seven devices hold a fresh `T_Connect` open for is shorter than
the gap between `connect()` returning and the first `exchange()` call
actually reaching the wire, and/or the devices silently drop a connection
attempt that a following data frame does not itself renew. Root-causing this
further, and any change to `exchange_inner`'s retry shape, is future work —
recorded as spec §13 R20 and design-spec §14.1, not fixed in this pass.

This retry-shape difference cannot be what separated `1.1.24` from the other
seven, though: the scan ran at `ProbePolicy::default()`, `vacant_confirmations
= 1` (`crates/knx-net/src/scan.rs:120`, loop at `:302-309`), so with one
confirmation the scan made exactly one pass per address and never retried at
all. `1.1.24`'s success and the seven time-outs were each decided on a single
`T_Connect` + data-frame pair on both sides — the difference in *how many*
times each method would have retried never came into play for this run, and
is not the discriminator for the first attempt.

**What this does and does not mean:** it does not mean these seven devices
are unreachable for reading — the scan probe reaches them fine, and a fixed
`ManagementSession` presumably would too. It does mean that, as shipped
today, a caller cannot treat a `ManagementSession` read time-out as proof of
absence, on this installation, at all — seven of nine addresses would have
been wrongly recorded as vacant or unreachable if this section trusted that
signal alone.

One confound this pass cannot exclude: `1.1.24` was the first address in the
loop, and all nine sessions shared one tunnelling connection. "The first
session on a fresh tunnel works and subsequent ones do not" fits the data as
well as any per-device explanation, and would be separated by probing the
nine in reverse order, or one tunnel per address.

#### 8.8.3a Follow-up — order ruled out; tunnel lifecycle implicated (2026-09-18)

The approved read-only comparison was run against the same installation with
only `A_DeviceDescriptor_Read`, no property reads, no authorisation and no
writes **[V]**. The target list was the nine explicit addresses from R-SAFE-2;
`1.1.220` was asserted excluded before any socket opened.

In reverse order on one shared tunnel, `1.1.32` answered with mask `0701h` and
all eight subsequent targets timed out. This rules out a special property of
`1.1.24`: whichever approved target owns the first management session on a
fresh shared tunnel answers, while later sessions do not.

With one newly opened and cleanly disconnected tunnel per target, still in
reverse order, `1.1.32`, `1.1.30`, `1.1.28`, `1.1.26` and `1.1.24` answered
with `0701h`; `1.1.31`, `1.1.29`, `1.1.27` and `1.1.25` timed out. The strict
alternation means a fresh tunnel per target is not by itself a reliable fix;
immediate tunnel teardown/recreation or gateway channel lifecycle is also in
the causal path. It does not prove which endpoint retains state or which delay,
sequence or acknowledgement is missing. R20 therefore stays open, and callers
must continue using the independent scan probe for presence rather than
interpreting a `ManagementSession` timeout as absence.

#### 8.8.3b Specification audit — KNXnet/IP disconnect completion is not observed (2026-09-18)

A targeted audit of the PDFs under *The KNX Standard v3.0.0* identifies one concrete protocol defect that fits the fresh-tunnel alternation, without yet proving it caused that hardware result **[D+V]**:

- *03_08_02 KNXnet/IP Core v01.06.02 AS* §5.3.4 (page 13) requires an independent sequence counter per communication channel, reset to zero for a newly established channel. `TunnelClient` does this. *03_08_04 KNXnet/IP Tunnelling v01.07.01 AS* §2.6.1 (page 9) requires one repeat with the same counter after a one-second missing `TUNNELLING_ACK`; `send_frame` does this too. These rules expose no discrepancy.
- Core §5.5 (pages 13–14) says the peer receiving `DISCONNECT_REQUEST` shall acknowledge with `DISCONNECT_RESPONSE`, and that this data packet signals the **final termination** of the communication channel. The KNXnet/IP system conformance test (*08_TSSH ... KNXnet_IP_1_3_AS*, §3.6.1, pages 21–22) likewise sends a request and expects the matching response before cleanup.
- `TunnelClient::disconnect` sends `DISCONNECT_REQUEST`, immediately signals both background tasks to stop and returns without waiting. Its sole socket reader handles server-initiated `DISCONNECT_REQUEST`, but has no `DISCONNECT_RESPONSE` arm; the response decoder exists unused outside unit tests. KNXBench therefore cannot know the server completed channel teardown before opening the next tunnel. That is a specification mismatch, not an inferred gateway quirk.

This was a plausible explanation for strict alternating results when a new tunnel was opened immediately after each disconnect: each next connect could have raced the gateway's unfinished release of the previous channel. The shared-tunnel first-session failure involves KNX Transport Layer sessions inside one unchanged IP tunnel, so the missing IP `DISCONNECT_RESPONSE` could not by itself explain that half of R20.

The same audit rules out replacing the existing presence probe casually. *03_05_02 Management Procedures v02.01.02 AS* §2.19 (pages 34–35) defines the general `NM_IndividualAddress_Check` exactly as `T_Connect`, connected `A_DeviceDescriptor_Read(0)`, then `T_Disconnect`, which is the procedure `scan.rs` implements. §2.17's connectionless descriptor scan (pages 32–33) explicitly applies only to a KNX RF subnetwork and documents that it misses devices detected only through `T_Connect`. §3.2 (pages 68–70) permits both connection-oriented and connectionless `DM_Connect`; the current `ManagementSession` connection-oriented choice is therefore valid.

The causal test is now implemented. A real UDP loopback peer holds channel release until it sends `DISCONNECT_RESPONSE`; the regression test failed against the old client because `disconnect()` returned immediately, then passed after `TunnelClient` began waiting for the matching successful response. A ten-second local error bound matches the existing control-response budget; it is not a delay or retry intended to influence gateway behaviour. All 174 `knx-net` library tests and package Clippy with warnings denied pass.

#### 8.8.3c Real-gateway causal rerun after response-aware disconnect (2026-09-18)

The bounded fresh-tunnel comparison from §8.8.3a was repeated with the response-aware disconnect implementation **[V]**. The safety boundary was unchanged: the nine literal targets in reverse order (`1.1.32` through `1.1.24`), the excluded `1.1.220` checked before socket creation, one fresh tunnel per target, `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`. No property read, authorisation request, write service or scan was sent.

Every KNXnet/IP disconnect received its matching successful `DISCONNECT_RESPONSE`, proving that the gateway had finally terminated each channel before the next was opened. Nevertheless the result was identical to the earlier run: `.32`, `.30`, `.28`, `.26` and `.24` answered mask version `0701h`, while `.31`, `.29`, `.27` and `.25` timed out after the defined three descriptor-read attempts. Waiting for final IP-channel termination therefore does **not** remove the alternation and is ruled out as its cause on this gateway. The fix remains required for Core §5.5 compliance. R20 stays open because the evidence now points below or outside KNXnet/IP channel teardown; it still does not identify the missing state transition, acknowledgement or timing rule. No retry or delay constant may be inferred from this result.

#### 8.8.3d Full listed-device sequence test (2026-09-18)

The response-aware fresh-tunnel test was extended, with user approval, to all 34 literal targets in `devices.md`: `1.1.1` through `1.1.32`, then the listed IP interfaces `1.1.250` and `1.1.253` **[V]**. The user-excluded `1.1.200` and the project-excluded `1.1.220` were both absent from the target set and checked before socket creation. The wire boundary remained `ManagementSession::read_only`, `AuthorisationPlan::Skip`, and only `A_DeviceDescriptor_Read(0)`; no property read, authorisation request, write service or scan was sent.

The 34 sessions split exactly by ordinal position. All 17 odd-position attempts answered: `1.1.1`, `.3`, `.5`, `.7`, `.9`, `.11`, `.13`, `.15`, `.17`, `.19`, `.21`, `.23`, `.25`, `.27`, `.29`, `.31` and `.250`. All 17 even-position attempts timed out: `1.1.2`, `.4`, `.6`, `.8`, `.10`, `.12`, `.14`, `.16`, `.18`, `.20`, `.22`, `.24`, `.26`, `.28`, `.30`, `.32` and `.253`. `1.1.23` returned mask `0012h`; the other 16 answers returned `0701h`. Every fresh tunnel received its matching successful `DISCONNECT_RESPONSE`.

This reverses the previous outcome for the same addresses `1.1.24` through `.32`: ascending order makes `.25/.27/.29/.31` answer and `.24/.26/.28/.30/.32` time out, while reverse order did the opposite. Device address, manufacturer and mask version are therefore ruled out as selectors for the failure. The alternating state follows attempt order across 34 independently terminated IP tunnels. Its owner remains unidentified below or outside the KNXnet/IP channel lifecycle, so R20 remains open and no delay or retry constant is justified.

#### 8.8.3e Frame-level isolation of the failed connection (2026-09-18)

A two-target read-only diagnostic recorded the gateway-assigned address, incoming KNXnet/IP sequence counters and decoded cEMI frames without changing protocol timing **[V]**. Both success and failure tunnels were assigned `1.1.249`; every incoming channel began at sequence zero and every counter matched the client's expectation. Tunnel address assignment and client-side KNXnet/IP receive-sequence rejection are therefore ruled out.

The successful `1.1.1` session received, in order, successful `L_Data.con` for its `T_Connect`, successful `L_Data.con` for `A_DeviceDescriptor_Read(0)`, `T_ACK` from `1.1.1`, then `A_DeviceDescriptor_Response(0, 0701h)`. The following timed-out `1.1.2` session first received a late `L_Data.con` carrying `T_Connect` for the previous destination `1.1.1`. Its three descriptor-read attempts each received successful `L_Data.con` toward `1.1.2`, but no `L_Data.con` for the current `T_Connect`, no `T_ACK` from `1.1.2` and no descriptor response arrived.

This identifies the direct failure mechanism. `ManagementSession::connect()` awaits `TunnelClient::send_frame()`, which completes on the gateway's KNXnet/IP `TUNNELLING_ACK`; it does not await a matching cEMI `L_Data.con` proving the bus-level `T_Connect` progressed. The failed session therefore sends connected descriptor reads without evidence that its current transport connection was established. The trace does not yet explain why the gateway or bus omits every alternate current-target connect confirmation. A causal correction must synchronize with the matching successful bus confirmation or another specification-grounded readiness event, not an unexplained delay.

#### 8.8.3f Specification-timed connection confirmation fix (2026-09-18)

`ManagementSession::connect()` now subscribes before sending `T_Connect` and does not establish session state until a matching positive cEMI `L_Data.con` arrives **[V+D]**. The match binds the assigned tunnel source, current target, `T_Connect` and `NoApplicationPdu`, so the late previous-target confirmation observed in §8.8.3e cannot release the next session. `SessionTiming::connection_timeout` defaults to Transport Layer clause 4's normative six seconds. Silence returns `NoAnswer` after that bound; a matching negative confirmation returns `ConnectRejected` immediately. The simulator emits the same confirmation shape. TDD RED proved the old gateway-ACK-only return and the old wait-through-negative behavior before both changes.

A final fresh-tunnel, read-only run covered all 34 literal `devices.md` targets **[V]**. Thirty-three positive confirmations arrived in 2.851–144.245 ms (median 134.530 ms), and all 33 subsequent `A_DeviceDescriptor_Read(0)` requests answered. `1.1.253` returned an explicit negative `L_Data.con` after 162.709 ms and was reported as `ConnectRejected`; no descriptor read was sent after rejection. The earlier strict alternation disappeared completely. The relevant lateness was therefore roughly 0.1–0.16 seconds, while the old session allowed zero time for confirmation; six seconds is a specification bound, not a measured delay guess. R20's false-connected timeout mechanism is fixed. A later management timeout remains inherently ambiguous and must not by itself prove absence, but connection establishment no longer proceeds without bus evidence.

#### 8.8.4 Reconciliation against the spec

- **`PID_ERROR_CODE`, `PID_DEVICE_CONTROL` and `PID_OBJECT_INDEX`**, named in
  design spec §14's phase 3 checklist, were **not** read against real hardware
  this pass — the properties this test reads are the ones named in this
  task's own dispatch brief (device presence, mask version, descriptor reads,
  property reads, load-state reads). That set is **different from** §14's
  list, not merely narrower than it: three of §14's six were skipped, and two
  it never named — `PID_HARDWARE_TYPE` and `PID_PROGRAM_VERSION` — were read
  as well. Noted as a residual coverage gap in the design spec (§14.1), not
  closed here — a second read-only pass, or an extension of this same test,
  is the natural next step and needs no new safety reasoning to run.
- **`1.1.24`'s access-level refusal on `PID_HARDWARE_TYPE`/`PID_PROGRAM_VERSION`
  while `PID_MANUFACTURER_ID` succeeded, all under `AuthorisationPlan::Skip`**
  is a live confirmation of design spec §10.2, not a contradiction of it:
  *"the maximum access level protected with `FFFFFFFFh`"* is whatever the
  device's own Profile grants an unauthorised client, and §10.2 already
  predicted *"a client that skips authorisation therefore works until it does
  not."* **[V]** This is the first live evidence for that sentence.
- **The mask `0701h` result matches §8.5 Finding 4's prior scan of this same
  installation** (2026-09-13, "nine consecutive occupied addresses... the same
  Mask Version, `0x0701`", addresses unnamed there for the reason given in
  that section). This section names the addresses because they are the
  already-publicly-approved nine from spec §2.2, not because the redaction
  policy changed — §8.5's broader, unapproved inventory stays unnamed.
  Finding 4 did not record which addresses were in its nine, so this is
  corroboration of "this stretch of the line runs the same mask", not a
  re-identification of Finding 4's own sample.
- **Design spec §14's implicit assumption — that phase 3's read-only checklist
  is something `ManagementSession::read_only` can just run — is corrected**
  by §8.8.3 above: presence must be established with the scan probe first.
  design spec §14.1 and §13 R20 carry the correction.

See [KNOWN_LIMITATIONS.md §7](KNOWN_LIMITATIONS.md#7-commissioning-and-device-download-are-required-but-blocked)
for the durable record.

### 8.9 Extraction hazard: `pdftotext -layout` silently misreads Volume 6 Annex A's tables (2026-09-20, task C18 fix round)

**Standing caution for anyone auditing Volume 6 (`06 Profiles`) Annex A's
per-object Property tables (A.2.3 through A.2.10, pp. 138-152): do not trust
`pdftotext -layout` column alignment on these tables. Render the page as an
image and read it visually instead.**

Found while re-verifying a task C18 finding: `PID_MCB_TABLE` was first read
via `pdftotext -layout` as carrying a mandatory-existence symbol for System B
(masks 07B0h/17B0h) in A.2.4 (p. 143) and A.2.5 (p. 145). It does not — the
printed symbol is `(3/3)`, the parenthesised *optional*-existence family per
Table 3's legend (p. 134). Rendering the pages at 250 DPI
(`pdftoppm -f 143 -l 143 -r 250 -png "06 Profiles v02.01.01.pdf" out`), then
cropping/upscaling the right-hand columns for legibility, showed why:
each system group (System 2, System 300, System 7, System B, ...) carries
**one extra data column with no mask number** — it inherits only the
rotated group-name header — in addition to the explicitly mask-labelled
columns (07B0h, 17B0h, 57B0h for System B). `pdftotext -layout` does not
know this unlabelled column exists as a distinct field; it silently merges
or shifts it against the neighbouring mask column, so a value read by
counting columns out from the mask-number headers lands one column off.
Text extraction gave a false mandatory reading; the rendered image gave the
correct optional one.

Practical rule: for any Annex A table read that matters (i.e. feeds a
compatibility or compliance conclusion, not just a sanity check), verify by
rendering the page as an image and counting columns visually against the
table's own header row on that same page — never against a remembered
column count from `pdftotext` output on a different page or a different
system group.

---

## 9. KNX Secure

Not present in our sample installation (`data_secure=false` on all 514 group addresses, no keyring) — everything here is **[D]**.

* **Data Secure**: each secured group address has a 128-bit runtime key. Each secure device has a tool key. ETS stores them in the `.knxproj` in protected form and can export them to a password-protected `.knxkeys` **keyring** file, which is the supported route for non-ETS clients.
* **FDSK**: factory key printed on the device label / QR code as a 36-character device certificate (FDSK + KNX serial number). On first commissioning ETS replaces it with a per-tool Device Key. Without the project password, stored FDSKs are inaccessible and secure devices cannot be reprogrammed.
* **Keyring format**: KNX publishes an official article; FDSK inside is separately encrypted, and the XML is signed using the same length-prefixed serialization scheme as product data. `xknx` already implements keyring decryption.
* **ETS6 tunnel clients**: the keyring is exported per IP tunnel via *Export Interface Information*, not from the project security page.

Design consequence: **key material must be a separate, isolated subsystem** with its own storage and access rules from day one. It must never be flattened into the general project model, never written to logs, exports, or reports, and must be omitted by default from any diagnostic dump. Retrofitting this is how secrets leak.

Open question for a later session: can we read secured runtime keys directly out of a `.knxproj`, or only from a `.knxkeys` keyring? Untested.

---

## 10. Legal and licensing constraints

**Not legal advice.** These are the constraints as best established from public sources; anything with commercial consequences needs a lawyer.

* **Protocol vs. document.** Copyright covers the specification text, not the protocol. Implementing from a lawfully obtained specification is standard practice; redistributing the specification text is not. [D]
* **Trademark.** "KNX" is a registered trademark. An independent tool can interoperate but cannot call itself KNX-certified without membership and conformance testing. Existing projects consistently use "KNX-compatible". **We should adopt the same wording in all user-facing text.** [D]
* **ETS is proprietary.** Reading `.knxproj` we own is fine. We must not bundle ETS binaries, DLLs, or converters (`KnxCvNext.exe` requires an installed ETS). [D]
* **Manufacturer product data.** `.knxprod` files are distributed through the KNX online catalog under KNX/manufacturer terms. Users importing their own downloaded product data is one thing; **us redistributing a product database is another**. Product data must stay strictly separable from application code and must not be committed to this repository. [D] — matches the rule already in `CLAUDE.md`.
* **`knx_master.xml` extracted from a project** is KNX Association content. Read it from the user's own file at import time; do not vendor a copy. [A — conservative default]
* **Dependency licensing — hard architectural constraint [V]:**

  | Package | License | Consequence |
  | --- | --- | --- |
  | `xknx` 3.20.0 | MIT | Safe to depend on under any license. |
  | `xknxproject` 3.10.0 | **GPL-2.0-only** | Linking it forces the whole application to GPL-2.0. |

  Since §7.1 already establishes that we need our own parser, the clean resolution is: **`xknxproject` is a development/test-only dependency, never a runtime dependency.** This must be enforced mechanically (separate dependency group + a test that the runtime import graph never reaches it), and recorded as an ADR in Session 1.

* **The `.knxprod` container** is the same XML family as `.knxproj` (master data scheme 11 vs. 12+); newer files cannot be read by older ETS. Its encryption/obfuscation layer for newer schemes was **not** established by this research and remains an open question. [D/open]

---

## 11. Risks

| # | Risk | Severity | Mitigation |
| --- | --- | --- | --- |
| R1 | Single-sample bias: everything verified here is schema 11 / ETS 4.1 | High | Acquire ETS5 (13/14, 20) and ETS6 (21+) sample projects before Session 3. Treat §3 as version-specific until then. |
| R2 | No authoritative XSD available | High | Tolerant parser + exhaustive unknown-element/attribute inventory, reported to the user (§12). |
| R3 | `Dynamic` tree (`choose`/`when`) evaluation is the real complexity | High | **Done (2026-09-11).** The dedicated Session 4 research spike ran — see §4.3. The Standard normatively specifies the `@test` value grammar; the surrounding structural grammar remains corpus-observed only, not Standard-normative. This closes the research risk; it does not build the evaluator. T18 (the parameter editor, [GAP_ANALYSIS_ETS.md](GAP_ANALYSIS_ETS.md)) is no longer blocked on research — it now needs a no-match design decision and a defensive parser, both implementation work. |
| R4 | Round-trip cannot be byte-exact (signatures, attribute ordering, ETS-internal ids) | Medium | Define round-trip fidelity as *semantic* equality over a declared model + verbatim passthrough of opaque parts. Never claim byte-exactness. |
| R5 | Vendor plug-in DLLs make some devices unconfigurable by us | Medium | Detect `Baggages`, mark affected devices read-only, report clearly. |
| R6 | GPL-2.0 contamination via `xknxproject` | Medium | Test-only dependency, enforced by CI. |
| R7 | Product database size/performance (22 MB for 12 programs) | Medium | Indexed, cached, versioned product DB layer; never re-parse per open. |
| R8 | Secret leakage once KNX Secure is supported | High | Isolated key subsystem, excluded from exports/logs by default. |
| R9 | Writing an unsigned `.knxproj` may be rejected by ETS on re-import | Medium | Test explicitly. If rejected, our export is a one-way documentation format and must say so. |
| R10 | Commissioning/device-download procedures might be undocumented outside ETS internals | High | **Partly done (2026-09-11).** The R5 research spike ran — see §8.4. The generic download/unload/reset/memory-write procedures and the Load State Machine are Standard-normative and now cited in full; "Differential Download" is a formal Glossary term. This closes the research risk on the *generic* procedure; it does not build a downloader, it does not verify anything against real hardware, and the product-specific `Legacy*` compatibility-flag matrix and vendor `Baggage` DLL involvement (R5 above) remain genuinely undocumented in this corpus — see §8.4 Q4/Q9. |

---

## 12. Conclusions for Session 1 (Architecture)

Recommendations carried forward, each traceable to a finding above:

1. **Write our own `.knxproj` reader.** §7.1. `xknxproject` stays as a test oracle only, and as a test-only dependency for license reasons (§10).
2. **Model the three-layer override chain explicitly** (ComObject → ComObjectRef → ComObjectInstanceRef), keeping provenance per resolved value. §3.2 — this affects 758 of 907 objects and cannot be bolted on later.
3. **Two orthogonal hierarchies over one device set**: topology (Area/Line, plus unassigned) and building (recursive typed `BuildingPart`). Neither owns the device. §3.1.
4. **Directional group-address links** (send / receive), not undirected associations. §3.1.
5. **Commissioning state is domain data**, not import metadata: `*Loaded` flags, `LastDownload`, `CompletionStatus`, `Broken`. §3.1.
6. **Language-aware strings from the start.** Translations are a side table with 5919 entries in a single application program. §4.1.
7. **Every import produces a report**: unknown elements/attributes encountered, opaque data preserved, values inferred (e.g. DPT from linked objects), conflicts, and unsupported features. This is a core deliverable of the importer, not a logging afterthought. §6.1, §7, R2.
8. **Opaque-passthrough store** keyed by source path, so binaries, signatures and legacy plugin data survive a round trip untouched. §7.
9. **Product database is a separate, versioned, cached layer**, keyed by (manufacturer, application program, version), never bundled with the application. §4.1, §10.
10. **Key material is an isolated subsystem** even before KNX Secure is implemented. §9.
11. **Retain low-level programming data** (`Memory`, `AbsoluteSegment`, `LoadProcedures`, mask/resource data) at import even though commissioning has not started, so that path stays open. §8.3.
12. **User-facing wording is "KNX-compatible", never "KNX certified" or "full ETS compatibility".** §7, §10.

### Open questions to resolve before or during Session 3

* ETS5/ETS6 schema deltas (13, 14, 20, 21+) — needs sample projects. (R1)
* `Functions` element semantics — absent from our sample. (§3.1)
* ~~`when/@test` expression grammar in the `Dynamic` tree. (R3)~~ — **answered, §4.3 (2026-09-11).** What is still open, carried forward from that section: the `Dynamic`/`Channel`/`ParameterBlock`/`choose`/`When_t`/`ChannelIndependentBlock` *structural* (complexType) grammar, which no schema document available here defines; the no-match evaluation rule for a no-default `choose`; and `Access`/`Visible` as gating mechanisms independent of `choose`/`when`.
* Whether ETS re-imports an unsigned `.knxproj` written by a third-party tool. (R9)
* Whether Data Secure runtime keys are readable from `.knxproj` or only from `.knxkeys`. (§9)
* `.knxprod` encryption for master data scheme 12+. (§10)

---

## Sources

* [Project schema description – KNX Association](https://support.knx.org/hc/en-us/articles/4408207190674-Project-schema-description)
* [Unexpected XML namespace "http://knx.org/xml/project/14" – KNX Association](https://support.knx.org/hc/en-us/articles/360007208400-Unexpected-XML-namespace-http-knx-org-xml-project-14)
* [calimero-project/import-ets-xml (archived)](https://github.com/calimero-project/import-ets-xml)
* [calimero-project/calimero-core](https://github.com/calimero-project/calimero-core)
* [XKNX/xknxproject](https://github.com/XKNX/xknxproject)
* [KNX Data Secure – KNX Association](https://support.knx.org/hc/en-us/articles/360012689639-KNX-Data-Secure)
* [Keyring File Format – KNX Association](https://support.knx.org/hc/en-us/articles/18968368409874-Keyring-File-Format)
* [Manufacturer Product Databases – KNX Association](https://www2.knx.org/ie/software/ets/manufacturer-product-databases/index.php)
* [File formats used during registration/certification – KNX Association](https://support.knx.org/hc/en-us/articles/4659247971346-File-formats-used-during-registration-certification)
* [ISO 22510:2019 — KNXnet/IP communication](https://www.iso.org/standard/73364.html)
* [thelsing/CreateKnxProd](https://github.com/thelsing/CreateKnxProd)
