/** Toast queue state and holiday/late-night joke selection for startup and error toasts. */
import { useEffect, useRef, useState } from "react";
import { ERROR_WRAPPERS, HOLIDAYS, LATE_NIGHT_MESSAGES } from "./toastCopy";
import type { HolidayEntry } from "./toastCopy";
import { formatTemplate, translate } from "./i18n";
import { readableArchiveSizeError } from "./archiveSizeError";
import { emitAchievementEvent } from "./achievementEvents";
import type { TranslatableKey } from "./i18n";
import type { AchievementTier } from "./achievementCatalog";
import type { WorkbenchIconName } from "./WorkbenchIcon";
import type { LogEntry } from "./api";

/** Browser-session records; IDs are local, never backend log identities. */
export interface ToastNotification { id: number; entry: LogEntry }
export const MAX_NOTIFICATION_HISTORY = 1000;

export type ToastKind = "error" | "fun" | "achievement" | "update";

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
  /** Fixed-origin release action; ordinary toast details keep their existing owner. */
  releaseUrl?: string;
  /** The toast is playing its exit animation; `finishExit` removes it. */
  leaving?: boolean;
}

/** Shared visible duration before any toast starts its exit (ADR-0089). */
export const TOAST_MS = 9000;
export const ACHIEVEMENT_TOAST_MS = TOAST_MS;

/** Removes a leaving toast even when no exit animation reports its end
 * (motion switched off or reduced). Longer than any exit animation. */
export const TOAST_EXIT_FALLBACK_MS = 1000;

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

/** At most this many cards are on screen; their timers run, the rest wait. */
export const MAX_VISIBLE_TOASTS = 3;
/** Waiting cards beyond this retire oldest-first; the session log keeps them. */
export const MAX_QUEUED_TOASTS = 100;

/** Pause reasons: pointer over the card, focus inside it, document hidden. */
export type ToastPauseReason = "hover" | "focus" | "hidden";

interface ToastClock {
  kind: ToastKind;
  visible: boolean;
  remaining: number;
  started: number;
  timer?: ReturnType<typeof setTimeout>;
  exit?: ReturnType<typeof setTimeout>;
  pauses: Set<ToastPauseReason>;
}

export function useToasts() {
  const [entries, setToasts] = useState<ToastEntry[]>([]);
  const toasts = entries.slice(0, MAX_VISIBLE_TOASTS);
  const [queueDropped, setQueueDropped] = useState(0);
  const nextId = useRef(0);
  const timers = useRef(new Map<number, ToastClock>());
  const [history, setHistory] = useState<{ entries: ToastNotification[]; dropped: number }>({ entries: [], dropped: 0 });

  function remember(id: number, kind: ToastKind, message: string, detail: string | null) {
    const notification: ToastNotification = { id, entry: {
      timestamp: new Date().toISOString(), severity: kind === "error" ? "error" : "info",
      source: `ui:toast:${kind}`, message, detail, location: null,
    } };
    setHistory(previous => {
      const entries = [...previous.entries, notification];
      const excess = Math.max(0, entries.length - MAX_NOTIFICATION_HISTORY);
      return { entries: entries.slice(excess), dropped: previous.dropped + excess };
    });
  }

  function stopTimer(id: number) {
    const clock = timers.current.get(id);
    if (!clock || clock.timer === undefined) return;
    clearTimeout(clock.timer);
    clock.timer = undefined;
    clock.remaining = Math.max(0, clock.remaining - (Date.now() - clock.started));
  }

  /** The exit animation of `id` has ended (or the fallback fired). */
  function finishExit(id: number) {
    const clock = timers.current.get(id);
    if (clock) { clearTimeout(clock.timer); clearTimeout(clock.exit); }
    timers.current.delete(id);
    setToasts((ts) => ts.filter((t) => t.id !== id));
  }

  /** Every toast follows the same animated exit before removal. */
  function dismiss(id: number) {
    const clock = timers.current.get(id);
    if (!clock || clock.exit !== undefined) return;
    stopTimer(id);
    clock.exit = setTimeout(() => finishExit(id), TOAST_EXIT_FALLBACK_MS);
    setToasts((ts) => ts.map((t) => t.id === id ? { ...t, leaving: true } : t));
  }

  /** A card's lifetime runs only while it is on screen and nothing pauses it. */
  function startTimer(id: number) {
    const clock = timers.current.get(id);
    if (!clock || clock.timer !== undefined || clock.exit !== undefined || !clock.visible || clock.pauses.size) return;
    clock.started = Date.now();
    clock.timer = setTimeout(() => dismiss(id), clock.remaining);
  }

  function pause(id: number, reason: ToastPauseReason) {
    timers.current.get(id)?.pauses.add(reason);
    stopTimer(id);
  }

  function resume(id: number, reason: ToastPauseReason) {
    timers.current.get(id)?.pauses.delete(reason);
    startTimer(id);
  }

  useEffect(() => {
    const clocks = timers.current;
    const visibility = () => {
      for (const id of clocks.keys()) {
        if (document.visibilityState === "hidden") pause(id, "hidden");
        else resume(id, "hidden");
      }
    };
    document.addEventListener("visibilitychange", visibility);
    return () => {
      document.removeEventListener("visibilitychange", visibility);
      for (const clock of clocks.values()) { clearTimeout(clock.timer); clearTimeout(clock.exit); }
      clocks.clear();
    };
    // The handlers only touch refs; one registration per mount is intended.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const shown = new Set(entries.slice(0, MAX_VISIBLE_TOASTS).map((entry) => entry.id));
    for (const [id, clock] of timers.current) {
      clock.visible = shown.has(id);
      if (clock.visible) startTimer(id);
      else stopTimer(id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries]);

  function add(entry: Omit<ToastEntry, "id">, id: number, duration = TOAST_MS) {
    // Protect the cards on screen; on overflow retire the oldest waiting one.
    // The clock map's order is the queue's order (both append and remove in
    // step), so position decides, not the `visible` flag: a burst of pushes
    // runs before the effect that refreshes that flag.
    const retired = timers.current.size >= MAX_QUEUED_TOASTS
      ? [...timers.current.keys()][MAX_VISIBLE_TOASTS] : undefined;
    if (retired !== undefined) {
      timers.current.delete(retired);
      setQueueDropped((count) => count + 1);
    }
    timers.current.set(id, { kind: entry.kind, visible: false, remaining: duration, started: Date.now(),
      pauses: new Set<ToastPauseReason>(document.visibilityState === "hidden" ? ["hidden"] : []) });
    setToasts((previous) => [...previous.filter((t) => t.id !== retired), { id, ...entry }]);
  }

  /** Shows a new error card next to any unread one; nothing replaces it.
   *
   * `serverText` defaults to `true`: a call site that forgets to pass it
   * gets an over-disclosure (an honest hint on a fully-translated message)
   * rather than an under-disclosure (silently claiming a raw server
   * sentence is translated, which is the actual lie B4 exists to prevent).
   * Annoying beats dishonest. */
  function pushError(rawMessage: string, options?: { serverText?: boolean }) {
    const serverText = options?.serverText ?? true;
    const readable = serverText ? readableArchiveSizeError(rawMessage) : undefined;
    const id = nextId.current++;
    const message = readable ?? humorizeError(rawMessage);
    remember(id, "error", rawMessage, message);
    add({ kind: "error", message, serverText: readable ? false : serverText }, id);
    // Outside the state updater, which StrictMode may run twice.
    emitAchievementEvent({ type: "errorToastShown" });
  }

  /** Explicit clear by a call site (a later successful retry). The session
   * log keeps every cleared error; only the cards go. */
  function clearErrors() {
    setToasts((ts) => ts.filter((t) => t.kind !== "error"));
    for (const [id, clock] of timers.current) if (clock.kind === "error") {
      clearTimeout(clock.timer); clearTimeout(clock.exit);
      timers.current.delete(id);
    }
  }

  function pushFun(message: string, autoDismissMs = TOAST_MS) {
    const id = nextId.current++;
    remember(id, "fun", message, null);
    add({ kind: "fun", message, serverText: false }, id, autoDismissMs);
  }

  /** Shows a batch of unlocks; each popup leaves by itself like a fun toast. */
  function pushAchievements(
    popups: AchievementPopup[],
    summary: (count: number) => string,
    autoDismissMs = ACHIEVEMENT_TOAST_MS,
  ) {
    for (const planned of planAchievementToasts(popups, summary)) {
      const id = nextId.current++;
      remember(id, "achievement", planned.message, planned.achievement?.description ?? null);
      add({ kind: "achievement", serverText: false, ...planned }, id, autoDismissMs);
    }
  }

  function pushUpdate(message: string, releaseUrl: string) {
    const id = nextId.current++;
    remember(id, "update", message, releaseUrl);
    add({ kind: "update", message, releaseUrl, serverText: false }, id);
  }

  return { pushUpdate, toasts, pendingCount: Math.max(0, entries.length - MAX_VISIBLE_TOASTS), queueDropped,
    notifications: history.entries, notificationDroppedCount: history.dropped,
    pushError, clearErrors, pushFun, pushAchievements, dismiss, finishExit, pause, resume };
}
