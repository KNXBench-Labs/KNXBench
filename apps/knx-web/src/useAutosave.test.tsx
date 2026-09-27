/** Fake-timer tests for ISSUE-04's autosave engine: the five-second countdown, manual-save cancellation, edits mid-countdown, missing Save-As path, concurrent-save suppression, and autosave failure. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAutosave } from "./useAutosave";
import type { UseAutosaveOptions } from "./useAutosave";

let host: HTMLDivElement | undefined;

function Harness(props: UseAutosaveOptions & { onResult: (r: ReturnType<typeof useAutosave>) => void }) {
  const { onResult, ...options } = props;
  const result = useAutosave(options);
  onResult(result);
  return null;
}

async function mount(options: UseAutosaveOptions) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  let latest: ReturnType<typeof useAutosave> | undefined;
  await act(async () => {
    root.render(<Harness {...options} onResult={(r) => { latest = r; }} />);
  });
  return {
    root,
    rerender: async (next: UseAutosaveOptions) => {
      await act(async () => {
        root.render(<Harness {...next} onResult={(r) => { latest = r; }} />);
      });
    },
    get current() {
      return latest!;
    },
  };
}

function baseOptions(overrides: Partial<UseAutosaveOptions> = {}): UseAutosaveOptions {
  return {
    enabled: true,
    intervalMinutes: 5,
    hasStorePath: true,
    isModified: true,
    onSave: vi.fn().mockResolvedValue(undefined),
    onSaveFailed: vi.fn(),
    countdownSeconds: 5,
    ...overrides,
  };
}

describe("useAutosave", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(async () => {
    if (host) {
      await act(async () => {
        // Unmounting clears the interval timers; nothing left running
        // between tests.
        host!.remove();
      });
      host = undefined;
    }
    vi.useRealTimers();
  });

  it("starts a five-second countdown once the interval elapses, and calls onSave when it reaches zero", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));
    expect(harness.current.secondsRemaining).toBeNull();

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);

    await act(async () => {
      vi.advanceTimersByTime(5_000);
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(harness.current.secondsRemaining).toBeNull();
  });

  it("cancelCountdown stops the current cycle's save without disarming the next cycle", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);

    await act(async () => {
      harness.current.cancelCountdown();
    });
    expect(harness.current.secondsRemaining).toBeNull();

    await act(async () => {
      // The rest of the original countdown must not fire a save even
      // once the interval that was left over elapses.
      vi.advanceTimersByTime(5_000);
      await Promise.resolve();
    });
    expect(onSave).not.toHaveBeenCalled();

    // But the next cycle is still armed.
    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);
  });

  it("a manual save calling cancelCountdown mid-countdown prevents the autosave's own save from also firing", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);

    // Simulate the toolbar's Save button, which the real wiring calls
    // `cancelCountdown` from before its own save.
    await act(async () => {
      harness.current.cancelCountdown();
      vi.advanceTimersByTime(5_000);
      await Promise.resolve();
    });
    expect(onSave).not.toHaveBeenCalled();
  });

  it("skips the cycle silently when there is no Save-As path yet", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(
      baseOptions({ onSave, hasStorePath: false, intervalMinutes: 5, countdownSeconds: 5 }),
    );

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
    });
    expect(harness.current.secondsRemaining).toBeNull();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("skips the cycle silently when nothing is modified", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(
      baseOptions({ onSave, isModified: false, intervalMinutes: 5, countdownSeconds: 5 }),
    );

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
    });
    expect(harness.current.secondsRemaining).toBeNull();
    expect(onSave).not.toHaveBeenCalled();
  });

  it("an edit landing mid-countdown does not reset the countdown already in flight", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);

    // isModified flips false-then-true mid-countdown (an edit landed);
    // the countdown itself is read through refs and must not restart.
    await harness.rerender(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5, isModified: true }));
    await act(async () => {
      vi.advanceTimersByTime(2_000);
    });
    expect(harness.current.secondsRemaining).toBe(3);

    await act(async () => {
      vi.advanceTimersByTime(3_000);
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it("suppresses a concurrent save: two cycles racing only ever run one save at a time", async () => {
    let resolveSave: (() => void) | undefined;
    const onSave = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveSave = resolve;
        }),
    );
    await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledTimes(1);

    // The save is still in flight; the next cycle must not start a
    // second one even after its own full interval elapses.
    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveSave?.();
      await Promise.resolve();
      await Promise.resolve();
    });

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
      await Promise.resolve();
    });
    expect(onSave).toHaveBeenCalledTimes(2);
  });

  it("reports a failed autosave through onSaveFailed and still reschedules the next cycle", async () => {
    const onSave = vi.fn().mockRejectedValue(new Error("disk full"));
    const onSaveFailed = vi.fn();
    const harness = await mount(
      baseOptions({ onSave, onSaveFailed, intervalMinutes: 5, countdownSeconds: 5 }),
    );

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000);
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(onSaveFailed).toHaveBeenCalledTimes(1);
    expect(onSaveFailed.mock.calls[0][0]).toBeInstanceOf(Error);

    // The failure must not wedge autosave: the next cycle still offers.
    // Advance to where the next countdown *begins* — one interval minus the
    // countdown window — not a whole interval, which would run that countdown
    // to zero and fire a second save, leaving `secondsRemaining` null again.
    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5 * 1_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);
  });

  it("disabling autosave clears an in-flight countdown and arms no new one", async () => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const harness = await mount(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5 }));

    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 - 5_000);
    });
    expect(harness.current.secondsRemaining).toBe(5);

    await harness.rerender(baseOptions({ onSave, intervalMinutes: 5, countdownSeconds: 5, enabled: false }));
    expect(harness.current.secondsRemaining).toBeNull();

    await act(async () => {
      vi.advanceTimersByTime(60 * 60_000);
      await Promise.resolve();
    });
    expect(onSave).not.toHaveBeenCalled();
  });
});
