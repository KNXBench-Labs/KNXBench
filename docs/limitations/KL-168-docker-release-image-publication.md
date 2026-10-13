# Docker release-image publication

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-168)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 168. The Docker Hub image follows release tags only, and `latest` follows alphas

**Limitation.** `.github/workflows/docker-release.yml`
([ADR-0097](../adr/0097-docker-hub-release-image.md)) publishes
`knxbench/knxbench-server` for `linux/amd64` and `linux/arm64` when a `v*` tag
is pushed. Nothing is published for commits between tags, and
`v0.1.0-alpha.5` predates the workflow, so it has no image. `latest` moves with
every release tag, pre-releases included. The image is not signed; it carries
BuildKit SBOM and provenance attestations only. Base-image security updates
arrive only with the next tag, not through scheduled rebuilds. The OCI
`org.opencontainers.image.version` label carries the git tag with its `v`
(`v0.1.0-alpha.6`), while the image tag drops it (`0.1.0-alpha.6`).

**Why.** Owner decisions of 2026-10-09: Docker Hub only, both architectures,
`latest` for every release while only alphas exist. The earlier drift between
the server crate version and the release tag ended with the alpha.6 bump
(`042509c6`): server, CLI, desktop, web and `knx-mcp` all report the tag.

**Impact.** Pin a version tag (`knxbench/knxbench-server:0.1.0-alpha.6`) if an
update must be a deliberate step. Newer source than the last tag still needs
a local `docker build`. `v0.1.0-alpha.6` was the first publication
([run 37893327218](https://github.com/KNXBench-Labs/KNXBench/actions/runs/37893327218), 2026-10-09): both tags resolve to one index
with `linux/amd64` and `linux/arm64`, the repository is public, and an
anonymous pull of the amd64 image passed the smoke test. The arm64 image was
smoke-tested on its build runner only, not on a Raspberry Pi.
<!-- END retained catalogue record -->
