# ADR-0091: Maintainer-reviewable community evidence, not automatic compatibility

- Date: 2026-10-08
- Status: accepted; implemented, in the repository and consolidated into the public main repository on 2026-10-08
- Scope: read-only project/product analysis, versioned evidence export and explicit public GitHub/private email handoff

## Repository destination — owner clarification (2026-10-08)

This section records the earlier pre-launch decision. The same-day execution is
recorded under **Consolidation done** below; the temporary endpoint is no longer
the current contribution destination.

The owner clarified that `KNXBench-Labs/KNXBench` is private only while pre-launch
work is completed and will become public later. The long-term destination is
therefore **the main application repository**, including contribution guides,
issue forms, ordinary public support-gap issues and app handoff links. A separate
permanent contribution repository is not desired. This supersedes the original
hosting assumption below, not the public/private disclosure rules.

The existing `KNXBench-Contributions` publication is a temporary pre-launch
endpoint, not a second permanent product/community. Do not make the main repo
public, delete/archive the temporary repo or redirect external users to a private
repository as an automatic consequence of this decision. Consolidation belongs
to the coordinated public-launch change: install the form in the main repo's
`.github/ISSUE_TEMPLATE/`, retain EN/DE guides and existing issue templates,
update app/guide/contact links, verify public read access and the complete manual
submission path, then agree the temporary repo's retirement handling. Preserve
any existing reports/attachments; do not infer deletion permission.

No launch date, visibility change, production deployment or automatic submission
is authorized by this clarification. Private originals/known-key guards and the
unverified mailbox status remain unchanged.

## Consolidation done (2026-10-08)

The main repository went public on 2026-10-08. The same day the form moved to
`.github/ISSUE_TEMPLATE/analysis.yml` (blank issues stay enabled, since this
is the main repository), the guides stayed in `docs/contribution-intake/`, and
the app, guides and form now link the main repository. The interim
`KNXBench-Contributions` had no issues and was deleted by the owner on 2026-10-08 after its four files were confirmed
in this repository (0 issues, PRs, releases, forks); a verified Git bundle
stays in the local backups, not published.

## Decision

Analyze selected `.knxproj`/`.knxprod` bytes on the user's own instance. Reuse actual format parsers, encounter reports and offline evaluation/download preparation in application services. Use a fresh temporary product database, never the user's project/product state. A bounded structural inventory of refused schemas does not grant them typed import semantics. No manufacturer scripts, hardware adapters, credential input or external submission runs during analysis.

Return ordered findings with their owning check, location, occurrence count and diagnostic; keep unavailable, partial and refused distinct from measured zero. Offline plan/inference results do not establish hardware Verified. Limit analysis input and work independently of normal-import limits; stopping a client request abandons its result, not a promise that synchronous parser work immediately stops.

Export a versioned ZIP containing a manifest, findings and reproduction/validation guidance. Default reduced reports omit source filenames, attribute values, full diagnostics, concrete object identities and source hashes. Contextual samples require explicit selection and preview; they are source data, not automatically anonymized. An original requires separate private-only consent. Known secret-bearing XML/key files and password-protected input are not contribution samples; no comprehensive anonymity/secrets guarantee is made for other source data. Whole XML-member context preserves same-document references, not a guarantee of a self-contained complete project. Refuse unsupported disclosure/format combinations rather than silently dropping requested content.

The contribution UI must truthfully show the exact content and selected disclosure tier. Public handoff opens the public main repository's support-gap issue form; attachment and submission remain the user's browser actions. Private handoff prepares an email to `contribute@knxbench.com`; manual attachment and send remain the user's actions. No embedded maintainer token, SMTP password, success claim for external submission or public original attachment. Mailbox setup/delivery remains unverified until actually configured and tested.

GitHub hosts documentation, issue forms and explicitly public evidence; originals do not enter public Git history. The main application repository is public and is the canonical contribution destination (see consolidation above). Platform constraints and primary sources: [community intake research](../research/community-intake.md).

## Validation workflow

Maintainer reproduces a finding from disclosed evidence, checks specification/product/project facts, derives a permission-reviewed minimal regression fixture with independently justified expectations, implements the change and reruns owning regressions. Newly recognized semantics do not change historical evidence or become certified/hardware verified automatically. Contributions support ordinary reviewed releases, not executable plugin admission or automatic fixes.

## Acceptance

Tests must cover genuine project and product parser findings, refused namespaces with structural-only results, offline refusals/inferences, exact counts and availability, bounded malformed inputs, deterministic same-input results/export, reduced disclosure omissions, explicit sample selection, private-only original gating, known-secret sample refusals, and seeded project/product immutability through HTTP. Exercise API/client/Dialog and real-binary/browser paths; test stale response invalidation/cancel and blocked browser handoff. Document unsupported or untested scopes explicitly. Corpus checks use private data read-only and publish no identifying source values.

## Consequences

No domain/storage schema migration or arbitrary adapter/plugin framework. Existing native persistence, normal importer namespace/size policies and bus guards remain unchanged. Central custom upload service, OAuth/direct background upload, mailbox provisioning/retention and automatic generated-code approval are out of this package. Repository intake is a browser-mediated contribution workflow, not a confidential GitHub Pages backend.
