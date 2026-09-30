# ADR 0053: Contributions come with a license grant, so KNXBench can be dual-licensed

Date: 2026-09-30
Status: Accepted
Session: licensing (user request, outside the numbered sessions)

## Context

The maintainer wants commercial **use** of KNXBench allowed, but no one selling a
product built on it without the maintainer's consent.

Verified facts:

- The Open Source Definition forbids that restriction outright. Criterion 1
  ("Free Redistribution") says the license shall not restrict any party from
  selling the software, and criterion 6 ("No Discrimination Against Fields of
  Endeavor") forbids excluding business use. Licenses that forbid selling
  (PolyForm Noncommercial, PolyForm Shield, Commons Clause, BUSL-1.1, FSL-1.1)
  are source-available, not open source.
- KNXBench is `AGPL-3.0-or-later` (`Cargo.toml` `license`, inherited by every
  crate). AGPL-3.0 §13 extends the copyleft to users reached over a network,
  which covers `knx-server`. A vendor may sell a modified KNXBench, but must
  hand every recipient the complete source under the AGPL. A closed product
  based on KNXBench is therefore not possible under the public license.
- A proprietary license can only be granted by whoever holds the rights to all
  of the code. On 2026-09-30, all 1,706 commits on `origin/main` carry the
  maintainer's address `github@knxbench.com`. No outside contribution has been
  merged yet.
- Until this ADR, `docs/manual/development/01-contributing.md` stated that
  there is "no separate contributor license agreement and no copyright
  assignment". That was true, and it made dual licensing impossible once the
  first outside contribution was merged.

Assumed and not verified: that `CLA.md` is legally effective in the form
written. No lawyer has reviewed it.

## Decision

KNXBench stays `AGPL-3.0-or-later` and remains open source. In addition, it is
kept **dual-licensable**: the maintainer can grant commercial licenses to
vendors who want to build a proprietary product.

For that, every outside contribution needs the Individual Contributor License
Agreement in [`CLA.md`](../../CLA.md). It grants the maintainer a
non-exclusive, perpetual, sublicensable license, including the right to
license proprietary terms, and a patent license. Copyright is **not**
transferred. In return, the maintainer promises that each contribution stays
available under the AGPL or another OSI-approved copyleft license. If he breaks
that promise, the grant falls back to the rights the AGPL gives everyone.

Contributors agree by a fixed sentence in the pull request and a checkbox in
`.github/pull_request_template.md`. Organizations need a separate signed
agreement. Contributions without the sentence are not merged.

## Alternatives considered

- **Source-available license (BUSL-1.1, FSL-1.1, PolyForm Shield).** These
  block selling directly, but KNXBench would stop being open source, would not
  be accepted by distributions' main repositories, and would deter
  contributors. The user chose to stay open source.
- **AGPL without a CLA.** Simplest, and it already prevents a closed fork. But
  the first merged outside contribution would make commercial licensing
  impossible without asking every contributor again.
- **Copyright assignment.** Stronger for the maintainer, but under German law
  copyright cannot be transferred as a whole (§ 29 UrhG); only rights of use
  can be granted. It also deters contributors more. A license grant achieves
  the same thing.
- **Developer Certificate of Origin only (`Signed-off-by`).** Records origin
  but grants no right beyond the AGPL, so it does not enable dual licensing.
- **Harmony or Fiduciary License Agreement templates.** Reasonable bases, but
  their option matrix needs a lawyer to fill in as well. A short
  project-specific text with an explicit fallback clause is easier to read and
  review.

## Consequences

- KNXBench can still grant a commercial license to anyone, because no outside
  rights exist yet. Every future pull request must carry the CLA sentence, and
  review must check it before merging. There is no CLA bot. This is a manual
  review step.
- Commercial use without a commercial license stays allowed under the AGPL,
  including selling copies, as long as the full source goes with them. The
  commercial license is an offer, not a requirement for AGPL-compliant
  selling. `README.md`, the FAQ and the contributing guide say so.
- The maintainer is bound by the promise in `CLA.md` section 4. Relicensing a
  future version to a source-available license would break it for every
  version that contains outside contributions.
- Open: legal review of `CLA.md` (governing law, liability wording, whether
  checkbox acceptance is sufficient), a template for an organization-level
  agreement, and the terms of the commercial license itself. Recorded in
  `docs/KNOWN_LIMITATIONS.md`.
