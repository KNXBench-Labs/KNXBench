# Technical prose localization

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-066)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

<a id="66-server-composed-prose-and-the-documentation-export-are-not-translated-by-any-ui-language-or-pack--partially-resolved-2026-09-14-t14"></a>
<a id="66-server-composed-prose-and-documentation-are-only-partly-localized--partially-resolved-2026-09-23-t14"></a>
## 66. Server-composed prose is only partly localized

**Parameter presentation follow-up (2026-10-08):** evaluation messages now live
only in the device Diagnostics tab, grouped without dropping occurrences or
scopes. Raw English `.detail` is explicitly inspectable under collapsed
Technical details as well as copyable; the normal headline remains translated.
Access Read/None fields have a separate inspection-only Manufacturer fields tab.
Only evaluated DTO fields are available: this is not a complete static inventory,
and exact controlling-value/condition facts are not reconstructed from Debug
text. No evaluator/compatibility boundary is lifted. See
[Parameter workspace](../PARAMETER_WORKSPACE.md).


**Included fun languages (2026-10-07):** `bar` (**Boarisch**) and `tlh`
(**Klingonisch/Klingon**) intentionally translate only 335 interface keys each.
They are playful overlays, not full or authoritative language translations.
Missing keys, including detailed technical/safety notices, use the unchanged
English fallback. The Settings panel discloses this boundary; product and
project content are not translated by these packs. See [Language packs](../LANGUAGE_PACKS.md).

**Resolved, one surface.** `ParameterDiagnostic.message` — the parameter
panel's own diagnostic headline — now follows the `CreationDiagnostic`
pattern this section's own "Lifted when" paragraph named as the way
across this boundary. `ParameterDiagnosticDto`
(`apps/knx-server/src/routes.rs`) gained a `kind` field
(`ParameterDiagnosticKindDto`, 22 variants — 15 of them mirror, one for
one, the 15 cases of `pub enum Diagnostic`
(`crates/knx-productdb/src/dynamic/evaluate.rs:756`); the other 7 —
`ParametersUnreadable` through `MalformedModuleInstanceId` — have no
`Diagnostic` counterpart at all and are constructed directly in
`apps/knx-server/src/domain.rs`, where the read fails before a
`Diagnostic` could even be produced); `diagnostic_kind_and_message` now
returns `(kind, message)` instead of just `message` for the 15 mirrored
cases, and all eight `ParameterDiagnosticDto` construction sites carry
a `kind`. `.message` itself is unchanged text — still English, still
composed server-side — but it is now redundant: `ParameterPanel.tsx`'s
`describeParameterDiagnosticMessage` looks `kind` up in
`PARAMETER_DIAGNOSTIC_MESSAGE_KEYS` (22 entries, one per
`parameters.diagnostic.*` catalogue key in `messages/en.ts`/`de.ts`) and
renders the catalogue's translation, falling back to the raw `.message`
only for a `kind` this build's frontend doesn't recognise — the same
"trust the wire shape, degrade to English for the unknown case"
contract `CreationDiagnostic.kind` already documents. This was cheap
specifically because every one of the 22 messages is a fixed sentence
with no interpolated value of its own (`ParametersUnreadable` through
`UnresolvedTextPlaceholder`, spanning both `domain.rs`'s own
construction sites and its `diagnostic_kind_and_message` match); every
dynamic value — ids, node numbers, counts — already lived in `.detail`,
never in `.message`, so tagging the closed set cost one enum and a
lookup table, not a server-side templating layer. Test:
`apps/knx-web/src/ParameterPanel.test.tsx`, "§66: a diagnostic's message
is translated with the UI language; its detail stays English".

**The boundary rule (apply this to the next string you add).** A
server-composed string crossing into `apps/knx-web` is translatable
work, not a given, and the question to ask before wiring it into the
catalogue is: *does a user read this during normal use of the feature,
and is its content a closed, enumerable set (or safely decomposable
into a translated template plus untranslated data slots, `toast.ts`'s
`{msg}` style)?* If yes, give the Rust side a `kind` tag (or reuse an
existing enum's discriminant) the way `ParameterDiagnostic.message` and
`CreationDiagnostic` (T25 Task 5) both do, and translate the *kind*
client-side — never the composed sentence itself, and never by trying
to parse or pattern-match English prose back into a language. If no —
the text is developer-facing (read for a bug report, not during the
task), or it is arbitrary/unbounded (a panic message, a raw debug
formatter, a validator's free-form detail) — it stays English, on
purpose, and this document is where that decision is recorded so the
next person doesn't have to re-litigate it per string.

There is a third case, and it is not the same as either answer above:
the string passes the "yes" test — user-facing, closed and enumerable —
but the component composing it architecturally cannot reach a
catalogue at all (no injected dependency, no language parameter on its
entry point, a layering rule that forbids the dependency a catalogue
would ride in on). That string also stays English, but for a reason
that has nothing to do with being arbitrary or unbounded, and filing it
under that clause teaches the wrong lesson to whoever reads this
document next. It stays English *until someone gives the component a
catalogue to inject* — which is itself the follow-up task, not a
reason to leave the boundary undecided.

**Ruled out under the first two buckets — developer-facing, or
arbitrary/unbounded — and why (still open, not scheduled):**

- `ParameterDiagnostic.detail` (`apps/knx-web/src/api.ts`) — kept
  English by design, not merely untranslated. It is `format!("{:?}",
  diagnostic)` (`domain.rs`), a Rust `Debug` dump behind
  `ParameterPanel.tsx`'s "Copy details" button, meant to be pasted into
  a bug report or read by whoever wrote `domain.rs`, not by an end user
  during normal use — arbitrary, not a closed set, and nobody reads it
  in German. `api.ts`'s doc comment on the field says so; the test
  above asserts `.detail`'s content (`"NoBranchMatched { choose_node:
  4821 }"`) reaches the clipboard byte for byte through the "Copy
  details" button's handler, untranslated — not merely that it is
  absent from the banner's rendered text, which it always would be
  regardless of translation, since `.detail` is never printed there in
  the first place (fix round 1, Q3).
- `LogPanel.tsx`'s `entry.message`/`entry.location`/`entry.detail` — the
  session log is diagnostic output for whoever is debugging a bus
  session, the textbook case of "log text nobody reads in German" this
  task's brief named explicitly. Only the row's *severity* label
  (`SEVERITY_LABEL_KEYS`) is UI chrome and was already translated before
  this task. **Fix round 2 (B4):** this is §67's shape again — German
  chrome over an English message, with nothing telling the reader it is
  deliberate — so the panel now says so where the reader actually is,
  not just here: a translated `logPanel.entryTextIsEnglish` line
  (`"Message, location and detail are the server's own text, in
  English."`) renders under the header whenever there is at least one
  entry. Disclosure only; `entry.message`/`location`/`detail` are
  exactly as untranslated as before.
- API error strings surfaced in toasts (`api.errorMessage`,
  `toast.ts`'s `humorizeError`, `BusMonitorPanel.tsx`'s
  `stopSummary.warning` fed by `BusSessionSummary::drain_panic` in
  `apps/knx-server/src/bus_routes.rs`) — these are not a closed,
  enumerable set the way `ParameterDiagnostic.message` and
  `LanguagePackRejectionReason` (§67) are; they're every `Result::Err`
  path across the whole Rust backend, funnelled through one generic
  `Error.message` unwrap at the HTTP boundary. Closing this would mean
  auditing and tagging every fallible call site server-wide — a
  cross-cutting rewrite well beyond one task, explicitly out of scope
  per this task's brief ("do not perform unrelated refactors"). The
  wrapper sentence around the message is already translated
  (`toast.error.*`); only the substituted server text is not. **Fix
  round 2 (B4):** every error toast now says so — a translated
  `toast.error.messageIsEnglish` line (`"This message is the server's
  own text, in English."`) renders under the wrapper sentence, `{msg}`
  and all. Disclosure only; the substituted text itself is unchanged.

**The documentation-export case changed on 2026-09-23, but remains open.**
`ReportOptions::language` now injects an English/German choice and localizes
the document title, primary navigation, selected headings and mandatory
limits; `knx-app` requests product strings in the same locale. Detailed table
labels, debug/enum values and several diagnostic sentences remain English.
The report has its own small built-in language choice, not access to the web
message catalogue or imported packs, so a pack cannot add a third report
language. Completing this requires extending the renderer's injected prose
catalogue while retaining its enforced independence from `knx-productdb` and
frontend code. See
[§48](../KNOWN_LIMITATIONS.md#48-project-documentation-export-renders-in-one-language-only).

**Cause (original, still true except for `ParameterDiagnostic.message` and
the report's built-in primary EN/DE chrome).** The remaining strings are
composed by Rust crates and cross the HTTP boundary as opaque text, not as a
message key plus parameters. A frontend catalogue can only translate a key it
was given; a language pack can only override a key this build already defines.
Neither mechanism has anywhere to attach to a string it never sees structured.

**Impact.** A user running any UI language — German, an imported pack,
or an invented one — now sees a translated parameter-diagnostic
headline, but still sees English `.detail` text (by design), English log
entries (by design), and English error-toast bodies inside translated
wrappers. A report explicitly requested in German has German primary chrome
and product strings but still contains the detailed English text above; an
imported pack cannot add a report language. The gap is smaller, not closed.

**Lifted when.** Partially done, 2026-09-14 (T14): the
`ParameterDiagnostic.message` surface closed, using the exact mechanism
this section previously said would be needed. On 2026-09-23 the report gained
an injected EN/DE choice and localized primary chrome, but not a complete
catalogue or language-pack integration. The remaining surfaces above stay
open for the stated reasons; none is scheduled.

**Update 2026-10-05 (AR10): accepted boundary.** Re-checked against the code:
the parameter-diagnostic headline is translated by `kind`
(`PARAMETER_DIAGNOSTIC_MESSAGE_KEYS` onto the `parameters.diagnostic.*`
catalogue keys in `messages/en.ts`/`de.ts`); `.detail` and the session log stay English by
design, and the log panel and every error toast say so where the reader is
(`logPanel.entryTextIsEnglish`, `toast.error.messageIsEnglish`, with tests in
`LogPanel.test.tsx`/`Toast.test.tsx`). Error bodies are not a closed set;
the report's detailed labels are §48's accepted boundary. No server string
added by AR10 is prose: the new language markers are identifiers.
<!-- END retained catalogue record -->
