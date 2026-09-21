# ADR 0030: Group-address notation is a display preference, rendered last

Date: 2026-09-21

Status: Accepted

## Context

A KNX group address is a 16-bit number. How many levels it is written in is
project data: `GroupAddressStyle` (`crates/knx-core/src/address.rs`) is
`Free`, `TwoLevel` or `ThreeLevel`, it lives in `ProjectInfo`, it is chosen
when a project is created and editable afterwards, and `GroupAddress::format`
renders it. What was never anybody's choice is the *separator*. `format`
joins with `/`, so a user who reads and writes `1.2.3` saw someone else's
punctuation on every screen. The user asked for the choice.

Two things about that request are worth writing down before the code is
read:

- The separator carries no information. `1/2/3` and `1.2.3` denote the same
  address with the same level count; converting between them is lossless in
  both directions and always will be, because there is nothing to lose.
- The dotted notation collides with individual addresses, which are written
  `area.line.device` and have no second spelling. A string of three dotted
  numbers is, on its own, ambiguous. This is a real cost, not a bug to be
  fixed later, and the user asked for the feature knowing the notation
  exists.

## Decision

The notation is a display preference. It is applied in the frontend, at the
last step before a person reads the string, and nowhere else.

`crates/knx-core` does not learn about it. `GroupAddress::format` keeps its
current behaviour, the server keeps sending canonical `/` strings in every
DTO, and the project file, the CSV export, the documentation export, the
debug report and every bus telegram are byte-identical whichever notation a
user picks. There is one canonical spelling on the wire and on disk, and it
is the one with slashes.

The transform lives in `apps/knx-web/src/gaNotation.ts` — one module, one
hook (`useGroupAddressFormat`), one preference key in the settings record
(ADR-0029). Three rules hold it together:

1. **Conversion goes through the address's levels, never through a
   `replace`.** A string that does not match the level grammar comes back
   untouched, so a device named `Kitchen / Ceiling`, a filesystem path, a
   DPT id like `DPST-1-1` and a `Free`-style address with no separator at
   all are all left alone. A mixed `1/2.3` is not half-converted; it is
   rejected.
2. **Input accepts both notations, always, whichever one is displayed.**
   A preference about reading is no reason to refuse a spelling. Anything
   unrecognised is handed to the server untouched so the validation error
   names what the user actually typed.
3. **Search and filter match both spellings, always.** A user who types
   `1/2` finds an address shown as `1.2.3`, and the reverse. This applies
   to group addresses only: an individual address has one spelling, and a
   slashed needle must not find it.

The ambiguity is answered structurally rather than typographically. The
screen already says which kind of address it is showing — a column header
(`Source` against `Destination`), a tree branch, a section heading, the
search overlay's kind groups — and that labelling is the primary cue. A
`.ga-address` class taking `--knx-accent-tertiary` is the supplementary one.
The residue is recorded honestly in
[KNOWN_LIMITATIONS.md §123](../KNOWN_LIMITATIONS.md#123-with-dots-selected-a-group-address-and-an-individual-address-are-spelled-alike).

## Alternatives considered

**Store the separator in `ProjectInfo` next to `GroupAddressStyle`.**
Rejected: it would make one user's preference travel inside another user's
project file, and every export would have to decide whether to honour it.
The style is project data because the level count changes which addresses
are representable; punctuation changes nothing.

**Have the server render the preferred notation.** Rejected: it puts a
per-viewer preference into an API that also feeds exporters, the debug
report and the bus-write parser, and it would make every DTO's meaning
depend on who asked. The server keeps one answer.

**Teach `GroupAddress::format` a separator argument.** Rejected for the
same reason, one layer lower: the domain would gain an opinion about
punctuation that only a screen has, and `check-layering` exists to keep
that direction of dependency shut.

**Accept only the selected notation on input.** Rejected: it would break
pasting an address out of a document, an email or ETS, and it is the
"quietly refuse the dotted notation somewhere" answer the ambiguity must
not be solved by.

## Consequences

Easier: any new render site is one hook call, and the preference can be
moved, defaulted or extended without touching Rust. A third notation, if
one is ever wanted, is a member of one union and one entry in one record.

Harder: the frontend is now the only place that knows a group address from
an individual one, and it knows it by call site, not by type — the DTO
field is a `string` in both cases. `formatGroupAddress` applied to a
`DeviceNode.address` would render an individual address with slashes and
make it a lie. The module comment names the fields that are group addresses
and the fields that are not, and the test file pins the search case.

Enforced by test: both notations render the same address; a `Free`-style
address, a name, a path, a DPT id and a mixed-separator string are
untouched; the preference survives a reload; input in either notation
reaches `api.writeBusValue` as `1/2/3` under either setting; search matches
across notations for group addresses and not for individual ones; and the
`InstallationNode` the server sent is byte-identical before and after the
preference changes — the closest seam the frontend has to "what is
persisted does not change".
