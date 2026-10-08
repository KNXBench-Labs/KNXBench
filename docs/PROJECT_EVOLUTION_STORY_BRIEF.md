# KNXBench: From One Prompt to a Project

## Status and purpose

Product and editorial brief agreed in the 2026-10-03 discovery conversation.
It records the intended experience, not permission to publish.

**Update 2026-10-04:** a first private, locally working version exists in
[`story/`](../story/README.md), built on a source-archaeology pass over Git,
documentation and local Claude Code, Codex, Hermes and Paperclip records. The
stack is a static, offline companion prepared by a stdlib-only tool
([ADR-0068](adr/0068-project-evolution-story-is-a-static-offline-companion.md)).
The current candidate is `2026-10-08.4`, **approved for publication by the
owner on 8 October 2026** (`story/approvals/2026-10-08.4.json`); earlier
editions were removed from the tree at the user's request and remain in Git
history. The owner has a domain; hosting, the public page variant and the
deployment step are not designed yet.

The central question is:

> How did a starting prompt grow into such a complex engineering project?

Create an engaging, humorous, public, English-language web experience that
explains KNXBench's evolution from its earliest available prompts to a clearly
identified project snapshot. It must explain why the project grew, not simply
animate commit counts.

## Audience and narrative

Serve KNX beginners, home users, and system integrators with one main story and
optional depths. Do not require KNX or software-architecture knowledge to follow
the main narrative. Explain specialist terms at the point of use; provide deeper
technical evidence on demand.

Weave together three stories:

- **Product:** capabilities, architecture, and the application taking shape.
- **Decisions:** prompts, choices, turning points, abandoned approaches, and
  outcomes, including setbacks where the evidence supports them.
- **Human–AI collaboration:** the user's direction, agents' contributions, and
  changes in the development process. Attribute contributions only where verified.

Include the full relevant development ecosystem, but keep KNXBench the main
character. Supporting projects such as `knx-spec-kb` and ProjectStats have
supporting roles: introduce them where they explain the main project's evolution,
not as independent stories of equal weight. Their actual relationships and
historical appearances still need evidence gathering.

Target approximately **5–8 minutes** for the guided story. The optional detail
layers are not constrained to that duration. The short narrative is a curated
selection, not a claim to show every historical event.

## Visual concept: a growing ancestry blueprint

Use a branching ancestry tree rendered with the clarity of an engineering
blueprint. Its main nodes are **significant development steps**, not individual
messages or Git commits. A step may be a consequential decision, capability,
structural change, or change of direction.

Prompts, conversations, commits, documentation, and test evidence are attached
sources. A source can support multiple steps; several sources can support one
step. Do not assume a one-prompt/one-commit/one-feature relationship.

The intended journey is:

1. Begin with the earliest verified founding prompt, translated and sanitized
   where necessary. Do not present an available later prompt as the original.
2. Establish the initial intention and early decisions as the trunk.
3. Grow branches as the story reveals new development directions.
4. Introduce supporting tools on the relevant branches when their connection is
   established by evidence.
5. Show convergence where development strands genuinely meet again.
6. End with a zoomed-out, freely explorable overview of the approved history.

The tree is present and grows throughout the guided journey; it is not merely a
final illustration. Focus the active development strand while retaining enough
previous growth to convey its context.

A strict single-parent tree is not required. Allow multiple incoming connections
and cross-links. These are semantic development relationships, not a visualization
of Git branch names. Distinguish documented causation, documented association,
and editorial interpretation; temporal proximity alone proves none of them.
Missing dates or uncertain ordering must remain explicit.

### Two views

**Story view** is the default: guided scrolling, readable chapters, highlighted
turning points, and expandable evidence. Keep the main path understandable and
avoid making visitors manipulate a graph before they can follow the story.

**Full complexity view** is optional: all approved nodes and connections, with
zoom, search, and development-strand highlighting. Greater visual density can
communicate the project's complexity. It must remain navigable and must not
introduce speculative edges merely for visual impact.

Both views use the same approved public material. The complex view is not a
backdoor to original private conversations.

## Selected visual direction: Phosphor Atlas

The user selected an entirely dark experience, combining the compositional
clarity of the Dropbox brand reference with an updated green 1980s CRT identity.
The alternative of a light website containing a dark CRT window was not selected.
Borrow principles, not Dropbox logos, assets, or its bespoke typeface.

Design plan:

- **Base palette:** screen `#03150c`, raised surface `#0b2d1c`, reading ink
  `#eef5e9`, phosphor `#88ffad`, secondary violet `#c1aff2`, annotation amber
  `#e7c688`. These are prototype tokens, not immutable production values.
- **Typography:** large, strong sans-serif headlines and readable sans-serif
  prose; monospace for prompts, evidence metadata, and compact technical labels.
  The offline study uses system-font fallbacks; final font choice/licensing is
  still open. Do not put every paragraph in terminal-style monospace.
- **Layout:** generous, left-aligned editorial composition, with the growing
  ancestry graphic as its memorable visual element. Desktop pairs headline and
  graph; mobile stacks them. Details sit beneath the main reading path.
- **CRT personality:** precise green paths, quiet engineering rules, restrained
  bloom on an active node, and faint static scanlines behind artwork, never over
  prose. Violet/amber are sparse supporting accents, not confetti.
  After reviewing the first study, the user requested a stronger CRT character:
  brighter phosphor headlines, richer green surfaces, clearer trace bloom, and
  more visible artwork scanlines. Prose remains crisp and off-white.
- **Interaction:** meaningful growth, branching, and convergence; native,
  keyboard-reachable controls and evidence disclosures. No continuous flicker,
  glitch loop, artificial blur, fake monitor bezel, or mandatory boot sequence.
  Reduced motion must preserve immediate understanding and access.
- **Motion update (user decision, 2026-10-04):** the user asked for more life.
  Three changes:
  - Scrolling back now retracts the later steps (growth in reverse), and the
    current chapter pulses once.
  - Each visible connection carries a looping signal pulse, travelling from
    start to end like a telegram on the bus.
  - Random headline letters roll through in place, one or two every few
    seconds (inspired by the letter swap on eszterbial.com, own
    implementation).

  These ambient loops are calm travelling light, not flicker or glitch. They
  pause off screen, and they stop entirely with *Motion off* or the OS
  reduced-motion preference. Headings keep their text as their accessible
  name.

Self-review: a green monochrome terminal would undermine the broad readership;
uniform rounded cards would undermine the genealogy. Retain the dark phosphor
character while using off-white reading text, distinct type roles, spatial
hierarchy, and an actual branching illustration. Complexity belongs to the tree,
not to unnecessary interface decoration.

Official reference pages inspected for design principles:
[framework](https://brand.dropbox.com/framework),
[typography](https://brand.dropbox.com/typography), and
[motion](https://brand.dropbox.com/motion). They describe clarity for beginners
and experts, distinctive headline typography, and purposeful, subtly playful
motion. This project does not reuse their proprietary assets.

A [local, offline style study](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/design/project-evolution-phosphor-atlas.html)
demonstrates the selected direction with explicitly synthetic content. It is a
design artifact, not the verified historical story, full graph explorer, update
pipeline, or a deployed website. The user selected the direction; approval of
the stronger rendered study and its finite growth animation has now been given
explicitly. This is design approval, not approval of historical content or public
publication.

A [desktop capture](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/design/project-evolution-phosphor-atlas.png) records the
current local rendering; the HTML provides the actual interactive study.
The revised study includes a finite CSS growth sequence: branches draw, their
nodes appear, paths converge, and a supporting thread becomes visible. Native
stage controls replay it. A Motion off checkbox and OS reduced-motion preference
both immediately stop active effects and show the complete selected-stage state.
This is a motion design sample, not the full scroll-driven historical experience.

The [first-private-version goal](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/archive/PROJECT_EVOLUTION_GOAL.md) supplies the bounded
implementation and verification handover for a new user-started `/goal` session.
Preparing that goal document does not activate it or authorize publication.

## Voice and accessibility

All visitor-facing narrative and public prompt excerpts are in English. Use plain,
specific explanations with restrained, playful commentary. Humor belongs in the
presentation, not in altered historical facts, inflated compatibility claims, or
invented incidents.

**Narrator (user decision, 2026-10-04):** the story is told from the AI's point
of view, in a voice that pays homage to Marvin, the gloomy robot from Douglas
Adams' *The Hitchhiker's Guide to the Galaxy*, matching the depressed-robot
commit messages the project has used since 10 September. Rules for that voice:

- The persona lives only in narration (hero, chapter ledes and bodies) and in
  the asides. Event summaries, *why it mattered*, excerpts, evidence,
  uncertainty, relations and gaps state the record without persona.
- Write original lines. Do not borrow Adams' text; the only exception is a
  quotation that is itself project evidence (a commit message).
- The narrator speaks for itself, not for the agents in the record; it must not
  imply that it personally wrote code the record does not attribute to it.
- Gloom never shrinks a safety, privacy or compatibility limit. Where a limit
  is stated, the narrator steps back and lets it stand as written.
- Every narrated edition shows a visible disclosure of the persona and its
  inspiration (`edition.narrator.disclosure`, enforced by the schema).

Illustrative copy, **not historical quotations or final chapter titles**:

> One prompt. A few reasonable decisions. An unreasonable number of branches.

> Show me the whole beautiful mess.

Make the story usable on mobile and with a keyboard. Provide visible focus,
readable contrast, reduced-motion behavior, and a non-graph way to reach the
narrative and evidence. Animation supports understanding; it must not be necessary
to understand a development step or access its sources.

## Public prompts, provenance, and privacy

Real prompt excerpts can appear after translation and privacy review. They may be
edited to preserve their meaning without exposing private or protected content.
Public labels must accurately describe the transformation, for example:

- `Translated from German`
- `Edited for privacy`
- `Translated from German · Edited for privacy`

An edited excerpt must not be represented as a verbatim original. Privacy edits
must not make historical decisions look more informed or successful than they
were. An editorial summary is not a prompt quotation. If the founding prompt is
unavailable, identify the gap rather than fabricate one.

Audit for private projects, identities, hardware and network details, credentials,
protected material, local paths, and confidential discussion before publication.
Public source references must not reveal protected information through URLs,
filenames, metadata, downloadable files, or hidden client-side payloads.

Maintain private traceability to sources without shipping the private source
archive to the browser. The publication pipeline must use a reviewed public
content boundary, not rely on hiding private details with a UI toggle.

Evidence requirements:

- Mark observation, editorial interpretation, and uncertainty distinctly.
- Preserve the difference between an aspiration and an implemented result.
- Preserve the scope of test, compatibility, and hardware evidence.
- Do not infer AI authorship from commit metadata or chronology alone.
- Do not use line count or branch density as proof of correctness or maturity.
- Verify historical claims against their historical sources; use current source,
  tests, Git state, and maintained documentation for claims about current status.
- Acknowledge missing or unassignable source history. Earliest available is not
  automatically earliest ever.

## Continuing updates and explicit release approval

The story continues as KNXBench grows, but updates are initiated when the user
judges them worthwhile. No unattended schedule or automatic public deployment is
authorized by this brief.

Keep two separate approvals permanently:

1. **Prepare an update.** Autonomously gather the authorized new evidence,
   propose significant events and relationships, translate and sanitize copy,
   and produce a private candidate preview. Include a change summary, source
   coverage, unresolved uncertainties, and privacy-review warnings. Automation
   must not silently promote its own interpretation to documented fact.
2. **Publish this version.** After manual review, publish exactly the approved
   candidate. Later additions are not included in that approval. If its content
   changes after approval, it needs a new approval.

The first iterations will be manually reviewed before actual publication. The
separate publication approval also applies to later updates; autonomous preparation
does not imply autonomous publication permission.

Publish versioned releases with a visible evidence cutoff and traceable approved
content. Retain earlier versions so the history does not silently rewrite itself.
The exact release-identity, staging, rollback, and deployment mechanisms need a
later engineering design; they are not implemented here.

## Next work, not yet authorized implementation

1. Audit available historical sources and identify the earliest usable prompt.
   Establish coverage across the relevant local and, only where separately
   authorized, remote agent histories, documentation, and Git history. Inspect
   existing collectors before writing a duplicate collector.
2. Resolve the evidence baseline: local checkout, published revisions, and
   unmerged work must not be conflated into one undefined "current state".
3. Create a private, source-backed candidate outline and event inventory. Flag
   gaps, uncertain connections, and privacy-sensitive material.
4. Design a small branching/converging visual slice using verified events.
   Establish the story-to-graph mapping and public-content boundary before
   selecting the smallest suitable implementation approach.
5. Obtain approval for a separate implementation scope, source-access boundaries,
   deployment target, and operational workflow. No implementation or publication
   is requested merely by completion of this discovery interview.

Future implementation acceptance must exercise the actual browser experience,
source-to-public-content transformation, update diff, exact-version release gate,
and accessibility fallbacks. A design mockup alone cannot prove those workflows.

## Repository context

The story is an explanatory companion to KNXBench, not a reason to couple its KNX
domain to a website or expose product/bus operations to visitors. No integration
into the engineering application has been decided.

Repository authority and existing product context remain in:

- [Project context](PROJECT_CONTEXT.md)
- [Architecture](ARCHITECTURE.md)
- [Implementation status](IMPLEMENTATION_STATUS.md)
- [Known limitations](KNOWN_LIMITATIONS.md)
- [Roadmap](ROADMAP.md)

This brief does not update product completion claims, compatibility guarantees,
hardware permissions, or the existing development roadmap.
