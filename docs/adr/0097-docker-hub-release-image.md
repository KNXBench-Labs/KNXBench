# ADR 0097: Release tags publish a multi-arch server image to Docker Hub

Date: 2026-10-09
Status: Accepted (first publication: `v0.1.0-alpha.6`, 2026-10-09)

## Context

Until now the container existed only as a recipe: `README.md` and the manual
tell users to clone the repository and run `docker build` themselves, a build
of several minutes that needs the full Rust toolchain stage. A release
(`v*` tag) published an AppImage and the `knx-mcp` binary
(`.github/workflows/linux-appimage.yml`), but no image.

The owner asked on 2026-10-09 for releases to publish the container on Docker
Hub automatically, and decided:

- repository `knxbench/knxbench-server` (the Docker Hub account `knxbench`
  exists; it had no public repositories when this was written);
- `linux/amd64` and `linux/arm64`, the latter for example for a Raspberry Pi;
- `latest` follows every release tag, pre-releases included, as long as only
  alphas exist;
- Docker Hub only, no mirror in the GitHub Container Registry.

Verified facts this decision rests on:

- Events caused by `GITHUB_TOKEN` do not start other workflows. The GitHub
  release is created by `linux-appimage.yml` with that token, so an
  `on: release: published` trigger would never fire. The tag push is the only
  event both workflows can see.
- All three base images of `apps/knx-server/Dockerfile` (`node:22-alpine`,
  `rust:1.98-slim`, `debian:bookworm-slim`) are published for amd64 and arm64.
  The dependencies need no cross-compilation setup (rustls with `ring`,
  bundled SQLite), so each platform builds natively on its own runner.
- GitHub provides `ubuntu-24.04-arm` hosted runners for public repositories;
  `KNXBench-Labs/KNXBench` is public since 2026-10-08. Building arm64 under
  QEMU would emulate the whole Rust release build instead.
- `apps/knx-server/scripts/smoke-test.sh` had silently stopped working when
  ADR-0088 made a password imply HTTPS: plain HTTP now gets `307`, so the
  script failed at its first 401 check. It was repaired (HTTPS with `-k`, a
  `KNXBENCH_IMAGE` input, root-owned `/data` handed back before cleanup) and
  passed against a locally built image on 2026-10-09.

## Decision

`.github/workflows/docker-release.yml` runs on every `v*` tag push and on
manual dispatch.

1. One build job per platform on a native runner (`ubuntu-24.04`,
   `ubuntu-24.04-arm`). It stamps `KNX_BUILD_SHA`, loads the image locally and
   runs the full smoke test against it.
2. On a tag only, the same build is repeated with identical inputs (served
   from the builder's cache, so the pushed bits are the tested bits) and
   pushed to Docker Hub by digest, with SBOM and provenance attestations.
3. A publish job joins both digests into one manifest list tagged
   `<version>` (the tag without its `v`, e.g. `0.1.0-alpha.6`) and `latest`,
   then checks that both tags resolve to exactly `linux/amd64` and
   `linux/arm64`.

Before the build starts, a tag run fails at once if the tag is not
`vMAJOR.MINOR.PATCH[-PRERELEASE]` or if the repository variable
`DOCKERHUB_USERNAME` or the secret `DOCKERHUB_TOKEN` is missing. A manual
dispatch is a dry run: both platforms are built and smoke-tested, nothing is
pushed. When both existing repository credentials are configured, login
precedes the first build so base-image pulls can authenticate too. With no
credentials, dispatch still uses anonymous pulls; tag runs still fail closed
when credentials are absent.

The alpha.7 dry run encountered HTTP429 on the Rust base-image manifest
pull twice, before any compilation. Login originally followed the smoke
build; it now precedes it when credentials exist. This separates pull
authentication from the unchanged tag-only publication gate; it does not
guarantee that all registry throttling disappears. Four named workflow
regressions cover ordering, optional credential presence, environment
privacy and dry-run publication refusal. Docker documents authentication
for pulls and distinguishes pull limits from generic abuse-rate HTTP429
responses: https://docs.docker.com/docker-hub/usage/pulls/ and
https://docs.docker.com/docker-hub/troubleshoot/.

The image itself is unchanged: same Dockerfile, same runtime contract
(`/data`, `KNX_PORT=8080`, HTTPS when a password is set).

## Alternatives considered

- **`on: release: published`** — never fires for releases created with
  `GITHUB_TOKEN` (see above); would need a personal access token in the
  AppImage workflow only to trigger this one.
- **A job inside `linux-appimage.yml`** — couples two unrelated artefacts: a
  failed AppImage smoke test would block the image and the other way round,
  and a rerun repeats both.
- **One runner, both platforms under QEMU** — simpler YAML, but emulates the
  Rust release build; slower by a large factor for no gain.
- **Docker's reusable `github-builder` workflow** — distributes platforms the
  same way, but hides the build steps, so the smoke test cannot run between
  build and push.
- **`latest` only for stable releases** — offered to the owner and declined
  while every release is a pre-release; revisit at the first stable version.
- **GHCR mirror** — offered, declined.

## Consequences

- Users can `docker pull knxbench/knxbench-server` instead of building, from
  `v0.1.0-alpha.6` on. `v0.1.0-alpha.5` predates the workflow and has no image.
- `latest` moves with every alpha. Users who want to stay put pin a version
  tag. When the first stable release comes, this rule needs a deliberate
  decision, not a silent change.
- The owner has to create a Docker Hub access token (read and write on
  `knxbench/knxbench-server`) and store it as the repository secret
  `DOCKERHUB_TOKEN`, plus the variable `DOCKERHUB_USERNAME`. Without them the
  tag run fails before building; the AppImage release is unaffected.
- `knx-server --version` reports the server crate version. It had drifted
  from the release tag (`0.1.0-alpha.2` against `v0.1.0-alpha.5`); the alpha.6
  release bumped the server, CLI, desktop, web and `knx-mcp` together, so the
  first published image reports `knx-server 0.1.0-alpha.6+g042509c`. Keeping
  them aligned is part of release preparation (`xtask check-appimage` only
  checks desktop against web).
- Tested so far: workflow syntax (actionlint with shellcheck), the smoke test
  against a local amd64 build, and a dry-run dispatch
  ([run 37890172997](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37890172997), `cc5b1b9c`) that built and smoke-tested
  `linux/amd64` and `linux/arm64` (`aarch64` runner) in under four minutes
  each. The first tag run ([run 37893327218](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37893327218), `v0.1.0-alpha.6`)
  pushed, merged and verified both platforms; an anonymous pull of the
  published amd64 image passed the smoke test.
- Docker Hub description sync is separate from image releases (owner request,
  2026-10-09): `apps/knx-server/DOCKERHUB.md` is the maintained overview;
  `.github/workflows/dockerhub-description.yml` publishes it plus the short
  description on scoped pushes to `main` or manual dispatch, then verifies
  anonymous API readback against the exact source. It never builds images,
  moves tags or replaces containers. Root README relative assets and
  source-build instructions are deliberately not copied verbatim.
- The description action is pinned to `peter-evans/dockerhub-description`
  v5.0.0 (`1b9a80c056b620d92cedb9d9b5a223409c68ddfa`, Node 24).
  Its [documented contract](https://github.com/peter-evans/dockerhub-description/blob/1b9a80c056b620d92cedb9d9b5a223409c68ddfa/README.md)
  requires repository administration and a personal access token with
  read/write/delete scope. An image-push-only token may be insufficient;
  optional secret `DOCKERHUB_DESCRIPTION_TOKEN` overrides `DOCKERHUB_TOKEN`
  for this job only. Credentials are never written into the description.
  The limits are 100 bytes for the short description and 25,000 bytes for
  the overview; publication preflight refuses oversized input.
- Not done: image signing (cosign), scheduled rebuilds for base-image
  security updates. Each would be its own decision.
