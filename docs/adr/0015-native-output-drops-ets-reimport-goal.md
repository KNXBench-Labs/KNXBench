# ADR 0015: Native output format drops the ETS-reimport goal

Date: 2026-09-08
Status: Accepted
Session: 7 (planning input for future sessions)

## Context

ADR-0003 already made SQLite the working format and confined `.knxproj` to
import/export. ADR-0007 already refused to claim byte-exactness or untested
ETS compatibility. What Session 3 then built on top of that was a
`.knxproj` *exporter* aimed at eventually being opened by real ETS again
(`ExportWarning::Unsigned`, risk R9 in ROADMAP.md, KNOWN_LIMITATIONS.md §5).

That target is dropped by product decision, not by new technical evidence.
The user no longer needs the application's project output to satisfy ETS on
reimport. `.knxproj` reading (import) stays required — projects have to come
from somewhere. `.knxproj` writing aimed at ETS acceptance stops being a
project goal.

## Decision

Going forward, the application's own project file (SQLite, ADR-0003) is the
one true output artifact. It is not required to be re-expressible as a
`.knxproj` that ETS accepts.

`.knxproj` import remains fully supported and maintained — it is the
on-ramp for existing projects and stays held to the fidelity bar in
ADR-0007 (semantic equality, opaque hash equality).

`.knxproj` export, where it already exists (`knx-etsproj::export`,
`knx-app::export_ets_project`), is not ripped out by this ADR. It keeps
working as a diagnostic/interop convenience and nothing already shipped
needs to be reverted. But it is no longer a goal to chase further:

- Risk R9 ("does ETS re-import an unsigned third-party `.knxproj`") is
  deprioritized. No session is scheduled to verify it, and its answer no
  longer gates anything.
- Closing schema-≥21 export completeness gaps (KNOWN_LIMITATIONS.md §21,
  §34) for the sake of ETS acceptance is no longer prioritized work. Gaps
  found incidentally are still documented, same as any other limitation.
- The "textual export/import format" mentioned in ADR-0003 as a possible
  future diagnostic/VCS aid is unaffected by this decision either way.

## Alternatives considered

**Keep pursuing ETS-reimport as the export target.** Rejected — it is the
user's explicit call, made independent of any new technical finding; the
architecture already treats `.knxproj` as one format among others (ADR-0003),
so nothing structural forces this outcome, and no structural change is
required to drop it either.

**Remove the existing `.knxproj` exporter.** Rejected. It is tested,
working code that costs nothing to keep, and it remains useful for anyone
who does want to hand a project to ETS, with the unsigned-export caveat
intact. Only the *forward-looking priority* changes, not the shipped
surface.

## Consequences

Future roadmap items that exist solely to chase ETS reimport (R9
verification, further schema-≥21 export parity) drop off the active
roadmap; ROADMAP.md and KNOWN_LIMITATIONS.md are updated to say so.

Import fidelity work (reading whatever `.knxproj` schema versions show up)
is unaffected — that is a separate, still-required capability.

Any future "what should our export look like" design question defaults to
"what serves our own format and our own users" rather than "what does ETS's
schema require."
