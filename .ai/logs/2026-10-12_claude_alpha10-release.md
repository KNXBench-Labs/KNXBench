# Alpha 10 release (claude-release-alpha10, session f76f9c1bb397)

- Owner decision 2026-10-12 21:4x: cut v0.1.0-alpha.10 after the native-evidence port and the
  DPST-9-1 layout-test fix land; freeze notice 44bc16da paused all other sessions.
- Watcher: first trigger (f6c23bbe) was a false positive (marker quoted inside the web-fixups
  lock-take prose); corrected to standalone marker lines only. Real trigger at 4a0294b8.
- Cutoff main: 4a0294b8 (contains 45a38079 native-evidence port, e1088312 DPST-9-1 fix,
  09229157 #6 AppImage EGL, b1f3f8db/9f1ba70b #7 schema 20). Release branch release/alpha10-20261012.
- Notes: every highlight commit verified absent from the v0.1.0-alpha.9 tree. Two draft claims
  corrected before commit: notes' "include in documentation" is a stored flag only (no export
  reads it); catalogue updates are CLI + web catalogue, not a separate desktop feature.
- Native schema 13 -> 14 (project notes tables, e974624e); backup/refusal disclosure in notes.
- Gate 1 on a5903841 (29 stages, inputs frozen): all green except three harness/doc findings:
  check-appimage --tag (needs artifact; covered by tag workflow), diff --check (trailing blank
  line in docs/design-studies/group-addresses.md, fixed in 73420e13), corpus 137/143 because
  project_dump.json was not linked (both affected targets rerun 5/5 + 5/5 on 73420e13).
- Dry runs 38304346494 / 38304349304 green on a5903841; tag workflows 38309517272 / 38309517206
  green on 73420e13. Tag object b39ea6a3. Release created with github-notes (relative links ->
  blob/v0.1.0-alpha.10, all 8 targets checked).
- External: assets re-downloaded, checksums + digests equal; knx-mcp version; AppImage extracted,
  no libwayland-client; Mesa-pinned startup survived 12 s. Docker version == latest digest,
  both platforms, revision label, anonymous pull + smoke test; user container identity unchanged.
- Docs pass in docs/alpha10-release-20261012 worktree; freeze released in the same commit.
