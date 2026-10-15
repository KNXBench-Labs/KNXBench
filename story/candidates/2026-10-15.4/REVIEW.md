# Review checklist for candidate 2026-10-15.4

This candidate is **private** and **not approved for publication**. Preparing it does not
approve it. Publication needs a separate, explicit approval of exactly
`story_sha256 = 081d2c4c42b03a0b97ae803ac7f78375e4ca5593f3687453872200aab8398cf8`; any later change needs a new approval.

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

- warning: possible KNX individual address or version number at `events[alpha-backlog].aside`: “…as written: version 0.1.0-alpha.1, no tag, no…”

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
- `ui-redesign` (user): Translated from German · Shortened
- `licence-agpl` (user): Translated from German
- `loading-jokes` (user): Translated from German · Shortened
- `drop-ets-export` (user): Translated from German
- `history-scrub` (user): Translated from German
- `notation-reversal` (user): Translated from German
- `first-device-programmed` (user): Translated from German
- `first-device-programmed` (user): Translated from German
- `licence-cla-reversal` (user): Translated from German
- `licence-cla-reversal` (user): Translated from German
- `theme-packs-crt` (user): Translated from German · Shortened
- `evolution-story` (user): Translated from German · Shortened
- `story-narrator` (user): Translated from German
- `telegram-flow` (user): Translated from German · Shortened
- `telegram-flow` (user): Translated from German · Shortened
- `alpha-review` (user): Shortened
- `first-alpha` (user): Translated from German
- `evidence-of-record` (user): Translated from German · Shortened · Edited for privacy
- `fun-features` (user): Translated from German
- `fun-features` (user): Translated from German · Shortened
- `lcars-theme` (user): Translated from German · Shortened
- `goals-closed` (user): Translated from German · Shortened
- `functions-not-sites` (user): Translated from German · Paraphrased · Shortened

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
- `loading-jokes` → `fun-features`: The same appetite for jokes, now with trophies.
- `hermes-shared-memory` → `mcp-adapter`: From agents sharing notes about the project to agents reading the project itself.
- `public-front-door` → `fictional-demos`: Two public entry points, grouped editorially; adjacency does not establish causation.
- `public-front-door` → `users-find-edges`: Public use and subsequent issue reports are a narrative connection, not evidence that the website caused each report.
- `exact-or-refused` → `native-memory`: Both protect native project integrity; this is not a claimed single-prompt dependency.
- `source-not-semantics` → `functions-not-sites`: Two approaches to modelling only evidenced structure, not an asserted implementation dependency.
- `two-more-live-scopes` → `tester-not-verdict`: The reporting design preserves similar distinctions; this edition does not infer that the trials caused it.
- `evidence-of-record` → `system-two-still-no`: Both illustrate explicit evidence boundaries; no direct causal chain is claimed.
- `split-goals` → `measured-gates`: Separate work packages and measured shared verification are associated editorially, not attributed to one prompt.
- `system-two-still-no` → `universal-report-design`: A support gap and a reporting proposal coexist; universal reporting must not imply universal safe access.

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
- `ui-redesign`: Which lines each agent wrote cannot be told from Git metadata; every commit carries the same project identity.
- `appimage`: The first AppImage pre-release followed on 7 October 2026, in the still private repository.
- `test-transmitting-building`: What, if anything, the stray messages did in the building is not recorded.
- `history-scrub`: Commit hashes from before 20 September differ from the hashes the agents saw at the time, and all hashes changed again on 7 October.
- `hermes-shared-memory`: The shared-note indexing tool was later decommissioned on 8 October (203b601b). Historical use is not a claim that it remains the current coordination service.
- `project-stats`: The script itself lives outside this repository; only its reports are versioned here.
- `project-stats`: The date marks reports from late September; the script's own start date is not established here.
- `project-stats`: date precision is approximate
- `paperclip-experiment`: Ticket and run counts come from the last local backup (27 September); the detailed run logs were not reviewed.
- `first-device-programmed`: Verified on one device with one application program. This does not establish commissioning support for devices in general.
- `first-device-programmed`: The date marks the first of four steps between 26 and 30 September.
- `first-device-programmed`: date precision is approximate
- `split-goals`: Whether the end of the orchestration experiment led directly to the split is not documented; the connection shown is editorial.
- `fail-closed-writes`: date precision is approximate
- `alpha-backlog`: Whether the README's wording was drafted with AI help is not recorded.
- `theme-packs-crt`: Native desktop rendering and screen-reader behaviour of the theme are documented as not yet accepted.
- `public-product-analysis`: The source product databases are not included in this story. Their availability for analysis does not establish successful device programming or universal product compatibility.
- `story-first-preview`: The page itself is published only after a separate approval of one exact edition.
- `story-narrator`: The voice is an editorial choice for this edition; earlier candidates keep their original narration.
- `alpha-review`: The reviewers were AI sessions, independent of the work, but not of the kind of tool that did it.
- `first-alpha`: This withdrawn release is evidenced by the retained publication record in commit 541b2bbd, not a current release listing. Its original private publication was not a public download. Later Alpha 5 republication is a separate event.
- `one-name`: The user's requests for this change are not quoted because they contain the very details that were removed.
- `evidence-of-record`: 'Untested' means exactly that: these plans have not been tried on the devices.
- `evidence-of-record`: The no-new-hardware statement belongs to this early-October decision. Two later complete-download hardware trials are documented separately on 9 October.
- `mcp-adapter`: This describes the original read-only adapter. An opt-in, previewed literal-rename writer was added on 12 October; it has no bus access and is a separate event in this edition.
- `release-alpha-5`: Historical original release and current republished record are distinguished. Old data-folder migration was manual; no portability or universal compatibility claim.
- `legacy-vd`: The earlier size and planner limits were superseded on 9 October by 28d8f6af and a7e338e7. Supported legacy plans remain Untested unless their exact application program and scope have separate hardware evidence. The converted detector application in the later hardware run is not the original legacy application.
- `public-front-door`: This edition checks the pinned deployment source and recorded delivery; it is not a fresh measurement of public-site availability.
- `native-memory`: History inside one file is not an independent backup if that file or its storage is lost.
- `two-more-live-scopes`: The earlier detector trial is recorded in commit 59f4df15. Both runs left Basic Restart unconfirmed; later functional observations are distinct evidence, not a confirmed restart acknowledgement.
- `two-more-live-scopes`: The two new entries cover complete downloads only. Partial downloads, every instance sharing the program, all device memory and physical blind movement were not proved by these runs.
- `release-alpha-6`: These are the Alpha 6 release contents. Later legacy download planning is not retroactively included, and CI launch checks are not every-desktop hardware tests.
- `source-not-semantics`: Selective import was delivered separately in 99e05425 (ADR-0106); own-instance read-only values in d050c7e7 (ADR-0108). Their association here is editorial, not a claim of one causal patch.
- `source-not-semantics`: Functions-bearing selective source import remains refused at this cutoff. No complete Repeat evaluation, ETS export or signature verification follows.
- `release-alpha-7`: Published at 2026-10-10T00:11:11+02:00, after midnight locally; the preceding source commits are dated 9 October. Release publication did not itself perform a hardware trial.
- `release-alpha-8`: Retained source bytes do not establish complete ETS semantics, signatures, Add-in execution or broader hardware support.
- `release-alpha-9`: Project notes, Functions/Sites and later parallel work were not swept into this release. Old native readers refuse upgraded files.
- `native-writing-desk`: Notes have a separate plain-text contract. Rich instruction support is bounded; other schemas, arbitrary RTF and ETS export of edited overlays are not claimed.
- `native-writing-desk`: Native WebKitGTK and Orca accessibility acceptance remain open.
- `catalogue-with-return`: The web path follows in ADR-0118. There is no automatic Online Catalog access or supported device application-version upgrade.
- `mcp-narrow-writer`: The server cannot prove that a human rather than an auto-accepting client answered. Real third-party client acceptance remains open; there is no bus access or general mutation permission.
- `users-find-edges`: The AppImage packaging change is in 09229157. Source fixes and reporter outcomes have different dates; the original Kubuntu launch confirmation, schema 22 and nested sub-module object numbering remain open.
- `release-alpha-10`: Functions/Sites and the later UX-feedback package were not in this snapshot. Browser/CI evidence is not universal Linux startup or real-device acceptance.
- `tester-not-verdict`: No automatic upload, automatic support promotion or proof of private delivery is implied. Pre-plan refusal guidance was delivered in source after Alpha 11.
- `functions-not-sites`: Web editing followed in f6c866fe. Real supported-schema ETS Function samples, unresolved source semantics and multi-installation site requirements remain evidence gaps.
- `project-carries-programs`: Source delivery after Alpha 11. Hash integrity is not a distribution licence; older native readers do not gain downgrade compatibility.
- `system-two-still-no`: Research follow-up is documented in 0d2701b4. No new hardware operation or successful download is established. Exact firmware identity remains a prerequisite for separately authorized future hardware evaluation.
- `measured-gates`: Measured improvement applies to the functional browser lane on the recorded host, not the whole gate or universal product performance. No retries, skipped assertions or hardware certification follow.
- `pending-is-belief`: Source delivery after Alpha 11; no new hardware trial. Pending marks are project beliefs, not a fresh comparison, functional verification or device-sync verdict. Recovered snapshot undo may conservatively mark additional parts pending.
- `pending-is-belief`: The separately active device-comparison work is outside this pinned source cutoff.
- `universal-report-design`: This cutoff contains a reservation and design direction, not the universal reporter implementation. Existing support guidance is implemented; the proposed unified acquisition/report workflow is not. No live read, unlock, write or upload authority is granted.
