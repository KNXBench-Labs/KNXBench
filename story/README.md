# KNXBench project-evolution story

A private, offline companion that tells how KNXBench grew from its earliest
surviving prompt into an engineering project. It is **not** part of the KNX
application and does not touch the KNX domain, project files or any bus.

Status: edition `2026-10-15.4` was explicitly approved by the project owner on
15 October 2026 for the exact digest in `approvals/2026-10-15.4.json`. The website
pin now targets it. The earlier edition is retired from the current tree and
remains immutable in Git; deployment is verified separately from approval.
Design and boundaries: [ADR-0068](../docs/adr/0068-project-evolution-story-is-a-static-offline-companion.md),
[brief](../docs/PROJECT_EVOLUTION_STORY_BRIEF.md), [goal](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/PROJECT_EVOLUTION_GOAL.md).

## Layout

| Path | What it is |
| --- | --- |
| `content/edition.json` | Curated public-candidate content (English, labelled, sanitised). The only input to the public payload. |
| `storytool/` | Stdlib-only Python tool: validate, prepare, build, serve (loopback), release-check, publish (always refuses). |
| `site/` | `style.css` and `app.js`, inlined into each preview. No framework, no remote assets. |
| `candidates/<id>/` | Immutable, read-only prepared candidates: `story.json`, `manifest.json`, `CHANGES.md`, `REVIEW.md`. |
| `previews/<id>.html` | Versioned single-file page for each candidate, committed so it can be opened without the tool. A test rebuilds each one and fails if it is stale. |
| `dist/<id>/index.html` | Scratch build (git-ignored, reproducible from the candidate). |
| `tests/` | Unit tests, a synthetic fixture (not history) and the Playwright browser check. |

Private provenance (session identifiers, local paths, raw prompt extracts) lives
outside the repository, by default in `../KNXBench.story-private/` next to the
checkout, owner-readable only. It is never bundled. The relative path in the
commands below assumes the main checkout; from a worktree, pass the ledger's
absolute path (a missing ledger stops `prepare` before anything is written).

## Commands

Run from this directory.

```bash
python3 -m storytool validate --repository ..
python3 -m storytool prepare --private-provenance ../../KNXBench.story-private/provenance.json
python3 -m storytool build 2026-10-15.4          # writes dist/2026-10-15.4/index.html
python3 -m storytool build 2026-10-15.4 --preview  # writes previews/2026-10-15.4.html (commit it)
python3 -m storytool serve 2026-10-15.4          # loopback only
python3 -m storytool release-check 2026-10-15.4 --approval approvals/2026-10-15.4.json
python3 -m storytool publish                     # always refuses, exit code 3
```

Every built page opens directly from disk, including the committed
`previews/<id>.html`. On GitHub a page shows as source text only: download it
(or use the raw file) and open it locally. The approved edition is served at
knxbench.com/story/ by the Pages workflow; a local candidate is not that
published edition.

Tests:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -t tests
python3 tests/browser/make_hostile.py <scratch-dir>      # synthetic hostile-text preview
node tests/browser/check_story.mjs dist/<id>/index.html <scratch-dir>/dist/index.html <receipt.json>
```

The browser check needs Playwright; set `PLAYWRIGHT_DIR` to the `node_modules`
directory that contains `playwright` if Node cannot resolve it from `story/`. It
is not a repository dependency. `STORY_BROWSER=firefox` or `STORY_BROWSER=webkit`
selects another engine (default `chromium`). Outside Chromium the phone pass
drags with the mouse instead of real touch events, and Firefox gets the phone
viewport without mobile emulation, which it does not support.

## Preparing an update (never publishing)

1. Gather new, authorized evidence privately. Add provenance entries for every
   new event to the private ledger.
2. Edit `content/edition.json`: give the edition a **new id**, update the cutoff
   and source coverage, add or change events, relations and gaps. Translate and
   label excerpts (`translated`, `edited_for_privacy`, `shortened`, or a
   `paraphrase`). Keep editorial interpretation in `aside` or in relations of
   type `editorial`.
   Editorial rules from the user: never describe KNXBench as mirroring,
   cloning or copying ETS (say "independent alternative" or "comparable
   functionality"), and leave out remarks about AI session or weekly usage
   limits.
3. `prepare` refuses invalid content, hard privacy findings and leaked private
   locators. It writes a new candidate and diffs it against the latest earlier
   one. Earlier candidates are never rewritten. Superseded candidates may be
   removed from the tree on request and stay in Git history; since 8 October
   2026 only the latest edition is kept.
4. Read `CHANGES.md` and `REVIEW.md`, then `build` and read the preview on a
   desktop and a phone. Run the tests and the browser check.
5. Write the versioned page with `build <id> --preview` and commit it with the
   candidate. After any change to `site/`, regenerate every page in `previews/`;
   the preview test names the stale ones.

## Narrator voice

An edition may set `edition.narrator` to tell the story in a persona (since
`2026-10-04.3`: a gloomy AI in homage to Marvin; rules in the
[brief](../docs/PROJECT_EVOLUTION_STORY_BRIEF.md#voice-and-accessibility)). It
supplies the hero (`kicker`, one to three `title_lines`, `lede`, `aside`), the
`aside_label` shown on every event aside, a short `label` for the edition facts
and a **mandatory** `disclosure` shown in the label guide. The persona belongs in
chapter text and asides only; the record fields stay plain. `CHANGES.md` reports
a narrator change and `REVIEW.md` adds voice checks. Without the field the page
renders exactly as before.

**Publishing this version** is a different decision. It needs an approval
record naming the exact `story_sha256` of one candidate
(`schema: knxbench-evolution-approval/1`, `decision: publish-exact-version`,
`approved_by`, `approved_at`). `release-check` only verifies that match; no
deployment action is performed by storytool itself. The website release builder
and Pages workflow are the separate publication route. Any change after approval
needs a new approval.

Approval records live in `approvals/<id>.json`, committed with their candidate;
`tests/test_approvals.py` fails as soon as one no longer matches. When a later
edition replaces an approved one, remove or renew the record with it.
**Current state:** `2026-10-15.4` is approved for publication (record of
15 October 2026, `approved_by: project owner`). `python3 -m storytool build
<id> --out <dir> --approval approvals/<id>.json` renders the published
variant (no preview banner, no `noindex`, "Published edition") and refuses,
writing nothing, unless the record matches the exact candidate. Committed
previews never take an approval. The website's release build uses this and
is deployed to knxbench.com/story/ by `.github/workflows/pages.yml`.

## Source archaeology, editions 2026-10-04 to 2026-10-08

Pinned baseline since edition `2026-10-08.3`: published history `origin/main` at
`bca2d3336b9692e7a6ece18804d187ecdda6968f` (8 October 2026, 13:18 CEST), 2,295
commits from 2 September 2026. The first editions were pinned to the 4 October
state, now reachable as `88a079a2` after the recorded privacy rewrites.

**Hashes after the 7 October rewrite.** On 7 October the whole history was
rewritten to the single identity `KNXBench <github@knxbench.com>`, so every
commit hash changed. Edition `2026-10-08.3` remapped all commit references
through `docs/history/COMMIT_MAP_2026-10-07.txt` (each one unique, each one an
ancestor of the pinned baseline). After the partial rewrite of 8 October
(`docs/history/COMMIT_MAP_2026-10-08.txt`), edition `2026-10-08.4` remapped the
three affected hashes. A subsequent public-launch purge remapped the baseline
again to `bca2d333`; the maps, not an old edition's prose, resolve current hashes.
Later editions must cite current hashes only.

Edition `2026-10-08.3` added 13 steps from 4 to 8 October 2026 (exact-or-refused
save, telegram flow, independent alpha review, first alpha and its replacement,
one project identity, built-in HTTPS, evidence of record, achievements and fun
languages, LCARS, read-only MCP, wizards, legacy ETS3 product files, goals
closed) and a ninth chapter. Sources: Git, ADRs 0067–0094, the implementation
status and user prompts in the project's Hermes profile up to the cutoff.

Read locally, KNXBench-attributable only:

- Git history, ADRs, maintained docs and `.ai/` handovers (self-reports, cross-checked).
- Claude Code: 111 transcripts across the project folder under its earlier name
  `KNX`, the renamed `KNXBench` folder, its worktrees and `knx-spec-kb`
  (2–24 September), plus `~/.claude/history.jsonl`.
- Codex: 202 project threads (7–25 September) plus `~/.codex/history.jsonl`.
- Hermes: 120 sessions in the `knxbench` profile (22 September – 8 October) and
  cwd-filtered project sessions in the default profile.
- Paperclip: local database backups (25–27 September); only the issue, agent,
  goal, project and company tables. Secret and credential tables, `.env` files
  and run logs were not opened.
- Personal chat exports (ChatGPT, claude.ai), added for edition `2026-10-05.1`:
  provided by the user, kept in the git-ignored `.private/` folder and searched
  locally by keyword. Only project conversations were opened, and only four
  ChatGPT conversations (2 and 4–7 September) are used. Everything else stayed
  private and is deliberately not mentioned.

Not accessed: cloud agent sessions, the remote repository host, the ETS
installation data and the protected manufacturer corpus.

Findings that shaped the story:

- **Earliest surviving project prompt:** 2 September 2026, 13:50 CEST, in the
  folder then named `KNX`: install tools to analyse the user's home ETS project
  and access the bus. The story deliberately starts here; anything earlier is
  out of scope. The **founding prompt** for the application followed at
  14:23 CEST: read the German strategy document and get going.
- **Origin of the strategy (edition `2026-10-05.1`):** written in ChatGPT between
  14:10 and 14:16 CEST the same day (prompt, session plan, Markdown summary,
  CLAUDE.md). Whitespace-normalised similarity: strategy 98.7 % to the first
  commit, CLAUDE.md 96.4 % to `61dae496` (15:12 CEST).
- **Origin of the specification knowledge base:** a ChatGPT-guided extraction
  pipeline with a local model via Ollama from 4 September, an audit of 2,232
  results by 5 September, a clean restart as `knx-spec-kb` on 7 September and
  Claude Code from 10 September.
- The folder and product were renamed `KNX` → `KNXBench` on 5 September.
- The early Git history was rewritten twice for privacy on 20 September and the
  remote recreated; commit hashes from before then differ from those the agents
  saw at the time. The first commit's message still describes data that is gone.
- All commits carry the project identity; agent contributions are named only
  where a conversation shows the hand-off.
- Chronology is not causation: for example, the first newer-schema refusal
  (5 September) and the schema-21/23 design (6 September) are linked only as a
  documented association, because the design cites the backlog, not the report.
- Raw prompt extracts contain sensitive material that must never be published.
  They stay in the private ledger, which is why automated collection never feeds
  the public content directly.

## Update prepared on 15 October 2026

Candidate `2026-10-15.4` pins `origin/main` at
`2d4ec5f90ccc66ed7d03c65fdd02213faf34fc69`, timestamp
`2026-10-15T07:04:36+02:00`. The guided narrative has 1,448 words (chapter
ledes and bodies), 74 events, 93 relations and twelve chapters. Expandable
records retain the technical detail; this is a curated story, not a changelog.
The public-users, reversible-engineering and evidence-boundaries chapters cover
website/demos, real-user import/launch feedback, native history, previewed edits
and catalogue updates, the narrow MCP writer, Functions, travelling manufacturer
files, exact hardware scopes, Alpha 11 versus later source delivery, pending
programming and unresolved System-2 support. The universal support reporter is
labelled a design reservation only at this cutoff.

Owner review clarified two publication boundaries: the specification knowledge
base is a **private, local development aid**, not published or distributed with
KNXBench; publicly accessible manufacturer product databases were **provided for
analysis**, without describing acquisition mechanics. Every evidenced alpha
release has its own event and atlas node: withdrawn Alpha 4 and Alpha 5–11.
Alpha 5 distinguishes original private publication from the 8 October public
republication; Alpha 7 was published after midnight CEST on 10 October. Versions
without publication evidence are not invented as releases.

Private provenance is a separate ledger outside Git. Earlier curated excerpts
are inherited; new project-scoped human requests were filtered locally. Native
Claude/Codex stores were inventoried rather than described as fully reread.
No new personal-export search or pre-project narrative was needed. Agent
self-reports were checked against published source and maintained evidence.

Use `validate --repository ..` to verify the cutoff timestamp, commit ancestry
and document paths. It does not interpret prose or grant publication permission.
Old hashes are remapped with `docs/history/COMMIT_MAP_*.txt`; the one removed
schema-design reference points to its archive locator instead of a missing file.

**Publication gate:** the owner explicitly approved `2026-10-15.4` and
`081d2c4c42b03a0b97ae803ac7f78375e4ca5593f3687453872200aab8398cf8`
after reviewing the preview, changes and actual checks. The new approval, website
pin and latest-only retirement form one cutover. The earlier candidate/preview/
approval remain in Git; approval is not copied or redated. Any content change
requires another edition and explicit approval. Build, push and public deployment
readback remain separately checked operations.

## Known limitations

- The privacy scan is pattern-based. It blocks local paths, e-mail and IP
  addresses, session identifiers and credential assignments, but it cannot
  recognise every private fact. Human review of every excerpt is required.
- The graph uses a deterministic lane-per-strand, row-per-event layout. On
  narrow screens the guided graph suppresses some labels, but remaining labels
  can still touch nodes/edges at dense stages. The full text remains available
  through event cards, Text only and the searchable/zoomable atlas.
- System fonts only; final typography and licensing are open.
- Edition `.4` passes 76 story tests, website24, Chromium46/46 and Firefox46/46
  on its actual frozen bytes. Four engine/viewport contexts verify all eight
  release nodes and both editorial corrections. All44 main frames and8
  supplemental unoccluded-card frames were inspected. No horizontal page overflow
  or external requests were observed. WebKit/Safari, native WebKitGTK and real
  screen readers were not tested. [Current .4 acceptance](../docs/evidence/story-update-2026-10-15-editorial.json);
  [historical .2 acceptance](../docs/evidence/story-update-2026-10-15.json) stays
  separately source-bound, not relabelled as a later run.
- Coverage of the human–AI workflow depends on local logs. Cloud sessions and
  anything outside the read sources are missing from this edition.
- Commit references assume the repository's history may be shown publicly; that
  is a review item, not a decision taken here.
