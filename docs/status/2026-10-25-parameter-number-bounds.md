# Parameter number-bound admission acceptance — 25 October 2026

## Published source

`87d71d4aa` is published on `main` with matching live and tracking-ref readback.
The main-first merge preserves upstream `ef7589476` outside seven reviewed paths.
The shared image value check now refuses an unreadable declared TypeNumber bound
instead of silently removing it ([contract](../contracts/parameter-number-bound-admission.md)).
Source defaults and requested values share this check. Raw imported attributes
remain unchanged; the importer, core model and encoding are not modified.

Absent bounds remain absent. Present bounds are trimmed and read using `u64` for
unsigned types and `i64` for signed types; both are parsed before range checks,
minimum first. Errors identify the parameter, attribute, raw text and number type.
Negative unsigned bounds are not converted to zero. A leading plus sign and leading
zeros retain the integer parser's existing admission; this is application behaviour,
not a normative XML lexical grammar or a claim of invalid source XML.

The inspected Project Schema23 v01.00.00 §1.1.3.19 (p. 30) describes parameter
values, not a lexical rule established here for bound attributes. Float, hexadecimal
and scientific bound interpretation remain unsupported. Experimental retained-source
admission is unchanged. A refused probe parameter becomes `SkipReason::Refused`;
other probe candidates remain available instead of the whole probe failing.

## Source and regression evidence

- The original test revision against original runtime passed seven library controls
  and failed seven intended assertions; both public image integration methods failed.
  Parent replay of the retained binaries confirmed these results.
- Independent review requested changes: an unsupported negative-minimum-to-zero rule
  was removed. A separate policy-correction RED against that rejected runtime passed
  sixteen methods and failed exactly the negative unsigned minimum admission method.
  This evidence is separate from the original RED and remains bound to its own sources.
- Final parent replay passed seventeen bound methods, both public System-1/System-2
  image integration methods and the optional probe method: twenty distinct methods.
  Coverage includes unreadable minimum/maximum, signed and unsigned domains, defaults,
  range/width controls, diagnostic order and retained raw text.
- The corrected owning-crate worker run passed 774 library and 796 integration tests,
  with 31 integration tests ignored. That run used a non-root, capability-free,
  loopback-only namespace. An earlier root-mapped permission-test failure is retained
  as historical evidence, not relabelled as passing.
- Independent finding-specific re-review approved all I1/M1–M4 corrections. Reviewed
  source and retained executable hashes were authenticated by the controller.

## Integrated acceptance on `87d71d4aa`

All fourteen stages selected by the maintained change-scoped package runner passed
in order, with identical start/end input commitments and empty touched/unchecked
user-data lists.

- Rust: 4,215 passed, zero failed, 154 ignored across 24 result blocks. All twenty
  named regression methods passed here too; focused and owning runs are not added.
- Private corpus: 97/97 across six selected targets. This is selected regression
  evidence, not an exhaustive census of TypeNumber bounds or product compatibility.
- Formatting, Clippy, tooling, documentation/local-information, layering, headers,
  anchors, ledger, corpus registration and final diff checks passed.
- The frontend was freshly built for the desktop Rust prerequisite. Browser execution
  was not selected; interactive desktop, release and deployment acceptance are not claimed.

## Remaining boundaries

The repair is fail-closed image admission, not new encoding support. Whether other
real products use negative unsigned bounds or other refused spellings remains open;
a previous image that silently ignored such a bound may now refuse. No physical
installation was contacted, and no hardware, whole-device recovery, family expansion,
KNX Secure or broad ETS compatibility is established. Inactive/Union/customer-write
policy remains unresolved. [PRG-01](../open-work/PRG-01-programming-gaps.md) and
[KL-007](../limitations/KL-007-bounded-commissioning-support.md) remain open.
