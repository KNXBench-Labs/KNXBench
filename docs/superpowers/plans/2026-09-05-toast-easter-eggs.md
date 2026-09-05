# Toast Notifications, Humor, and Easter Eggs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `knx-desktop`'s single persistent error banner with a
toast stack; wrap error messages in a humor template that keeps the
original text intact; add a one-shot startup toast for holidays and
late-night sessions.

**Architecture:** Frontend-only. `toastCopy.ts` holds pure display-string
data (error-wrapper templates, holiday list, late-night messages).
`toast.ts` holds pure logic over that data (`isLateNight`, `findHoliday`,
`pickStartupToast`, `humorizeError` — all unit-testable, randomness and
inputs injected) plus a `useToasts()` hook managing the toast array.
`Toast.tsx` is a small presentational stack component. `App.tsx` swaps
its `error` state for `useToasts()` and adds a mount-only effect that
pushes at most one startup toast.

**Tech Stack:** React 19, TypeScript, Vitest (`environment: "node"`, no
DOM — matches existing project convention: pure logic gets unit tests,
DOM-touching components don't).

**Spec:** `docs/superpowers/specs/2026-09-05-toast-easter-eggs-design.md`

## Global Constraints

- No backend change. Frontend-only, like cycles 6 and 7.
- Backend `CommandError`/`ValidationError` text crossing `invoke` is
  never edited — humor wraps it in the frontend only, original message
  survives verbatim inside the wrapper.
- `kind: "error"` toasts never auto-dismiss (persist until manually
  dismissed or replaced by a new error) — matches the old banner's
  behavior exactly. `kind: "fun"` toasts auto-dismiss after 6000ms.
- Startup toast precedence: holiday beats late-night beats nothing.
  Never more than one toast pushed at startup.
- Holiday list: New Year's Day (Jan 1), Valentine's Day (Feb 14), April
  Fools' (Apr 1), Halloween (Oct 31), Christmas Eve (Dec 24), Christmas
  Day (Dec 25), New Year's Eve (Dec 31) — exact `HolidayEntry[]` array
  and message text below, not a placeholder list.
- Late-night window: local hour `>= 23 || < 5`.
- No new npm dependency. No `.knxdb`/backend persistence for toast
  state — a UI-only concern, same reasoning as `theme.ts`.

---

## File Structure

- Create: `apps/knx-desktop/src/toastCopy.ts` — `HolidayEntry` type,
  `ERROR_WRAPPERS`, `HOLIDAYS`, `LATE_NIGHT_MESSAGES`.
- Create: `apps/knx-desktop/src/toast.ts` — `ToastKind`, `ToastEntry`,
  `isLateNight`, `findHoliday`, `pickStartupToast`, `humorizeError`,
  `useToasts`.
- Create: `apps/knx-desktop/src/toast.test.ts` — Vitest coverage for the
  four pure functions.
- Create: `apps/knx-desktop/src/Toast.tsx` — the `ToastStack`
  presentational component.
- Modify: `apps/knx-desktop/src/styles.css` — replace `.error-banner`
  with `.toast-stack`/`.toast`/`.toast--error`/`.toast--fun`.
- Modify: `apps/knx-desktop/src/App.tsx` — replace `error`/`setError`
  with `useToasts()`, add the startup-toast effect, render
  `<ToastStack>`.

---

### Task 1: `toastCopy.ts` + `toast.ts` — data, pure logic, hook

**Files:**
- Create: `apps/knx-desktop/src/toastCopy.ts`
- Create: `apps/knx-desktop/src/toast.ts`
- Test: `apps/knx-desktop/src/toast.test.ts`

**Interfaces:**
- Produces (from `toastCopy.ts`): `export interface HolidayEntry { month:
  number; day: number; messages: string[] }`, `export const
  ERROR_WRAPPERS: string[]`, `export const HOLIDAYS: HolidayEntry[]`,
  `export const LATE_NIGHT_MESSAGES: string[]`.
- Produces (from `toast.ts`): `export type ToastKind = "error" | "fun"`,
  `export interface ToastEntry { id: number; kind: ToastKind; message:
  string }`, `export function isLateNight(date: Date): boolean`,
  `export function findHoliday(date: Date, holidays?: HolidayEntry[]):
  HolidayEntry | undefined`, `export function pickStartupToast(date:
  Date, random?: () => number, holidays?: HolidayEntry[]): string |
  undefined`, `export function humorizeError(message: string, random?: ()
  => number, wrappers?: string[]): string`, `export function
  useToasts(): { toasts: ToastEntry[]; pushError: (raw: string) => void;
  clearErrors: () => void; pushFun: (message: string, autoDismissMs?:
  number) => void; dismiss: (id: number) => void }`. Task 2
  (`Toast.tsx`) imports `ToastEntry` by this exact name. Task 3
  (`App.tsx`) calls `useToasts()` and destructures this exact shape,
  and calls `pickStartupToast(new Date())`.

- [ ] **Step 1: Write the failing test**

```ts
// apps/knx-desktop/src/toast.test.ts
import { describe, expect, it } from "vitest";
import { findHoliday, humorizeError, isLateNight, pickStartupToast } from "./toast";
import { LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";

describe("isLateNight", () => {
  it("is false at 22:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 22, 0))).toBe(false);
  });
  it("is true at 23:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 23, 0))).toBe(true);
  });
  it("is true at 00:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 0, 0))).toBe(true);
  });
  it("is true at 04:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 4, 0))).toBe(true);
  });
  it("is false at 05:00", () => {
    expect(isLateNight(new Date(2026, 0, 1, 5, 0))).toBe(false);
  });
});

const holidays: HolidayEntry[] = [{ month: 1, day: 1, messages: ["New Year A", "New Year B"] }];

describe("findHoliday", () => {
  it("finds a listed date", () => {
    expect(findHoliday(new Date(2026, 0, 1), holidays)).toEqual(holidays[0]);
  });
  it("returns undefined for a non-listed date", () => {
    expect(findHoliday(new Date(2026, 0, 2), holidays)).toBeUndefined();
  });
});

describe("pickStartupToast", () => {
  it("prefers a holiday message even inside the late-night window", () => {
    const date = new Date(2026, 0, 1, 23, 30); // Jan 1, 23:30 — holiday AND late-night both true
    expect(pickStartupToast(date, () => 0, holidays)).toBe("New Year A");
  });

  it("falls back to a late-night message when not a holiday", () => {
    const date = new Date(2026, 0, 2, 23, 30);
    expect(pickStartupToast(date, () => 0, holidays)).toBe(LATE_NIGHT_MESSAGES[0]);
  });

  it("returns undefined when neither a holiday nor late night", () => {
    const date = new Date(2026, 0, 2, 12, 0);
    expect(pickStartupToast(date, () => 0, holidays)).toBeUndefined();
  });

  it("uses the injected random to select the index", () => {
    const date = new Date(2026, 0, 1, 12, 0);
    expect(pickStartupToast(date, () => 0.99, holidays)).toBe("New Year B");
  });
});

describe("humorizeError", () => {
  it("keeps the original message intact inside the wrapper", () => {
    expect(humorizeError("Duplicate group address", () => 0, ["Nope: {msg}"])).toBe(
      "Nope: Duplicate group address",
    );
  });

  it("uses the injected random to select the wrapper", () => {
    expect(humorizeError("x", () => 0.99, ["A: {msg}", "B: {msg}"])).toBe("B: x");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/knx-desktop && npm test -- toast`
Expected: FAIL — `Cannot find module './toast'`

- [ ] **Step 3: Write `toastCopy.ts`**

```ts
// apps/knx-desktop/src/toastCopy.ts

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

export const HOLIDAYS: HolidayEntry[] = [
  {
    month: 1,
    day: 1,
    messages: ["Happy New Year! May your group addresses stay unique.", "New year, same group addresses."],
  },
  {
    month: 2,
    day: 14,
    messages: ["Roses are red, buses are twisted pair.", "Be my Valentine, my favorite communication object."],
  },
  {
    month: 4,
    day: 1,
    messages: ["No bugs today. Probably.", "Everything in this build is 100% real. Trust us."],
  },
  {
    month: 10,
    day: 31,
    messages: ["Spooky season: even the ghosts use KNX for the lighting.", "Boo! Your project is still safe."],
  },
  {
    month: 12,
    day: 24,
    messages: ["Ho ho ho, don't forget to save your project.", "Silent night, wired bright."],
  },
  {
    month: 12,
    day: 25,
    messages: [
      "Merry Christmas! Even Santa needs a group address for the chimney sensor.",
      "Season's greetings from your KNX app.",
    ],
  },
  {
    month: 12,
    day: 31,
    messages: ["One more save before midnight?", "See you next year, project file."],
  },
];

export const LATE_NIGHT_MESSAGES: string[] = [
  "Burning the midnight oil? So is your KNX bus.",
  "It's late. Even the bus line needs rest.",
  "Still awake? The group addresses admire your dedication.",
  "Night owl mode engaged.",
];
```

- [ ] **Step 4: Write `toast.ts`**

```ts
// apps/knx-desktop/src/toast.ts
import { useRef, useState } from "react";
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
    setToasts((ts) => [
      ...ts.filter((t) => t.kind !== "error"),
      { id, kind: "error" as const, message: humorizeError(rawMessage) },
    ]);
  }

  function clearErrors() {
    setToasts((ts) => ts.filter((t) => t.kind !== "error"));
  }

  function pushFun(message: string, autoDismissMs = 6000) {
    const id = nextId.current++;
    setToasts((ts) => [...ts, { id, kind: "fun" as const, message }]);
    setTimeout(() => dismiss(id), autoDismissMs);
  }

  return { toasts, pushError, clearErrors, pushFun, dismiss };
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd apps/knx-desktop && npm test -- toast`
Expected: PASS, 11 tests

- [ ] **Step 6: Commit**

```bash
git add apps/knx-desktop/src/toastCopy.ts apps/knx-desktop/src/toast.ts apps/knx-desktop/src/toast.test.ts
git commit -m "feat(knx-desktop): add toast data, pure logic, and useToasts hook"
```

---

### Task 2: `Toast.tsx` — presentational stack

**Files:**
- Create: `apps/knx-desktop/src/Toast.tsx`

**Interfaces:**
- Consumes: `ToastEntry` type from `./toast` (Task 1).
- Produces: default export `ToastStack(props: { toasts: ToastEntry[];
  onDismiss: (id: number) => void })`. Task 3 (`App.tsx`) renders
  `<ToastStack toasts={toasts} onDismiss={dismiss} />`.

No component-testing library exists in this project (`vitest.config.ts`
sets `environment: "node"`) — this task is verified by type-checking,
not an automated test, matching `ThemeToggle.tsx`'s precedent.

- [ ] **Step 1: Write the component**

```tsx
// apps/knx-desktop/src/Toast.tsx
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

- [ ] **Step 2: Type-check**

Run: `cd apps/knx-desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/knx-desktop/src/Toast.tsx
git commit -m "feat(knx-desktop): add ToastStack presentational component"
```

---

### Task 3: Wire toasts into the app (`App.tsx`, `styles.css`)

**Files:**
- Modify: `apps/knx-desktop/src/App.tsx`
- Modify: `apps/knx-desktop/src/styles.css`

**Interfaces:**
- Consumes: `useToasts`, `pickStartupToast` from `./toast` (Task 1);
  `ToastStack` default export from `./Toast` (Task 2).
- Produces: nothing for later tasks — this is the integration point and
  the plan's final task.

- [ ] **Step 1: Add imports and replace the `error` state**

In `apps/knx-desktop/src/App.tsx`, replace:

```ts
import ThemeToggle from "./ThemeToggle";
import Dashboard from "./Dashboard";
import { useTheme } from "./theme";
```

with:

```ts
import ThemeToggle from "./ThemeToggle";
import Dashboard from "./Dashboard";
import { useTheme } from "./theme";
import ToastStack from "./Toast";
import { pickStartupToast, useToasts } from "./toast";
```

Replace:

```ts
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);
```

with:

```ts
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const { toasts, pushError, clearErrors, pushFun, dismiss } = useToasts();
```

- [ ] **Step 2: Add the startup-toast effect**

Directly after the existing `useEffect` that registers `handleKeyDown`
(the one ending `return () => window.removeEventListener("keydown",
handleKeyDown);\n  });`), add:

```ts
  const startupToastShown = useRef(false);
  useEffect(() => {
    if (startupToastShown.current) return; // StrictMode double-invoke guard
    startupToastShown.current = true;
    const message = pickStartupToast(new Date());
    if (message) pushFun(message);
  }, []);
```

`useRef` and `useEffect` are already imported at the top of the file
(`import { useEffect, useRef, useState } from "react";`) — no import
change needed for this step.

- [ ] **Step 3: Replace every `setError` call site**

Replace each of the following exactly (all in `apps/knx-desktop/src/App.tsx`):

In `selectEntity`:
```ts
    setError(null);
```
→ (the one right after `setSelection(sel);`)
```ts
    clearErrors();
```

```ts
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        setError(String(e));
        setDeviceDetail(null);
      }
```
→
```ts
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        pushError(String(e));
        setDeviceDetail(null);
      }
```

In `handleTreeUpdate`:
```ts
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        setError(String(e));
      }
    }
  }

  async function pickProject() {
```
→
```ts
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        pushError(String(e));
      }
    }
  }

  async function pickProject() {
```

In `pickProject`:
```ts
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_project", { path }));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      setError(String(e));
    }
  }

  async function openNativeProject() {
```
→
```ts
    if (typeof path !== "string") return;
    clearErrors();
    try {
      resetTree(await invoke<ProjectTree>("open_project", { path }));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      pushError(String(e));
    }
  }

  async function openNativeProject() {
```

In `openNativeProject`:
```ts
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_native_project", { path }));
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProjectAs() {
```
→
```ts
    if (typeof path !== "string") return;
    clearErrors();
    try {
      resetTree(await invoke<ProjectTree>("open_native_project", { path }));
      setHasStorePath(true);
    } catch (e) {
      pushError(String(e));
    }
  }

  async function saveProjectAs() {
```

In `saveProjectAs`:
```ts
    if (typeof path !== "string") return;
    setError(null);
    try {
      await invoke("save_project_as", { path });
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProject() {
```
→
```ts
    if (typeof path !== "string") return;
    clearErrors();
    try {
      await invoke("save_project_as", { path });
      setHasStorePath(true);
    } catch (e) {
      pushError(String(e));
    }
  }

  async function saveProject() {
```

In `saveProject`:
```ts
    if (!hasStorePath) return saveProjectAs();
    setError(null);
    try {
      await invoke("save_project");
    } catch (e) {
      setError(String(e));
    }
  }

  async function undo() {
    setError(null);
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("undo"));
    } catch (e) {
      setError(String(e));
    }
  }

  async function redo() {
    setError(null);
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("redo"));
    } catch (e) {
      setError(String(e));
    }
  }
```
→
```ts
    if (!hasStorePath) return saveProjectAs();
    clearErrors();
    try {
      await invoke("save_project");
    } catch (e) {
      pushError(String(e));
    }
  }

  async function undo() {
    clearErrors();
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("undo"));
    } catch (e) {
      pushError(String(e));
    }
  }

  async function redo() {
    clearErrors();
    try {
      await handleTreeUpdate(await invoke<ProjectTree>("redo"));
    } catch (e) {
      pushError(String(e));
    }
  }
```

- [ ] **Step 4: Replace the rendered banner with `<ToastStack>`**

Replace:

```tsx
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
```

with:

```tsx
      <ToastStack toasts={toasts} onDismiss={dismiss} />
```

- [ ] **Step 5: Update `styles.css`**

Replace:

```css
.error-banner {
  color: var(--knx-error-color);
  border: 1px solid var(--knx-error-color);
  padding: 0.5rem;
  border-radius: 4px;
}
```

with:

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

- [ ] **Step 6: Type-check and run the full frontend test suite**

Run: `cd apps/knx-desktop && npx tsc --noEmit && npm test`
Expected: no type errors; all Vitest suites pass, including the 11 new
`toast.test.ts` cases.

- [ ] **Step 7: Commit**

```bash
git add apps/knx-desktop/src/App.tsx apps/knx-desktop/src/styles.css
git commit -m "feat(knx-desktop): wire toast stack, humor wrapping, and startup easter eggs into App"
```

---

## Follow-up (not part of this plan's tasks)

- Manual smoke check (per spec's Testing section): trigger a real error
  and confirm the wrapped message contains the real text and stays
  until dismissed or replaced; temporarily fake the system clock/date to
  confirm a holiday and a late-night toast each appear once at startup
  and auto-dismiss after ~6s. No display available in this environment
  — leave for the user or a future session with one, same as cycles
  4-9's unperformed manual checks.
- Update `docs/IMPLEMENTATION_STATUS.md` and `docs/ROADMAP.md` for
  Session 5 cycle 10 once all three tasks land, and move the three
  `ideas.md` entries this cycle covers into its "Erledigt" section.
