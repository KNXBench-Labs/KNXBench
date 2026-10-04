# KNXBench project-evolution story

A private, offline companion that tells how KNXBench grew from its earliest
surviving prompt into an engineering project. It is **not** part of the KNX
application and does not touch the KNX domain, project files or any bus.

Status: first private version. Nothing here is approved for publication.
Design and boundaries: [ADR-0068](../docs/adr/0068-project-evolution-story-is-a-static-offline-companion.md),
[brief](../docs/PROJECT_EVOLUTION_STORY_BRIEF.md), [goal](../docs/PROJECT_EVOLUTION_GOAL.md).

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
python3 -m storytool validate
python3 -m storytool prepare --private-provenance ../../KNXBench.story-private/provenance.json
python3 -m storytool build 2026-10-04.2          # writes dist/2026-10-04.2/index.html
python3 -m storytool build 2026-10-04.2 --preview  # writes previews/2026-10-04.2.html (commit it)
python3 -m storytool serve 2026-10-04.2          # http://127.0.0.1:8765/, loopback only
python3 -m storytool release-check 2026-10-04.2 --approval <approval.json>
python3 -m storytool publish                     # always refuses, exit code 3
```

Every built page opens directly from disk, including the committed
`previews/<id>.html`. On GitHub a page shows as source text only: download it
(or use the raw file) and open it locally. Online hosting comes later.

Tests:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -t tests
python3 tests/browser/make_hostile.py <scratch-dir>      # synthetic hostile-text preview
node tests/browser/check_story.mjs dist/<id>/index.html <scratch-dir>/dist/index.html <receipt.json>
```

The browser check needs Playwright with Chromium; set `PLAYWRIGHT_DIR` to the
`node_modules` directory that contains `playwright` if it is not at the default
local path. It is not a repository dependency.

## Preparing an update (never publishing)

1. Gather new, authorized evidence privately. Add provenance entries for every
   new event to the private ledger.
2. Edit `content/edition.json`: give the edition a **new id**, update the cutoff
   and source coverage, add or change events, relations and gaps. Translate and
   label excerpts (`translated`, `edited_for_privacy`, `shortened`, or a
   `paraphrase`). Keep editorial interpretation in `aside` or in relations of
   type `editorial`.
3. `prepare` refuses invalid content, hard privacy findings and leaked private
   locators. It writes a new candidate and diffs it against the latest earlier
   one. Earlier candidates are never rewritten.
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
deployment step exists. Any change after approval needs a new approval.

## Source archaeology, edition 2026-10-04

Pinned baseline: published history `origin/main` at
`75ad9650351f80fd147d683f3a6aeb34ec0dfd64` (4 October 2026, 00:42 CEST), 1,909
commits from 2 September 2026. When this edition was started, the shared root
checkout was 112 commits behind it; the root is not the baseline. Edition `.2` adds this story's own local,
unpublished worktree as a separately labelled source.

Read locally, KNXBench-attributable only:

- Git history, ADRs, maintained docs and `.ai/` handovers (self-reports, cross-checked).
- Claude Code: 111 transcripts across the project folder under its earlier name
  `KNX`, the renamed `KNXBench` folder, its worktrees and `knx-spec-kb`
  (2–24 September), plus `~/.claude/history.jsonl`.
- Codex: 202 project threads (7–25 September) plus `~/.codex/history.jsonl`.
- Hermes: 81 sessions in the `knxbench` profile (22 September – 4 October) and
  cwd-filtered project sessions in the default profile.
- Paperclip: local database backups (25–27 September); only the issue, agent,
  goal, project and company tables. Secret and credential tables, `.env` files
  and run logs were not opened.

Not accessed: cloud agent sessions, the remote repository host, the ETS
installation data and the protected manufacturer corpus.

Findings that shaped the story:

- **Earliest surviving prompt:** 2 September 2026, 13:50 CEST, in the folder then
  named `KNX`: install tools to analyse the user's home ETS project and access
  the bus. The **founding prompt** for the application followed at 14:23 CEST:
  read the German strategy document and get going. The strategy document's own
  origin is not in any available source.
- The folder and product were renamed `KNX` → `KNXBench` on 5 September.
- The early Git history was rewritten twice for privacy on 20 September and the
  remote recreated; commit hashes from before then differ from those the agents
  saw at the time. The first commit's message still describes data that is gone.
- All commits carry the user's identity; agent contributions are named only
  where a conversation shows the hand-off.
- Chronology is not causation: for example, the first newer-schema refusal
  (5 September) and the schema-21/23 design (6 September) are linked only as a
  documented association, because the design cites the backlog, not the report.
- Raw prompt extracts contain sensitive material that must never be published.
  They stay in the private ledger, which is why automated collection never feeds
  the public content directly.

## Known limitations

- The privacy scan is pattern-based. It blocks local paths, e-mail and IP
  addresses, session identifiers and credential assignments, but it cannot
  recognise every private fact. Human review of every excerpt is required.
- The graph uses a deterministic lane-per-strand, row-per-event layout. On
  narrow screens the guided graph hides colliding labels; the text always
  carries the full information.
- System fonts only; final typography and licensing are open.
- The browser check covers Chromium via Playwright. Firefox, WebKit and real
  screen readers have not been checked.
- Coverage of the human–AI workflow depends on local logs. Cloud sessions and
  anything outside the read sources are missing from this edition.
- Commit references assume the repository's history may be shown publicly; that
  is a review item, not a decision taken here.
