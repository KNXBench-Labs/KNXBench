/** Runs the once-per-project-session missing-name check after a project load (WP-51.3b). */
import { useEffect, useRef, useState } from "react";
import { autoOpenKey, createAutoOpenTracker, hasEligibleCandidates, shouldAutoOpen, type AutoOpenTree } from "./autoOpen";
import { scanMissingNames } from "./client";

/** What blocks an automatic open right now; read when the scan answers, not when it started. */
export type AutoOpenGate = Readonly<{ overlayOpen: boolean; saving: boolean }>;

export type MissingNamesAutoOpenOptions = Readonly<{
  /** Key of the project the App shows now, or null without one. */
  currentKey: () => string | null;
  gate: () => AutoOpenGate;
  /** Opens the wizard in its automatic form (with the Later button). */
  open: () => void;
  scan?: typeof scanMissingNames;
}>;

/**
 * Returns the check to run when a project lands. The cheap one-item scan is
 * silent on every failure; its answer opens the wizard only if the same
 * project is still shown, nothing else is open or saving, the project is not
 * read-only (Q17), and this project session has not opened it automatically before.
 */
export function useMissingNamesAutoOpen(options: MissingNamesAutoOpenOptions): (tree: AutoOpenTree) => void {
  const latest = useRef(options);
  latest.current = options;
  const [tracker] = useState(createAutoOpenTracker);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const check = useRef((tree: AutoOpenTree) => {
    const key = autoOpenKey(tree);
    const scan = latest.current.scan ?? scanMissingNames;
    let pending: Promise<Awaited<ReturnType<typeof scanMissingNames>>>;
    try { pending = scan({ offset: 0, limit: 1 }); } catch { return; }
    void pending.then((result) => {
      const now = latest.current;
      if (!mounted.current || now.currentKey() !== key || !hasEligibleCandidates(result)) return;
      const { overlayOpen, saving } = now.gate();
      if (!shouldAutoOpen({ key, openedFor: tracker.openedFor(), overlayOpen, saving, readOnly: result.readOnly })) return;
      tracker.markOpened(key);
      now.open();
    }, () => {});
  });
  return check.current;
}
