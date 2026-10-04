# 2026-10-04 — Desktop shell recovers a terminated web process (KNOWN_LIMITATIONS §133)

Agent: Claude, UI session. Scope set by the user on 2026-10-04: deliver this
§133 shell fix, then stop; the parallel goal-ui owner session stays owner.

## Problem

Since §132 the frontend vetoes every window close through its
`tauri://close-requested` listener. If the WebKit web process dies, nothing can
answer, and × / Alt+F4 do nothing for good. Before this change the limitation
had been read from the pinned `tauri` 2.11.5 sources but never reproduced.

## Change

- `apps/knx-desktop/src-tauri/src/web_process.rs`: GTK-free policy
  (`WebProcessSupervisor`). `Crashed` / `ExceededMemoryLimit` / unknown reasons
  → reload, at most 3 within 60 s, then `GiveUp` (sticky). `TerminatedByApi`
  is ignored. The close decision is `LeaveToFrontend` while the page lives;
  once it is dead the result is `Close` if nothing is unsaved, otherwise
  `ConfirmDiscard` (shown once, then `AlreadyAsking` until answered). On Linux,
  `watch()` connects WebKit's `web-process-terminated` signal.
- `apps/knx-desktop/src-tauri/src/lib.rs`: manages the supervisor and the
  server state; `CloseRequested` routes through the decision. The discard
  question is a `tauri-plugin-dialog` GTK message dialog (non-blocking `show`).
- `apps/knx-server/src/domain.rs`: `AppState::has_unsaved_changes()`, the
  predicate behind the published `is_modified`, locking `project` before
  `clean_project` as the replacement transaction does.
- Dependency: `webkit2gtk = "2.0"` (workspace), Linux-only in `knx-desktop`
  with feature `v2_20`; already resolved at this version through `wry`.

## Evidence

- RED before every GREEN: policy stub (6 tests failed at assertions), reason
  mapping (1), close decision (2), server predicate (1).
- Mutants (compiled, named assertion failures, sources restored byte-exact,
  SHA-256 checked): deliberate termination reloads; no reload budget; budget
  window forgets everything; give-up not sticky; living frontend overridden;
  dead frontend ignores unsaved edits; discard prompts stack; answered prompt
  never resets; server reports every project clean. 9/9 caught.
- Native run (`unshare -rn`, `lo` only, signature-verified Xvfb 21.1.24-1,
  private HOME/XDG, explicit private D-Bus without service directories,
  debug binary with Vite on loopback). The harness kills the real
  `WebKitWebProcess` with SIGKILL and sends ICCCM `WM_DELETE_WINDOW`.

  | Binary / mode | Result |
  | --- | --- |
  | baseline `eafb0322`, one kill | no new web process; not exited 20 s after close (original bug) |
  | fix, one kill | `Crashed → Reload`, new web process, close → exit 0 |
  | fix, four kills | 3× Reload, then GiveUp, close → exit 0 |
  | fix, four kills, unsaved project via server API | GiveUp; close → still running + native question; dismissed → still running; next close → asked again |

- Not covered: choosing "Close and discard" natively, a full desktop window
  manager or Wayland compositor, a hung (non-terminated) web process.
- Integrated gate on ff71d759 + this change: fmt, clippy -D warnings, workspace tests 3,145 passed / 0 failed / 177 ignored in 169 blocks with 0 skip markers (knx-desktop 12/12), cargo deny bans/licenses/sources ok, four repository gates, Web typecheck and 1,739/98 files, whitespace; source frozen during the run.

## Boundaries

No `apps/knx-web` edit, no Web lock change, no Alpha ledger change, no KNX,
bus or LAN traffic. Earlier in this session a 24-row Alpha triage against a
stale baseline was reverted unpublished after the parallel owner session's
newer, user-decision-backed checkpoint was found on main.
