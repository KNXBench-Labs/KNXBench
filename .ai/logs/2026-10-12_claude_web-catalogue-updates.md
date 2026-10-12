# 2026-10-12 — Claude — catalogue updates in the web (web integration package 4)

## What changed
- ADR-0118 and the contract's web section: preview/apply/backups/restore routes over
  `knx_productdb::update` (`apps/knx-server/src/catalogue_update_routes.rs`), product
  mutex held throughout, stateless token-checked apply, backups by listed name only,
  connection reopened after restore. `ApiError::with_kind`; the install route shares
  `package_refusal` (behaviour unchanged, covered by the install tests).
- Web: `api.ts` update calls through one `postForm` helper; `CatalogUpdatePanel.tsx`
  (preview, back up and install, stale → preview again, lazily read backups,
  ask-first restore via `Overlay`); `catalogueUpdate.ts` pure helpers.
- `legacy-install.e2e.ts` now targets the picker that accepts legacy databases;
  `DiagnosticsCompanion.test.tsx` counts `postForm` uploads.

## Lessons
- Check whether a component is mounted for every session before giving it a
  mount-time fetch; twenty strict fixtures would have broken.
- A helper refactor can shrink a safety census silently; prove the census red.
- Server mutants that survive usually mean a missing test case (empty vs missing
  token, `read_dir` order, connection replacement), not a dead guard.
