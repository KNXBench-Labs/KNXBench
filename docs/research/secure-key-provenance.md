# KNX Secure key provenance — bounded research

Research date: 10 October 2026. Package R-SEC-01; **research only**.
Source inspection baseline: `fd6b1cff37b2713361e256282cc2aef46e2cbcb9`.
Primary articles were retrieved through KNX's public Help Center API; the
public Schema23 PDF was downloaded and its text inspected. No keyring,
protected user project, device certificate, live bus or real secret was opened.
Documentation evidence is not interoperability or hardware acceptance.

## Answer and access routes

**Both project XML and exported keyrings are documented sources.** KNX's
Data Secure article says ETS generates a separate runtime key for each secured
group address and installs it during commissioning; device commissioning keys
are also stored in the project export.[1] An FDSK bootstraps initial device
commissioning, not general group-address runtime access.[1][6]

Project Schema23 v01.00.00 (1 March 2024) explicitly declares these locations:[8]

| Purpose | Schema23 location | Inspected clause/page |
| --- | --- | --- |
| Data Secure runtime group key | `GroupAddress_t/@Key` | §1.2.7.3, pp.59–60 |
| Current/last-downloaded device tool key | `DeviceInstance_t/Security/@ToolKey`, `@LoadedToolKey` | §1.2.5.3, p.45 |
| IP routing multicast key | `Installation/@IPRoutingBackboneKey` | §1.2.3.13, p.38 |
| Last-downloaded routing key | `Security/@LoadedIPRoutingBackboneKey` | §1.2.5.3, p.45 |
| IP device credentials | `Security/@DeviceAuthenticationCode`, `@DeviceManagementPassword` and corresponding hash/loaded-hash attributes | §1.2.5.3, p.45 |
| Factory credential | `ProjectInformation/DeviceCertificates/DeviceCertificate_t/@FDSK` | §1.2.3.6, pp.36–37 |

The group key is shared by that group's participants. Presence is optional;
this does not establish availability, current device state or successful
extraction from a real protected export.[8] KNX requires a project password
for Secure projects and for their import/export/open operations; without
protection, exported XML could disclose the keys.[2] Authorized access to the
project and its password is therefore a prerequisite, not a bypass.[2]

### Two distinct keyring exports

- **Project Security → Backup keyring:** complete datasets for devices with
  Secure Commissioning enabled; FDSKs can optionally be included. The export
  password protects sensitive fields rather than encrypting the entire XML
  file. It is separate from opening the project.[2]
- **Device → Export Interface Information:** a selected tunneling interface's
  export contains only group-address information assigned to that interface.
  The interface's secure-group capability setting matters; do not assume every
  group or every commissioning credential is present.[5]

External clients are an explicitly documented use case: KNX publishes
password-derived decryption pseudocode for backbone keys, interface passwords
and authentication, device management credentials, tool keys and group keys.[3]
The recipe uses PBKDF2/HMAC-SHA256 with 65,536 iterations and the keyring-specific
salt `1.keyring.ets.knx.org`, plus AES-128-CBC; password payload treatment differs
from raw key payloads.[3] This establishes a supported **information source**,
not that KNXBench implements or has validated that recipe.[3]

The keyring article additionally specifies canonical element/attribute
serialization and a password-dependent truncated SHA-256 signature, omitting
the signature attribute while calculating it.[4] Future import must verify
this before admitting any decrypted credential; no validation was implemented
or exercised here. The article's prose alone is not an exhaustive versioned
keyring grammar or known-answer corpus.[4]

## Keep the security layers separate

| Layer | Material and purpose | Boundary |
| --- | --- | --- |
| Data Secure | Runtime group key; device tool key for secured device management | Application/group security, not an IP tunnel password.[1][6] |
| IP Secure | Routing backbone key; tunneling/interface and management credentials | IP transport security is not a substitute for Data Secure.[7][3] |
| Factory device authentication | Manufacturer certificate binds FDSK to device serial | Bootstraps commissioning; reset/recovery is manufacturer-specific, not authorized by this research.[6] |
| Project password protection | Password-derived protection of exported project container | Schema23 §4.2.4 p.64 describes AES-256 nested ZIP protection; not a runtime group key.[8] |
| Project source signature | ETS/trusted-manufacturer origin policy | Schema23 §1.2 p.4: separate trusted-origin import restriction; knowing a password/key does not authorize edited-project signing.[8] |
| Keyring integrity/signature | Password-dependent canonical serialization check | Different purpose and mechanism from trusted project-origin signing.[4] |
| Legacy device access key | KNXBench's existing 32-bit access-key service | Not a Data Secure tool key or factory credential; do not rename/reuse it as one. |

**Documentation inconsistencies remain visible.** The general device article
and Data Secure article disagree on whether the tool/device key is visible to
an ETS user; the Security/device articles also give inconsistent certificate
character-count descriptions.[6][1][2] This research does not derive a parser,
certificate validator or universal UI-visibility rule from those statements.
Field provenance and export scope are stronger evidence than those counts.
The retrieved IP Secure article labels itself deprecated; use it for historical
terminology only, and obtain the current normative transport specification
before any implementation.[7]

## Current KNXBench source boundaries

Verified by reading the named source, **not by a new runtime test**:

- `crates/knx-secure/src/lib.rs`: only project ZIP-password derivation and
  `zipcrypto` are exposed. `ZipPassword` redacts `Debug`, has no serializer,
  and offers explicit `expose()`. Its documentation disclaims zero-on-drop;
  this must not be advertised as a secure credential vault.
- `crates/knx-etsproj/src/container.rs`: ZipCrypto and AES container admission
  remain distinct. ZIP-password derivation does not establish AES-project
  import, keyring import or Secure telegram support (KL-13).
- `crates/knx-etsproj/src/parse/installation_v21.rs`: device `Security` is
  retained as an uninterpreted per-device subtree, not a typed credential.
  `known.rs` recognizes only the existing counter/timestamp vocabulary there.
- `crates/knx-etsproj/src/opaque.rs` retains original container/member and
  unmodelled content; native preservation can therefore retain sensitive
  source data. **Uninterpreted does not mean sanitized or encrypted.** A
  project/source backup still needs appropriate access controls.
- `crates/knx-app/src/contribution_structure.rs`: secret-like XML names mark
  reduced contribution analysis sensitive; this is a conservative disclosure
  guard, not a keyring parser or guarantee that every unknown secret is found.
- `crates/knx-app/tests/secure_capable_not_activated.rs` pins synthetic plain
  behavior for secure-capable products without activated project security.
  Its existence and prior ledger evidence are not a freshly executed hardware
  or protected-project test. KL-8 remains the runtime boundary.
- `crates/knx-app/src/access_key.rs` is legacy access authorization. No typed
  Data Secure runtime-key store, keyring decoder or secure telegram adapter
  was found among the inspected exported modules.

Do not turn preserved Security XML or an unknown `Key` attribute into a new
runtime credential through an ordinary DTO, report, log or MCP response.
Scope, project/installation ownership and installed-state freshness must be
admitted by a dedicated credential subsystem first.

## Bounded follow-up proposal (not authorized implementation)

Start with **offline `.knxkeys` admission**, not direct project extraction:

1. Obtain the current normative keyring grammar/crypto serialization contract
   and publisher known-answer vectors; include Unicode, lengths, malformed
   XML, namespaces, duplicate identities, wrong passwords and tampered input.
2. Design an isolated secret type/vault with explicit exposure, no ordinary
   serialization/debug, lifetime/zeroization policy and encrypted-at-rest
   storage decision. Threat-model retained source/native/history separately.
3. Admit only a user-supplied keyring/password; authenticate before use, bound
   input/resources, bind project/installation/interface scope and refuse
   ambiguous, missing, foreign or stale material. Never silently use a current
   project key as proof of a last-downloaded device key.
4. Initially expose only value-free coverage/refusal metadata. Document limited
   interface exports distinctly from complete backups. No automatic key
   synthesis, certificate recovery, rotation or protocol operation.
5. Independently verify offline vectors and authorized exports. Only a new
   explicit scope and suitable hardware go may add Secure runtime/commissioning;
   anti-replay counters, resynchronization, transport security and recovery each
   need their own verified contracts.

Open: actual modern protected-project extraction; complete keyring/version
coverage; credential freshness after partial commissioning; current normative
IP Secure source; every secret-bearing export path; native/history secret
storage policy; ETS/hardware interoperability. None is claimed solved here.

## Sources

[1] https://support.knx.org/hc/en-us/articles/360012689639-KNX-Data-Secure — KNX Data Secure
[2] https://support.knx.org/hc/en-us/articles/360018505379-Security — Security
[3] https://support.knx.org/hc/en-us/articles/360001582259-Use-keyring-outside-ETS-Falcon-SDK — Use keyring outside ETS & Falcon SDK
[4] https://support.knx.org/hc/en-us/articles/18968368409874-Keyring-File-Format — Keyring File Format
[5] https://support.knx.org/hc/en-us/articles/360022048859-Devices-panel-Detailed — Devices panel (Detailed)
[6] https://support.knx.org/hc/en-us/articles/360000216419-KNX-Secure-Devices — KNX Secure Devices
[7] https://support.knx.org/hc/en-us/articles/360012666599-KNX-IP-Secure — KNX IP Secure
[8] https://support.knx.org/hc/nl/article_attachments/17389755651474 — Project Schema23 v01.00.00 (1 March 2024)
