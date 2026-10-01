# ISSUE-04 completion evidence and last-save locale

Scope: the UI-track completion condition requires named evidence for every owned issue-plan checkbox. The ISSUE-04 plan retained five unchecked boxes even though the native saved-baseline state, autosave engine and settings were shipped in DIN-12. Worktree `ui-goal-close-audit` holds the Web lock; root foreign edits are untouched.

## Verified source and regression

- `apps/knx-server/src/domain.rs::project_is_modified` compares user content with a clean project baseline, not the undo stack. `http_project_routes.rs::modified_state_clears_only_after_successful_save_or_save_as` already asserts failure keeps dirty state and timestamp. New `saved_edit_can_be_replaced_without_a_discard_confirmation` and `undo_to_saved_baseline_then_branch_tracks_the_new_unsaved_edit` assert 409 before Save, no prompt after, clean undo, invalidated redo and timestamp stability on an unsaved branch. Targeted server test: 13 passed / 0 failed / 3 ignored.
- `apps/knx-web/src/App.tsx::formatLastSaved` used `Intl.DateTimeFormat(undefined, ...)`, i.e. browser locale, even with German UI text. New `App.test.tsx::uses the selected UI language and advances only after a successful save` failed RED on English-formatted status in German UI, then passed after the formatter took `useUiLanguage()` explicitly. A failed Save leaves the previous status; an accepted refreshed snapshot advances it. Full Web suite after the change: 82 files / 1,297 passed; build and Rust formatting pass.
- Existing `autosaveSettings.test.ts`, `SettingsPanel.test.tsx` and nine `useAutosave.test.tsx` cases cover enabled-by-default/five-minute configurable interval, disabled state, five-second countdown, manual save cancellation, edits mid-countdown, missing path, concurrent suppression and failure/retry. The plan's `is_dirty` shorthand is corrected to the actual additive `is_modified` / `last_saved_at` project snapshot.

## Integrated candidate gates

- Corpus-backed Rust: 139 suites / 2,793 passed / 0 failed / 161 ignored / 0 `SKIP:`. Focused HTTP route tests: 13 passed / 3 ignored.
- Web: 82 files / 1,297 passed, TypeScript no diagnostics, Vite build green. Local mocked Chromium suites: Site 4/4, Service Control 4/4, Device checks 4/4, monitor 4/4, ISSUE-09 10/10.
- Strict workspace Clippy, `cargo fmt --check`, `check-headers`, `check-anchors`, `check-layering`, `check-corpus-gates` and `git diff --check` all passed. An initial typo `check-corpus` returned “unknown task”; rerunning the actual `check-corpus-gates` succeeded; this was not a source or corpus failure.

No hardware action, KNX tunnel, device write, live gateway or credentials were involved. This work is ISSUE-04 evidence and a locale bug fix, not the U13 closing review. ISSUE-12's two discovery checkboxes remain open pending wire/gateway evidence; U13 requires the user's review decision. Publish only after scoped diff review and rebase/remote SHA readback.
