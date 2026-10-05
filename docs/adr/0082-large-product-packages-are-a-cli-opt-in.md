# ADR 0082: Large product packages are an explicit command-line opt-in

Date: 2026-10-05
Status: Accepted
Session: Alpha release (AR06P, KL-151)

## Context

`install_package` refuses a product ZIP whose decoded member exceeds 64 MiB or
whose decoded members sum to more than 256 MiB. Fifteen of 853 public
manufacturer downloads (crawler run of 2026-10-03, KNOWN_LIMITATIONS §151) hit
these bounds: complete bundles of 296 MiB to 2.7 GiB expanded, including
Siemens' only current download, and single-device packages whose application
XML is 65–139 MiB. The raw file bound (256 MiB) refuses none of them.

The bounds protect against decompression bombs and unbounded memory/time. They
are not wrong; they are too small for real bundles. Measured on 2026-10-05 with
a release build of this change, each package in a fresh database
(`docs/PRODUCT_ZIP_LARGE_PROFILE.md`):

- standard profile: all 15 refused atomically with a typed size limit;
- large profile (256 MiB per member, 4 GiB total): 14 installed and pass
  `knx products verify`; one is still refused, by its unsupported scheme-10
  namespace (§153);
- worst single package: 760.5 MiB peak RSS, 256 s, 7.2 GiB database growth.
  The Siemens bundle: 402 MiB, 91 s, 2.5 GiB;
- 20 standard-sized control packages give identical results and summaries
  under both profiles.

The HTTP catalog install runs synchronously while it holds the product
database mutex. Four minutes of a blocked server is not acceptable as a
default, and the HTTP body is already bounded at 256 MiB.

## Decision

`knx_productdb::PackageLimits` names two expansion profiles. `STANDARD`
(64 MiB / 256 MiB) stays the default for every caller, including
`install_package` and the HTTP catalog route. `LARGE` (256 MiB / 4 GiB) is
reachable only through `install_package_with_limits` and, for users, only
through the explicit `knx products ingest --allow-large-package` flag. Every
other bound is the same in both profiles: the 256 MiB raw input, 4,096 members,
ZIP64 refusal, path/metadata budgets, XML evidence budgets, namespace and
grammar gates, atomic rollback and byte retention. Without the flag, a size
refusal on the command line names the flag and its cost.

## Alternatives considered

- **Raise the default.** Rejected: the HTTP route would block the server for
  minutes and every caller would accept a 16× larger decompression budget
  without asking for it.
- **Streaming ingest.** Removes the per-member memory peak, but the measured
  peak (760 MiB) is acceptable for an explicit command-line action, and
  streaming touches every parser. Kept as the route for a future HTTP opt-in.
- **Remove the limits.** Rejected: an unbounded ZIP is a denial-of-service
  input.
- **A per-manufacturer allow-list.** Rejected: manufacturer data must not be
  hard-coded, and size is the actual risk, not the vendor.

## Consequences

- Siemens' complete download and the large ABB packages are installable from
  the command line; the web catalog still refuses them, with the standard size
  message.
- Tests pin both profiles exactly (declared-size boundaries at, and one over,
  each bound; a real 65 MiB member installs only under `LARGE`; the CLI refuses,
  hints, installs with the flag and refuses the flag for project files).
- Any later HTTP opt-in needs its own decision on blocking, progress and
  cancellation; this ADR does not grant it.
