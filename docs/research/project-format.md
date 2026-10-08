# Research — Project file format

The `.knxproj` container, the project content model, `knx_master.xml`, group addresses and DPT resolution, and opaque data. Part of [RESEARCH](../RESEARCH.md), which holds the evidence
tags (**[V]** verified here, **[D]** documented, **[A]** assumption), the
section index and the sources. Section numbers are global and stable;
dated entries are newest first. Moved here verbatim from `RESEARCH.md` on
2026-10-04 (AR14D D4); only relative links changed.

## 2026-09-29 — Read-only inventory of local ETS installation data

*Recovered 2026-10-05 from an uncommitted edit in the shared root checkout; the entry was written on 2026-09-29 (with the 2026-10-01 TEMP-capture addendum) before the AR14D split and never reached `main`. Content unchanged; the `PRODUCT_DATABASE_CORPUS.md` link is adjusted for this directory.*

- **[V]** The off-repository ETS installation-data directory was inspected
  read-only (filesystem inventory, XML roots/namespaces, ZIP directories and
  bounded ZIP member hashes). Its 948 files contain **no** `.knxproj` or
  `.knxprod`. ZIP members were read in memory for bounded hashing but not
  extracted onto disk; nothing was altered, executed, imported into a
  database, or placed in Git. `Licensing/` and connection configuration were
  deliberately not inspected for content.
- **[V]** ETS5 and ETS6 each have one live project-store tree and respectively
  eight and two `.restorepoint` files. Each restore point is a readable ZIP with
  319 entries (298 files), including extensionless XML fragments and manufacturer
  files, but without the `Project.xml` / `0.xml` structure the KNXBench
  `.knxproj` importer requires. The newest restore point of each generation
  matches its corresponding live tree byte-for-byte on all 298 included files;
  older restore points differ, so they are potential *historical* regression
  sources, not ten independent projects. ETS5 has a populated separate product
  store; the ETS6 product store is empty. The internal stores and restore points
  are **not** direct KNXBench import fixtures. KNX Association documents them as
  ETS-local stores/restore points and recommends restoring/exporting through ETS:
  https://support.knx.org/hc/de/articles/360020712719-Datenablage and
  https://support.knx.org/hc/de/articles/115001130890-Datenablage .
- **[V]** Six loose `knx_master.xml` files cover namespaces 11, 12, 14 (two
  distinct files), 20 and 23. The copy in a directory named `project-13` has
  **namespace 14**, illustrating why folder names cannot determine scheme.
  The scheme-11 and scheme-23 master files are byte-identical to masters in
  the existing private `.knxproj` demo fixtures. The other four are useful
  read-only master-data/differential fixtures, **not** complete projects or
  standalone product packages. There are also three `knx_cvexc.xml` files;
  their semantics were not assessed. The existing product corpus already
  exercises schemes 12, 14 and 20 in actual packages
  ([PRODUCT_DATABASE_CORPUS.md](../PRODUCT_DATABASE_CORPUS.md)); no new package
  compatibility is established by these loose XML files.
- **[V]** ETS4 holds a SQL Server-style `.mdf`/`.ldf` database pair (the
  `.mdf` is about 849 MB), not a `.knxproj`. Its contents were not opened.
  KNX Association describes conversion of ETS4 `.mdf` through its separate,
  end-of-life project exporter to `.knxproj`:
  https://support.knx.org/hc/de/articles/360001690379-ETS-Projekt-Exportassistent .
- **[A] Next useful experiment:** With an authorized ETS environment, export
  selected *different* restore-point revisions to `.knxproj` on a copy, then
  run the existing importer and compare per-version counts, warnings, unknowns
  and opaque preservation. Do not reverse-engineer or modify the live store as
  a shortcut; never publish project identifiers, addresses, licensing data or
  raw exports as test fixtures. This inventory alone does not establish
  compatibility or losslessness for any new project schema.

### Additional user-profile data (same day)

- **[V]** A read-only filename scan of the off-repository Windows user profile
  found two `.knxproj` files in Documents/Downloads. The first is byte-identical
  to the existing private scheme-11 demo project. The second has the same 39
  ZIP *files* as the existing private scheme-23 demo project: 38 are
  byte-identical and only `knx_master.xml` differs (an older master-data
  revision). Thus neither supplies a new project topology or application
  program. The production `knx import --no-product-db` path was run on the
  second file and the existing scheme-23 sample, each with a temporary native
  store and an ephemeral report: both exited 0, mapped 35 device instances,
  514 group addresses and 867 communication-object references, and reported
  9 unknowns, 789 opaque entries, 3 unsupported features and no errors or
  warnings. This is an importer comparison, **not** proof of semantic
  losslessness or roundtrip compatibility. No project or report was retained.
- **[V]** One separate small XML file is a
  `{http://knx.org/xml/telegrams/01}CommunicationLog` with 71 `Telegram`
  records. Each has a hex `RawData` attribute beginning with `29` (a cEMI
  `L_Data.ind` marker according to `knx-net/src/cemi.rs`), plus timestamp and
  capture metadata. It may be useful as a **private, offline decoder fixture**
  after sanitization and verification against `decode_l_data`; KNXBench does
  not currently import this capture XML. No addresses, frames, connection
  names or times were published.
- **[V] 2026-10-01, separate TEMP captures:** Two more private ETS Group Monitor
  XML files under `/mnt/system/TEMP/` use the same `CommunicationLog` namespace
  and contain 578 and 582 `Telegram` records. The first frame sequence is an
  exact prefix of the second; the second adds four group-communication frames.
  These are two exports of effectively **one recording**, not two independent
  fixtures. **[D]** KNX Association describes its XML telegram export as
  largely ETS-internal and the telegram data as unencrypted hex cEMI; that
  article does not specify a stable third-party interchange contract:
  https://support.knx.org/hc/en-us/articles/360019034599-Telegram-recording-options .
  **[V]** All 1,160 records have valid hex `RawData`, matching cEMI
  `L_Data.ind` (327/331) or `L_Data.con` (251/251) markers and parseable
  timestamps; confirmation error flags are clear. The files also carry private
  capture metadata and must not be committed as fixtures. A bounded, DTD-free
  offline XML read fed the raw bytes in memory to the production
  `knx_net::cemi::decode_l_data` implementation: **578/578 and 582/582
  decoded**, with semantic `decode(encode(decoded))` equality for every frame.
  In the larger export the decoded services include 43 group writes, 20 group
  reads, 20 group responses, 108 memory reads, 108 memory responses, 20 memory
  writes, four property reads and responses each, and one each of authorization
  request/response, descriptor read/response and restart; 250 control PDUs are
  present. The smaller export omits four group-communication frames but covers
  the same other service categories. This is useful **offline regression
  evidence for existing cEMI/commissioning decoding**, especially against
  real management traffic; it reveals no current decoder failure and proves
  neither byte-exact replay, semantic validity of ETS metadata, bus write
  safety, nor `.knxproj`/manufacturer import compatibility. KNXBench still has
  no ETS `CommunicationLog` XML importer. Any future checked-in regression
  fixture must be explicitly sanitized or synthetic, with addresses, payloads,
  timestamps and connection metadata reviewed before publication.
- **[V]** The scoped Documents, Downloads, Desktop, OneDrive and KNX-specific
  AppData subtrees contained no additional product packages or project exports.
  AppData's KNX trees hold settings, caches, workspaces and logs, not additional
  `.knxproj` files; their content was deliberately not opened. A broader
  filename-only pass was stopped at its entry budget, so this is **not** a
  claim to have searched every file in the entire user profile. Nothing was
  changed in the source directory or on the KNX bus.

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

**Amendment, measured during Session 3 implementation: the 758 `DatapointType` occurrences are not all alike.** 497 of the 758 are the empty string (`DatapointType=""`); only 261 carry an actual value. The empty string is ETS's way of saying "the program's datapoint type is deliberately cleared here" — it is not the same as the attribute being absent (149 of the 907 instances carry no `DatapointType` attribute at all: 497 + 261 + 149 = 907). A model that collapses "present and empty" into "absent" cannot write the file back the way it read it. See [ADR-0010](../adr/0010-per-attribute-override-representation.md).

758 of 907 instances carry a `DatapointType` attribute at all (497 empty, 261 valued), leaving 149 with none. **An import that reads only the application program produces wrong data for the large majority of objects.** The core model must represent each layer separately (or store resolved values *plus* the layer they came from), otherwise export cannot reconstruct the original file and edits cannot be attributed.

`ComObjectInstanceRef/@RefId` is a compound key of the form
`M-006A_A-0001-22-26C0-O0079_O-0_R-10001` — application program id + `_O-<ComObject number>` + `_R-<ComObjectRef id>`. It simultaneously identifies the base object and the variant. **This is schema-11-specific — see §3.3.**

**The communication-object number is the `O-<digits>` segment immediately preceding the final `R-<digits>` segment, not the first `O-` token in the string.** The application program identifier that precedes it can itself contain an `O`-looking token — for example `M-006A_A-0001-22-26C0-O0079_O-0_R-10001`, where the *program* id already contains `-O0079` and the true object number is the `0` in the trailing `_O-0_R-10001`. Verified against all 907 `ComObjectInstanceRef/@RefId` values in the reference project during Session 3 implementation (`knx-etsproj`'s `com_object_number`): 0 mismatches, 0 parse failures.

**`ParameterInstanceRef/@RefId` references two distinct parameter kinds**, measured during Session 3 implementation: 1174 plain parameters (`..._P-<n>_R-<n>`) and 216 union parameters (`..._UP-<n>_R-<n>`), 1390 total. A parser that matches only the `_P-` form silently drops the 216 union ones — 15.5% of the parameter values in this project.

### 3.3 Content-model differences, ETS4 (schema 11) vs. ETS6 (schema 23) [V]

Same diff as §2.4, one level down, on `0.xml` itself. These are real, load-bearing format changes, not noise:

* **`RefId` is no longer self-contained.** Schema 11 writes the full compound key (`M-0083_A-0019-16-ECA7_O-59_R-149`); schema 23 writes only the local part (`O-59_R-149`). The application-program identity must now be recovered from the owning `DeviceInstance/@Hardware2ProgramRefId`, not from the reference string itself. A parser that treats `RefId` as globally unique and self-describing (reasonable for schema 11) breaks silently on schema 23.
* **Group address links move from child elements to an attribute, and gain a device-wide index.** Schema 11: `ComObjectInstanceRef` nests `Connectors/Send` and `Connectors/Receive` elements, each with `@GroupAddressRefId` (fully qualified, `P-0512-0_GA-373`). Schema 23: the link is a `Links` attribute directly on `ComObjectInstanceRef` (short id, `GA-373`; presumably space-separated for multiple links, unverified here since every observed instance has exactly one), **and** every `DeviceInstance` gains a `GroupObjectTree/@GroupObjectInstances` attribute — a space-separated list of every communication-object `RefId` on that device, including ones with no override and no link. Send vs. Receive direction is no longer encoded positionally; it must come from the application program's `ComObject/@ReadFlag`/`WriteFlag` defaults (or the instance-level flag overrides), which schema 23 still carries as before. **Amendment 2026-09-29 — both guesses in this bullet were wrong.** Project Schema23 v01.00.00 (the direct PDF), `ComObjectInstanceRef/@Links`: "The list of (shortened) group address ids that are linked with this object. The first group address in the list is always the sending one." So the list *is* space-separated, and direction *is* positional: the first entry sends, every later one receives. The same house's ETS 6.3.0 export has multi-link objects after all (e.g. 1.1.22 object 6 `Links="GA-96 GA-200"`, 1.1.24 object 56 `Links="GA-232 GA-700"`), and against its ETS4 export — which states direction with explicit `Send`/`Receive` elements — all 543 objects with one ETS4 sender and an identical address set agree on that sender, 2 of them with more than one link (`crates/knx-etsproj/tests/links_direction.rs`). The importer used to map every `Links` entry to `Send`, which made both objects look like they send on two addresses and the download planner refused both devices (§19.12).
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
* `GroupObjectTree` — present per `DeviceInstance`, alongside `ComObjectInstanceRefs` (both coexist, as in schema 23), **but its internal shape differs from schema 23's.** Schema 23 (§3.3): a flat `GroupObjectTree/@GroupObjectInstances` attribute. Schema 21 (this sample): `GroupObjectTree/Nodes/Node[@Type='Channel']/@GroupObjectInstances` — one `Node` per module channel, each listing that channel's live communication-object `RefId`s space-separated. Both shapes serve the same role: verified on all 4 KV devices that every `ComObjectInstanceRef/@RefId` is a member of `GroupObjectTree`'s id set with zero exceptions, while `GroupObjectTree` carries substantially more ids (e.g. one device: 5 `ComObjectInstanceRef`s against 26 `GroupObjectTree` ids) — the same undercount §3.3 measured for schema 23 (24%), now confirmed on independent data and worse for module-heavy devices. See [ADR-0014](../adr/0014-group-object-tree-authoritative-source.md).
* **`ComObjectInstanceRef` carries almost no overrides for module-based devices.** Measured across every `ComObjectInstanceRef` in the sample: only `RefId`, `ChannelId`, `Links` ever appear — never `DatapointType`, `Text`, `Description`, or a flag override. `ChannelId` (new, not in schema 11 or §3.3's schema-23 diff) points at the `GroupObjectTree/Nodes/Node` grouping it belongs to. DPT/Text for these objects must be resolved through the module chain (below), not the instance layer.
* **`ModuleInstance` → `ModuleDef` resolution chain**, measured against the KV project's `M-00FA` application programs: `DeviceInstance/ModuleInstances/ModuleInstance/@RefId` (local id, e.g. `MD-2_M-1`) resolves via the owning `DeviceInstance/@Hardware2ProgramRefId` to `ApplicationProgram/ModuleDefs/ModuleDef/@Id` (e.g. `M-00FA_A-2504-10-C071_MD-2`) — the same short-id-recovered-via-owning-element pattern §3.3 already established for schema-23 `ComObjectInstanceRef/@RefId`, now shown to apply to `ModuleInstance/@RefId` too. `ModuleDef` internally repeats the existing `ApplicationProgram` shape one level deeper: its own `Static/ComObjects` + `ComObjectRefs` (identical attribute set to the top-level ones already modelled by `knx-productdb`), plus a `Dynamic/Channel/ParameterBlock/choose/when` tree (the same grammar covered by §4.3: its `when/@test` value grammar is now documented from the KNX Standard, while the surrounding structural grammar remains corpus-observed only) that picks which `ComObjectRef`s are active for given argument values. `ModuleInstance/Arguments/Argument/@RefId` supplies those argument values per instance (e.g. `argCH=1` — which channel number this repetition represents). Import does not need to evaluate `choose`/`when` itself: `GroupObjectTree` already carries ETS's own evaluation of it (previous bullet), so the active-object set is read, not recomputed.
* `Puid` — present (`Area`, `Line`, `GroupRange`, `GroupAddress`, …). **Corrects `known.rs`'s existing test comment, which called `Puid` "schema 23 only" — it is at least schema-21-and-up.**
* `Locations`/`Space` — present, replacing `Buildings`/`BuildingPart` already at schema 21.
* `GroupAddress` gained a `DatapointType` attribute directly (not present at schema 11) — potentially resolves the ambiguous space-separated `DatapointType` list problem (§4.2/KNOWN_LIMITATIONS §12) for schema ≥ 21 specifically, since the group address itself states its type rather than requiring inference from a linked communication object. Unverified whether it is ever ambiguous/multi-valued here — this sample's 13 group addresses all carry single, unambiguous values.
* `ProjectInformation` gains `Comment`, `Guid`, `LastUsedPuid`, `ProjectType`; loses `CompletionStatus`, `Hide16BitGroupsFromLegacyPlugins`, `ProjectId`, `ProjectTracingLevel` (relative to schema 11 — not cross-checked against schema 23 here).
* `ProjectTraces`/`ProjectTrace` (new) — an audit-log-shaped element, not seen in schema 11 or documented in §3.3's schema-23 diff. Purpose not investigated.
* **New, not seen in §3.3 at all:** `ModuleInstances`/`Arguments` per `DeviceInstance` — modular application programs, where a device's configuration is composed from reusable modules with their own argument sets rather than one monolithic application program. This is a real domain-model gap (absent from [DATA_MODEL.md](../DATA_MODEL.md)), not just a parser detail.
* `Security` element per `DeviceInstance` — present, matching §3.3.
* `Line`/`Area` lose `CompletionStatus` (and `Line` loses `Name`) relative to schema 11 in this sample — possibly because this demo project was never "downloaded" to real devices, not necessarily a schema-wide change; needs a second schema-21+ sample with real completion-status history to confirm.

**Session 7 (2026-09-06) amendment, brainstorming pass following cycle 1:** the domain-model decision this section called for is now made — [ADR-0013](../adr/0013-module-instance-representation.md) (`ModuleInstance` as a first-class entity) and [ADR-0014](../adr/0014-group-object-tree-authoritative-source.md) (`GroupObjectTree` as the authoritative object-list rule, schema-version-generic). Design: [`docs/superpowers/specs/2026-09-06-schema-21-23-import-support-design.md`](https://github.com/KNXBench-Labs/KNXBench/blob/6a1ba6ae5d54/docs/superpowers/specs/2026-09-06-schema-21-23-import-support-design.md). Building `known_schema(21)`/`(23)` and wiring the mapper is the implementation that design hands off to — tracked in [ROADMAP.md](../ROADMAP.md) and [KNOWN_LIMITATIONS.md](../KNOWN_LIMITATIONS.md) §1.

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
