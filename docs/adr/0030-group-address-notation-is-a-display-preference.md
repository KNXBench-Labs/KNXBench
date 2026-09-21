# ADR 0030: Group addresses use fixed slash notation

Date: 2026-09-21
Status: Accepted (amended after the original selectable-notation decision)

## Context

A group address is a 16-bit identifier. *03_05_01 Resources v01.10.01 AS*,
§3.4.1–§3.4.2 defines that identifier and its installation-wide uniqueness.
*03_05_03 Configuration Procedures v02.01.01 AS*, §3.7.5.4.3 explains that
main- and middle-group structure is optical organisation and does not change
the address itself. KNX documents consistently spell structured examples with
slashes, including `1/1/50` in *08_TSSH System Conformance Testing –
KNXnet/IP_1_3_AS* and `02/0/001` in *08_03_03 NL v01.01.04 AS*.

The first version of this ADR treated slash versus dot as a selectable display
preference. That created an avoidable ambiguity with individual addresses,
which already use `area.line.device`, and a setting for a value that carries no
project information. The user subsequently required the specification-style
slash form everywhere and no notation selector.

## Decision

KNXBench displays structured group addresses with `/` everywhere. There is no
group-address notation setting and no stored preference. `knx-core`, APIs,
project storage, CSV, reports and bus operations continue using the same slash
form, so the UI does not introduce a second representation.

Dotted two- and three-level text remains accepted at explicit group-address
input and search boundaries. `canonicalGroupAddress` converts it to slash form
before an API call; `groupAddressSpellings` lets search find either spelling.
This compatibility does not affect rendering. Free-style numeric addresses,
mixed separators, paths, names and DPT identifiers remain untouched.

The conversion stays in `apps/knx-web/src/gaNotation.ts`. Call sites remain the
type boundary: the same dotted grammar could also be an individual address, so
the helper must only receive fields already known to contain group addresses.

## Consequences

- Group and individual addresses are visually unambiguous again.
- Settings no longer carry a stale `groupAddressNotation` choice; an old value
  is harmless because no code reads it.
- Pasted dotted group addresses continue to work and are immediately
  canonicalised.
- There is one display form to test and document. The old limitation about
  both address kinds looking identical is resolved.

## Alternatives considered

**Keep the selector but default to slash.** Rejected: it preserves the
ambiguity and configuration surface the fixed-format decision removes.

**Reject dotted input.** Rejected: accepting it at a typed group-address
boundary is lossless and useful for existing notes or copied values; it does
not weaken the fixed display rule.

**Move formatting into the server or domain model.** Rejected: the domain
already emits slash notation. Input compatibility and search aliases are UI
boundary concerns and do not justify another wire or persistence format.
