# AR19 release (Claude, 2026-10-07)

User decision (clarify, answered once): "Tag v0.1.0-alpha.4 auf 514c0c54 setzen
und ein GitHub-Pre-Release (im privaten Repo) mit AppImage und SHA-256 anlegen".

- Annotated tag v0.1.0-alpha.4 (object 623c303b) → 514c0c54, tagger
  KNXBench <github@knxbench.com>; pushed, ls-remote ok.
- gh release create --verify-tag --prerelease, notes from scratch notes.md
  (absolute blob/v0.1.0-alpha.4 links, each target checked with
  git cat-file -e), assets AppImage + SHA256SUMS.
- Verified: release view (pre=true, draft=false, GitHub digest sha256:138444b4…),
  downloaded both assets, sha256sum -c ok, cmp identical to evidence copy.
- Docs: LEDGER RELEASE-04 DONE (snapshot DONE=38, WAITING_DECISION removed),
  Matrix, ALPHA_FINAL_GATES §13, goal AR19 DONE, installation chapter,
  IMPLEMENTATION_STATUS.
- Not done (not authorized): repo visibility, CI release workflow.
