# KL-158 — owned AppImage display policy (codex, 2026-10-07)

## Scope and identity

User request: implement KL-158 after alpha.4 publication. Packaging only;
no KNX/domain/storage/schema/API change and no hardware permission implied.
Worktree `/mnt/daten-i/Sourcecode/KNXBench.worktrees/kl-158`, branch
`fix/kl-158`, tested base `b54cd5a5e8fcf37d5fbc89212e4c4acb20bd25e0`.
No commit, push or tag/asset replacement was requested or performed.

The root checkout is concurrently owned by another Flow Visualizer interview
session and has that session's uncommitted handover entry. It was not edited,
stashed or reset. Latest published closing tip was `c9d2a483` (README/manual
and recording helpers, not runtime application changes). This package stays
on its explicitly tested base; future integration must preserve the newer
README/manual entries and the other session's root handover.

## Implementation

- `tools/appimage/display-backend.sh`: preserve nonempty caller backend;
  Wayland session hints choose `wayland,x11`; X11 default otherwise; DMABUF
  workaround only when Wayland is permitted; exact renderer override retained.
- `tools/prepare_appimage_tools.py`: primary-source-verified Tauri CLI 2.11.4
  local-tool/before-bundle mechanism, pinned source/digest, bounded fetch,
  verified offline cache, quoted-heredoc guard and atomic executable write.
- `tauri.conf.json`: `useLocalToolsDir` + before-bundle preparation.
- Linux workflow: Python/Weston test prerequisites, launcher unit suite,
  explicit X11/native-Wayland startup bodies. No production sandbox bypass.
- Docs: APPIMAGE_LAUNCHER receipt, ADR-0021 amendment, ARCHITECTURE, KL §158,
  troubleshooting, roadmap, implementation status, ledger/matrix recount.
  Only KL-158's ledger row changed; 191 IDs preserved, all other rows unchanged.

## Frozen source / artifact

Five-file launcher fingerprint:
`5852c1e91fab7ea51fd9054a63df2d7c35658f41427644c4acde61b7f734ceed`.
Development AppImage: 110,041,592 bytes, SHA-256
`2d88a4f282254499253571f2c7f60ae12c05980c65d5c62e2d857ca6704b936d`.
Actual Tauri build completed, before-bundle hook ran; package gate passed
against the completed artifact. This is a dirty-tree development build;
its Git stamp identifies the base, not the uncommitted patch.

## Executed acceptance

- Launcher: 19 passed; RED observations retained for absent hook/tool policy,
  out-of-heredoc assignment and Wayland session-type hint.
- Eight behavioral mutants caught in throwaway replicas; source hashes unchanged.
- `xtask`: 85 unit + 12 runtime-scope tests, all passed.
- Seven positive direct-image scenarios: automatic X11; automatic Hyprland;
  automatic/explicit native Weston; stale Wayland → X11; explicit X11 despite
  a Wayland hint; private-Weston framebuffer capture. Each performed seven
  HTTP 200 checks over new/save-as/reopen + fictional sample import/save/reopen.
- Two expected GTK refusals (101), plus one unchanged-alpha artifact refusal
  under the same Wayland-only control. Every API smoke: lo-only namespace,
  private HOME/XDG, real `.AppImage` entrypoint with automatic extraction mode.
  FUSE mounting itself is not acceptance evidence.
- Private Weston screenshot inspected: real UI, alpha label, first-run guide,
  v0.1.0-alpha.4 footer. Xvfb/fallback screenshots inspected.
- Normal UID 1000: native (not Xwayland) mapped Hyprland window, correct actual
  environment, own frontend/version loopback endpoints; no KNX/project mutation
  requests. Host crop was occluded, rejected/deleted, not pixel acceptance.
- Exact CI startup bodies replayed locally, real Xvfb/Weston, both exit 0.
  No GitHub-hosted run dispatched.
- Released artifact digest unchanged:
  `138444b4cf7f5664ce2f64e88b82574f88dcc5b382dca7f2e49ff1eae7d6c3fc`.
  Global GTK cache digest unchanged:
  `cb379f9b0733e9ad9f8bd78f8c2fa038aef2478523bb7d4c8e64ff6a1ea3501a`.

## Non-green attempts, not silently counted as passes

- Full Python tools: 56 tests, one existing shared-memory-marker failure in
  CLAUDE.md; same named test fails in untouched root. Not repaired in this scope.
- Initial empty-project 200 expectation was a harness error (actual contract 400).
- UID-preserving nested namespace hit sandboxed SVG-loader failure; normal-user
  window mapping and root-mapped offline smokes worked. No disable-sandbox fix.
- Foreground/crop path was invalid/occluded; unrelated pixels deleted. Private
  compositor capture, not an image fabricated from a browser or mockup, accepted.
- First capture inventory looked under HOME instead of CWD; corrected harness
  writes/reads in its owned directory and completed green on unchanged artifact.
- An early package gate correctly refused zero files while bundling still ran;
  the completed-artifact gate is the accepted observation.
- Local CI replay setup first lacked external xvfb-run on PATH, then used an
  overly long Unix-socket runtime path. Owned wrappers/shorter scratch fixed
  the replay envelope; unchanged workflow bodies completed green.

## Evidence and continuation

Compact receipts/scripts/logs and accepted private-compositor screenshots are
retained outside Git in the maintainer's post-alpha evidence directory
`kl-158-appimage-launcher-20261007`. Source/artifact manifest and
`aggregate_acceptance.py` bind counts to retained final receipts; failed
attempts stay classified separately. No private ETS/product/hardware data.

Final doc-gate and cleanup disposition is in this worktree's CURRENT_STATE.
The implementation/artifact remains local, uncommitted and unpublished. If
commit/integration is authorized later: reconcile latest origin and both complete
handover prefixes, repeat relevant gates on the integrated state, commit only
this package as github@knxbench.com with no co-author, and do not replace/move
v0.1.0-alpha.4 or its asset without a separate release decision.
