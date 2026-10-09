# Community evidence and schema-extension workflow

Status: implemented under ADR-0091; in the repository since 2026-10-08.
Implementation log: `.ai/logs/2026-10-08_codex_community-evidence-implementation.md`.

## Repository destination and public launch

Consolidated on 2026-10-08, the day the main repository went public: the
issue form lives in `.github/ISSUE_TEMPLATE/analysis.yml` of
`KNXBench-Labs/KNXBench`, the EN/DE guides in `docs/contribution-intake/`, and
the app's guide and issue links point there. The interim
`KNXBench-Contributions` repository had no reports and was deleted by the owner on 2026-10-08 after its four files were confirmed
in this repository (0 issues, PRs, releases, forks); a verified Git bundle
stays in the local backups, not published. All preview/permission, public-original
refusal and private-contact limitations continue unchanged.

## Beginner instructions

The app includes a six-step EN/DE guide, a state-dependent next-action hint,
result explanations, permission/export recovery and honest manual-submission
instructions. The full user guide is mirrored here as
[English](contribution-intake/README.md) / [Deutsch](contribution-intake/DEUTSCH.md)
and served from this repository on GitHub. German UI opens
the German guide directly. A description without a ZIP is accepted; technical
IDs, specification citations and version metadata are optional. Only the analyzer
version is passed to GitHub's input-field prefill; source names/findings are not.
Private mailbox setup/delivery remains unverified.

## User workflow

Choose **File → Analyze support gaps…** (German: **Unterstützungslücken
analysieren…**), select `.knxproj` or `.knxprod`, and explicitly upload for
analysis on **this KNXBench instance**. A browser may be connected to a remote
instance; this is not a promise of processing only on the workstation.

Analysis uses disposable product storage and the real project/product import
and offline support/preparation paths. It neither installs into the user's
product database nor changes the active project. It does not open a bus
connection, execute vendor scripts or automatically submit data externally.
Normal product installation remains a different action.

The report distinguishes:

- `measured`: the named check actually ran; counts can legitimately be zero.
- `partial`: a bound or failed structural pass prevented complete examination.
- `refused`: the production typed parser did not admit this input.
- `unavailable`: the check cannot provide evidence (including authoritative XSD validation).
- `not-examined`: work was deliberately not run, including all hardware checks.

Top-level `complete` means the analysis completed its declared scope, **not full
schema, ETS or device support**. Structure rows observe expanded XML names and
full ancestor arrays, without values; they do not predict typed storage. A
future namespace can be inventoried while production import stays refused.
Product metrics come directly from the install encounter/write ledger;
project metrics retain the actual importer read/mapped counts. They are not
recomputed from a generic XML counter. Conflicts, retained-but-uninterpreted
constructs, unknowns, named inferences and offline refusals remain distinct.
Product offline planning uses defaults; project preparation uses the project's
configuration and links. Neither establishes hardware `Verified`.

## Local offline AP1 sequences (2026-10-09)

The local implementation extends the same product analysis with source-bound
`MV-07B0` `Load/ap1` diagnostics. UI, API and CLI share one resolver; there is
no new download wizard, online fallback or bus execution. The ordered sequence
retains unknown steps, source hashes/offsets and separate unplaced declarations.
`expanded` is structural reconstruction only; `partial` and `unavailable` do
not become complete plans. Work/output limits produce explicit partial coverage.

Reduced previews/ZIPs clear all detailed sequence objects, program identities
and source hashes. Fixed resolver issue-code metrics and occurrence counts remain
shareable; detailed issues remain local. Known secret-bearing input withholds
sequence details locally too. Extra context still requires selection, inspection
and permission. An observation means new to this resolver, not globally novel.
Use the existing manual GitHub handoff to report it; nothing is sent automatically.
See [offline procedure contract](OFFLINE_PROCEDURE_RESOLUTION.md) and
[ADR-0098](adr/0098-offline-ap1-procedure-resolution.md).

## Disclosure tiers

1. **Reduced report (default):** `manifest.json`, `findings.json`, `README.md`.
   No original archive, source filename, attribute/text sample, original hash
   or source-bearing detailed refusal is included. Predicate-bearing XPaths
   are omitted; full value-free ancestry remains in the structure inventory.
   This is reduction, not a comprehensive anonymity guarantee: unusual XML
   names/namespaces can themselves be identifying.
2. **Selected context:** the same report plus up to 16 selected, **byte-exact
   unmodified XML members**, under generated `samples/member-N.xml` names.
   Preview their complete text. They can contain names, addresses, product
   identifiers and proprietary data; selection is not anonymization. They
   preserve same-document context, not all cross-document references.
3. **Private original:** only private audience, separate original consent and
   explicit export consent. The archive is byte-exact under
   `original/source.knxproj` or `.knxprod`, with its original SHA-256 in the
   manifest. Known key-like XML fields or any unexamined/non-XML member block
   this conservative original tier. Password-protected input is refused; no
   contribution password/key entry exists. This does not guarantee that every
   possible secret in an unfamiliar format can be recognized.

The exact manifest and every text artifact are previewed before export. Hashes
bind artifacts to their bytes. UI export supplies the preview manifest digest;
a changed manifest is refused and requires a new preview/consent. Changing the
file, audience or selection invalidates the UI's old preview and consent.
Cancel/close aborts the client request and ignores late responses; it does not
promise synchronous parser interruption on the server.

Public intake is the main repository: issue form
`.github/ISSUE_TEMPLATE/analysis.yml`, guides in `docs/contribution-intake/`.
An issue-form link is **manual handoff**, not upload or delivery. The user
attaches and posts the ZIP themselves. Original samples and the private corpus
never go into public issues or Git history.
Platform facts/sources: [community intake research](research/community-intake.md).

`contribute@knxbench.com` remains a proposed, **unverified mailbox**. A mailto
link prepares a draft; it does not attach/send the ZIP or confirm receipt.
Confirm a private contact route and handling terms before sharing confidential
files. No SMTP/OAuth/PAT credential is embedded. GitHub Pages is not an upload
backend. Source originals never belong in public issues or Git history.

## Limits (independent from normal-import policies)

- Raw input: 1 byte–32 MiB; HTTP multipart overhead allowance 16 KiB; options 8 KiB.
- Validated inventory: at most 4096 members and 64 MiB total declared expanded bytes.
- Structural XML: at most 8 MiB/member, depth 128, 200,000 events, 10,000 distinct
  shapes, 1024-byte individual names/namespaces, 16 KiB ancestor path and 8 MiB
  cumulative shape-copy work. Failure is partial, never measured zero.
- Product plans: first 64 programs, with examined/total and partial status above
  the limit. Project device planning does not run above 64 devices; the complete
  original project is never sliced into a misleading substitute project.
- Context samples: at most 16 reviewed plain XML members; non-UTF-8 text previews
  are refused. Originals also require all members to pass the conservative scan.
- Public ZIP: maximum 24,000,000 bytes; oversized output is refused, not truncated.
- One active HTTP analysis/preview/export worker per router. Further requests
  receive 429, not an unbounded queue. A cancelled request's blocking worker
  retains its permit until completion. Response bodies use `Cache-Control: no-store`.

These are conservative analysis-policy limits, not measured whole-corpus
compatibility or a process RSS guarantee. No normal importer budget/schema gate
was widened. Signed content/signatures are not verified by this service.

## Headless workflow

`knx contribution analyze <source.knxproj|source.knxprod>` prints the local,
source-bearing analysis JSON. Do not publish that raw JSON as an anonymous report.

`knx contribution preview <source> --options <options.json>` prints exact
outgoing preview files and their manifest digest without writing a ZIP.

`knx contribution export <source> <out.zip> --options <options.json>` writes
atomically to a **new** destination, never overwrites and never submits it.

Reduced public options:

```json
{"audience":"public","sampleIds":[],"includeOriginal":false,"consent":true}
```

Use member IDs from this input's analysis for selected context. Private originals
also require `"includeOriginal":true`, `"originalConsent":true` and private
 audience. `expectedManifestSha256` can bind a CLI/API export to a reviewed
preview. Unknown/duplicate options and unsupported combinations are refused.
Analyzer version currently names the `knx-app` manifest version, not a unique
fingerprint of a dirty development build; report the full application version
and source revision separately when investigating development-only differences.

HTTP: guarded `POST /api/contributions/analyze`, `/preview`, `/export`, with one
`file` multipart field; preview/export additionally require one JSON `options`
field. Client input errors remain 400/413, worker contention 429, internal
workspace/worker failures 500. No AppState product/project mutation is possible
through the analyzer interface.

## Making support extension simple, not careless

1. **Observe:** use actual importer findings and structural shapes to locate a
   dark spot. Compare full ancestor paths **and expanded namespaces**, not tag
   names or a namespace's trailing number alone.
2. **Get enough permitted evidence:** report + selected context or privately
   consented original. State original versus transformed/synthetic provenance.
   Do not mistake field occurrence for known semantics or successful storage.
3. **Validate facts first:** consult the relevant authoritative schema/spec,
   product/application versions and reference relationships. Record inspected
   sources and uncertainty in the relevant `docs/research/` note and ADR.
   An XSD result is only syntax; this analyzer currently reports it unavailable.
4. **Define a small adapter change:** project path is `knx-etsproj` (detect,
   parse, validate, map, retained data, report); product path is the dedicated
   `knx-productdb` package/XML readers, transaction and versioned evidence store.
   Keep schema admission independent from the normalized core. Do not repair a
   domain/adapter gap in UI, or hard-code a manufacturer's products.
5. **Regression before promotion:** write a reproducer with an independently
   justified expected result. Test exact and foreign/lookalike namespaces,
   qualified/unqualified same-local-name attributes, duplicate IDs/references,
   invalid addresses/version relationships, unknown/unsupported accounting,
   exact source retention and a valid late failure against seeded storage.
6. **Migration only if necessary:** a new external schema number does not itself
   require a native project/product DB migration. If the normalized/persisted
   model changes, version it explicitly and test historical availability,
   migration collision/rollback, reload and tampering. Do not retroactively
   label unavailable historical evidence as measured zero.
7. **Verify consumers and boundaries:** own-crate regressions, actual API/CLI
   outputs and UI meaning. Native save/load/roundtrip tests only establish the
   operations actually exercised; no `.knxproj` exporter is promised. Run opt-in
   private-corpus checks from a worktree that actually contains that corpus;
   publish aggregates only, never source names/values.
8. **Release reviewed adapters:** re-analyze the original permitted sample,
   report what became supported and what remains unknown, inferred or refused.
   No automatic compatibility approval, generated-code execution, free-form
   plugin marketplace or hardware certification follows from a contribution.

## Verification boundaries

Tests exercise production project/product paths, exact structural namespace
separation, measured parser metrics, malformed/deep/long-name/path refusals,
known-key sharing blocks, explicit selection/consent, byte/hash equality,
deterministic retry/export, preview binding, seeded HTTP project/product
immutability, session guards, upload limits and real CLI no-clobber behavior.
Frontend tests exercise parent-menu focus, validation, preview invalidation and
late-result disposal. The browser receipt uses real local server APIs and a
synthetic archive, with the existing App's startup bus discovery explicitly
intercepted/refused before any network/hardware contact. Bootstrap activity is
recorded separately from the exact contribution gesture sequence.
Native WebKitGTK downloads, Orca, live hardware, a new packaged release, mailbox
operation and complete private-corpus analysis coverage are not established.
