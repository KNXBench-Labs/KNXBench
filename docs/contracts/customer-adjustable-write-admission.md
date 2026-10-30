# Customer-adjustable source declarations: regular-write admission

Status: **source-admitted conservative policy; implementation and software
acceptance pending**, 30 October 2026. Baseline `17e45dc62`. Source rationale:
[primary-source investigation](../research/2026-10-30-customer-adjustable-write-admission.md).
This contract implements neither ETS customer-write rights nor preservation
of calibration bytes. It adds no device/family or hardware compatibility claim.

## Meaning and boundaries

The documented ETS feature protects some manufacturer-marked runtime values
under status-dependent conditions. The reviewed public descriptions do not
independently define the complete XML grammar/value domains, status precedence,
downloaded marker, memory merge/checksum rules, `Sync` operation or EX-IM
representation. Those are blockers to **honouring writes**, not to a deliberately
stricter KNXBench refusal.

The exact spellings and syntax contexts below are **KNXBench policy matchers**,
not a recovered normative XML schema. A match means an uninterpreted source
declaration blocks regular write preparation. An absence of matches proves neither absence
of customer-adjustable semantics nor general programmability. Present `false`,
`0`, `1`, `true`, `TRUE`, `Sync`, whitespace, empty and arbitrary text all refuse
equally. There are no positive token classes, Boolean conversion, trimming,
status exemptions or inferred rights. Attribute values are retained after the
existing XML attribute normalisation; the original XML remains retained too.

## Exact observation scope

Start at the selected `ApplicationProgram`, using existing element-local-name
conventions; do not introduce a new namespace framework. Match attributes by
**unqualified raw key**, using `Attrs::evidence_value`, never the local-name
fallback in `Attrs::get`.

Admitted parameter contexts for `CustomerAdjustable`:

1. Each own `Static/Parameters/Parameter` of the selected application.
2. Each direct `Static/Parameters/Union/Parameter` member.
3. `ModuleDefs/ModuleDef/Static/Parameters/Parameter` and its direct Union
   members. The first `ModuleDefs` may be directly beneath the application or
   beneath its own `Static`. Recurse only through explicitly entered
   `ModuleDefs/ModuleDef` edges beneath a `ModuleDef` or its own `Static`.

For `CustomerAdjustableParameters`, match **only own root `Static/Options`**.
Module-local Options are explicitly excluded in this increment. Do not silently
broaden to arbitrary descendants, Dynamic, Extension or wrapper elements.
`x:CustomerAdjustable` and `x:CustomerAdjustableParameters` alone are not the
unqualified trigger. If both prefixed and unqualified attributes exist in either
order, consume only the unqualified value. These exclusions are matcher limits,
not proof that excluded data is semantically irrelevant or safe to write.
Existing unknown diagnostics, parse errors and all other refusals still apply.

Collect a deterministic ordered typed vector before placement/ID-map/Options
replacement can erase evidence. Each occurrence has a closed context class,
raw normalised token, and a structural occurrence path/index distinguishing
module path and repetitions. Preserve repeated declarations and duplicate IDs;
a later markerless Options element must not erase an earlier marker. Duplicate
same-name attributes on a single XML start element keep the existing XML error.
Do not translate module definitions or insert module segments/parameters into
executable root maps. Missing placement, inactivity or an unused module does
not exempt an admitted marker from the program-level refusal.

## Write chokepoints and precedence

Use one pure checker over the retained `ProgramCode`, returning a typed
`DownloadPlanError` and the stable unsupported category
`customer-adjustable-parameters`. Apply it in exactly the two current App
chokepoints:

- `device_download::plan_request`, only inside `Purpose::Download`, immediately
  after `check_download_plugin`.
- `download_support::check_kind`, immediately after `check_download_plugin`.

The precedence is **kind (mask, style) → vendor plug-in → this refusal → image
→ plan**. Do not alter `check_program_kind`, the low-level plan/executor APIs,
retry/wait policies, authorisation, family or recovery gates. No scatter of
additional checks or changes to `device_backup.rs` is needed: existing restore
preparation already traverses `offline_plan`/`check_kind`.

Complete preparation and every concretely requested partial/Changes scope
inherit the refusal, including GA-only changes. Changes with no pending scope
remains no plan, not an invented failure. Offline planning, readiness, coverage
and support reports retain the named category; previous Verified evidence cannot
promote the source back into write admission. A previously valid complete or
partial plan no longer matches a now-declaring source via `source_still_plans`.

Read-only download preparation, comparison/adoption and existing diagnostics
remain unchanged. Low-level public plan/executor APIs and a plan prepared for
reading are not an unforgeable capability barrier; this increment secures the
current regular App writer paths, not arbitrary future library callers. No new
writer may repurpose the read path to bypass policy.

## Data integrity, consumers and privacy

The projection is ephemeral, populated on demand from retained XML; no database
migration, stored-report rewrite, unknown-row retirement or corpus repin.
Absence of the XML matcher in EX-IM code is not evidence of flag freedom.
Existing raw source/options/unknown reporting and markerless plans remain
unchanged. Unrelated retained attributes remain outside this policy.

New error payloads carry only declaration counts and closed context classes.
Display, CLI, HTTP, Web, support and any persisted failure must not expose raw
marker values, parameter IDs/values, arbitrary option text or full occurrence
paths. Do not add a public Debug/Serde dump of the private projection. This does
not promise deletion/redaction of all pre-existing private raw data.

Actual CLI and HTTP Plan/Start consumers must refuse before opening a tunnel or
sending a frame; HTTP uses the existing typed preparation-refusal projection,
not text parsing. Web must give explicit EN/DE category explanations and an
appropriate support action such as requestSupport, never an invented status
change, permission unlock or preservation workflow. Existing History/report
shapes remain unless an affected existing consumer requires a tested change;
no new persisted schema is invented for a preflight refusal.

## Executable acceptance — not yet performed

- First required execution on untouched production: demonstrate that the
  existing public API returns `Ok`/a writable plan for the supported synthetic
  MV-0701/ProductProcedure plus an unplaced parameter marker while the test
  requires the new refusal category. This expected baseline outcome is
  statically supported but **has not yet been executed**. Preserve that original
  semantic RED, then repeat for a root option marker. Current APIs must compile
  first; compiler/setup/ingest errors and unrelated kind/plug-in refusals are
  uncredited.
- Test root/Union/both module anchors/nested admitted modules/root Options;
  all token spellings, empty values, repeated IDs/Options and structural order;
  foreign-prefix and excluded-context negatives; no module translation.
- Preserve exact markerless image bytes/steps and reading/comparison-adoption
  twins. Exercise complete, all partial and concrete Changes scopes, status/
  download-flag independence, no-pending Changes and source freshness.
- Prove kind/style/plug-in precedence and guard-before-image. Prove offline,
  readiness, coverage/support and a genuinely valid source-bound Loaded backup
  refuse for the new reason; test complete and partial restore without weakening
  backup identity or format gates.
- Run actual CLI/HTTP entrypoints including Start after a source change, with
  other prerequisites valid and explicit zero-tunnel/frame assertions. Exercise
  Web EN/DE/support consumers and owning fixture tsconfigs. Pin sentinel privacy
  in real error/report/UI projections, not merely an enum helper.
- Independently remove each App check, substitute local-name attribute lookup,
  narrow to positive-only values and use an inert control; require the intended
  assertions and restore byte-exact. These are calibration, not original RED.
- Independent whole-feature review, maintained branch/main-first gates, an actual
  built-server isolated smoke, source-bound evidence and publication/readback.
  Tests use non-root loopback-only/caps-zero namespaces and process-scoped XDG;
  no real bus/device/VM, Secure, DIAG-03, deployment or release.

The broader customer-write/inactive/Union rules, calibration-preserving image
algorithm, other identity/runtime layouts, complete mutation/recovery and
historical fixture/category provenance remain open under PRG-01. This packet
must not be marked as their completion or as physical-device verification.
