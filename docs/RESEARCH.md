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
* **`ModuleInstance` → `ModuleDef` resolution chain**, measured against the KV project's `M-00FA` application programs: `DeviceInstance/ModuleInstances/ModuleInstance/@RefId` (local id, e.g. `MD-2_M-1`) resolves via the owning `DeviceInstance/@Hardware2ProgramRefId` to `ApplicationProgram/ModuleDefs/ModuleDef/@Id` (e.g. `M-00FA_A-2504-10-C071_MD-2`) — the same short-id-recovered-via-owning-element pattern §3.3 already established for schema-23 `ComObjectInstanceRef/@RefId`, now shown to apply to `ModuleInstance/@RefId` too. `ModuleDef` internally repeats the existing `ApplicationProgram` shape one level deeper: its own `Static/ComObjects` + `ComObjectRefs` (identical attribute set to the top-level ones already modelled by `knx-productdb`), plus a `Dynamic/Channel/ParameterBlock/choose/when` tree (the same grammar already flagged unresearched, KNOWN_LIMITATIONS §3) that picks which `ComObjectRef`s are active for given argument values. `ModuleInstance/Arguments/Argument/@RefId` supplies those argument values per instance (e.g. `argCH=1` — which channel number this repetition represents). Import does not need to evaluate `choose`/`when` itself: `GroupObjectTree` already carries ETS's own evaluation of it (previous bullet), so the active-object set is read, not recomputed.
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
│           └── Channel → ParameterBlock → choose/when → ParameterRefRef / ComObjectRefRef
└── Languages → Language → TranslationUnit → TranslationElement → Translation
```

### 4.1 Findings

**The `Dynamic` tree is a conditional UI/visibility program, not a flat list.** `choose`/`when` nodes keyed on `ParamRefId` decide which parameters and which communication objects are visible and active for a given parameter configuration. Scale in one real device: 1211 `Parameter`, 2236 `ParameterRef`, 767 `ComObjectRef`, 526 `choose`, 1282 `when` [V]. Rendering a device editor faithfully means **evaluating this tree**, which is the single largest piece of work in an ETS alternative. `test` expressions on `when` need dedicated study (Session 4).

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

---

## 5. `knx_master.xml`

Content of the ETS4 master data file shipped inside our project [V]:

* `DatapointTypes`: 46 main types, 289 subtypes.
* `MediumTypes`: `MT-0` = TP, `MT-1` = PL, `MT-5` = IP. (RF absent in this ETS4 master file.)
* `Manufacturers`: 447 entries, `M-0001` Siemens, `M-0002` ABB, …
* `MaskVersions`: 29 entries, e.g. `MV-0010` mask `16` "1.0" `ManagementModel=Bcu1`, `MV-0020` mask `32` "2.0" `Bcu2`.
* `Languages` / translations for the above.

Each `MaskVersion` carries a `HawkConfigurationData` block containing `Resources` (611 `Resource` entries with `ResourceType`, `Location`, `AccessRights`), `MemorySegments` (47), `Procedures` (74 `Procedure` with `LdCtrlConnect`, `LdCtrlLoad`, `LdCtrlWriteMem`, `LdCtrlWriteProp`, `LdCtrlMerge`, `LdCtrlRestart`, …), `InterfaceObjects` (32) with `Property` definitions, `Features`, and `DownwardCompatibleMasks` [V].

**This is the device-programming rulebook in machine-readable form.** Combined with the application program's own `LoadProcedures` and `AbsoluteSegment` data, the load procedure for a device is *data-driven*, not hardcoded per manufacturer. That makes commissioning technically approachable in principle — see §8 for why it is still out of scope for now.

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

It is nevertheless **out of scope for the current roadmap**, for reasons that are not going to change soon:

1. Writing wrong memory images to a real device bricks it. This needs hardware we can afford to destroy.
2. The `Legacy*` option matrix and partial-download rules are undocumented publicly.
3. Vendor `Baggages` DLLs participate in download for some devices.
4. KNX Secure devices require the key material handling of §9.

Recommendation: build toward *read/diagnose/monitor* first (Session 6), and treat programming as a separate, later, explicitly-flagged research effort. Nothing in the architecture should preclude it — hence keeping `LoadProcedures`, `Memory`, `AbsoluteSegment` and mask data in the model rather than discarding them at import.

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
| R3 | `Dynamic` tree (`choose`/`when`) evaluation is the real complexity | High | Dedicated research spike in Session 4 before any device editor UI. |
| R4 | Round-trip cannot be byte-exact (signatures, attribute ordering, ETS-internal ids) | Medium | Define round-trip fidelity as *semantic* equality over a declared model + verbatim passthrough of opaque parts. Never claim byte-exactness. |
| R5 | Vendor plug-in DLLs make some devices unconfigurable by us | Medium | Detect `Baggages`, mark affected devices read-only, report clearly. |
| R6 | GPL-2.0 contamination via `xknxproject` | Medium | Test-only dependency, enforced by CI. |
| R7 | Product database size/performance (22 MB for 12 programs) | Medium | Indexed, cached, versioned product DB layer; never re-parse per open. |
| R8 | Secret leakage once KNX Secure is supported | High | Isolated key subsystem, excluded from exports/logs by default. |
| R9 | Writing an unsigned `.knxproj` may be rejected by ETS on re-import | Medium | Test explicitly. If rejected, our export is a one-way documentation format and must say so. |

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
11. **Retain low-level programming data** (`Memory`, `AbsoluteSegment`, `LoadProcedures`, mask/resource data) at import even though commissioning is out of scope, so that path stays open. §8.3.
12. **User-facing wording is "KNX-compatible", never "KNX certified" or "full ETS compatibility".** §7, §10.

### Open questions to resolve before or during Session 3

* ETS5/ETS6 schema deltas (13, 14, 20, 21+) — needs sample projects. (R1)
* `Functions` element semantics — absent from our sample. (§3.1)
* `when/@test` expression grammar in the `Dynamic` tree. (R3)
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
