# Unverified package signatures

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-085)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 85. A `.signature` package member is stored with role `Signature`, never verified

**Limitation.** `install_package` (`crates/knx-productdb/src/package.rs`)
recognises any ZIP member whose path ends in `.signature`, records it in
`package_member` with `role = 'Signature'`, and stores its bytes verbatim
in `source_file` — the same treatment `notes.txt` gets under
`role = 'Unrecognized'`. No code path anywhere in this crate, in
`apps/knx-server` or in `apps/knx-web` reads a `'Signature'`-role member
back out to check it against a key, a hash, or anything else **[V]**
(`grep -rn '"Signature"' crates/knx-productdb apps` finds exactly one
writer — `package.rs` — and two readers that only forward the string for
display: `apps/knx-server/src/routes.rs`'s `CatalogInstallMemberDto`,
which since the display fix below qualifies the text rather than passing it
through verbatim, and `apps/knx-web/src/api.ts`'s matching TypeScript type,
which still passes it through untouched). Neither reader opens the file. A row that said `Signature` looked, to anyone reading the
install report, like something was signed and checked. Nothing was — see
"Lifted when" below for the display fix task 05 (T05) shipped for that.

Every `.knxprod` file in the local corpus (`OriginalData/ProductDatabases/`,
copied to a scratch directory for inspection, never modified in place)
carries exactly one such member, and every one observed is 175 bytes: a
UTF-8 byte-order mark followed by about 172 base64 characters with no line
terminator — decoding to roughly 129 raw bytes, the size of a single
RSA-1024 signature **[V]** (`file` and a byte count against the extracted
member). That last interpretation — that it *is* an RSA-1024 signature —
remains this report's own inference from the byte count, not a confirmed
algorithm **[A]**; T05 found nothing that raises or lowers that confidence
and did not re-derive it.

**Cause.** Verifying the member requires two things this project does not
have: the `.signature` file's own format/algorithm/canonicalization, and
the manufacturer's public key to check it against. Neither exists in any
corpus this project can reach, checked two ways:

- The accessible KNX Standard corpus
  (`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`)
  was searched by direct text grep for `.signature`, `knxprod`, and
  `signature` generally. It documents a *different* concept under the same
  word: a "registration signature" is a value ETS/the Manufacturer Tool
  computes over registration-relevant XML data so that a later change to
  that data can be detected on an XML→DB→XML round trip — Project
  Schema23 §1.1.3.18/.19 **[D]** and the Certification Manual's
  import-checks section, which warns that changing registration-relevant
  data invalidates "the signature in the registration data" **[D]**
  (`05 KNX Certification of Products - Procedure v01.07.09 AS.md:1542`).
  That is a content-integrity checksum stored as an XML attribute
  (`hardware.rs`'s own `RegistrationSignature`, discussed below), not a
  detached cryptographic signature file.
- T05 additionally queried both of the project's queryable KNX spec
  knowledge bases (SQLite, curated facts with per-fact evidence, distinct
  from the raw-text grep above) via
  `knx-spec-kb/scripts/05_knowledge_base_v1.py --query`, against
  `knx_spec_kb_programming.sqlite` (27 programming-scoped PDFs plus
  figures) and `knx_spec_kb_full179_clean.sqlite` (177 PDFs, text only)
  **[V]**. Search terms: `digital signature`, `package signature`,
  `manufacturer key`, `public key`, `certificate`, `code signing`,
  `knxprod`, `signing key`, `key distribution`, `product database
  signature`, `.signature`, `RSA`, `detached signature`, `signature
  file`, `registration signature`, `manufacturer signing`, `product
  package integrity`. The programming base returned zero hits for every
  term except `knxprod` (an unrelated MT3→MT4 conversion note) and
  `.signature`/`signature` (an unrelated AES-CBC-MAC mode name, KNXnet/IP
  Secure). The full base's `public key`/`certificate`/`manufacturer key`
  hits are all KNX IoT Secure / KNXnet-IP Secure material — SPAKE2+
  session keys, device X.509 certificates (`LDevID`/`IDevID`) — a
  different security domain (device authentication on the bus) from
  signing a manufacturer's product package file. `registration
  signature` and `XML signature` returned zero hits in the full base even
  though the concept exists in Project Schema23 and the Certification
  Manual **[V]**: those documents' `.signature`/`registration` facts were
  not extracted into that base's curated fact table, which is a gap in
  the knowledge base's extraction, not evidence the Standard is silent —
  the raw-text grep above is what actually found that content. Recorded
  here so neither search is repeated expecting a different answer.

Nothing in either search names a `.signature` package member, its format,
its algorithm, or a manufacturer key distribution mechanism. Building real
verification without that specification — or the manufacturer's public
key, which this project does not have either way — would be guessing at a
proprietary scheme, which is out of scope and worse than doing nothing.

`hardware.rs`'s own `RegistrationSignature` attribute (stored into
`hardware2program.registration_signature`, never read back by any query,
DTO, or UI in this codebase **[V]**, `grep -rn registration_signature
crates apps`) is the same gap in a different member — with one difference
worth naming: unlike the package `.signature`, its defining document
*is* in the accessible corpus (Project Schema23, cited above), so what
verifying it would take is at least namable — parsing the
registration-relevant XML subset the Schema defines, recomputing the
checksum by whatever algorithm ETS/the Manufacturer Tool uses (not
specified in the excerpt found), and comparing. That algorithm was not
located, so this is also presently a dead end, not a task with a known
shape **[D]**.

**Impact.** A `'Signature'`-role member is cosmetic. Installing a package
with a corrupted, empty, or entirely fabricated `.signature` member
succeeds identically to installing one with a genuine one — pinned by
`signature_members_are_stored_verbatim_and_never_verified`
(`crates/knx-productdb/tests/standalone_packages.rs`). Nothing downstream
currently treats the role as a trust signal, so today's blast radius is a
misleading label rather than a bypassed check — but that is exactly the
kind of guarantee a future feature could be built on by mistake, reading
`role == "Signature"` and concluding a package was authenticated.

**Lifted when.** Verification proper is lifted when the `.signature`
format is obtained from KNX Association documentation this project does
not currently have access to and implemented deliberately against it (a
separate task, not a drive-by addition to ingestion). The display half is
already done, T05, 2026-09-20: `install_package` still writes the bare
`role = 'Signature'` to `package_member` — the stored value is a stable
domain identifier other code and its tests key on, and changing it was
out of scope — but `apps/knx-server/src/routes.rs`'s
`CatalogInstallMemberDto` now renders it as `"Signature (stored, not
verified)"` before it ever reaches JSON, pinned by
`a_signature_members_role_is_qualified_as_unverified_in_the_install_report`
(`apps/knx-server/tests/http_product_install.rs`). `apps/knx-web`'s
`CatalogBrowser` install report additionally names the count of such
members in its own sentence, in both UI languages it has
(`catalog.installReport.unverifiedSignature`, English and German), so a
person reading the one place this report is actually shown cannot come
away thinking a signature was checked.
<!-- END retained catalogue record -->
