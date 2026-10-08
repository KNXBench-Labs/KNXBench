# ADR 0095: Marketing is a static companion, not a hosted engineering application

Date: 2026-10-08
Status: Accepted; public deployment of the site remains separately gated

## Context

The user approved the `grill-me` Q1–Q12 brief and local implementation on
2026-10-08. The objective is a DE/EN marketing landing page for knxbench.com on
GitHub Pages that encourages trying the actual application. Linux explains the
origin; the main promise is KNX engineering, with container/browser use first and
native Linux as an additional path. The look is calm and dark. CRT/LCARS are
secondary screenshots, not the site's identity. Tone is positive and respectful
toward KNX and ETS; no competitor disparagement.

Existing source evidence:

- The application architecture separates domain, services and UI. Marketing
  does not require any of them at runtime.
- [ADR-0068](0068-project-evolution-story-is-a-static-offline-companion.md)
  already defines a standalone static story and an exact-edition approval gate.
- [Manual acceptance](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/MANUAL_ACCEPTANCE.md) (archived) keeps the full manual on GitHub
  and uses fictional media data. Existing README recordings show actual app UI;
  synthetic traffic is explicitly disclosed.
- The authenticated repository check during planning found main private and
  alpha.5 a non-draft prerelease. The repository went public on 2026-10-08
  (after the site was built); its entry points were then checked signed out.

## Decision

1. `website/` is a dedicated companion in the main repository. Plain HTML/CSS,
   minimal progressive JavaScript and a stdlib-only deterministic Python build;
   no new application or npm dependency, backend, framework or mirrored manual.
2. DE/EN have explicit URLs. One primary CTA leads to an on-page real-product
   start section: Docker/browser prominent, Linux AppImage secondary. Private
   targets remain visibly disclosed in the local preview; no public reachability
   is claimed and no browser imitation/live demo is shipped.
3. Videos, posters, screenshots and fonts are self-hosted and explicitly
   inventoried by hash. Clips show actual app UI with fictional data; synthetic
   API/traffic is labelled, edited timing is not performance evidence. No live
   server, project, manufacturer corpus or KNX hardware is used for capture.
4. The existing story companion is reused at `/story/` for local review. Pin
   edition `2026-10-05.1` and its candidate digest; retain the preview notice.
   No new history, raw provenance or implied publication approval is included.
5. Build only explicit artifact sources. Do not copy repository/docs/history
   directories wholesale. Refuse ambiguous JSON, source/output symlinks,
   unowned output, additional output files/directories and edited prior output.
   The output manifest binds source/renderer and artifact hashes.
6. Preview serving binds loopback and serves the built directory, never the
   repository. `--release` refuses before writing. No auto-Pages workflow or
   publication action is created by the local-build go.
7. No analytics, cookies, third-party video embeds or remotely hosted fonts.
   This is a technical boundary, not a legal-compliance claim. Contact/privacy
   pages clearly identify information still needed for public operation.
8. Publication requires a separate go and verified public installation links,
   exact story approval, real contact/publisher and host-specific privacy
   information, domain/Pages/HTTPS verification, then a reviewed release-mode
   implementation. Do not silently turn a preview build into a public artifact.

## Alternatives

- **Reuse the React app build:** unnecessary coupling and dependencies for a
  small static site; the actual production frontend is used only to capture media.
- **Separate website repository:** not chosen; keeping product claims and their
  source together reduces maintenance drift. Pages delivery is a later concern.
- **Hosted live app/demo:** outside scope; users try the actual product on their
  own instance. Videos provide a low-friction first look without a bus endpoint.
- **Full duplicate documentation:** rejected; the maintained manual remains on
  GitHub, with only concise start/boundary information on the landing page.

## Consequences

Story pin: point 4 named edition `2026-10-05.1`; that candidate was replaced by
the story track, so delivery pins the current `2026-10-08.4` instead.

Numbering: written as ADR-0094 in a parallel session; renumbered to 0095 on
delivery because 0094 had meanwhile been taken by the legacy EX-IM ADR.


The local site can be built/reviewed now while main stays private. Its content
and media need explicit maintenance when workflows change; changing numerical
test/corpus counters are not marketing badges. No ETS equivalence, KNX
certification, every-OS live-bus operation, new application release or complete
accessibility certification follows from website verification. See
[website contract](../WEBSITE.md) and `website/README.md` for reproducible checks.
