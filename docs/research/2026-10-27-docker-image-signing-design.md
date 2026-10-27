# Docker image signing — bounded design proposal

Date: 2026-10-27. Status: **proposed, not implemented or activated**.
Scope: the Docker-signing design requested in [OWN-01](../open-work/OWN-01-owner-decisions.md).
No signing operation, key generation, external login or image publication was performed.
This proposal changes neither Alpha 17 nor the current release workflow.

## Recommendation and present baseline

Recommend keyless Cosign signing of the **final multi-architecture image-index
digest**, using the existing GitHub Actions release workflow's identity. This
is a proposed project policy, not an owner-approved activation or a claim that
Docker Hub interoperability has been tested.

At source baseline `6c9bded56`,
[the workflow](../../.github/workflows/docker-release.yml) builds and smokes
`linux/amd64` and `linux/arm64`, pushes platform outputs by digest with SBOM and
BuildKit provenance, then creates version and `latest` tags together. The publish
job has no signing step or OIDC permission. Manual dispatch does not publish.
[ADR-0097](../adr/0097-docker-hub-release-image.md) remains the current policy;
BuildKit provenance is not a Cosign identity-verification result.

The proposal avoids a new long-lived project signing secret. Sigstore documents
short-lived certificates and in-memory ephemeral signing keys, bound to OIDC
identity; it does not remove reliance on the identity provider, certificate
issuer or transparency infrastructure. [3] Existing Docker Hub credentials are
still needed to write registry objects; GitHub's `id-token: write` only permits
requesting an OIDC token, not registry writes. [4]

## Identity and trust boundary

Proposed accepted identity for each release:

- Issuer: `https://token.actions.githubusercontent.com`.
- Certificate identity:
  `https://github.com/KNXBench-Labs/KNXBench/.github/workflows/docker-release.yml@refs/tags/<release-tag>`.
- `<release-tag>` is the independently selected exact Git tag, including `v`;
  record its tag-object identity and resolved source commit before the run. Verify
  the certificate's GitHub workflow SHA against that exact source commit; refuse
  an absent/mismatching claim, including another commit under the same tag name.
  Fulcio documents the deprecated legacy SHA extension and its generic successor;
  the first implementation must pin and test its accepted representation, not fall
  back between unverified fields. [9] [13] [14] Do not infer trusted inputs from an image
  tag, untrusted verification output or the current `latest` value.
- Scope: only the existing tag-push publication route. Manual dispatch, pull
  requests, forks and branch builds must not request signing authority or sign.
- Keep the signing job in this workflow for the first implementation. A reusable
  workflow, rename or repository transfer requires a reviewed identity-policy
  change and new certificate evidence; never widen to an unrestricted regex.

Sigstore's CI example uses workflow/ref certificate identity, the Actions OIDC
issuer and job-level `id-token: write`. [2] These are the basis for the proposed
exact KNXBench identity above; its actual certificate remains an acceptance
requirement, not an observed result. Give OIDC permission only to the signing
job, and keep existing credential scopes separate. Require owner-reviewed tag
protection and release authorization before activation. GitHub recommends
restricted trust conditions and, when using environments, deployment protection
rules. [4] This proposal does not create or change those settings.

A valid signature means the configured identity signed those bytes. It does not
prove benign code, reproducibility, independently tested builds, KNX compatibility
or protection from a compromised authorized workflow. Maintain the local release
gates, architecture smokes, review and public digest readback independently.

## Trusted release record

Proposed authority: an owner-approved, independently reviewed commit containing
`docs/evidence/release-<version>-signing.json`, following the existing release
record convention. Its exact commit hash must be supplied through the owner's
independently authenticated release approval, not discovered from `latest`, a
signer's artifact or a moving branch. Only the owner/authorised release maintainer
may approve it; the image job's read-only repository token cannot author approval.
Record the source commit, tag object/ref, approved index/platform digests,
workflow identity, monotonic promotion sequence and prior accepted record. Build
outputs are proposals until this digest-bound record is approved; promotion waits
for that approval. A changed candidate requires new approval.

Proposed approval transport: an owner-configured, per-release protected Actions
environment `docker-release-<release-tag>` supplies `SIGNING_POLICY_RECORD_SHA`.
A separate approval-only job waits there and exports that exact configured
record revision only after owner approval. The promotion job depends on its
successful result; only then may it enter the shared promotion concurrency group.
After entering that group, refuse an expired owner-approved `not_after` deadline
before any tag write. Pending approval can outlive that deadline but cannot
then authorize promotion; the owner can cancel it without holding the shared
promotion slot. Missing/malformed approval configuration refuses promotion,
never falls back to finding a record on `main`. Environment
provisioning and the exact runtime delivery of this owner-managed variable must
be separately approved and demonstrated before activation; this is not an input
silently supplied by the signer or an already configured environment.

Consumers and the promotion job read that exact record revision and reject
mismatch. A withdrawal advisory is an owner-approved successor in the same record
chain; consumers must obtain its current owner-authenticated head to check for
withdrawal, not merely replay an older valid record. Record approval and this
bootstrap/update procedure need separate acceptance before activation. An
owner-account or approval-channel compromise requires out-of-band re-establishment
of trust; the signer's own verification output cannot repair it. This defines
proposed authority, not already configured repository/environment protection.

## Subject, registry and verification

1. Sign `docker.io/knxbench/knxbench-server@sha256:<index-digest>`, never a mutable
   tag. Retain the raw index and its exact digest, plus the two runnable platform
   descriptors and their source build receipts. Attestation descriptors must not
   be mistaken for additional runnable architectures.
2. Verify the index signature and exact issuer/identity, with normal digest-claim,
   certificate and transparency checks enabled. Cosign checks the signed image
   digest by default; disabling claim checks would remove that binding. [1]
3. Check each runnable descriptor against its accepted platform output. This
   design signs the index, not each child independently. Direct pulls or separate
   verification of a child manifest are outside this first signing policy.
4. Current Sigstore documentation describes OCI 1.1 referrer storage, by default
   in the image repository, and lists Docker Hub as supported. [8] That is vendor
   documentation, not evidence for this account, token, retention behaviour or
   chosen client version. Registry upload, discovery and anonymous verification
   require a separately authorised disposable integration test before activation.
5. Preserve the signature bundle/material, index bytes, verification output,
   resolved tool versions and trust-root provenance in the release evidence.
   Establish an independently authenticated trust-root/bootstrap procedure;
   never trust an arbitrary root supplied alongside an image. Offline validation
   and retention/restore are future acceptance cells, not promised functionality.

Illustrative **read-only verifier shape**, not an executed acceptance recipe:

```sh
# Trusted release record supplies RELEASE_TAG, SOURCE_COMMIT and INDEX_DIGEST.
cosign verify "docker.io/knxbench/knxbench-server@${INDEX_DIGEST}" \
  --certificate-identity "https://github.com/KNXBench-Labs/KNXBench/.github/workflows/docker-release.yml@refs/tags/${RELEASE_TAG}" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  --certificate-github-workflow-sha "${SOURCE_COMMIT}" \
  --certificate-github-workflow-trigger push
```

The implementation must validate input syntax, install a reviewed pinned Cosign
release/action by immutable identity, and test the flags and trust-root behaviour
of that exact version. Documentation examples are not a version lock. Ordinary
`docker pull`/Compose deployment is not this verification step; describe verified
pull-by-digest explicitly rather than implying automatic enforcement.

## Publication sequence and failure handling

Proposed order, to be implemented only after separate Go:

1. Keep native builds, local smokes and digest handoff. Construct a candidate
   index under a unique staging tag tied to run ID and attempt; record its digest.
   Apply all index annotations here, never again during promotion. The staging
   tag and artifacts are public registry writes, so even this test needs explicit
   publication approval. Mark the staging tag as a non-release; retain it until
   an owner-approved cleanup/retention rule exists. Deletion needs separately
   verified authority and must preserve retained release evidence. It must not
   move a release or `latest`.
2. Validate architecture membership and build receipts, sign the captured index
   digest, then perform fresh anonymous verification in an isolated client.
   Wait for the owner's digest-bound release-record approval before promotion.
3. Promote that exact index to the version tag with a single-source copy and no
   added annotations, then read back equality. Promote `latest` under the
   applicable channel policy, and read back equality again. Docker documents
   copying an existing index from a single source; still reject any changed
   digest rather than assuming byte identity. [6]
4. Serialize signing/promotion jobs across tags with the proposed fixed job
   concurrency group `knxbench-docker-promotion`, `cancel-in-progress: false` and
   `queue: max`. The current per-ref group is insufficient. GitHub documents
   pending-run replacement under the default queue, a bounded extended queue and
   no guarantee of release ordering. [11] Refuse an older promotion sequence
   against the owner-approved record chain, never just against `latest`.
   A canceled/overflowed job is incomplete, not success: the maintainer must
   reconcile its GitHub terminal status and last registry state before retry.
   Validate the queue syntax on the actual workflow before activation. Preserve
   the current alpha channel rule; first-stable policy remains separate.
5. Advertise signing success only after all readbacks and verification pass.
   OCI registry updates are not assumed atomic: retain the precise last completed
   step when a version tag exists but alias promotion fails. A partially published
   release is not success and must not be silently rebuilt or retagged.

| Failure | Required disposition |
| --- | --- |
| OIDC, issuer, transparency or registry signing unavailable | Stop before promotion; no unsigned fallback or disabled verification flags. |
| Wrong identity, issuer, digest, platform set or missing signature | Reject; retain original diagnostics and candidate identity. |
| Failure after a tag moved | Record partial publication and actual tag digests; no blind whole-workflow rerun. Reconcile before an authorised retry. |
| Signature/referrer lost, image unchanged | Recover only authenticated retained material for that same digest and reverify; no rebuild substituted as recovery. Sigstore documents later bundle attachment, but the pinned-client restore path must first pass the integration test. [7] |
| Authorized workflow/account compromised | Stop publication, restrict the compromised authority, publish a digest/commit advisory and select a previously independently accepted digest. |

Recovery never requires backing up an ephemeral private key. Expiring a
certificate does **not** revoke an already accepted historical signature;
Sigstore verifies signing-time validity using transparency evidence. [3]
Therefore maintain a separately authenticated withdrawn-digest/release advisory
and require consumers to consult it. Restoration to a previously accepted digest
is owner-authorised deployment work, not an action this design performs. Do not
rewrite a withdrawn version tag, delete transparency history, or treat successful
cryptographic verification as proof a compromised release is safe.

## Finite implementation acceptance, still pending

- Owner approves the identity, public certificate/log metadata, service terms and
  activation scope. Even test signatures leave public transparency evidence;
  a disposable registry scope does not make that history disposable. [3]
  No new account, persistent key, sign request or public write is authorised here.
- Pin and inspect the chosen tool/actions and trust-root bootstrap. Exercise
  positive verification and negatives for another workflow/ref/issuer, another
  source commit under the same tag name, an absent commit claim, another digest,
  an unsigned subject, missing/corrupt material and unavailable services. Verify
  owner-approved record authority, exact revision/digest binding, withdrawal
  freshness and failure of signer-authored/self-approved records.
- In an explicitly authorised disposable registry scope, prove OCI referrer
  upload/discovery, anonymous verification, retained-material recovery, and the
  signed index's exact amd64/arm64 membership using the actual token/client.
- Prove branch, fork, pull-request and manual-dispatch routes cannot sign or
  promote; verify OIDC permissions are absent from unrelated jobs.
- Inject failure at signing and each promotion boundary. Prove no unsigned alias
  advance, exact partial-state reporting, source-bound retry and serialized
  older/newer release handling; never weaken the existing dry-run boundary.
- Only after those checks: independent review, maintained integrated gate and
  a new operation-specific release/publication Go. Existing images remain as
  published; no retroactive Alpha 17 signing is included.

**Delivered here:** the design and acceptance checklist only. All operational
cells above remain unexecuted. [OWN-01](../open-work/OWN-01-owner-decisions.md)
remains open for activation decisions and unrelated evidence; no ledger status
or container trust claim changes.

## Sources

Primary documentation retrieved directly and inspected on 27 October 2026.
The retained source receipts record URLs, response metadata and body digests;
these pages are documentation evidence, not executions of the proposed workflow.

[1] https://docs.sigstore.dev/cosign/verifying/verify — verification identity and digest claims.
[2] https://docs.sigstore.dev/quickstart/quickstart-ci — Actions identity, issuer and OIDC permission.
[3] https://docs.sigstore.dev/about/security — ephemeral keys, signing-time validity and trust limits.
[4] https://docs.github.com/en/actions/how-tos/secure-your-work/security-harden-deployments/oidc-in-cloud-providers — OIDC permission and constrained trust.
[6] https://docs.docker.com/reference/cli/docker/buildx/imagetools/create — single-index copy and result metadata.
[7] https://docs.sigstore.dev/cosign/signing/signing_with_containers — retained bundles and later attachment.
[8] https://docs.sigstore.dev/cosign/system_config/registry_support — referrer storage and registry support.
[9] https://docs.sigstore.dev/certificate_authority/oidc-in-fulcio — GitHub token claims and certificate extraction.
[11] https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency — bounded queue and scheduling/cancellation behaviour.
[13] https://raw.githubusercontent.com/sigstore/fulcio/62a73e3fc56696ad7fe8ef9d49b8c82dd2689d59/docs/oid-info.md — commit-bound extensions and deprecated legacy SHA field; inspected commit `62a73e3fc56696ad7fe8ef9d49b8c82dd2689d59`.
[14] https://raw.githubusercontent.com/sigstore/cosign/ef56e6999cdad185e5e155cccf1223951abfc77a/doc/cosign_verify.md — workflow SHA/trigger verification flags; inspected source commit, not a selected or executed client release.
