/** Toast queue state and holiday/late-night joke selection for startup and error toasts. */
import { useRef, useState } from "react";
import { ERROR_WRAPPERS, HOLIDAYS, LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";
import { formatTemplate, translate } from "./i18n";
import type { TranslatableKey } from "./i18n";
import type { AchievementTier } from "./achievementCatalog";
import type { WorkbenchIconName } from "./WorkbenchIcon";

export type ToastKind = "error" | "fun" | "achievement";

/** What an unlocked achievement's popup shows, already translated (ADR-0089). */
export interface AchievementPopup {
  title: string;
  description: string;
  tier: AchievementTier;
  glyph: WorkbenchIconName;
}

export interface ToastEntry {
  id: number;
  kind: ToastKind;
  message: string;
  /** True when `message` quotes the server's own text verbatim (still
   * English, whatever the UI locale) — gates the B4 disclosure hint in
   * `Toast.tsx`. Meaningless for `kind === "fun"`, which never quotes the
   * server. */
  serverText: boolean;
  /** Set on an achievement popup; absent on the "+N more" summary. */
  achievement?: AchievementPopup;
}

/** How many achievement popups one batch shows before summarising the rest. */
export const MAX_ACHIEVEMENT_POPUPS = 2;

/**
 * One batch of unlocks as toasts: up to two popups, then one summary line
 * for the rest. Opening a big project can unlock several achievements at
 * once, and a wall of popups would cover the work it is celebrating.
 */
export function planAchievementToasts(
  popups: AchievementPopup[],
  summary: (count: number) => string,
): { message: string; achievement?: AchievementPopup }[] {
  const shown = popups.slice(0, MAX_ACHIEVEMENT_POPUPS).map((achievement) => ({ message: achievement.title, achievement }));
  const rest = popups.length - shown.length;
  return rest > 0 ? [...shown, { message: summary(rest) }] : shown;
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
   * old `setError(null)`-then-`setError(...)` pattern at every call site.
   *
   * `serverText` defaults to `true`: a call site that forgets to pass it
   * gets an over-disclosure (an honest hint on a fully-translated message)
   * rather than an under-disclosure (silently claiming a raw server
   * sentence is translated, which is the actual lie B4 exists to prevent).
   * Annoying beats dishonest. */
  function pushError(rawMessage: string, options?: { serverText?: boolean }) {
    const serverText = options?.serverText ?? true;
    const id = nextId.current++;
    setToasts((ts) => [
      ...ts.filter((t) => t.kind !== "error"),
      { id, kind: "error" as const, message: humorizeError(rawMessage), serverText },
    ]);
  }

  function clearErrors() {
    setToasts((ts) => ts.filter((t) => t.kind !== "error"));
  }

  function pushFun(message: string, autoDismissMs = 6000) {
    const id = nextId.current++;
    setToasts((ts) => [...ts, { id, kind: "fun" as const, message, serverText: false }]);
    setTimeout(() => dismiss(id), autoDismissMs);
  }

  /** Shows a batch of unlocks; each popup leaves by itself like a fun toast. */
  function pushAchievements(popups: AchievementPopup[], summary: (count: number) => string, autoDismissMs = 6000) {
    for (const planned of planAchievementToasts(popups, summary)) {
      const id = nextId.current++;
      setToasts((ts) => [...ts, { id, kind: "achievement" as const, serverText: false, ...planned }]);
      setTimeout(() => dismiss(id), autoDismissMs);
    }
  }

  return { toasts, pushError, clearErrors, pushFun, pushAchievements, dismiss };
}
