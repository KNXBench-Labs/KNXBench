/** ISSUE-04's autosave engine: a repeating countdown-then-save timer. */
// It keeps no state of its own about *what* dirty means — that stays the
// server's `is_modified`/`last_saved_at`.
// This hook is deliberately blind to the save mechanics. It calls
// `onSave` — the same `saveProject` App.tsx already wires to Save's own
// button — and never invents a second persistence path (Global
// Constraints, ISSUE-04's own interface note). Its only two jobs are
// timing (when does a countdown start, when does it fire) and never
// pretending a save happened when it did not.
import { useEffect, useRef, useState } from "react";

export interface UseAutosaveOptions {
  /** Settings-panel toggle. `false` disarms the whole engine — no
   * countdown ever starts, and any in-flight one is cancelled. */
  enabled: boolean;
  intervalMinutes: number;
  /** No Save-As path yet means there is nowhere to autosave *to* — the
   * engine must never prompt a file dialog on the user's behalf. */
  hasStorePath: boolean;
  /** The server's own authoritative dirty bit (T13/ISSUE-04). An
   * autosave with nothing to save is simply skipped, silently. */
  isModified: boolean;
  /** Calls the same save the toolbar's Save button does and awaits its
   * outcome; rejecting means the save failed, exactly as a manual
   * failed save leaves dirty state and the last-saved time untouched
   * (server-side, `apps/knx-server/src/domain.rs`). */
  onSave: () => Promise<void>;
  /** Told about every failure so the shell can toast it exactly like a
   * manual save's own `reportError`. */
  onSaveFailed: (error: unknown) => void;
  countdownSeconds?: number;
  /** Injectable clock for tests; production leaves this as the real
   * `window.setTimeout`/`clearTimeout`. */
  setTimeoutFn?: typeof setTimeout;
  clearTimeoutFn?: typeof clearTimeout;
}

export interface UseAutosaveResult {
  /** Seconds left in an active countdown, or `null` when none is
   * running — the exact shape a countdown toast needs to decide whether
   * to render at all. */
  secondsRemaining: number | null;
  /** Cancels an in-flight countdown without saving. Also what a manual
   * Save call wires in, so a deliberate save never races an autosave
   * that was about to fire the same operation a second later. */
  cancelCountdown: () => void;
}

const MINUTE_MS = 60_000;
const SECOND_MS = 1_000;

/**
 * Runs one repeating cycle: wait `intervalMinutes`, minus the countdown
 * itself, then count down `countdownSeconds`, then call `onSave` if
 * there is still something to save. The cycle reschedules itself after
 * every countdown regardless of the save's outcome — a failed autosave
 * does not stop the next one from being offered (Review Focus: a save
 * failure must never lose edits, but it must not wedge autosave either).
 */
export function useAutosave(options: UseAutosaveOptions): UseAutosaveResult {
  const {
    enabled,
    intervalMinutes,
    hasStorePath,
    isModified,
    onSave,
    onSaveFailed,
    countdownSeconds = 5,
    setTimeoutFn = setTimeout,
    clearTimeoutFn = clearTimeout,
  } = options;

  const [secondsRemaining, setSecondsRemaining] = useState<number | null>(null);

  // Mirrored into refs so the single long-lived scheduling effect below
  // never has to tear down and rebuild its timers just because a prop
  // changed mid-cycle — it reads the latest value at the moment it
  // actually needs it (when the countdown finishes), not at the moment
  // the timer was armed.
  const hasStorePathRef = useRef(hasStorePath);
  hasStorePathRef.current = hasStorePath;
  const isModifiedRef = useRef(isModified);
  isModifiedRef.current = isModified;
  const onSaveRef = useRef(onSave);
  onSaveRef.current = onSave;
  const onSaveFailedRef = useRef(onSaveFailed);
  onSaveFailedRef.current = onSaveFailed;

  // Concurrent-save suppression: sensitive to *any* save in flight, not
  // only this hook's own — a manual Save started while a countdown was
  // running must not let the countdown's own save race it.
  const savingRef = useRef(false);

  const mainTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const remainingRef = useRef<number | null>(null);
  // Set by the scheduling effect below on every run; `cancelCountdown`
  // reads it through the ref so a cancel that interrupts a countdown can
  // re-arm the next cycle without needing scheduleNextCycle in its own
  // closure — that function only exists inside the effect. The boolean
  // parameter mirrors scheduleNextCycle's own (see below): a cancel
  // needs the full interval, never the shortened one.
  const scheduleNextCycleRef = useRef<((fullInterval: boolean) => void) | null>(null);

  function clearAllTimers() {
    if (mainTimerRef.current !== null) {
      clearTimeoutFn(mainTimerRef.current);
      mainTimerRef.current = null;
    }
    if (tickTimerRef.current !== null) {
      clearTimeoutFn(tickTimerRef.current);
      tickTimerRef.current = null;
    }
  }

  function setRemaining(value: number | null) {
    remainingRef.current = value;
    setSecondsRemaining(value);
  }

  function cancelCountdown() {
    // Only a countdown actually in flight needs cancelling — and only
    // that case needs re-arming afterward. A manual Save called with no
    // countdown running (the common case) must not touch the main timer
    // at all, or every ordinary Save click would restart the whole
    // interval from zero.
    if (tickTimerRef.current === null) return;
    clearTimeoutFn(tickTimerRef.current);
    tickTimerRef.current = null;
    setRemaining(null);
    // Interrupting a countdown — by Cancel or by a manual save that beat
    // it — must not leave the engine permanently silent: the next
    // interval still has to be offered (Review Focus: a save/autosave
    // interruption must never lose the ability to save again). It gets
    // a full interval, not the shortened interval-minus-countdown wait:
    // the countdown that was just cut short never got the chance to run
    // down, so nothing has been "spent" against the next interval yet.
    scheduleNextCycleRef.current?.(true);
  }

  useEffect(() => {
    let active = true;
    if (!enabled) {
      scheduleNextCycleRef.current = null;
      clearAllTimers();
      setRemaining(null);
      return;
    }

    // Every scheduling path except a cancel offers on the shortened
    // interval-minus-countdown wait: `intervalMinutes` measured as
    // time-to-offer, with the countdown itself carved out of it, so an
    // "every 5 minutes" autosave means five minutes from one offer to
    // the next — not five minutes plus a five-second countdown tacked
    // on top of it. `cancelCountdown` alone asks for the full interval
    // (see there): a cut-short countdown never got the chance to run
    // down, so nothing has been "spent" against the next one yet.
    function scheduleNextCycle(fullInterval = false) {
      // A save can finish after disable/unmount or a cadence change. Its
      // obsolete closure must not add a timer or overwrite the new cycle's.
      if (!active) return;
      const waitMs = fullInterval
        ? intervalMinutes * MINUTE_MS
        : Math.max(0, intervalMinutes * MINUTE_MS - countdownSeconds * SECOND_MS);
      mainTimerRef.current = setTimeoutFn(startCountdown, waitMs);
    }

    function startCountdown() {
      mainTimerRef.current = null;
      // Nothing to save, or nowhere to save it to: skip this cycle
      // silently and try again next time, rather than opening a file
      // dialog or counting down toward a save that cannot happen.
      if (!hasStorePathRef.current || !isModifiedRef.current || savingRef.current) {
        scheduleNextCycle();
        return;
      }
      setRemaining(countdownSeconds);
      tick(countdownSeconds);
    }

    function tick(remaining: number) {
      if (remaining <= 0) {
        setRemaining(null);
        tickTimerRef.current = null;
        void runSave();
        return;
      }
      setRemaining(remaining);
      tickTimerRef.current = setTimeoutFn(() => tick(remaining - 1), SECOND_MS);
    }

    async function runSave() {
      // The countdown reaching zero races nothing but a manual save
      // that started in the same tick; either way only one save may run.
      if (savingRef.current || !hasStorePathRef.current || !isModifiedRef.current) {
        scheduleNextCycle();
        return;
      }
      savingRef.current = true;
      try {
        await onSaveRef.current();
      } catch (error) {
        onSaveFailedRef.current(error);
      } finally {
        savingRef.current = false;
        scheduleNextCycle();
      }
    }

    scheduleNextCycle();
    scheduleNextCycleRef.current = scheduleNextCycle;
    return () => {
      active = false;
      scheduleNextCycleRef.current = null;
      clearAllTimers();
      setRemaining(null);
    };
    // Deliberately re-armed only by the inputs that change the *cadence*
    // of the cycle. `hasStorePath`/`isModified`/`onSave`/`onSaveFailed`
    // are read through refs precisely so an edit landing mid-countdown
    // does not reset the countdown it is supposed to be caught by
    // (ISSUE-04: "edits during the countdown").
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, intervalMinutes, countdownSeconds, setTimeoutFn, clearTimeoutFn]);

  return { secondsRemaining, cancelCountdown };
}
