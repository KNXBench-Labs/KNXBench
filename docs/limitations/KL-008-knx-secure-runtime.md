# KNX Secure runtime

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-008)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 8. KNX Secure is not implemented

**Key provenance research, 10 October 2026:** [the primary-source review](../research/secure-key-provenance.md)
distinguishes project XML, complete keyring backups, interface exports, factory
credentials, Data/IP keys and project/keyring protection. This does not add a
keyring decoder, AES-project import, credential vault or Secure runtime.
Retained original/uninterpreted source can still contain sensitive material;
a native/source backup is not automatically encrypted or sanitized. Existing
Secure and container refusals stay unchanged.

**Update 2026-10-10 (secure-capable programs in a plain project).** A
privately supplied project contains secure-capable programs
(`IsSecureEnabled`) but no device `Security` element and no secure group
addresses. Today's behaviour for that case is now pinned by a synthetic
witness, `crates/knx-app/tests/secure_capable_not_activated.rs`, which
imports the same plain project with the same program once as
`IsSecureEnabled="true"` and once as `"false"`:

- **Import.** The flag is stored as catalogue text (PDB-7) and as one
  verbatim entry of the program's generic attribute map; nothing about
  Secure is reported, retained or modelled, and the imported project model
  is identical for both variants.
- **Readiness and planner.** Both variants grade the same (`untested`
  without recorded evidence) and plan the same memory download, step for
  step. Planning reads program attributes by name and never this one.
- **Access keys.** No memory-download step can write a key (the test matches
  every step kind exhaustively). `A_Key_Write` is reachable only through a
  `knx-net` plan that declares keys, which no application code builds, and
  such a plan is refused (§112).

What stays unverified: whether a real secure-capable device that is not
switched to Secure needs anything beyond a plain download (for example a
property ETS writes in that state). No documented source was found, so
KNXBench neither adds such a step nor claims the device works; a download
to one is "untested" like any other device without evidence.

**Limitation.** No Data Secure, no IP Secure, no keyring handling (RESEARCH
§9).

**Cause.** No sample key material was available in Session 0, so nothing about
it could be verified.

**Impact.** Secured installations cannot be fully represented or monitored.

**Lifted when.** Sample material and a real secured installation are available.
The isolation boundary already exists — `knx-secure` is a separate crate with
no dependency on `knx-core` — precisely so that this can be built without
retrofitting secret handling into the model
([ADR-0008](../adr/0008-key-material-isolation.md)).

**Update, 2026-09-11.** Asked whether T19 (KNX Secure) should wait until
hardware/sample key material exist, the user answered "raus erstmal, aber
als limitation dokumentieren" — deferred for now, but document it as a
limitation. This entry already does; nothing here is rejected, only deferred
behind the precondition above. See [GAP_ANALYSIS_ETS.md](https://github.com/KNXBench-Labs/KNXBench/blob/994ac08c0328/docs/GAP_ANALYSIS_ETS.md)'s
**T19** for the tracked task.
<!-- END retained catalogue record -->
