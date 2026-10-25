# Hardware identity comparison: fixed six-octet width

Bounded safety fix of 2026-10-25 (branch `fix/hardware-identity-width-20261025`).

## Evidence

*Resources* (`03_05_01` v01.10.01) §4.3.28, printed pp. 78-79: `PID_HARDWARE_TYPE`
(PID 78) in the Device Object is `PDT_GENERIC_06`, a single value without the
array aspect, coded as one format octet followed by five octets. One hardware
has one six-octet value.

## Contract

- `knx_core::commissioning::memory_download::property_matches` is unchanged and
  stays generic: the answer must be non-empty, a prefix of the product's
  `InlineData`, and every octet after it zero. Padded `InlineData` of 10 or 64
  octets remains valid.
- `resource_matches(object, property, answer, inline_data)` first requires
  `expected_response_octets(object, property)` octets where that width is
  known, then applies `property_matches`. Only Device Object (index 0) PID 78
  has a known width: six (`HARDWARE_TYPE_OCTETS`). Every other resource keeps
  the generic rule; no width is guessed.
- The `InlineData` length is never read as the answer's wire length.

## Consumers

- `knx-net` memory download executor, `CompareProperty` step: a PID 78 answer of
  0-5 or 7+ octets is a `PropertyMismatch` before the backup read, before the
  backup is handed to the caller and before any device change. Restores run
  through the same executor and get the same check.
- `knx-net` declared-memory diagnostic collector and the `knx-app` format-2
  archive re-check: their existing exact-six read guards stay; the final
  comparison uses `resource_matches` as well. A forged archive that records
  seven PID 78 octets with a matching expected length was already refused by
  the archive's per-query expected-length check; the shared comparison is a
  second, independent refusal.

## Tests

- Core: widths 0-5 and 7+ against six-octet and padded `InlineData`, 64-octet
  padding, other resources equal to the generic rule.
- Executor, on the simulator path with a zero-tailed hardware type: every wrong
  width refused with the received octets in the error, no backup callback, no
  memory read or write, no property write; six octets with padded `InlineData`
  still download.
- Collector: wrong widths read no memory (already true through the exact-six
  read guard; kept). Archive: the forged seven-octet case is refused.

## Limits

- The partial-download precondition compares `PID_PROGRAM_VERSION` with the
  generic rule; this fix does not change it.
- Tested only against the simulator. No device, bus or network access.
