# ADR 0002: Own `.knxproj` parser; `xknxproject` as a test oracle only

Date: 2026-09-02
Status: Accepted
Session: 1

## Context

Session 0 compared a full `xknxproject` parse against the raw XML of the same
project and measured what the library does not carry through (RESEARCH §7.1):
all 1390 `ParameterInstanceRef` values, the one device that is not assigned to
a line, `BusAccess`, `BinaryData`, `CompletionStatus`, the five `*Loaded`
flags, `Broken`, `BCUKey`, `SplitType`, `Central` and `Unfiltered`,
`DefaultLine`, and the send/receive direction of group object links. The
library also has no export path at all — it reads, and that is the whole of it.

It is licensed GPL-2.0-only.

## Decision

We write our own `.knxproj` reader and writer in `knx-etsproj`.

`xknxproject` stays where Session 0 left it, in `.venv`, and is used as a
cross-check oracle: a development and test aid that answers "does our reader
agree with an independent implementation on the subset that implementation
covers". It is never invoked by the Rust build, never shipped, and never a
runtime dependency.

## Alternatives considered

**Depend on `xknxproject`.** Rejected on two independent grounds, either of
which is sufficient: it forces the entire application to GPL-2.0, and it still
cannot export, which is half of what this application is for.

**Port `xknxproject` to Rust.** Rejected because a port inherits the data
losses listed above — they are design decisions of that library, not bugs to
fix in translation — and because a line-by-line port of a GPL-2.0 work raises a
derivative-work question we have no reason to invite.

## Consequences

More work up front: schema detection, tolerant XML parsing and the mapping
layer are all ours.

The separation is enforced mechanically rather than by discipline. `cargo-deny`
rejects GPL licences in the runtime graph, and beyond that, no Rust crate can
reach a Python package in a virtual environment at all.

Test scripts that call `xknxproject` are allowed and useful, and must stay
clearly separated from anything the build produces.
