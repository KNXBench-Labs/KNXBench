# OpenKNX product signing: observed mechanism and legal boundaries

Date: 2026-10-14
Scope: public-source inspection requested by the owner; no implementation or legal opinion.

## Evidence and method

Retrieved OpenKNX public source through GitHub raw/API endpoints and KNX Help
Center articles through its public API; retrieved the German KNX Tools license
attachment and German statute texts directly. Inspected the relevant source
passages and PDF text (`pdftotext -layout`), not rendered PDF pages. A general
search helper failed; locale-prefixed Help Center search endpoints returned 404;
the public `help_center/articles/search.json?locale=en-us` endpoint succeeded.
No ETS, OpenKNX executable, third-party add-in or bus operation was run. No
credential or private key was requested, extracted or redistributed.

The owner's third-party GitHub URL contains a literal ellipsis and does not
identify a complete repository URL. That repository and its alleged MCP/add-in
behavior were not assessed. No substitute repository was guessed.

The inspected source files were matched against Git blob hashes from retrieved
trees (not claimed to be tested releases):

| Repository / file | Blob |
| --- | --- |
| OpenKNX/OpenKNXproducer `Program.cs` | `30e324f831cb239773c98def1d9f06ca1fe19a94` |
| OpenKNX/OpenKNXproducer `OpenKNXproducer.csproj` | `0ae0fa069dfd1c351bcf7ceefb4757b12dd4c350` |
| OpenKNX/OpenKNX.Toolbox.Sign `SignHelper.cs` | `94ef07aac79b601ec8c8d46c4f104f48143242ae` |
| OpenKNX/OpenKNX.Toolbox.Sign `XmlSigner.cs` | `99953311ed9bf836e141b918b0f6ae6e13f61093` |

## Findings

**[D] Active path, not the commented historical implementation.**
`OpenKNXproducer.Program.ExportKnxprod` delegates to
`OpenKNX.Toolbox.Sign.SignHelper.ExportKnxprodAsync`. Its project references the
separate Sign project. The older signing block in `Program.cs` is commented
out and is not evidence of the current execution path.[1]

**[D] OpenKNX builds product packages using existing ETS signing code.**
`SignHelper` splits input XML, obtains the ETS path for the namespace, hashes
application data, signs hardware data, patches catalog references, signs the
manufacturer directory and packages it as `.knxprod`. `XmlSigner.cs` loads
`Knx.Ets.XmlSigning.dll` from that path and invokes a non-public static signing
method through .NET reflection.[2], [3]

**[A] Narrow conclusion.** This demonstrates a local-library reuse route, not
an independent KNX signing implementation. The inspected wrappers do not
supply a private RSA key. They do not establish the identity/number of keys
inside ETS, key extraction, a registration grant, certification, dongle
signature generation or arbitrary ETS add-in acceptance. It would overstate
the evidence to infer any of those from this pipeline.[1], [2], [3]

**[D] Project integrity and source signatures are different.** KNX's ETS6
project-export article describes an integrity signature for data validity
and a source signature linking the export to an ETS license. It lists a
five-device demo export limit. It does not establish the quoted universal
RSA-key identity, inner/outer cryptographic coverage or the purported rule
that a missing outer dongle signature proves a cracked ETS.[4]

**[D] Importable is not automatically registered or certified.** KNX describes
unregistered products as parameterizable: up to ETS 6.2.2 they could also be
copied between projects; from ETS 6.3.0 the article documents warnings and
copy/paste restrictions. A separate registration exchange file changes a
product/application registration status. A generic package-signing call
therefore is not evidence of formal registration or certification.[5], [10]

**[D] Add-in signing needs separate evidence.** KNX app validation distinguishes
strong-name signatures on .NET assemblies and multiple additional validation
rules. That does not verify the owner's claimed converter-key add-in workflow;
product XML signatures, assembly strong names and app validation must not be
conflated.[11]

**[A] Compatibility is not a perpetual acceptance guarantee.** Even assuming a
shared legacy credential, accepting old converted products would not logically
require ETS to accept every newly generated package or third-party add-in
forever. No claim about a planned KNX revocation or a tested enforcement bypass
is made.

## Legal reading — not legal advice

The retrieved German KNX Tools license is **v6.18, December 2025**, dated
**2025-12-01** in the PDF. It may differ from the version accepted for a
particular installed ETS. Page 3 limits reverse engineering, copying,
distribution and derivatives; it exempts certain ETS-created/exported project
outputs from copying/distribution/derivative restrictions and describes a
Manufacturer Tool exception for creating/testing unregistered products. Neither
exception expressly authorizes an independent process to reuse the internal
signer or to reproduce KNX origin/license credentials. Pages 3–4 address
proprietary rights/confidentiality; page 5 selects Belgian law and Brussels
courts. These are contractual assertions, not findings that every clause is
enforceable in every jurisdiction.[6]

For a German/EU-oriented preliminary assessment, UrhG section 69d(3) permits
qualified observation/testing during authorized use, section 69e permits
necessary interoperability decompilation under conditions, and section 69g(2)
invalidates conflicting contractual provisions within its specified scope.
Section 69g(1) also preserves other legal regimes. Those provisions do not
provide a blanket permission for extracting/reusing an authenticity key or
circumventing licensing. The contract's Belgian choice of law means that a
German-law summary is not a complete contract-law opinion.[7], [8], [9]

| Activity | Preliminary project assessment |
| --- | --- |
| Inspect public OpenKNX source to understand the dependency | No special concern identified in this bounded inspection; code reuse still requires separate license review. |
| Independently implement interoperability using authorized evidence | Preferred route, subject to the evidence/reuse conditions and other applicable rights; not blanket legal clearance.[7], [8], [9] |
| Call internal ETS signing DLLs from an independent process | Unresolved contractual/legal scope; a local installation or public wrapper is not sufficient proof of permission.[6] |
| Bundle ETS DLLs or KNX-owned signing credentials | High-risk route; not part of KNXBench's chosen architecture.[6] |
| Manufacture apparent KNX registration, certification or ETS-license origin | Not an acceptable KNXBench product claim; the corresponding facts would require actual authority/evidence.[4], [5], [10] |
| Sideload the unnamed add-in to avoid app approval/licensing | Not reviewed or recommended; product signing does not establish app entitlement.[6], [11] |

No evidence was found in the inspected materials establishing OpenKNX's
individual permissions, legal advice, court rulings or KNX's acquiescence.
Public availability and longevity alone do not resolve those questions.

## Consequence for KNXBench

No product decision or runtime change is introduced. Keep the independent
import/domain/storage architecture. [ADR-0028](../adr/0028-no-knxproj-export.md)
already prohibits writing `.knxproj`, so an ETS project-export signature is not
a required dependency. Preserve source evidence without asserting that an
original signature authenticates later modified native data. Do not adopt the
internal ETS signing path, distribute credentials, or add ETS add-in sideloading
on the basis of the quoted forum argument. A proposed adapter would need its
own bounded technical and independent legal assessment before implementation.

## Sources

[1] https://github.com/OpenKNX/OpenKNXproducer/blob/main/Program.cs — OpenKNXproducer active export path
[2] https://github.com/OpenKNX/OpenKNX.Toolbox.Sign/blob/main/SignHelper.cs — OpenKNX Toolbox product signing pipeline
[3] https://github.com/OpenKNX/OpenKNX.Toolbox.Sign/blob/main/XmlSigner.cs — OpenKNX Toolbox ETS library wrapper
[4] https://support.knx.org/hc/en-us/articles/360020990259-Project-export — KNX ETS6 project export signatures
[5] https://support.knx.org/hc/en-us/articles/26285336909330-Handling-unregistered-Products-in-ETS — KNX unregistered product handling
[6] https://support.knx.org/hc/en-us/articles/360002909959-KNX-Tools-Software-License-Agreement — KNX software license v6.18 December 2025
[7] https://www.gesetze-im-internet.de/urhg/__69d.html — UrhG section 69d
[8] https://www.gesetze-im-internet.de/urhg/__69e.html — UrhG section 69e
[9] https://www.gesetze-im-internet.de/urhg/__69g.html — UrhG section 69g
[10] https://support.knx.org/hc/en-us/articles/4659247971346-File-formats-used-during-registration-certification — KNX registration exchange formats
[11] https://support.knx.org/hc/en-us/articles/360001508820-App-validation-rules — KNX app validation and strong names
