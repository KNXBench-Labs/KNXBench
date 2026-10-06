# 2026-10-06 — Claude — AR18 conditions re-check (independent)

Fresh reviewer session, started by the user from
`docs/review/AR18_RECHECK_BRIEF.md`. Read-only on product code, offline, no
hardware, no subagents.

## What was done

- I built my own worktree of candidate `bd691b82`. Its product code is the
  code of `faa3955f` (verified: everything since then is docs-only). I used
  a fresh target directory and held all three leases.
- I reproduced the dossier §8 gate table. Every count matches; the anchor
  count differs because of later docs-only commits.
- Corpus 143/143. M7 runner without the corpus refuses with exit 2.
- My own adversarial inputs, built from the fictional sample:
  - F1 through the API and through Chromium against the real server and the
    production build;
  - F2 in the outer archive and in a hand-ZipCrypto-encrypted nested
    payload;
  - F3 with peak RSS under an 8 GiB cap;
  - F4 and M1 with SHA-256 before and after;
  - M2 with the server killed (SIGKILL) during *Save As*;
  - M3 with several part layouts;
  - M8 with a fake `HOME`.
- AppImage: extracted and scanned for paths. Started offline via the
  Wayland recipe. Its own server was probed with the new inputs.
- Read-only census of the corpus `.knxproj` files. Aggregates only.

## Result

`READY_WITH_CONDITIONS`. F1 and F4 are fixed, and M1–M4, M7 and M9 are
fixed. M6 and M8 are partly fixed. Two IMPORTANT residuals:

- **N1:** member names that zip *decodes* to the same name bypass the
  raw-byte duplicate check. The routes are the Unicode Path extra field
  (0x7075) and CP437 against UTF-8. One member can silently replace
  `0.xml`.
- **N2:** the nested-payload budget is checked only after full
  decompression. A 12 MB file reached 8 GB.

MINOR:

- N3: restore misses M6.
- N4: `AppState::new` in server tests creates the user's product DB.
- N5: refusals answer 500.
- N6: after a reload the web shows the welcome page while the server holds
  an edited project.

## Pitfalls met

- `pkill -f <pattern>` killed my own shell, because the command line
  contained the pattern. Use `pgrep -x <name>`.
- `tail` on a pipe that a backgrounded GUI child still holds open never
  returns. Redirect to a file instead.
- My M1 fixture loop ran a writer last (`import --replace`), so later
  *Open* probes saw a filled file. Rebuild the fixtures for every probe set.
