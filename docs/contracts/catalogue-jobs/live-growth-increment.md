# WP-04.3 actual-connection main growth

Bounded implementation of the D/W growth rows in
[protocol P5](protocol-artifacts.md#p5-named-resource-pipeline--admission-before-owned-materialization).
This increment does not complete WP-04.3 or expose production job routes.

## Required behaviour

The source/copy admission remains **8 GiB**. The SQL allocation policy is a
separate fixed **68,719,476,736-byte total main extent**, not an allowance added
to the existing database. Validate positive supported power-of-two page sizes
512–65536, positive page counts, checked extent multiplication and count at or
below `floor(ceiling / pageSize)` before setting the cap.

Apply and preview must reassert `main.max_page_count` on the **actual reopened
RW D connection**, after copy completion and validation, before the shared
installer. Independently read back the exact value; a successful-looking setter
alone is not admission. Do not assume backup or reopening retains the cap.

Live W must already observe synchronous FULL or EXTRA before BEGIN; refuse
lower, unknown or unreadable settings without repairing them. Under the original
stamp-matched IMMEDIATE reservation, admit the actual lane/page facts, capture
W's prior connection-local cap and record restoration debt **before** attempting
the setter or readback. Keep the job cap through native COMMIT or rollback.
After transaction settlement, restore and independently read back the exact
prior cap without cancellation, before deciding writer health.

A restoration failure is an independent cleanup fact: quarantine W and retain
allocated recovery B. Preserve a witnessed successful COMMIT and any independent
primary refusal. A prior cap below the now-committed extent can produce a genuine
clamped restoration; that is not exact restoration and does not undo the commit.
Before B allocation, a restoration failure cannot invent a recovery artifact.

Admit main-only, no application TEMP objects, DELETE journal and normal locking
by observation, without converting an unsupported connection. This is a private
controlled-job policy, not a global quota for existing quiet callers. No schema,
profile, binding-version, codec, server or UI change belongs to this increment.

## Finite verification

Tests pin the production constant independently of thread-local cfg(test)-only
finite D/W caps. Native fixtures force real SQLITE_FULL independently on D and W,
with a positive below-cap install; no 64 GiB allocation is required or claimed.
Other witnesses cover actual reopened-handle caps, denied/ignored setters and
independent readback, page facts, FULL/EXTRA versus NORMAL/unreadable admission,
unsupported lanes, restoration after success/refusal/cancellation, partial setter
failure, and independent commit/cleanup/recovery facts.

Focused tests, original-runtime comparisons, bounded compiled wrong variants,
installer source tracing and owning-suite checks are distinct evidence methods.
Source tracing of the reachable installer finds no cap-reset operation; it is
not a formal proof against future code changes. Independent review and maintained
branch/integrated gates remain required before publication.

## Explicit exclusions

Finite tests do not demonstrate a real 8–64 GiB database, aggregate disk/RSS,
journal/native-temp allocation bounds, complete panic/ambiguous-commit recovery,
artifact durability, full B08/B10, or production lifecycle/routes/UI readiness.
Those boundaries remain open. No bus/device access or ETS compatibility claim.
