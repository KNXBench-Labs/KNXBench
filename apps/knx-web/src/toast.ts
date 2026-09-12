/** Toast queue state and holiday/late-night joke selection for startup and error toasts. */
import { useRef, useState } from "react";
import { ERROR_WRAPPERS, HOLIDAYS, LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";
import { formatTemplate, translate } from "./i18n";
import type { TranslatableKey } from "./i18n";

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

/**
 * Resolves `key` via `translate()` with no params — for a real catalogue
 * key this returns the *unsubstituted* template text (`translate()`'s own
 * `formatTemplate` call is a no-op with `params` omitted, see `i18n.ts`),
 * and for a fixture string that isn't a catalogue key at all (as
 * `toast.test.ts` injects), `translate()`'s last-resort fallback returns
 * the string itself unchanged — either way, a template `params` can still
 * be substituted into via `formatTemplate` afterwards. This is what lets
 * `humorizeError`'s `{msg}` substitution work uniformly whether `key` is a
 * real `toast.error.*` entry or a raw test fixture template.
 */
function resolveTemplate(key: string, params?: Record<string, string | number>): string {
  return formatTemplate(translate(key as TranslatableKey), params);
}

/** Holiday beats late-night; neither beats nothing. */
export function pickStartupToast(
  date: Date,
  random: () => number = Math.random,
  holidays: HolidayEntry[] = HOLIDAYS,
): string | undefined {
  const holiday = findHoliday(date, holidays);
  if (holiday) return resolveTemplate(pickRandom(holiday.messages, random));
  if (isLateNight(date)) return resolveTemplate(pickRandom(LATE_NIGHT_MESSAGES, random));
  return undefined;
}

/** Wraps a raw backend error message in a random humor template; the
 * original message survives unmodified inside it. */
export function humorizeError(
  message: string,
  random: () => number = Math.random,
  wrappers: string[] = ERROR_WRAPPERS,
): string {
  return resolveTemplate(pickRandom(wrappers, random), { msg: message });
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
