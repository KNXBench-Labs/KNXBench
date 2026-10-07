# AR18 round 3 fixes — R6 (N11), N12, N13, R7 regate

- **Agent:** Claude, alpha-release-goal session (ledger owner `alpha`), no subagents.
- **Window:** 2026-10-06, ~21:00–23:58 CEST.
- **Commits:** `439b2789` merge of `review/alpha-recheck-3`; `296ba1bb` fix; `cba15ce4` tests; `2254eed0` N13; `62d25a24` docs. Pushed to `origin/main`.

## Design (R6)

- Central truth = what `zip` 8.6 reads: `entry.crc32()`, `compressed_size()`, `size()` (zip64 resolved), plus *raw* central flags, method and extra from our own central walk (`RawRecord` in `container.rs`). The parsed method is unusable: zip rewrites an AES member's method, which would turn the `UnsupportedEncryption` refusal into a false `InconsistentRecord`.
- Local: name, flags, method, CRC, sizes; local 0xFFFFFFFF from the local zip64 field (APPNOTE 4.5.3, both sizes, uncompressed first).
- Bit 3: local values zeros or exactly the central values; descriptor at data start + central compressed size, signature optional, 8-byte sizes iff the local header has a zip64 field.
- Unicode Path: list of 0x7075 bodies equal in both headers (not the whole extra block: 0x5455 timestamps differ by design, see the Info-ZIP fixture).
- Layout: sort records by start; each must end where the next begins, the last at the central directory start. Leading bytes stay allowed (SFX prefix).
- Directories: judged after the layout check (linear inflate bound). Carries data if size > 0, stored with bytes, or a deflate stream with output or trailing bytes (`flate2::Decompress`, 1-byte buffer, StreamEnd + total_out 0 + total_in == len). The `encrypted` branch was removed: it guarded no byte.

## Evidence

- RED: 21 of 46 tests failed before the fix (every N11 attack imported, `Ok(())`).
- First crate run after the fix: 4 ZipCrypto fixture tests red → Info-ZIP `zip -e` layout (flags 0x0009, real local values, signed descriptor). Rule widened, control test added.
- Structural census (aggregates): 106 archives + 7 nested, 1,517 records, 0 descriptors/gaps/overlaps/prefixes.
- Mutants: 27/27 killed by named tests; three needed `--no-fail-fast` to reach the named integration test (the lib's fixtures killed them first).
- Gate `2254eed0`: see ALPHA_FINAL_GATES §11; evidence dir `~/.hermes/profiles/knxbench/evidence/alpha-release/ar18-round3-fixes-20261006/`.

## Open

- Round 4 by a fresh reviewer.
- N6 (UI owner), §157 in-place upgrade, ~3× in-budget memory peak, SFX-prefix residual.
