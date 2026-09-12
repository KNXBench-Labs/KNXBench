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

### 2.3 Password protection [V — read from `xknxproject` source]

When protected, the archive contains a nested `<P-xxxx>.zip`.

* Schema < 21 (ETS4/ETS5): standard ZipCrypto, password used as UTF-8 bytes.
* Schema >= 21 (ETS6): AES ZIP (`pyzipper`), password derived as

```text
base64( PBKDF2-HMAC-SHA256(
    password = utf-16-le(user_password),
    salt     = b"21.project.ets.knx.org",
    iterations = 65536,
    dklen    = 32 ) )
```

Our sample is unprotected, so this path is **unverified in practice** here. Must be tested against a real ETS6 protected project before we claim support.

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
Zuhause` exports and `Weinzierl_730_KNX_IP_Interface_ETS4_v1.knxprod`.
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

---

## 5. `knx_master.xml`

Content of the ETS4 master data file shipped inside our project [V]:

* `DatapointTypes`: 46 main types, 289 subtypes.
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

### 8.5 Line-scan / bus-side device discovery — T17 spike (2026-09-12)

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

One further limitation is honest to carry forward rather than paper over:
`03_03_04 Transport Layer v01.02.03 AS.md` §5.5.1.5 ("Connect from the
local User to a non-existing Device") and §5.5.1.9 ("Connection timeout")
are both figure-only in this corpus's Markdown extraction — the sequence
diagrams did not survive extraction and no surrounding prose substitutes
for them. Whether the Transport Layer state machine adds its own
independent timeout/confirmation logic on top of the Data Link Layer's
ACK/retry cycle described above, or is a pure pass-through of it, is
therefore not fully pinned down from this corpus; §3.7's prose reads as the
latter but that is **[A]**, inferred from adjacent text, not a direct
reading of the two blank sections.

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

  **Finding 1 — the absent-address cost is set by the client, not the
  bus.** The vacant-probe figures above are the tell: **~6.28 s** median,
  with under 50 ms of spread across 219 samples, is far too tight to be
  bus retry behaviour and far too slow to be a KNX medium timing at all —
  it is a client timeout expiring on schedule. `xknx`'s own constants say
  so directly: `MANAGAMENT_ACK_TIMEOUT = 3` seconds with one resend, and
  `MANAGAMENT_CONNECTION_TIMEOUT = 6` seconds
  (`xknx/management/management.py:35-36`, the copy installed in this
  project's `.venv`) — the measured ~6.28 s tracks the 6 s connection
  timeout plus transport overhead, not the ~70 ms the **[A]** bus
  arithmetic above predicted.

  The consequence is the design conclusion this whole subsection was
  building toward: **the scan's duration is dominated by a policy
  constant KNXBench will choose for itself**, roughly two orders of
  magnitude away from the Standard-derived arithmetic. That arithmetic
  was not wrong about the bus — it modelled the bus faithfully. It
  modelled the wrong thing: the KNX medium's own retry budget, not a
  KNXnet/IP client's connection-timeout policy, and the policy is what
  actually governs how long "nobody answered" takes to conclude. Whoever
  implements T17 picks that timeout value, and that choice — not the KNX
  medium — decides whether a full-line scan takes twenty minutes or one.
  Too short, and a slow-but-present device is reported absent; too long,
  and the scan is unusable on any real line, which is mostly vacant
  addresses, not mostly occupied ones. The Standard does not hand over a
  number to copy here: the closest it comes is All TP1 Profiles'
  `nak_retry`/`busy_retry`, documented **[D]** as only
  *optionally* 3 (`06 Profiles v02.01.01.md:775`, already cited above) —
  a bus-level retry count, not a client connection timeout, and optional
  even as that. This is a genuine design decision for T17's
  implementation, not a lookup.

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

**What exists and what is missing for an implementation — documented, not
built.** The KNXnet/IP tunnelling transport, cEMI `L_Data` encode/decode,
point-to-point addressing, and a raw-APCI escape hatch already exist in
`crates/knx-net` and are directly reusable
(`crates/knx-core/src/address.rs:9-50`,
`crates/knx-net/src/cemi.rs:28-32,183-243,450-463`,
`crates/knx-net/src/client.rs:306-360,759`). What does not exist yet: the
KNX **bus-level** Transport Layer. TPCI packet-type and sequence-number
encoding is hardcoded to connectionless, unnumbered mode — `decode_l_data`
derives `short_apci` from `tpci_apci_hi & 0x03` only
(`crates/knx-net/src/cemi.rs:140`) and `encode_l_data` always emits a zero
packet-type/control field (`crates/knx-net/src/cemi.rs:230-231`) — so
there is no representation of `T_CONNECT`, `T_DISCONNECT`, `T_ACK`,
`T_NAK`, or numbered `T_Data_Connected` anywhere in the crate today, no
per-target connection state machine, and no correlation of a sent frame to
its resulting bus-level `L_Data.con` distinct from the gateway's own
`TUNNELLING_ACK`
(`crates/knx-net/src/client.rs:195-201,624-637`). None of this requires a
new crate or a change to `check-layering` — it is additive work inside
`knx-net`'s existing cEMI and client modules — but it is real, unstarted
work, and this section documents the gap; it does not close it.

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
