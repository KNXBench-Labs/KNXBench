# Hall of Fame bot: only fresh medals in comments (Claude, 2026-10-19)

## Request

The owner reported that the scoreboard bot's XP update text always lists every
medal the user holds. Medals belong in the Hall of Fame table; a comment should
mention a medal only when one was just earned.

## Cause

`closing_note` and `pull_request_note` both rendered `target["medals"]`, the
contributor's full collection, on every message ("Medals earned: …", or the
"None yet — the trophy cabinet is warming up." filler for PRs).

## Change

- Attribution by ordinal, not by "count minus one": the n-th event of a track
  unlocks the medal with threshold n. A count-difference approach would miss a
  milestone when two completions land in one refresh, and would credit every
  one of five PRs with the five-PR medal.
- Completion: the reporter's retained awards sorted by `(closed_at, number)`.
- PR: the author's PRs sorted by number; merges sorted by `(merged_at, number)`.
- Report-count medals are tied to no acknowledged event and stay on the board.
- `pull_request_note(snapshot, pull, pulls)` gets the full PR list explicitly
  (no defaulting guess).

## Verification

- RED first: 10 failures on the new/updated expectations; then 81/81 green.
- Rendered the page from the committed snapshot: only the rules text changes; the
  README block is byte-equal.
- Sample PR comments rendered locally (merged first PR names both first medals;
  a second open PR has no medal line).

## Live effect

Maintained PR credit comments change body, so the first refresh after the push
PATCHes each of them once. Completion notes are post-once and stay unchanged.
