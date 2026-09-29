# ADR 0024: Help is a tip and a panel, its text is an ordinary catalogue key, and `docs/` never ships

Date: 2026-09-19

Status: Accepted

## Context

`docs/GAP_ANALYSIS_ETS.md` row D12 has said the same thing since it was
written: this application has no in-application help of any kind. The
count was re-measured on 2026-09-19, on the branch this ADR lands on,
with the command the row itself publishes:

```sh
for p in 'title=' 'aria-label=' 'aria-describedby='; do
  grep -rho "$p" apps/knx-web/src --include='*.tsx' --exclude='*.test.tsx' | wc -l
done
```

Eight `title` attributes, **38** `aria-label`s (33 on 2026-09-13; five
arrived with the UI work of the intervening week), four
`aria-describedby`s — still all four on `BusComposeForm.tsx`, still
pointing at a disabled-state reason rather than at an explanation. No
`role="tooltip"` anywhere, no tooltip component, no help panel, no `F1`
handler. `grep -rn "'F1'" apps/knx-web/src` returns nothing.

The eight `title` attributes are the whole of the present mechanism, and
a `title` is not help: it is mouse-only, it cannot be styled, its delay
is the operating system's, and on the three of them that sit on a
*disabled* control it reaches nobody at all, because a disabled control
is out of the tab order. `BusMonitorPanel.tsx` says so in a comment
already.

`docs/ROADMAP.md`'s "Cross-cutting — In-application help and user
documentation" section deliberately refuses to decide the shape and
instead poses four questions. This ADR answers them, in order, and is
written before the implementation rather than after it.

What is verified here: the counts above, and that `messages/de.ts` is
annotated `Record<MessageKey, string>` so `npx tsc --noEmit` rejects a
German catalogue that is missing a key — that property is the load-bearing
fact behind the second decision. What is assumed: nothing about what
users find easier. No usability study was run, and none is claimed.

## Decision

### 1. Both, and they are not interchangeable

**A tooltip and a help panel, with a rule about which gets which text.**

- **`HelpTip`** — a focusable `?` trigger sitting beside a control, whose
  bubble holds **one or two sentences about that one control**. It exists
  because a person whose hands are already on a control should not have
  to leave it to find out what it is. It opens on hover *and* on focus,
  closes on blur, on pointer-leave and on `Escape`, and its text is wired
  to the trigger with `aria-describedby` at all times — not only while the
  bubble is visible — so the description is available to a screen reader
  on focus whether or not anything is painted.

  That last sentence is only true because of *how* the bubble hides. A
  closed bubble is `opacity: 0`, never `display: none` and never
  `visibility: hidden`: both of those remove the element from the
  accessibility tree, which would leave `aria-describedby` resolving to
  nothing and make the description depend on this component's `onFocus`
  state update winning a race against the screen reader's own lookup.
  Recorded here because it looks like a styling detail and is in fact the
  decision — `help.test.ts` reads `styles.css` and fails if either
  declaration reappears. (Corrected 2026-09-19 in T28's first fix round;
  the original wording claimed the property without naming what buys it,
  and the implementation did use `visibility: hidden`.)
- **`HelpPanel`** — a modal overlay on `F1`, holding **topics**: the KNX
  concepts and the application surfaces, several paragraphs each, with a
  topic list to move between them. It exists because "what is a group
  address" is not a sentence and does not belong beside a text field.

Each mechanism is wrong in the other's place, which is the only honest
justification for having two:

- A tooltip carrying three paragraphs is worse than no tooltip. It covers
  the control it explains, it cannot be scrolled without a focus trap it
  has no business owning, and — because the bubble is the
  `aria-describedby` target — a screen reader reads **all of it** the
  moment focus lands on the trigger, before the user has asked for
  anything. Length is not a style preference here; it is an
  accessibility failure with a specific mechanism.
- The panel is wrong for "what does this checkbox do". It costs a modal
  open, a topic selection and a focus round-trip to deliver six words,
  and `Overlay` restores focus on close, so the user pays that round trip
  whether they wanted it or not.

The rule between them: **a tip may point at a topic, and must never
restate it.** Where the same explanation is wanted in both places, the
panel owns the prose and the tip says the short version.

**What is deliberately not built: a persistent, docked context panel that
tracks the selection.** The workbench is already three columns — the
navigation pane, the centre workspace, the properties inspector — and a
fourth permanent region on a data-dense editor either takes width from
the tables this application exists to display, or becomes a toggle nobody
turns on. Worse, a selection-tracking panel needs a topic for every
selectable entity or it renders an empty state most of the time, and with
the topic count this task ships, "most of the time" is the honest
forecast. Revisit when there are enough topics that a docked panel would
usually have something to say.

Cost of taking both: two mechanisms to keep in step, two places prose can
drift, and a decision to make every time help text is written. Accepted,
because the alternative is picking one and being wrong in half the cases.

### 2. Help text is an ordinary catalogue key; there is no second store

**Every user-facing word of help lives in
`apps/knx-web/src/messages/en.ts` and `messages/de.ts`, under a `help.*`
namespace, exactly like every other string in the application.** No
Markdown files, no JSON bundle, no separate help catalogue, no build step
that reads prose off disk.

The reason is a compile-time property that nothing else reproduces:
`de.ts` is annotated `Record<MessageKey, string>` where `MessageKey` is
derived from `en.ts`'s object literal, so a German catalogue missing a
key is a **type error**, not a runtime fallback and not a test failure.
Help text is the largest body of prose this application has ever carried;
it is also the body most likely to be added to in a hurry and forgotten
in the other language. Putting it anywhere else trades the one mechanism
that cannot be forgotten for a mechanism that can. `languagePack.ts`
carries the same consequence outward: an installed pack's `messages` are
keyed by `MessageKey` too, and the English template export
(`exportEnglishTemplate`) enumerates the same set, so help text outside
the catalogue would be invisible to every third-party translator as well.

Two constraints follow from the storage choice and are part of the
decision, not side effects:

- **A key's value is one paragraph of plain text.** A multi-paragraph
  topic is several keys, listed in order in `help.ts`. The catalogue
  holds no markup, and no help string is ever passed to
  `dangerouslySetInnerHTML` — a translated value rendered as HTML is an
  injection hole with a friendly name.
- **The topic and paragraph key lists are `satisfies readonly
  MessageKey[]`,** the same device `loadFlavour.ts`'s `FLAVOUR_KEYS`
  uses, so a topic naming a key that does not exist is a compile error
  rather than a blank paragraph.

Cost, stated plainly rather than discovered later: prose in a TypeScript
object literal is awkward to write and worse to translate. `en.ts` was
972 lines before this task. A translator gets no segment view, no fuzzy
matching and no translation memory — they get a `.ts` file and a
`tsc` error when they miss one. Paragraph-per-key makes reflowing a topic
a rename. This is a real limitation and is recorded as one in
`docs/KNOWN_LIMITATIONS.md`; it is not a reason to invent a second
storage mechanism quietly, and if it is ever fixed it should be fixed by
generating the catalogue from a translator-friendly format, keeping the
generated `MessageKey` gate intact.

### 3. KNX concepts are explained in the application, and nothing links out

**In-app, at "enough to operate this application" depth. No hyperlinks to
the KNX Association or to anyone else.** Four reasons, in the project's
own priority order:

- **Correctness of claims.** A link to a standards body reads as
  endorsement in both directions. This project holds no KNX
  certification and claims no full ETS compatibility; it should not
  borrow authority by hyperlink, and a reader should not have to infer
  the absence of a relationship.
- **Availability.** Commissioning happens in basements and on building
  sites. Help whose substance is a URL is absent precisely when it is
  needed, and that is the one time anyone opens it.
- **Drift.** No gate in this repository can test an external URL. A dead
  link fails silently and indefinitely.
- **Fit.** What a user needs at the moment they open the panel is what a
  group address does *in this application*, and what the five
  communication-object flags do *to a communication object here*. That is
  a smaller question than the KNX Standard answers, and a better one to
  answer.

The boundary is stated inside the help text itself: each concept topic
says in one sentence that it describes how this application uses the
concept and is not a substitute for the KNX Standard. Help text states no
specification clause, table or figure number — this repository has a
queryable specification knowledge base for that, available to
maintainers, and quoting from it into user-facing prose would put claims
in front of users that the prose itself cannot cite.

Cost: the project now owns KNX prose and has to keep it true. A wrong
sentence about the `C` flag is a correctness defect in the one place no
test can catch, because no test can assert that a sentence is accurate —
the same limit ADR-0023 names about its phase labels. Mitigated only by
keeping every topic to behaviour this application demonstrably has.

Related and enforced by the same rule: where behaviour has been verified
only against this project's own simulator, the help text says so. It must
never imply verification against hardware.

### 4. `docs/` does not ship. Not one file

**No file under `docs/` is bundled, copied into `dist/`, linked from the
application, or summarised into it.** User-facing text is written
separately, from the start, in the catalogue.

- Every file in `docs/` is addressed to a maintainer. They cite crate
  names, function names, ADR numbers, fix-round finding ids and
  measurements from the development machine. That is the right register
  for their readers and the wrong one for a user.
- They are English-only and outside the translation mechanism entirely.
  Shipping them would install a permanently monolingual wall inside a
  bilingual application — and the German half of this application is not
  a courtesy, it is where its users are.
- They change for reasons the user does not share. An ADR's "Alternatives
  considered" section is a record of a decision, not an instruction.
- Several state limitations in a maintainer's calibration — "nothing
  proves a phase label is accurate" — which is either alarming or
  meaningless to a user, and is certainly not help.

Cost: two bodies of prose about one application, free to disagree. One
rule contains it: where the help panel describes a limitation, it states
the limitation in its own words and does not cite a document. The
`docs/` files stay the authority for developers; the help panel is not a
summary of them, is not maintained against them, and nothing should try
to keep the two in sync line by line.

### Motion and the keyboard, since help is the one feature that cannot be inaccessible

The `F1` handler accepts **`F1` with no modifier**, and `help.ts`'s
`opensHelp` is the predicate — a pure function, so the rule is testable
without a DOM. Modified `F1` is left alone for the browser and the window
manager.

The tip's bubble answers to both motion switches, with `helpTipAnimates`
built in the shape `loadFlavour.ts`'s `flavourRotates` established:
`<html data-motion-level="off">` or `prefers-reduced-motion: reduce`
means the bubble appears without a transition. A runtime with no
`matchMedia` is not a runtime asking for stillness, so an absent API
resolves to "animate" — the same reading, for the same reason.

### 2026-09-29: U5 contextual routing extension

A tip may now register the existing long-form topic it points to. F1 on
its focused trigger opens that exact topic, and click/Enter on the tip
opens the same topic deliberately; ordinary F1 falls back to the workbench
overview. `HelpPanel` labels its dialog with the active topic heading and
focuses/resets the scrollable prose when the topic changes. A new Group
address ranges topic explains nested containment separately from the
per-address datapoint type. The short `Range` tip in the table points to
it. The catalogue and the `MessageKey` gate remain the only help-text
store; no external help route or extra modal is introduced.

## Alternatives considered

**`title` attributes everywhere, and nothing else.** Free, zero
components, and already present eight times. Rejected: mouse-only, so it
is not help for a keyboard user; unreachable on a disabled control, which
is where three of the existing eight sit; unstyleable; and its show delay
belongs to the operating system. Kept where it already is — a `title`
duplicating an `aria-label` on an icon button is a fine mouse
affordance — but it is not the mechanism.

**The native `popover` attribute / `<dialog>` for the tip.** Real browser
machinery, top-layer rendering, no positioning fights. Rejected for this
slice: `popover` is a light-dismiss model aimed at menus, the test
environment (`happy-dom`) does not implement it, and a tooltip that only
works in a browser is a tooltip with no test. Worth revisiting when the
bubble needs to escape an overflow container.

**A floating-UI / positioning dependency.** Would solve collision
detection and flipping properly. Rejected as an unnecessary dependency
for bubbles that currently sit in roomy chrome; CSS positioning is
enough, and when it stops being enough that is a measured reason to take
the dependency rather than a speculative one.

**Help text in Markdown files under `apps/knx-web/src/help/`, imported as
raw strings.** Pleasant to write, real paragraphs, no escaping. Rejected
outright: it leaves the `MessageKey` gate, so a missing German topic
becomes a silent English paragraph in a German UI instead of a build
failure, and it is invisible to language packs. This is the decision this
ADR is least willing to revisit.

**A separate help catalogue module keyed the same way** (`helpMessages
/en.ts`, typed `Record<HelpKey, string>`). Keeps a compile-time gate and
keeps `en.ts` shorter. Rejected because it is two catalogues with two
`t()` functions, two fallback chains and two export paths for language
packs — a second mechanism whose only benefit is file length, which is a
formatting problem.

**Linking out to the KNX Association's material for concepts.** No prose
to own, no risk of explaining a standard wrongly. Rejected on the four
grounds above; the availability one alone decides it.

**Shipping `docs/` as a bundled manual.** Nine-plus files of real
material already written, no new writing. Rejected: wrong reader, wrong
language, wrong register, wrong change cadence.

**A persistent docked context panel instead of a modal.** Discussed under
decision 1 — rejected on layout cost and on how often it would be empty,
not on principle.

## Consequences

Easier: a new control gets an explanation by adding two catalogue keys
and one `<HelpTip>`; a new concept gets a topic by adding an entry to
`HELP_TOPICS` and its paragraph keys. Both are compile-checked. The panel
needs no route, no build step and no network.

Harder: `en.ts` and `de.ts` grow substantially and will keep growing,
and every help addition is a two-language edit or a build failure —
deliberately the latter, but it is friction and will be felt. Every new
UI surface now has a third question attached to it ("does this need a
tip, a topic, both, or nothing?") that did not exist before.

Enforced by tests: `help.test.ts` covers `opensHelp` (plain `F1` only,
every modifier rejected, other keys rejected) and `helpTipAnimates` (both
switches, and the absent-`matchMedia` reading), and asserts every topic's
paragraph list is non-empty. `HelpTip.test.tsx` covers the keyboard path
(the bubble appears on focus, not only on hover), the permanent
`aria-describedby` association, `role="tooltip"`, `Escape` dismissal, and
the motion guard reaching the rendered element. `HelpPanel.test.tsx`
covers the labelled heading, topic switching and that every topic renders
its paragraphs. `App.test.tsx` covers `F1` opening the panel and a
modified `F1` not opening it.

Not covered, and not coverable: whether any help sentence is *true*. No
test can assert that the description of the `U` flag matches what the
flag does. That is review's job, permanently.
