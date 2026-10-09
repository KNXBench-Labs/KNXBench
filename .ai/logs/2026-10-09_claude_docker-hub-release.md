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
