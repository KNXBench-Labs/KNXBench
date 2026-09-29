# 2026-09-29 — Claude: parameter fields across an octet boundary; module instances measured

## Done

- `crates/knx-core/src/commissioning/parameter_image.rs`: `write` lays out
  any 1–64-bit field at bit offset 0–7 bit by bit, MSB-first, continuing
  into the next octet. RED first: three new tests (6 bits at bit 5,
  11/16/12-bit crossings, overlap across the boundary) failed with
  `UnsupportedField`, then passed. The refused-shapes test now lists only
  what the schema excludes (bit offset > 7, zero width, > 64 bits).
- Sources (direct PDFs only): *Project Schema23* §1.1.3.17 (start of the
  field), *Configuration Procedures* §8.5.4 pp. 197–198 (bit offsets
  numbered 0…15 through a two-octet block; parameters not bound to
  octet boundaries), *Resources* §4.18.5.2.5 p. 268 (MSB-first; bit 0 of
  a U16B8's third octet is bit offset 23). No figure shows a crossing
  parameter; recorded as `[D]`, untested.
- Corpus: 1 verified, 77 untested (was 73), 168 refused. Freed exactly
  `A-008A-25-0499`, `A-008A-28-C2AA`, `A-008B-25-AF36`,
  `A-008B-28-5319` (all at `UP-290_R-839`); refusal lists diffed
  before/after the change, nothing else moved.

## Measured, not built: module instances

- `BaseOffset`, `BaseNumber`, `Allocates`, `NumericArg`: no direct PDF
  defines them (full `sources/` scan).
- Product probe (scratch only): module defaults at `argument + Offset`
  match the base image 53 % of the time where non-zero; at another
  instance's base 52 %; program-own parameters 73 %. 13 873 octets are
  claimed by two instances. The rule is not established, so a write
  could clobber another instance. Object numbers
  (`ObjNumberBase + Number`) match the GrOT 5033/5393, but objects alone
  open no program.
- Decision: module instances stay refused; the evidence that would settle
  it is a device read-back of a modular product.

## Gates

See the commit; workspace tests, clippy, fmt, xtask checks and the
release corpus run were run in the worktree before publishing.
