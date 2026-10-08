# ADR 0068: The project-evolution story is a static, offline companion with a separate publication gate

Date: 2026-10-04
Status: Accepted for the first private version; publication mechanism deliberately not designed
Session: outside the numbered sessions (project-evolution story, user request)

## Context

The user asked for a public, English, interactive story of how KNXBench grew
from its earliest prompt ([brief](../PROJECT_EVOLUTION_STORY_BRIEF.md),
[goal](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/archive/PROJECT_EVOLUTION_GOAL.md)). The approved Phosphor Atlas study fixes
the visual direction. The goal requires: real, source-backed events; private
originals that never reach a browser; hostile text that cannot execute;
reviewable, versioned candidates; and a permanent separation between preparing
an update and publishing it.

Verified constraints:

- The story must not depend on the KNX domain or expose engineering or bus
  operations. Nothing in the existing `apps/` or `crates/` is needed by it.
- The repository already uses stdlib-only Python for tooling (`tools/`), and the
  host provides Python 3.14 and Chromium with Playwright for checks.
- The content is small (tens of events, tens of relations). No measured need
  exists for a framework, a bundler, a database or a server.
- A hidden UI element or an unrendered JSON field still ships to the browser.
  Privacy therefore has to be decided before the payload exists, not in the UI.

## Decision

The companion lives in `story/`, independent of the application:

- `story/content/edition.json` is the curated **public-candidate** content:
  English text, labelled translations and privacy edits, evidence references
  limited to commit hashes and repository paths, explicit uncertainty, date
  precision and typed relations (documented cause, documented association,
  editorial link). It is the only input to the public payload.
- Private provenance (session identifiers, local paths, raw prompt extracts)
  stays outside the repository. `prepare` may read it to check traceability
  and to refuse when any private locator appears in the payload; it copies
  only counts into the manifest.
- `python3 -m storytool prepare` validates the schema, runs an automatic privacy
  scan (hard findings refuse, soft findings are listed for review), computes a
  deterministic layout and writes an **immutable** candidate directory
  (`story.json`, `manifest.json`, `CHANGES.md`, `REVIEW.md`, read-only). An
  existing candidate is never rewritten; the next edition gets a new id and a
  diff against the latest earlier candidate.
- `build` verifies the candidate digest and renders one self-contained HTML
  file: text and evidence are pre-rendered with HTML escaping, the payload is an
  inert JSON data island with `<`, `>`, `&`, U+2028 and U+2029 escaped, and a
  Content-Security-Policy admits only the hashes of the shipped style and
  script. The vanilla-JavaScript layer uses `textContent` and DOM APIs only.
  The output has no timestamps, so it is a pure function of the candidate and
  `site/`. `build --preview` writes the versioned page `previews/<id>.html`,
  which is committed so the page can be opened anywhere without the tool; a test
  rebuilds every committed page and fails when one is stale. Scratch builds in
  `dist/` stay ignored.
- `serve` binds to loopback addresses only. `release-check` reports whether an
  approval record matches one exact candidate digest. `publish` always refuses:
  no deployment target, host or workflow has been chosen or authorized.

## Alternatives considered

**React/Vite like `apps/knx-web`.** Rejected: it would couple the story to the
application's dependency tree and build for no measured benefit at this size,
and a bundle is harder to review for privacy than one HTML file.

**Static-site generator or graph library (for example a force layout).**
Rejected for now: a deterministic lane-and-row layout is reproducible
byte-for-byte, keeps diffs meaningful and needs about a hundred lines. A
force-directed layout would move nodes between editions.

**Collecting conversations automatically into the public content.** Rejected:
automatic sanitisation cannot prove an excerpt safe. Collection stays private;
the public content is an explicit, reviewed selection.

**A `publish` command behind a flag.** Rejected: the goal authorizes local
preparation only. A real release step needs its own design and approval.

## Consequences

- Each edition is reproducible from `content/edition.json`; earlier candidates
  remain byte-identical evidence of what was reviewed while they are kept.
  Revisited 2026-10-08 at the user's request: superseded candidates and their
  pages were removed from the tree after wording and content corrections, so
  only the latest edition is kept; Git history retains the rest.
- Committed previews duplicate content that is already in `candidates/` (about
  240 KB per edition). A change to `site/` regenerates every committed page, so
  older editions are always shown with the current page code; the candidate, not
  the page, is the reviewed record. Revisited 2026-10-04 at the user's request:
  the first version kept all built pages out of Git.
- Updating the story means editing the content, preparing a new edition id,
  reading `CHANGES.md` and `REVIEW.md`, and checking the preview in a browser.
- The privacy scan and the provenance canary check are review aids, not proof.
  A human review of every translated or edited excerpt remains mandatory.
- Tests (`story/tests`) and the Playwright check
  (`story/tests/browser/check_story.mjs`) must stay green for each candidate.
- Publication, hosting, fonts and licensing of the public page remain open and
  require separate decisions.
