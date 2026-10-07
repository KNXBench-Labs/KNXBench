# U3 — Browser project export wording (2026-09-28)

- Web lock reserved on `main` in `33aa8e4` before editing the isolated `ui-file-menu-rename` worktree. No device operations.
- `App.test.tsx` RED: 3 failures for the old English label, the absent new translation key, and the missing export button. GREEN: 5 focused tests passed. German catalogue is asserted directly, while browser behavior still uses `/api/project/download` and `project.knxdb`; Tauri omits the entry as before.
- Renamed the translation key and local handler to `exportProject`; user-visible label is `Export project… / Projekt exportieren…`. Kept the HTTP route name for compatibility and distinguished Export from Save As. Updated `docs/GLOSSARY.md`, project/UI manual, known issue and status entries.
- Gates and review results are recorded in the corresponding CURRENT_STATE entry. No network, KNX or persistence semantics changed.
