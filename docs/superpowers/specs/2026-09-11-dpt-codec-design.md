# Design — T29: a DPT codec and DPT-aware bus CLI (gap E4)

Date: 2026-09-11
Gap: **E4** (`docs/GAP_ANALYSIS_ETS.md`) — "No DPT-aware bus tooling.
`bus monitor`/`bus write` operate on raw `GroupValue::Short`/`Bytes` — no
decoding/encoding against a comm object's actual DPT."
Related, deliberately **not** closed here: **D5** (no Group Monitor GUI)
and the display half of E4, which belong to **T15**.

## 1. The problem, stated from the code

`crates/knx-core/src/dpt.rs` is 116 lines and contains exactly three
things: `DptRef`, `DptRef::parse`, `DptParseError`. There is no encoder
and no decoder anywhere in the workspace. Consequently:

- `apps/knx-cli/src/main.rs:1894` `format_service` prints
  `GroupValueWrite 0x01 (6-bit)` or `GroupValueWrite [2a, 99]`. A user
  watching a bus has to know that `0x01` on a DPT-1 object means "on" and
  that `[0c, 1a]` on a DPT-9 object is 21.0 °C.
- `apps/knx-cli/src/main.rs:1533` `parse_group_value` accepts `0`, `1`,
  or a hex byte string, and says so in its own doc comment: "no DPT
  interpretation (same scope cut as the read-only monitor's decode
  side)". Writing 21.5 °C to a thermostat requires the user to encode
  the 2-octet float by hand.

Everything needed to fix this on the domain side already exists:
`ComObjectInstance.dpt: Override<DptRef>` (`crates/knx-core/src/device.rs`),
`ComObjectInstance.links: Vec<GroupLink>` (`crates/knx-core/src/flags.rs`),
and `Project::devices.com_objects()` (`crates/knx-core/src/devices.rs`).

## 2. Scope

**In scope.** A pure encode/decode codec in `knx-core`; a group-address →
DPT resolution query in `knx-core`; wiring both into `knx bus monitor`
(decode) and `knx bus write` (encode); documentation reconciliation.

**Out of scope**, each for a stated reason:

- Any GUI. That is T15, and it will consume this codec rather than
  duplicate it.
- The `knx_master.xml` DPT catalogue (46 main types, 289 subtypes —
  RESEARCH §5). Subtype *units* and *enumeration value names* live there
  and in the Standard; ingesting the catalogue is its own task. The codec
  therefore decodes values, not vocabularies.
- Main types with no encoding implemented in this slice — see §4.3.
- Persisting a resolved DPT anywhere. RESEARCH §6.1 rule 1 is explicit:
  an inferred DPT must be stored *as inferred*, never written back as if
  the user had set it. This slice stores nothing, so the rule is
  satisfied by construction.
- `GroupAddress/@DatapointType` (present at schema ≥ 21 — RESEARCH §3.4,
  and measured here: 13/13 group addresses in `KV v2.5 - demo.knxproj`
  carry it, 0/514 in either `Unser Zuhause` project). It is preserved
  today as a retained attribute on `SourceGroupAddress::other`, but it is
  not in the domain model and adding it means a `GroupAddressEntry`
  field, a `knx-store` schema migration, import mapping and export
  round-trip work. That is a separate slice; §5.3 states the consequence
  honestly instead of pretending the gap is closed.

## 3. Decisions

Numbered **E4-D1 … E4-D9** to stay unambiguous against T18's `D1…D19`,
which live in a different design thread.

### E4-D1 — The codec lives in `knx-core`

DPTs are a KNX domain concept; CLAUDE.md lists "Datapoint Types (DPT)"
among the things the core model must represent, and `DptRef` is already
there. `knx-core` has no dependencies that a codec would violate, and
`xtask check-layering` keeps it that way.

`crates/knx-core/src/dpt.rs` becomes a module directory:

```
crates/knx-core/src/dpt/
    mod.rs        // DptRef, DptParseError, re-exports (today's file, minus nothing)
    codec.rs      // encode/decode + DptValue + DptCodecError
    resolve.rs    // group-address → DPT resolution over a Project
```

### E4-D2 — `GroupValue` moves down into `knx-core`

The codec has to speak the same payload type the bus layer speaks, and
that type already exists: `knx_net::cemi::GroupValue`, a two-case enum
(`Short(u8)` for a value carried in the APCI octet's low six bits,
`Bytes(Vec<u8>)` for one or more separate octets). Defining a
structurally identical `DptPayload` in `knx-core` would mean two names for
one thing plus conversions in both directions, forever.

So `GroupValue` moves to `knx_core::dpt`, and `knx-net` re-exports it
(`pub use knx_core::GroupValue;`) so every existing call site —
`cemi.rs`, `client.rs`, `tests/live_gateway.rs`, the CLI — compiles
unchanged. A group value is the payload of a group telegram: a domain
concept, not an IP-transport one. `knx-net` already depends on
`knx-core`, so the direction is right.

This is the only structural change to existing code in this slice.

### E4-D3 — A latent APCI-corruption bug gets fixed here, because this slice is what makes it reachable

`crates/knx-net/src/cemi.rs:238` computes
`apci_lo = ((short_apci & 0x03) << 6) | inline6`. A `GroupValue::Short`
whose value exceeds `0x3F` therefore overwrites the two APCI bits sitting
in that octet's top two bits: `Short(0x40)` on a `GroupValueWrite` is
transmitted as a *different application service*. Nothing produces such a
value today, because the only producer is a human typing `0` or `1`.
This slice makes the codec a producer, so the invariant stops being
theoretical.

`encode_group_value` will promote an out-of-range `Short(v)` to
`Bytes([v])` — the value survives, the APCI is not corrupted, and the
result is what the Standard requires for a value that does not fit the
six inline bits. A regression test pins it.

### E4-D4 — `DptValue` is wire-shaped, not unit-shaped

Decoding yields a `DptValue` whose variants mirror the Standard's own
format families, not per-subtype semantic types:

```rust
pub enum DptValue {
    Bool(bool),                                   // B1      — main 1
    ControlBool { control: bool, value: bool },   // B2      — main 2
    Step { increase: bool, step_code: u8 },       // B1U3    — main 3
    Unsigned(u32),                                // U8/U16/U32
    Signed(i32),                                  // V8/V16/V32
    Float(f64),                                   // F16/F32, and scaled U8
    Text(String),                                 // A[14]
    Scene { number: u8 },                         // main 17
    SceneControl { learn: bool, number: u8 },     // main 18
}
```

`f64` carries both `F16` (2-octet float) and `F32` (4-octet IEEE 754)
because the codec's job is the number, not its storage width; `F32`
decode widens exactly, and `F32` encode round-trips through `f32`.

A `DptValue` is always accompanied by the `DptRef` it was decoded under
when it is displayed, so a subtype whose *meaning* the codec does not
model (1.008 up/down rendered as `on`/`off`) is never mistaken for a
claim about that meaning.

### E4-D5 — Everything the codec cannot do is an error value, never a guess

`decode` and `encode` return `Result<_, DptCodecError>` with distinct,
matchable variants, at minimum:

- `UnsupportedDpt(DptRef)` — a main type this slice does not implement.
- `WrongLength { dpt, expected_bits, got }` — payload size disagrees with
  the DPT.
- `OutOfRange { dpt, value }` — an engineering value the DPT cannot
  represent.
- `Unparsable { dpt, input }` — the user's text is not a value of this
  DPT.
- `InvalidData { dpt }` — the payload carries the DPT's own documented
  "invalid data" code, where the Standard defines one.

No variant means "I guessed". The CLI prints the raw payload alongside
the reason whenever decode fails, so a telegram is never hidden by the
codec's ignorance.

### E4-D6 — Resolution reports conflicts; it never picks a winner

RESEARCH §6.1 rule 3: "Conflict case: multiple linked objects declaring
different DPTs. Must be detected and reported, not silently resolved."

```rust
pub enum GroupAddressDpt {
    None,                    // no linked com object states a DPT
    Single(DptRef),
    Conflict(Vec<DptRef>),   // sorted, deduplicated, ≥ 2 entries
}

pub fn resolve_group_address_dpt(project: &Project, ga: GroupAddressId)
    -> GroupAddressDpt;
```

Reading the model correctly matters here:

- Only `Override::Value` counts. `Override::Absent` (attribute not
  present — 149 of 907 instances in the reference project),
  `Override::Empty` (present and deliberately cleared — 497 of 907), and
  `Override::Malformed` (present, unparsable) all mean "this instance
  states no usable DPT". `Empty` in particular is ETS saying *cleared*,
  not *unset* (ADR-0010) — collapsing it into a DPT would invent data.
- Both `Direction`s of a `GroupLink` count. A sending and a receiving
  object on one group address describe the same value.
- `is_active` is **not** filtered on: an inactive object still documents
  what the address carries, and RESEARCH §6.1 offers no rule to the
  contrary. Stated here so a reviewer can disagree on purpose.

Measured consequence on the corpus, from RESEARCH §6.1: 194 of 514 group
addresses (38%) resolve to `None`, and 110 (21%) have no linked
communication object at all. A monitor that only decoded what it could
resolve would go quiet on more than a third of the bus. Hence E4-D8.

### E4-D7 — `bus write` gains `--dpt` and `--dry-run`; today's behaviour is the fallback

```
knx bus write --gateway <host:port> [--project <p.knxdb>] [--dpt <DptRef>]
              [--dry-run] <main/middle/sub> <value>
```

- `--dpt DPST-9-1` given: `<value>` is parsed as that DPT.
- No `--dpt`, `--project` given: resolve the address (E4-D6). `Single`
  encodes. `None` and `Conflict` are **errors** naming the addresses'
  candidates and telling the user to pass `--dpt` — never a fallback to
  raw, because silently reinterpreting `1` as a raw payload when the user
  believed they were writing a DPT value is exactly the class of mistake
  that puts an actuator in the wrong state.
- Neither given: today's `0`/`1`/hex parsing, unchanged and still
  documented as raw.
- `--dry-run` encodes, prints the resulting payload, and exits `0`
  without opening a socket. It exists because it is the only way to give
  the encode path end-to-end regression coverage in an environment with
  no KNX gateway, and because a user about to write to a live bus has a
  legitimate reason to look before they leap.

Validation happens before the socket is opened, in every path.

### E4-D8 — `bus monitor --project` decodes when it can and stays loud when it cannot

Output becomes one line per telegram, unchanged in shape apart from the
value:

```
1.1.5 -> 1/2/3 (Licht Küche): GroupValueWrite DPST-1-1 on
1.1.7 -> 4/0/1 (Aussentemperatur): GroupValueWrite DPST-9-1 -3.4
1.1.9 -> 7/7/7: GroupValueWrite [2a, 99] (no DPT resolved)
1.1.9 -> 0/1/2 (Szene): GroupValueWrite [01] (DPST-20-102: unsupported DPT)
1.1.2 -> 1/1/1 (Licht): GroupValueWrite [01, 02] (DPST-1-1: wrong length)
```

The raw payload is printed whenever the decoded value is not available,
together with the reason. Without `--project` the output is exactly
today's. Resolution runs once at startup into a `HashMap<u16, DptRef>`
built from the project; the monitor loop stays allocation-light and does
no project lookups per telegram.

### E4-D9 — Formatting lives in `knx-core`, so it can be tested

`apps/knx-cli` is a binary-only crate with no `#[cfg(test)]` module and
no library target; its tests drive the built `knx` binary. To keep the
decode path covered by ordinary unit tests, value → string formatting is
a `knx-core` function (`DptValue::format`/`Display`), and the CLI holds
only the glue that picks a DPT, calls the codec, and assembles the line.
The CLI's own regression coverage is then: `bus write --dry-run` (encode,
end to end) and the argument/resolution error paths, both of which need
no gateway.

## 4. The datapoint types this slice implements

### 4.1 Selection, by evidence

Occurrences of `DatapointType` references across the whole local corpus
(5 `.knxprod` product databases + 3 `.knxproj` demo projects), by main
type:

| Main type | Occurrences | In this slice |
| --- | --- | --- |
| 1 (B1, boolean) | 6414 | yes |
| 5 (U8) | 633 | yes |
| 2 (B2, control + value) | 538 | yes |
| 9 (F16) | 265 | yes |
| 14 (F32) | 226 | yes |
| 3 (B1U3, dimming/blinds) | 200 | yes |
| 13 (V32) | 126 | yes |
| 17 (scene number) | 36 | yes |
| 7 (U16) | 18 | yes |
| 20 (1-octet enumeration) | 18 | **no** — §4.3 |
| 18 (scene control) | 8 | yes |
| 8 (V16) | 2 | yes |
| 275 (manufacturer-range) | 2 | **no** — §4.3 |

Three more are implemented despite zero corpus occurrences, because they
are the symmetric siblings of types that are in it and omitting them
would leave a hole a user hits immediately: **6** (V8, sibling of 5),
**12** (U32, sibling of 13), **16** (A[14] string, the only textual DPT
in the same size family). Each still needs its Standard citation like
every other type.

### 4.2 What each implementation owes

Every main type's encode and decode must cite the section of
*03_07_02 Datapoint Types v02.02.01 AS* that specifies it, in a code
comment at the implementation. Per-type bit layouts, ranges, conversion
formulas, rounding rules and reserved values come from that document and
from the research record at
`/home/knxbench/.claude/jobs/8098e9e6/tmp/e4-dpt-encoding-research.md`
(a working artifact; the citations it carries are what survive into the
code). Where the two disagree, the Standard wins and the discrepancy is
reported, not smoothed over.

Subtypes change the encoding only where the Standard says so. Main type
5 is the one that matters: its scaled subtypes (percentage, angle) are
not the same encoding as its raw counts, and the codec must branch on the
subtype for exactly those and for nothing else. Main type 1's subtype
changes no bits at all.

### 4.3 What is deliberately not implemented, and why

- **10 (time), 11 (date), 19 (DateTime).** Zero corpus occurrences.
  Fully specified in the Standard, so this is a scope decision and not a
  knowledge gap; they are cheap to add later.
- **20 (1-octet enumeration), 275, and every other main type.** The wire
  value is a bare octet; the *meaning* of each value lives in the
  Standard's enumeration tables and in `knx_master.xml`, neither of which
  this application ingests. Decoding `0x01` to "1" would be a number
  pretending to be an answer. `UnsupportedDpt` is the honest result until
  the DPT catalogue exists.

## 5. Acceptance criteria

1. `knx-core` gains `dpt::codec` with `decode(DptRef, &GroupValue) ->
   Result<DptValue, DptCodecError>` and `encode(DptRef, &str) ->
   Result<GroupValue, DptCodecError>`, implementing main types 1, 2, 3,
   5, 6, 7, 8, 9, 12, 13, 14, 16, 17, 18, each with a Standard citation
   at its implementation.
2. Every implemented main type has round-trip unit tests (value → payload
   → value) covering minimum, maximum, zero and at least one interior
   value, plus its documented reserved/invalid codes where the Standard
   defines them.
3. Encoding chooses the inline six-bit form exactly for the datapoint
   types the Standard puts there, and separate octets for the rest,
   verified against the Standard's own rule rather than against what the
   code happens to do.
4. `GroupValue` lives in `knx-core` and is re-exported from `knx-net`;
   `cargo run -p xtask -- check-layering` still passes, and no call site
   outside those two crates changed its import.
5. `encode_group_value` no longer corrupts the APCI for an out-of-range
   `Short`, with a test that decodes the produced frame back to
   `GroupValueWrite`.
6. `knx-core` gains `resolve_group_address_dpt` returning
   `None`/`Single`/`Conflict`, with unit tests for: no linked object,
   one object with a DPT, two objects agreeing, two objects disagreeing
   (→ `Conflict`, never a pick), and one object each in the `Absent`,
   `Empty` and `Malformed` override states (→ not a DPT).
7. `knx bus write` accepts `--dpt` and `--dry-run`, resolves from
   `--project` when `--dpt` is absent, errors clearly on `None` and
   `Conflict`, and behaves exactly as it does today when neither flag is
   given. Covered by CLI integration tests that never open a socket.
8. `knx bus monitor --project` prints a decoded value where one is
   available and the raw payload plus a stated reason where it is not.
9. `docs/GAP_ANALYSIS_ETS.md` E4 is updated to say precisely what closed
   (core + CLI) and what did not (GUI, T15), `docs/KNOWN_LIMITATIONS.md`
   gains an honest entry for the codec's coverage and for the fact that
   resolution is inference over linked communication objects rather than
   a stated group-address DPT, `docs/COMPATIBILITY.md` records the
   evidence, and `docs/IMPLEMENTATION_STATUS.md` and `docs/ROADMAP.md`
   record T29. No existing limitation is downgraded.
10. Five gates green: `cargo fmt --all --check`, `cargo clippy
    --workspace --all-targets -- -D warnings`, `cargo test --workspace`
    (baseline 817 passed / 0 failed / 3 ignored), `cargo run -p xtask --
    check-layering`, `cargo deny check`.

## 6. Non-claims

This slice does not make KNXBench ETS-compatible, does not claim KNX
certification, and does not verify any decoded value against a real
device. The codec is verified against the Standard's text and against
round-trip properties, not against hardware; the live-gateway tests stay
`--ignored` and hardware-dependent as they are today.
