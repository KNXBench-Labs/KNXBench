# Session 5, cycle 10: toast notifications, humor, and easter eggs — design

**Status.** Approved, ready for implementation planning.

## Context

Three `ideas.md` entries land together this cycle, since two of them
need infrastructure that doesn't exist yet and the third reuses it:

- "Leichter humoristischer Ton bei Meldungen" — give error text some
  personality.
- "Toasts an wichtigen Feiertagen die weltweit gültig sind" — a short
  joke on load, on a handful of broadly-recognized dates.
- "Toasts beim Starten der App wenn es schon sehr spät ist" — same
  mechanism, triggered by the clock instead of the calendar.

**Current state** (`apps/knx-desktop`): there is no toast concept at
all. `App.tsx` holds one `error: string | null` state, set by every
`catch` block across `pickProject`/`openNativeProject`/`saveProject`/
`saveProjectAs`/`undo`/`redo`/`selectEntity`/`handleTreeUpdate`, cleared
by `setError(null)` at the start of each new attempt, and rendered as a
single persistent `<p role="alert" className="error-banner">`
(`styles.css:39`, themed via `--knx-error-color`). This cycle replaces
that with a small toast stack; nothing else in the app currently
resembles a transient notification.

**Scope decisions:**

- One toast system serves all three ideas, not three separate
  mechanisms. `kind: "error"` toasts replace today's banner exactly —
  same persistence-until-replaced-or-dismissed semantics, same trigger
  sites — just rendered as a stack entry instead of a fixed paragraph.
  `kind: "fun"` toasts are new: auto-dismissing, pushed once at
  startup.
- Backend error strings (`CommandError`/`ValidationError` `Display`
  text, crossing the Tauri `invoke` boundary as plain strings) are
  **not** touched. Rust-side error text stays exact and keeps its own
  tests meaningful. Humor is applied only in the frontend, by wrapping
  the untouched message in a randomly chosen template — the factual
  core survives verbatim inside the wrapper, matching the explicit
  instruction that the message's core must stay true.
- `kind: "fun"` auto-dismisses (~6s) because it's decoration, not
  information the user must act on. `kind: "error"` never
  auto-dismisses — a user must not lose an error they haven't read yet,
  same reasoning that kept the old banner persistent.
- Startup toast precedence: if today is a listed holiday, show the
  holiday message; otherwise, if the local clock is inside the
  late-night window, show a late-night message; otherwise, nothing.
  Never both — one toast at startup keeps it decoration, not clutter.
- Holiday list is short and deliberately not exhaustive: only dates
  broadly recognized across cultures/commercially (New Year's Day,
  Valentine's Day, April Fools', Halloween, Christmas Eve, Christmas
  Day, New Year's Eve) — not an attempt at a real worldwide-holiday
  calendar (religious/regional holidays vary too much for "weltweit
  gültig" to mean anything precise). Easy to extend later; not a data
  model, just a literal array.
- Local system clock/date, no timezone handling beyond `Date`'s
  built-in local-time getters — matches `theme.ts`'s existing precedent
  of using whatever the OS reports, no separate timezone library.
- No `.knxdb`/backend persistence for toast state — same reasoning as
  `theme.ts`: a UI-only concern, nothing here belongs in the KNX domain
  model.

## Backend

None. Frontend-only, like cycles 6 and 7.

## Frontend

### `toastCopy.ts` (new) — all display strings, pure data

```ts
/** Wrapping templates for kind: "error" toasts. `{msg}` is the untouched backend message. */
export const ERROR_WRAPPERS: string[] = [
  "Well, that didn't go as planned: {msg}",
  "The bus objects: {msg}",
  "Gremlins in the wiring: {msg}",
  "KNX says no: {msg}",
  "Not today: {msg}",
  "Houston, we have a problem: {msg}",
  "That's a hard pass: {msg}",
];

export interface HolidayEntry {
  /** 1-12, matches `Date#getMonth() + 1`. */
  month: number;
  /** 1-31, matches `Date#getDate()`. */
  day: number;
  messages: string[];
}

// HolidayEntry is defined here (the data file) and imported by toast.ts,
// which owns no shapes of its own — just logic over this file's data.

export const HOLIDAYS: HolidayEntry[] = [
  { month: 1, day: 1, messages: [
    "Happy New Year! May your group addresses stay unique.",
    "New year, same group addresses.",
  ] },
  { month: 2, day: 14, messages: [
    "Roses are red, buses are twisted pair.",
    "Be my Valentine, my favorite communication object.",
  ] },
  { month: 4, day: 1, messages: [
    "No bugs today. Probably.",
    "Everything in this build is 100% real. Trust us.",
  ] },
  { month: 10, day: 31, messages: [
    "Spooky season: even the ghosts use KNX for the lighting.",
    "Boo! Your project is still safe.",
  ] },
  { month: 12, day: 24, messages: [
    "Ho ho ho, don't forget to save your project.",
    "Silent night, wired bright.",
  ] },
  { month: 12, day: 25, messages: [
    "Merry Christmas! Even Santa needs a group address for the chimney sensor.",
    "Season's greetings from your KNX app.",
  ] },
  { month: 12, day: 31, messages: [
    "One more save before midnight?",
    "See you next year, project file.",
  ] },
];

export const LATE_NIGHT_MESSAGES: string[] = [
  "Burning the midnight oil? So is your KNX bus.",
  "It's late. Even the bus line needs rest.",
  "Still awake? The group addresses admire your dedication.",
  "Night owl mode engaged.",
];
```

### `toast.ts` (new) — pure logic + `useToasts` hook

```ts
import { ERROR_WRAPPERS, HOLIDAYS, LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";

export type ToastKind = "error" | "fun";

export interface ToastEntry {
  id: number;
  kind: ToastKind;
  message: string;
}

/** Local-hour window treated as "late night": 23:00-04:59. */
export function isLateNight(date: Date): boolean {
  const h = date.getHours();
  return h >= 23 || h < 5;
}

export function findHoliday(date: Date, holidays: HolidayEntry[] = HOLIDAYS): HolidayEntry | undefined {
  const month = date.getMonth() + 1;
  const day = date.getDate();
  return holidays.find((h) => h.month === month && h.day === day);
}

function pickRandom(options: string[], random: () => number): string {
  return options[Math.floor(random() * options.length)];
}

/** Holiday beats late-night; neither beats nothing. */
export function pickStartupToast(
  date: Date,
  random: () => number = Math.random,
  holidays: HolidayEntry[] = HOLIDAYS,
): string | undefined {
  const holiday = findHoliday(date, holidays);
  if (holiday) return pickRandom(holiday.messages, random);
  if (isLateNight(date)) return pickRandom(LATE_NIGHT_MESSAGES, random);
  return undefined;
}

/** Wraps a raw backend error message in a random humor template; the
 * original message survives unmodified inside it. */
export function humorizeError(
  message: string,
  random: () => number = Math.random,
  wrappers: string[] = ERROR_WRAPPERS,
): string {
  return pickRandom(wrappers, random).replace("{msg}", message);
}

export function useToasts() {
  const [toasts, setToasts] = useState<ToastEntry[]>([]);
  const nextId = useRef(0);

  function dismiss(id: number) {
    setToasts((ts) => ts.filter((t) => t.id !== id));
  }

  /** Clears any existing error toast, then shows the new one. Mirrors the
   * old `setError(null)`-then-`setError(...)` pattern at every call site. */
  function pushError(rawMessage: string) {
    const id = nextId.current++;
    setToasts((ts) => [...ts.filter((t) => t.kind !== "error"), { id, kind: "error", message: humorizeError(rawMessage) }]);
  }

  function clearErrors() {
    setToasts((ts) => ts.filter((t) => t.kind !== "error"));
  }

  function pushFun(message: string, autoDismissMs = 6000) {
    const id = nextId.current++;
    setToasts((ts) => [...ts, { id, kind: "fun", message }]);
    setTimeout(() => dismiss(id), autoDismissMs);
  }

  return { toasts, pushError, clearErrors, pushFun, dismiss };
}
```

`isLateNight`/`findHoliday`/`pickStartupToast`/`humorizeError` are pure
and take their randomness/inputs as parameters, the same testability
pattern `theme.ts`'s `loadTheme`/`saveTheme` already established for
this project. `useToasts` itself is untested for the same reason
`useTheme` is: it touches React state/timers, and this project has no
component-testing library.

### `Toast.tsx` (new) — presentational stack

```tsx
import type { ToastEntry } from "./toast";

export default function ToastStack(props: { toasts: ToastEntry[]; onDismiss: (id: number) => void }) {
  if (props.toasts.length === 0) return null;
  return (
    <div className="toast-stack">
      {props.toasts.map((t) => (
        <div key={t.id} role={t.kind === "error" ? "alert" : "status"} className={`toast toast--${t.kind}`}>
          <span>{t.message}</span>
          <button aria-label="Dismiss" onClick={() => props.onDismiss(t.id)}>
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
```

`role="alert"` for error toasts matches the old banner's accessibility
behavior exactly; `role="status"` for fun toasts is the correct
non-urgent equivalent.

### `App.tsx` wiring

- `const { toasts, pushError, clearErrors, pushFun, dismiss } = useToasts();` replaces the `error`/`setError` state.
- Every `setError(null)` call site becomes `clearErrors()`; every
  `setError(String(e))` call site becomes `pushError(String(e))`.
- `{error && <p ...>}` is replaced by `<ToastStack toasts={toasts} onDismiss={dismiss} />`, rendered unconditionally (it renders `null` itself when empty) at the same position in the tree the old banner occupied.
- New mount-only effect pushes the startup toast:

  ```ts
  const startupToastShown = useRef(false);
  useEffect(() => {
    if (startupToastShown.current) return; // StrictMode double-invoke guard
    startupToastShown.current = true;
    const message = pickStartupToast(new Date());
    if (message) pushFun(message);
  }, []);
  ```

  The ref guard is needed because `main.tsx` wraps `<App />` in
  `<StrictMode>`, which double-invokes effects in development.

### `styles.css`

New rules alongside the existing `.error-banner` block (which is
deleted, replaced by `.toast`/`.toast--error`):

```css
.toast-stack {
  position: fixed;
  bottom: 1rem;
  right: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  z-index: 100;
}

.toast {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.75rem;
  border-radius: 4px;
  max-width: 24rem;
  box-shadow: 0 4px 24px var(--knx-overlay-shadow);
}

.toast button {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
  color: inherit;
}

.toast--error {
  color: var(--knx-error-color);
  border: 1px solid var(--knx-error-color);
  background: Canvas;
}

.toast--fun {
  color: CanvasText;
  border: 1px solid currentColor;
  background: Canvas;
}
```

Reuses `--knx-error-color`/`--knx-overlay-shadow` (already theme-aware
from cycle 7) rather than introducing new custom properties.
`Canvas`/`CanvasText` match the system-color-keyword pattern
`search-panel` already uses, so both toast kinds adapt to the active
theme with no new tokens.

## Testing

`toast.test.ts` (Vitest, alongside `theme.test.ts`):

- `isLateNight`: boundary hours 22 (false), 23 (true), 0 (true), 4
  (true), 5 (false).
- `findHoliday`: an exact listed date returns its entry; a non-listed
  date returns `undefined`.
- `pickStartupToast`: a holiday date returns one of that holiday's
  messages regardless of the late-night window; a non-holiday date
  inside the late-night window returns one of `LATE_NIGHT_MESSAGES`; a
  non-holiday, non-late-night date returns `undefined`; an injected
  `random` deterministically selects a specific index.
- `humorizeError`: the returned string contains the original message
  verbatim; an injected `random` deterministically selects a specific
  wrapper.

No test for `useToasts`, `ToastStack.tsx`, or the `App.tsx` wiring —
DOM/timers/React state, same gap `useTheme`/`ThemeToggle.tsx` already
have (see Context). Manual smoke check (trigger an error and confirm
the wrapped message still contains the real text and stays until
dismissed or replaced; temporarily fake the system clock/date to
confirm a holiday and a late-night toast each appear once at startup
and auto-dismiss) is left unperformed here for the same reason cycles
4-9 left theirs unperformed — no display available in this
environment. Should be run before, or at, merge.
