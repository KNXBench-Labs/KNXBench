# Review checklist for candidate 2026-10-08.1

This candidate is **private** and **not approved for publication**. Preparing it does not
approve it. Publication needs a separate, explicit approval of exactly
`story_sha256 = 81692108d5b9101e9aaa0821fe33bbd3d7c4b312bf96e0434415fa12cf6ff992`; any later change needs a new approval.

## Manual checks

- [ ] Every translated excerpt preserves the meaning of its original.
- [ ] Every privacy edit removes the private detail without changing the decision it records.
- [ ] No excerpt makes a historical decision look better informed than it was.
- [ ] Editorial links and asides are acceptable as interpretation, not presented as fact.
- [ ] Commit references are acceptable to show publicly (repository visibility checked).
- [ ] Source gaps and coverage are stated accurately.
- [ ] The rendered preview was read on desktop and mobile.
- [ ] The narrator's voice stays in chapter text and asides; summaries, excerpts, evidence,
      uncertainty and gaps state the record without persona.
- [ ] The voice never makes a safety, privacy or compatibility limit look smaller than it is.
- [ ] The narrator disclosure is accurate (no borrowed lines beyond quoted evidence).

Automatic scanning cannot prove that content is safe to publish; it only flags patterns.

## Automatic privacy scan

- warning: possible KNX individual address or version number at `events[alpha-backlog].aside`: “Version 0.1.0-alpha.1, no tag, no…”

## Private traceability

- Events without a private provenance entry: none
- Private entries without a public event: 0
- Private locators found in the public payload: 0

## Excerpts with translation or edits

- `home-toolbox` (user): Translated from German · Edited for privacy
- `strategy-written` (user): Translated from German
- `strategy-written` (user): Translated from German
- `strategy-written` (user): Translated from German
- `strategy-seed` (user): Translated from German
- `architecture-day-one` (agent): Translated from German · Paraphrased
- `architecture-day-one` (user): Translated from German
- `schema-23-refusal` (user): Translated from German
- `rename-knxbench` (user): Translated from German
- `web-docker` (user): Translated from German
- `own-format` (user): Translated from German
- `codex-joins-goal` (user): Translated from German
- `codex-joins-goal` (user): Translated from German
- `spec-knowledge-base` (user): Translated from German
- `spec-knowledge-base` (user): Translated from German
- `languages` (user): Translated from German · Shortened
- `home-bus-boundary` (user): Translated from German · Edited for privacy
- `ui-redesign` (user): Translated from German · Shortened
- `ui-redesign` (user): Translated from German
- `licence-agpl` (user): Translated from German
- `loading-jokes` (user): Translated from German · Shortened
- `drop-ets-export` (user): Translated from German
- `history-scrub` (user): Translated from German
- `notation-reversal` (user): Translated from German
- `first-device-programmed` (user): Translated from German
- `first-device-programmed` (user): Translated from German
- `split-goals` (user): Translated from German
- `licence-cla-reversal` (user): Translated from German
- `licence-cla-reversal` (user): Translated from German
- `theme-packs-crt` (user): Translated from German · Shortened
- `evolution-story` (user): Translated from German · Shortened
- `story-narrator` (user): Translated from German

## Editorial links

- `strategy-seed` → `rename-knxbench`: Same project, new name.
- `ui-redesign` → `theme-packs-crt`: The redesign asked for a themeable interface; theme packs came three weeks later.
- `strategy-seed` → `licence-agpl`: An independent alternative eventually needs a licence.
- `paperclip-experiment` → `split-goals`: After the shutdown, work continued in separate goal sessions.
- `drop-ets-export` → `alpha-backlog`: Import boundaries are part of what the alpha may claim.
- `theme-packs-crt` → `evolution-story`: The story's green CRT look follows the same afternoon's theme work.
- `project-stats` → `evolution-story`: Counting the project came first; explaining it came next.
- `alpha-backlog` → `evolution-story`: Taking stock, two days apart.
- `test-transmitting-building` → `story-narrator`: The commit messages had spoken like a depressed robot for weeks; no source says that inspired the narrator.

## Recorded uncertainty

- `home-toolbox`: The story deliberately begins with the project's first surviving prompt. Anything earlier is outside its scope.
- `strategy-written`: The committed files are close to, but not identical with, ChatGPT's replies: 98.7% of the whitespace-normalised text of the strategy and 96.4% of the CLAUDE.md match. Who made the small edits before committing is not recorded.
- `strategy-written`: The conversation records the model only as 'auto'.
- `strategy-seed`: The prompt names the document but does not say where it came from; the link to the ChatGPT conversation rests on the matching title, text and times.
- `architecture-day-one`: ADR-0003 adopted SQLite and recorded the text format as optional future work. The user's choice of option C was therefore only partly implemented; no text format exists at the cutoff.
- `ets-import`: At this point only one schema version, from one real project, had been tested.
- `schema-23-refusal`: The design document cites the project backlog and a second sample project, not this prompt. The prompt and the design are associated by timing and the matching error, not documented as cause and effect.
- `live-bus-first`: Verification covered one gateway in one installation.
- `codex-joins-goal`: The goal file was committed on 12 September; earlier versions existed only in the working tree.
- `codex-joins-goal`: date precision is approximate
- `spec-knowledge-base`: The audit figures and the pipeline's steps come from the user's messages in those conversations, not from the knowledge base's own files, which were not read for this edition.
- `spec-knowledge-base`: date precision is approximate
- `ui-redesign`: Which lines each agent wrote cannot be told from Git metadata; all commits carry the user's identity.
- `appimage`: README at the cutoff states that no release has ever been published.
- `test-transmitting-building`: What, if anything, the stray messages did in the building is not recorded.
- `history-scrub`: Because of the rewrite, commit hashes from before 20 September differ from the hashes the agents saw at the time.
- `project-stats`: The script itself lives outside this repository; only its reports are versioned here.
- `project-stats`: The date marks reports from late September; the script's own start date is not established here.
- `project-stats`: date precision is approximate
- `paperclip-experiment`: Ticket and run counts come from the last local backup (27 September); the detailed run logs were not reviewed.
- `first-device-programmed`: Verified on one device with one application program. This does not establish commissioning support for devices in general.
- `first-device-programmed`: The date marks the first of four steps between 26 and 30 September.
- `first-device-programmed`: date precision is approximate
- `split-goals`: Whether the end of the orchestration experiment led directly to the split is not documented; the connection shown is editorial.
- `fail-closed-writes`: date precision is approximate
- `alpha-backlog`: The README is committed under the user's name; whether its wording was drafted with AI help is not recorded.
- `theme-packs-crt`: Native desktop rendering and screen-reader behaviour of the theme are documented as not yet accepted.
- `crawler-corpus`: Evidence from one run, not a repeatable gate; the downloaded files are not part of the repository.
- `story-first-preview`: This step is local and unpublished; it is not part of the published history at the cutoff.
- `story-first-preview`: Whether and when this story is published is still undecided.
- `story-narrator`: The voice is an editorial choice for this edition; earlier candidates keep their original narration.
