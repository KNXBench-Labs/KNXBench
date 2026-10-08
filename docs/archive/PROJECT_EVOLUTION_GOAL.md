# Goal: KNXBench Project Evolution — First Private Working Version

## Outcome

Build and exercise a locally usable, private first version of KNXBench's
English-language, interactive project-evolution story. Explain how a starting
prompt developed into a complex engineering project, using actual evidence.
Deliver a working artifact and a reviewable release candidate, not just a plan,
mockup, collector dump, or fictional tree. Do not publish it.

The user approved the stronger **Phosphor Atlas** visual study and its finite
growth animation. Do not restart general style discovery. Use the
[agreed brief](../PROJECT_EVOLUTION_STORY_BRIEF.md),
[approved HTML study](../design/project-evolution-phosphor-atlas.html), and
[desktop capture](../design/project-evolution-phosphor-atlas.png).
Approval covers the visual direction and motion sample, not historical content,
implementation quality, or a public release.

This document is an executable handover objective when the user starts it in a
new `/goal` session. Creating this document does not start that session.

## Startup and repository boundaries

1. Read `docs/PROJECT_CONTEXT.md`, `.agent-memory/PROJECT_MEMORY.md`,
   `.ai/CURRENT_STATE.md`, this goal, the agreed brief, and applicable repository
   instructions. Read relevant maintained documentation before implementation.
2. Recheck Git state and active worktrees. Preserve every unrelated edit and
   ongoing work package. Do not pull/reset/stash the shared root to make it clean.
3. Work in an isolated task-owned worktree for implementation. The accepted
   brief, goal, and design artifacts may be local/untracked in the root checkout;
   do not assume they exist on the chosen base. Bring only these task-owned
   artifacts into the worktree with exact byte preservation, without replacing
   unrelated current documents or source.
4. Distinguish the local checkout, published history, and unmerged development.
   Establish a pinned source baseline and visible evidence cutoff for this story.
   Do not present one undefined "current state" covering all of them.
5. Use the simplest suitable independent companion-site architecture. Inspect
   existing collectors and dependencies before adding code. Do not couple this
   website to KNX domain internals or expose engineering/bus operations.

## Work sequence

### 1. Source archaeology and evidence inventory

Read project-attributable local Git history, ADRs, maintained documentation,
handover/work logs, and relevant local Hermes/Claude/Codex conversation history.
Use available retrieval tools and inspect existing collection code rather than
blindly reimplementing it. Do not modify external collectors merely to reuse them.

Stay within KNXBench-attributable material. Do not inspect credentials, `.env`,
authentication stores, unrelated private projects, or protected manufacturer
corpora. Do not enable cloud collection, use authenticated remote histories, or
change agent configuration without separate authorization.

Identify the earliest available founding prompt and source coverage. If the
actual first prompt is unavailable, mark the gap honestly. Derive significant
steps and relationships from evidence; distinguish direct causation, association,
and editorial interpretation. Preserve uncertainty, unknown dates, source scope,
failed/abandoned work, and verified compatibility limits. Do not attribute AI
contributions from commit metadata alone.

Keep private originals and detailed provenance outside public/browser payloads
and version control. Public candidate content is a separate, explicitly selected,
translated/sanitized representation. Retain private source-to-event traceability
without exposing local paths, raw transcripts, or sensitive identifiers.

### 2. English story and graph

Prepare a source-backed chapter outline, then implement the main story using
verified events. The primary path should be understandable to KNX beginners,
home users, and integrators in roughly 5–8 minutes, with optional deeper reading.

Interweave product evolution, consequential decisions, and human–AI collaboration.
Include the wider ecosystem, but keep tools such as `knx-spec-kb` in supporting
roles. Use accurate, humorous English prose; never manufacture history for a joke.

Main graph nodes represent significant development steps. Attach prompt excerpts,
decisions, outcomes, and sources as detail. Permit branching, convergence, and
cross-links where evidence supports them; semantic branches are not Git branches.
Replace the study's synthetic nodes and quotations rather than silently passing
them off as historical content.

Public candidate prompt excerpts must identify translation and privacy edits.
Preserve meaning and distinguish edited excerpts, direct quotations, and editorial
summaries. Do not label automatic sanitization as proof that content is safe to
publish. Flag unresolved privacy questions for manual review.

### 3. Working interactive companion

Preserve the approved dark Phosphor Atlas direction: large green editorial
headlines, crisp off-white prose, mono evidence labels, luminous green traces,
restrained artwork-only scanlines, and limited violet/amber accents. No copied
Dropbox assets, permanent flicker, glitch loop, or unreadable blur.

Implement a guided growing-tree narrative, event/evidence inspection, and a final
explorable overview. Add an optional full-complexity view with pan/zoom, search,
and branch focus; visitors must not need it to understand the main story.
Both views contain only the same selected public-candidate content.

Motion should reveal growth and convergence, not decorate every control. Preserve
finite effects, replay/navigation behavior, immediate cancellation during active
motion, a motion-off option, and OS reduced-motion support. Provide responsive
layout, usable keyboard/focus behavior, and a readable non-graph alternative.

Choose stack, graph layout, and storage based on inspected requirements. Research
architecture-critical/library facts through primary sources before relying on
them. Avoid unnecessary frameworks, speculative abstractions, and unrelated
refactors. Do not change the engineering application's feature backlog.

### 4. Reviewable update preparation, not publication

Provide a locally exercised way to prepare a versioned candidate from the selected
sources, plus an incremental update candidate and reviewable change summary.
Keep stable event/source identities, explicit cutoff, source coverage, uncertainties,
and privacy warnings. Retain earlier candidates without silently rewriting them.

Permanently distinguish **Prepare an update** from **Publish this version**.
This goal authorizes only local preparation and preview. Do not implement or
exercise an automatic public deployment, expose the preview on a public/LAN
interface, push source/candidate material, or schedule continuing updates.
Publication requires separate explicit approval of an exact reviewed version.

## Verification and completion evidence

Do not claim completion until all of the following have actual execution evidence:

- A reproducible local run/build and working first private story using real,
  source-backed events, with disclosed history gaps rather than invented content.
- Tests for event/schema validation, malformed/duplicate inputs, stable identities,
  uncertainty preservation, and representative branching/converging relationships.
- Privacy-boundary and text-rendering regressions: private originals never ship
  in browser payloads; hostile excerpt text cannot execute as markup/script.
- Tests for deterministic candidate generation, incremental diff, version retention,
  and inability to publish through prepare-only flows or stale approval.
- Real browser checks at desktop/mobile sizes for narrative navigation, evidence
  inspection, full-view search/pan/zoom/branch focus, keyboard/focus, and text fallback.
- Live cancellation checks for motion-off and OS reduced motion while effects are
  running, not only checks performed before animation starts.
- Relevant tests, lint/type/build and documentation gates pass. Check genuine
  scope/nonempty output and disclosed skips; do not substitute mock data for
  missing history or call a skipped suite passing evidence.
- Maintained documentation, implementation/limitations records where applicable,
  `.ai/CURRENT_STATE.md`, and an owned work log accurately describe what exists.
- Final handoff provides artifact location, reproduction commands, source cutoff,
  verification receipts, review instructions, and remaining limitations.

The release candidate remains **private and awaiting manual content/publication
review**. A local first version can be complete without public deployment; it
cannot be declared complete merely because the visual study works.

## Execution rules and stopping boundaries

Continue autonomously through inspection, implementation, tests, review, and
handoff. Do not stop after a plan or ask for routine preference decisions already
covered by the brief. Work without delegate-task subagents or spawned coding
agents. Do not add quota-check pauses or change Hermes configuration/goal limits.

Do not commit, push, merge, publish, access KNX hardware, or touch unrelated
worktrees under this goal. Any later authorized commit must use
`github@knxbench.com` and no co-author trailer.

Stop when the verified local first version and review package are complete; when
the user explicitly pauses/stops; or at an unavoidable blocker involving missing
required access, contradictory requirements, or authorization beyond this scope.
Record the exact blocker and preserved partial result rather than inventing
sources, silently widening scope, or endlessly retrying. Nonessential historical
gaps should be disclosed and must not automatically block all useful work.
