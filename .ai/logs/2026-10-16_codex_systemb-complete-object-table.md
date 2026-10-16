# P4d — Complete explicit System-B object tables

## Scope

Extend the existing pure Format-1 Core codec with `build_complete`. It accepts a
complete descriptor set plus an explicit caller capacity; no inactive entries
are invented. Product/profile selection, source-to-ASAP mapping, relative-image
placement, source substitution, executable planning and whole-mutation recovery
remain separate prerequisites. No public mask, transport, UI, schema or dependency
changes; no hardware, vendor execution or subagents.

## Evidence and implementation

Original Resources 03_05_01 v01.10.01 §4.18.6.2.4.1, printed pages270–272,
Tables86–88 were rendered and visually inspected. Current count is distinct
from maximum capacity; descriptors form a complete one-based array. Licensed
renders and product source stay private.

The constructor checks count/capacity before narrowing, then reuses `rewrite`
for duplicate/range validation and all flag/type encoding. A complete unique
permutation of1..=count is required. Empty input only encodes a literal zero count.

## Executed verification

- Missing `build_complete` API produced the intended E0432 RED before implementation.
- Six new independent literal/boundary tests pass; ten existing rewrite tests pass.
- Capacity-padding mutant failed the named assertion; original source bytes were
  restored and all six tests passed again before subsequent wording-only cleanup.
- Separate in-session self-review: no critical/important/security findings. One
  minor reused "source table" diagnostic label was generalized and rechecked.
- Reviewed Core:850passed/0failed/0ignored across18targets; owning all-target Clippy
  and format pass. Documentation and five repository checks pass.

## Remaining acceptance

The first full branch attempt failed in unchanged Net test diagnostic acquisition
(`/proc/net/igmp` incomplete snapshot), not a table assertion; that failure is
preserved and no test guard was changed. One full unchanged-input retry passed
all18stages: Rust4678/0/208ignored in343targets, Vitest3626/263files;2922source
paths, external commitments and Git fingerprints matched.

Focused commit, actual non-FF merged acceptance including
selected isolated corpus, publication/readback, private evidence archive and owned
retirement remain pending. No full family/device-support claim follows.
