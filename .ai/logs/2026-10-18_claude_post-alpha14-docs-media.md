# Manual, README and screenshots after Alpha 14

- **Agent:** Claude
- **Date:** 2026-10-18 15:31
- **Branch:** `docs/post-alpha14-refresh-20261018` (worktree `KNXBench-docs-alpha14-20261018`)

## What was done

- Pulled only the version tag `knxbench/knxbench-server:0.1.0-alpha.14` (image id equals
  `latest`, which the owner's container runs; its id/image/start time were recorded before
  and after and did not change). Extracted `knx-server` (SHA-256 `58aae893…`) and
  `/app/frontend` from a stopped `--network none` container.
- `manual-screenshots.shots.ts`: unfolds Workspaces/Project explorer (+ Expand all), the
  Diagnostics block before the bus monitor, the address ranges before every address shot,
  picks the parameter area that holds the asserted field, opens Channel A for the Graphite
  parameter shot, takes screenshots with `animations: "disabled"`, and checks the login
  version from `MANUAL_EXPECTED_VERSION`.
- `manual-group-address-views.shots.ts`: unfolds the navigation, opens the closed matrix
  column chooser to load objects and folds it again for the shot.
- Two full runs of both specs: 2/2 and 1/1 passed each; 33 of 50 stills byte-identical,
  the rest differ only in anti-aliasing, timestamps or the import banner's random status line.
- README stills and theme gallery now use the manual captures (no website/release-set paths).
- Manual text: "since Alpha N" labels with `<a id>` aliases for the renamed headings;
  alt texts rewritten from the actual frames (folded navigation on the overview, parameter
  workspace, object size, palette 22 commands, help 13 topics); size limits raised on `main`
  after the tag labelled as source-only next to the Alpha 14 bounds; project-status note on
  what `main` adds after Alpha 14.

## Split with the website claim

A parallel session claimed `website/`, `docs/WEBSITE.md`, `docs/assets/README.md`,
`docs/assets/releases/alpha14/` and the release-media spec/config at 15:02, while this
package was already capturing. Work on those paths (adapted release-media spec with notes
decision, diagnostic dialog and folded tree stills; 14 PNGs; website copy/tests/WebPs, all
website tests and both browser recipes green) was parked on the **local** branch
`wip/alpha14-release-media-handoff-20261018` (`c84152f1`, not pushed) for that session to use
or ignore. It does not follow the owner's newer "no disclaimers" wording for the site.

## Gates

check_documentation 0 errors (656 files), xtask check-anchors 1,510 links none dead,
check-headers 155/155, check_local_info 0, tools tests 223 OK (1 skip), tsc of the edited
capture specs, git diff --check.

## Follow-up

`docs/assets/README.md` (media inventory) still describes the Alpha 9 manual capture;
update it after the website claim's release entry (claimed path).
