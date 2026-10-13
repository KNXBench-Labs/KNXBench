# AP1 — notifications and existing import details

Own branch/worktree: feature/ux-feedback-20261010 / ux-feedback-20261010.
No integration or push yet. No server/domain/import interpretation changes.

## Implemented

All toast kinds keep the existing nine-second reference and animation. Timers
start only in the three visible slots, pause on hover/focus/hidden document,
resume their remaining time, and are disposed on unmount. Error pushes no
longer replace each other; imports and selection no longer clear unread cards.
A bounded 100-entry raw-error history remains accessible after clearing,
expiry or navigation, only in this window's memory. Queue retirement and
history truncation are counted, not silently implied complete. Long text is
scrollable and keyboard-focusable. Session log/export remains available.

Dashboard and Explorer error/warning counts are native buttons opening the
existing server import rows at their actual severity. All existing source,
message, location, detail and timestamp are shown. No object mapping invented.
A notice explicitly distinguishes project counts from retained session rows.

## Evidence

- Timer RED -> GREEN13, then queue/history RED -> GREEN17 tests.
- Dashboard/log/history component RED before implementation.
- App integration RED before shell wiring; Explorer links semantic RED before
  wiring; production browser RED reproduced one surviving error after two
  failed imports, corrected the implicit clear at import start.
- Final focused Vitest233 passed /0 failed /7 files.
- Explicit src/spec/fixture/config TypeScript check and production build pass.
- Full Web/browser chain cancelled while waiting on the shared foreign lease,
  before any stage started. Final browser/visual and full regression remain open.
- Self-review only; not independent approval. Other worktrees/leases untouched.

## Boundaries / next

Window-memory history is not indefinite disk persistence; export significant
session details explicitly. No private/project text is automatically written
to localStorage. Existing save/CSV callers may explicitly clear active error
cards, but cannot clear the retained raw-error history. Final integrated gates
and offline Chromium screenshots required before acceptance/publication.
Continue AP3 independently while shared gates are busy; no main push inferred.
