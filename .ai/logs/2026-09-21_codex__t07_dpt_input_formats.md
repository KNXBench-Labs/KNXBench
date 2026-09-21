# T07 — explicit DPT input formats and visible encoding rulings

## Outcome

T07 replaces the bus-facing codec's implicit text-grammar choice with
`DptInputFormat` on `knx_core::encode`. CLI writes accept `--input-format`, HTTP
writes accept `inputFormat`, and the web compose form exposes a visible format
selector. The default `Auto` option is explicitly a legacy-compatibility mode;
it calls the clearly named `encode_inferred_format` so existing values keep
their previous bytes.

`encoding_rulings(DptRef)` now returns stable, public metadata for every
project-judgment encoding recorded in `KNOWN_LIMITATIONS.md` §61: identifier,
Standard document/section, and the chosen behavior. No encoder's wire mapping
was intentionally changed. The eighteen 200-series LTE/system DPTs remain an
accepted scope exclusion because the stack has no LTE addressing path.

Specification evidence was located with the local knowledge base and checked
against the source set under
`/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0`, especially
`03_07_02 Datapoint Types v02.02.01 AS.pdf`. Future KNX specification work must
use those PDFs as primary evidence and cite the document plus section.

## Review findings and fixes

The independent whole-branch review found one Important compatibility defect:
known-DPT web writes silently labelled arbitrary text decimal. For DPT 21,
`00000010` therefore changed from `0x02` to `0x0A`. The final design exposes a
format selector; `Auto` preserves the inferred legacy path, while explicit
binary/decimal/hexadecimal/canonical/text choices reach the strict encoder.

The same review found that blanket prefix rejection treated `0B` as a binary
prefix even under an explicitly hexadecimal grammar. Prefix rejection is now
radix-specific: hexadecimal rejects `0x`, while `0B` remains valid hex eleven.
Documentation claims about omitted CLI/HTTP/web formats were corrected.

RED→GREEN evidence:

- CLI omitted-format DPT 21 `00000010`: RED `[0a]`, GREEN `[02]`.
- HTTP fake-tunnel omitted-format DPT 21: RED wrong payload, GREEN `[02]`.
- Web known-DPT selector: RED implicit `decimal`/missing control, GREEN explicit
  selection plus visible `Auto` compatibility.
- Core explicit hexadecimal `0B`: RED `InputFormatMismatch`, GREEN `[0b]`.
- Diagnostic shell stylesheet ownership: RED orphan class, GREEN styled select.

## Verification

No KNX gateway, bus, or hardware was contacted. CLI coverage used `--dry-run`;
HTTP coverage used `FakeTunnel`.

Final branch evidence on `11c8131`:

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace --no-fail-fast`: pass
- `cargo run -p xtask -- check-layering`: pass
- `cargo run -p xtask -- check-headers`: pass, 181 present / 162 absent
- `cargo run -p xtask -- check-anchors`: pass, 377 links / 174 Markdown files
- `cargo deny check`: pass
- `npx vitest run`: pass, 812 tests / 58 files
- `npx tsc --noEmit`: pass
- `knx-core --lib`: 479 tests, baseline 474
- `cli_bus_dpt`: 10 tests
- `http_bus_write`: 13 tests

The source volume ran out of space during the first full build. Only the
worktree's recoverable Cargo `target/` cache was removed. Final Rust gates used
a clean temporary target under `/tmp/knxbench-t07-target`, which also defeats
the previously documented stale-binary risk.

## Commits

- `7aa5ee7` — explicit format boundary and ruling metadata
- `e988d64` — preserve omitted-format bitset compatibility
- `fd023e4` — visible web format choice and radix-specific prefix handling
- `11c8131` — style ownership for the new selector

