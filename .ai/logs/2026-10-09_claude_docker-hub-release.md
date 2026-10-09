# 2026-10-09 — Claude: Docker Hub release image

## Request

Owner (Andre): releases should publish the container on Docker Hub
automatically. Decisions via clarify: `knxbench/knxbench-server`,
`linux/amd64` + `linux/arm64` (native Arm runner), `latest` on every release
tag incl. pre-releases, Docker Hub only (no GHCR).

## Delivered

- `.github/workflows/docker-release.yml` (ADR-0097): tag `v*` → per-platform
  native build, smoke test, push by digest (SBOM + provenance), manifest list
  `<version>` + `latest`, platform verification. Dispatch = dry run.
- Trigger is the tag push, not `release: published`: the release is created
  by `linux-appimage.yml` with `GITHUB_TOKEN`, whose events never start other
  workflows.
- `apps/knx-server/scripts/smoke-test.sh` repaired (stale since ADR-0088:
  HTTP → 307). HTTPS `-k`, `KNXBENCH_IMAGE`, `KNXBENCH_SMOKE_PORT`,
  `--version`, chown-before-cleanup of root-owned `/data`.
- Docs: ADR-0097 + index, KL §168, manual (web/docker, installation,
  building-from-source, known-issues), README, IMPLEMENTATION_STATUS,
  OPEN_WORK §5.

## Evidence

- actionlint (rhysd/actionlint, incl. shellcheck): new workflow clean
  (linux-appimage.yml has two pre-existing shellcheck warnings, untouched).
- shellcheck: smoke script clean.
- Local amd64 image of `0553f43c` built; smoke test passed
  (`knx-server 0.1.0-alpha.2+g0553f43c`). Old script verified broken
  (HTTP 307 vs HTTPS 200 with password).
- xtask check-anchors/headers/ledger/layering, `tools/check_documentation.py`,
  tools unittests, `git diff --check`: green.
- Commits `69055302` (smoke fix), `cc5b1b9c` (workflow + docs), pushed.
- Dry-run dispatch run 37890172997 on `cc5b1b9c`: amd64 + arm64 (aarch64) built and smoke-tested, push/publish skipped as designed. Tag-path jq/argument assembly checked locally with sample metadata.

## Open

- Owner: Docker Hub access token (read/write) → repo secret
  `DOCKERHUB_TOKEN`; repo variable `DOCKERHUB_USERNAME` (`knxbench`).
- First tag run = first publish; `v0.1.0-alpha.5` has no image.
- Crate version drift (`knx-server` 0.1.0-alpha.2 vs tag alpha.5) disclosed,
  not fixed.

## Follow-up: release v0.1.0-alpha.6 (same day)

- Owner set `DOCKERHUB_TOKEN` (secret) and `DOCKERHUB_USERNAME=knxbench`
  (variable), then chose "release alpha.6 now".
- Found: `linux-appimage.yml` had never run on CI (alpha.5 built locally) and
  its release job would have created a full ("Latest") release. Fixed with
  `--prerelease` for tags containing `-` (`fe7e0e26`).
- Version bump `042509c6` incl. server (alpha.2) and knx-mcp (alpha.1) to end
  the version drift. Local full gate + both dry runs green on that commit.
- Tagged `042509c6` explicitly because main moved twice (Codex docs + the
  TypeNone fix) while gating; that fix is not in alpha.6.
- Release created by hand with notes right after the tag push; the workflow
  only uploaded assets. Docker publish verified from Docker Hub
  (index sha256:dfacb646…6703, amd64 + arm64, public), anonymous pull smoke
  test passed. Assets re-downloaded and checksummed.
- Pitfall for docs: SHA256SUMS lists knx-mcp too, so single-file download
  recipes need `sha256sum -c --ignore-missing`.
