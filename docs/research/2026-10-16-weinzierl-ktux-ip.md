# Weinzierl kTux IP 1.2.0: useful independent test material

Date: 16 October 2026. Scope: owner-supplied archive inspection, original manual,
XML inventory and bounded static native analysis. No manufacturer executable was
run, no bus was accessed and no KNXBench implementation was changed.

## Decision

**Keep this package as a private test/research candidate, not a KNXBench dependency.**
Its strongest contributions are an independently implemented software KNX device,
a comparatively convenient exact `57B0h` relative-download example, and a large
manufacturer-data fixture. Neither execution interoperability nor production
import compatibility was established in this inspection. It does not resolve the
Siemens UP258/System-2 source-encoding and recovery blockers.

## Identified source

The supplied stem resolves to the existing file
`OriginalData/Misc/weinzierl-ktux-ip-v1.2.0.tar.gz`, not an extracted directory.

- Archive: 1,787,492 bytes; SHA-256
  `3a5d41cc37f5cbdd7c4bb5cd4e09e44f5fae4f210ddfeb56d7c4da706789378f`.
- Twelve tar entries: four directories and six ELF files, one PDF and one project.
  All entries are regular files/directories, with unique safe relative names;
  only the PDF, project and two desktop ELFs were copied to private scratch.
- Desktop, Raspberry Pi and Raspberry Pi 64-bit distributions each have
  `kTux-ip` and `kTux-ip.debug`. Only the desktop pair was analyzed as native code.
- No source files or standalone LICENSE file occur in this archive. The manual
  describes a free, as-is version and separately offered commercial solutions;
  it is not evidence of permission to copy code or redistribute the binaries.
  No legal clearance or license change is asserted.

Selected member SHA-256 values:

| Member under `kTux-ip-v1.2.0/` | SHA-256 |
| --- | --- |
| `Manual.pdf` | `96a33899da5c72d1323e923b45bfa71b93bf765d13c20a63e02661d900edf9f0` |
| `linux/kTux-ip` | `d97262dd4f20fc2e01951fcdeee79f949fe4cbb450cab37d798f78ec711a6467` |
| `linux/kTux-ip.debug` | `f2a9f9e944fd78fe2960ae4281347a2e0b16c7762a4b6624eb8c7e02e2174dac` |
| `kTux-ip.knxproj` | `a3f4599754fa6a29cbfb8b6ca45491f7c4709791a338209551def0b42401e46e` |

The original manual, dated 12 February 2024, introduction and §§4.3–5 describe
an ETS-configurable device stack with a BAOS application interface on port 12004,
not an engineering client or ETS replacement. The official product page was also
retrieved successfully on 16 October 2026:
<https://weinzierl.de/en/stacks-for-knx/ktux-linux/>. It lists the IP demo v1.2.0
and confirms that architecture; its general Secure advertising is not verified
Secure support in the inspected binaries. Nothing was requested from the vendor.

## 1. Independent software-device candidate — high value, not yet exercised

The desktop binaries are x86-64 Linux executables. A lab containing only software
could potentially exercise actual wire framing, routing, management, load-state
transitions and readback against a device implementation not written by us.
This would complement, not replace, the existing fault-injection simulator.
That simulator's own module contract explicitly excludes cEMI-octet and real-time
interoperability coverage (`knx-net/src/commissioning/simulator.rs`).

Static evidence includes named routing, property read/write, extended-memory,
load-control and persistent-memory functions. This is evidence of code paths,
not proof that every service is enabled or works in a running demo. The manual's
BAOS control is an application/device API, not automatically a KNXnet/IP tunnel.
Do not assume the current management transport can simply connect to BAOS.

The appropriate next experiment is an optional, network-isolated, disposable
software-device lab with a verified IP management path, original executable,
explicit interfaces and captured wire traffic. Do not run the default interface
selection on the building network. Startup, management reachability, successful
configuration and post-restart readback need separate acceptance results.

## 2. Exact 57B0 relative-image witness — high research value

The nested project contains seven unencrypted ZIP members. Its XML identifies
ETS5 `5.7.1428.39779` and namespace `http://knx.org/xml/project/20`.
The installation has **one device and one group address**, not 1,000 installed
devices. Its embedded application `M-00C5_A-FE09-10-3B92` declares:

- mask `MV-57B0`, `MergedProcedure`, relative segment on load-state machine 4;
- one 62,126-byte segment, with Data and Mask independently base64-decoded and
  each checked to contain exactly 62,126 bytes;
- address/association capacities of 2,000 entries each;
- merge fragments 2, 4 and 7, with full/parameter relative allocation,
  relative memory writes, six memory-control-block property writes and image
  properties for object indices 1–4;
- 1,000 distinct base communication objects numbered **1 through 1,000**,
  with every base descriptor's communication/read/write/transmit/update/
  initial-read flag disabled.

Compared with the published KNXBench baseline
`32b4bb81746899861e134b6d5970aa9414b37a98`, the relative source reader, explicit
System-B link-table codecs and complete descriptor constructor already exist.
They must not be duplicated. The contracts
`offline-relative-source-segments`, `offline-systemb-link-tables` and
`offline-systemb-object-table` still distinguish those primitives from a complete
source-bound relative image and executable procedure.

This exact product is a promising next witness because its dense one-based
source object numbers and complete disabled base declarations avoid the sparse
source-number issue of some previously selected products. That does **not** yet
prove Dynamic evaluation, every resulting descriptor, source-number-to-ASAP
binding, image substitution, generated table placement or recoverable execution.

Ghidra inspection of the **debug executable** corroborates this family:
`KnxLc_OnLoadSegment` selects relative-allocation subtype `0Bh` in Loading;
`KnxLc_OnDataRelAllocation` reads a four-octet big-endian size, checks the object's
bound and conditionally fills memory. `KnxLc_GetMemStartRefByObjIdx` has explicit
mappings for indices 1–4. These are observations about this shipped build, not
portable placement constants for the domain model.

## 3. Product-data and UI stress fixture — immediately useful as a candidate

Independent XML counts for this one embedded application:

| Source declarations | Count |
| --- | ---: |
| Parameters | 4,002 |
| Parameter references | 4,002 |
| Unions | 3,002 |
| Base communication objects | 1,000 |
| Communication-object references | 16,000 |

The application XML alone is 18,327,616 bytes. This is useful for importer,
activation, text-substitution and large parameter/object-editor regression work.
It is **not** 16,000 simultaneously active communication objects, and the tiny
saved installation is not a large-topology benchmark. Run the production importer
and evaluator in disposable storage before adopting expected persisted counts or
claiming compatibility; only the existing read-only project inspector was run here.

## 4. Debug information and storage caveats

`linux/kTux-ip.debug` is a complete, unstripped ELF **executable**, not merely a
split-symbol sidecar. It contains DWARF types, function names and build paths;
Ghidra enumerated 1,327 procedures. Its GNU build ID differs from the stripped
executable. Bind every observation to the exact digest and do not attach its
symbols to the other executable as though the builds were proven identical.
No original C source is supplied, and decompiler output is not original source.

`PfiEeprom_WriteMemFile` and `PfiVmm_WriteFile` show file-backed memory persistence,
including encrypted data and a checksum. A `.json` suffix does not make this a
plaintext memory oracle. No key extraction, decryption or storage modification
was performed. Future reset/replay tests must verify which complete state is
saved, restored and flushed, rather than treating these files as KNXBench recovery.

`KnxMem_ReadUint16` assembles a big-endian internal memory word. This does **not**
resolve ETS parameter encoding: the supplied application has only 8-bit numeric/
restriction types plus text types and contains no `ParameterByteOrder` token.
It provides no discriminating multibyte-numeric Siemens/System-2 evidence.

## Verification and retained evidence

- Tar/ZIP inventories, strict base64 lengths, XML ancestry/counts, complete object
  numbering and disabled base flags checked with Python stdlib.
- `python3 tools/inspect_knxproj.py <private-copy>/kTux-ip.knxproj`: exit 0;
  this is container/project inspection, not the production importer.
- `file`, `readelf`, `nm` and bounded Ghidra decompilation used without executing
  the supplied programs. Initial optional pwntools layout provider was unavailable;
  Ghidra was available. One guessed table-function name did not resolve; no
  finding relies on that failed lookup.
- Private REA export: profile evidence `ktux-inspection-20261016/rea-evidence.json`,
  12 records. Key evidence IDs: allocation `ev_c04adbbe7e1804e1b1ad0c48c0ff605c51b700bcf3ec0c6fbbdac35ca7d08c5a`,
  memory-word read `ev_403e1b470790395751b5d30f5b5029b21cebc178261390855cba6416506ca63c`,
  persistence `ev_d86e0ddd4d2cfba383ba7ca499959f37594140dcf1684c82c3bf8cef5c47ce38`.
- No production code, dependency, hardware admission or compatibility claim was
  changed. No runtime tests/build, real-device operation, vendor contact, commit
  or push is part of this bounded inspection. Documentation checks and final
  preservation results are recorded in the task handover.
