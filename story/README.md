# KNXBench project-evolution story

A private, offline companion that tells how KNXBench grew from its earliest
surviving prompt into an engineering project. It is **not** part of the KNX
application and does not touch the KNX domain, project files or any bus.

Status: first private version. Nothing here is approved for publication.
Design and boundaries: [ADR-0068](../docs/adr/0068-project-evolution-story-is-a-static-offline-companion.md),
[brief](../docs/PROJECT_EVOLUTION_STORY_BRIEF.md), [goal](https://github.com/KNXBench-Labs/KNXBench/blob/138403ed6084/docs/archive/PROJECT_EVOLUTION_GOAL.md).

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
python3 -m storytool build 2026-10-08.4          # writes dist/2026-10-08.4/index.html
python3 -m storytool build 2026-10-08.4 --preview  # writes previews/2026-10-08.4.html (commit it)
python3 -m storytool serve 2026-10-08.4          # http://127.0.0.1:8765/, loopback only
python3 -m storytool release-check 2026-10-08.4 --approval <approval.json>
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

The browser check needs Playwright; set `PLAYWRIGHT_DIR` to the `node_modules`
directory that contains `playwright` if it is not at the default local path. It
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
deployment step exists. Any change after approval needs a new approval.

Approval records live in `approvals/<id>.json`, committed with their candidate;
`tests/test_approvals.py` fails as soon as one no longer matches. When a later
edition replaces an approved one, remove or renew the record with it.
**Current state:** `2026-10-08.4` is approved for publication (record of
8 October 2026, `approved_by: project owner`). The page that would be published
still carries the private-preview banner and `noindex`; producing the public
variant and hosting it belong to the publication step, which is not designed yet.

## Source archaeology, editions 2026-10-04 to 2026-10-08

Pinned baseline since edition `2026-10-08.3`: published history `origin/main` at
`138403ed60846d7cf2a964c3e0a360e52ae6155e` (8 October 2026, 13:18 CEST), 2,295
commits from 2 September 2026. The first editions were pinned to the 4 October
state (old hash `75ad9650…`, now `4a4e89b3`).

**Hashes after the 7 October rewrite.** On 7 October the whole history was
rewritten to the single identity `KNXBench <github@knxbench.com>`, so every
commit hash changed. Edition `2026-10-08.3` remapped all commit references
through `docs/history/COMMIT_MAP_2026-10-07.txt` (each one unique, each one an
ancestor of the pinned baseline). After the partial rewrite of 8 October
(`docs/history/COMMIT_MAP_2026-10-08.txt`), edition `2026-10-08.4` remapped the
three affected hashes, including the baseline, which is now `138403ed`. Later
editions must cite current hashes only.

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
  commit, CLAUDE.md 96.4 % to `fdcc5ab9` (15:12 CEST).
- **Origin of the specification knowledge base:** a ChatGPT-guided extraction
  pipeline with a local model via Ollama from 4 September, an audit of 2,232
  results by 5 September, a clean restart as `knx-spec-kb` on 7 September and
  Claude Code from 10 September.
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
- The browser check passes in Chromium and Firefox (46/46 each on
  `2026-10-05.1`, 5 October 2026). WebKit, the engine behind Safari, has not run
  here: Playwright's WebKit build needs Ubuntu libraries (ICU 74, flite,
  libWPEWebKit) that this Arch-based host lacks, and installing them would change
  the system. Real screen readers have not been checked either.
- Coverage of the human–AI workflow depends on local logs. Cloud sessions and
  anything outside the read sources are missing from this edition.
- Commit references assume the repository's history may be shown publicly; that
  is a review item, not a decision taken here.
