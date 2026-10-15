/** Owns volatile comparison results and the explicit reviewed, cancellable read-only run. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { runComparisonQueue, type ComparisonEntry, type ComparisonTarget } from "./deviceComparisons";
import { useTranslate } from "./i18n";

export function useDeviceComparisons(tree: ProjectTree) {
  const t = useTranslate();
  const scopeKey = JSON.stringify([tree.server_incarnation, tree.project_incarnation]);
  const liveScope = useRef(scopeKey); liveScope.current = scopeKey;
  const [records, setRecords] = useState<{ scope: string; entries: Map<number, ComparisonEntry> }>({ scope: scopeKey, entries: new Map() });
  const [review, setReview] = useState<{ scope: string; targets: ComparisonTarget[]; started: boolean } | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [progress, setProgress] = useState({ completed: 0, current: null as ComparisonTarget | null });
  const [error, setError] = useState<string | null>(null);
  const run = useRef<{ cancelled: boolean } | null>(null);
  useEffect(() => {
    setRunning(false); setStopping(false);
    return () => { if (run.current) run.current.cancelled = true; run.current = null; };
  }, [scopeKey]);

  function entry(id: number): ComparisonEntry | null {
    const value = records.scope === scopeKey ? records.entries.get(id) : null;
    if (!value) return null;
    return value.revision !== tree.comparison_revisions?.[id] ? { ...value, tag: "outdated" } : value;
  }
  function open(targets: readonly { id: number; name: string; address: string | null }[]) {
    if (running) return;
    setError(null); setSelected(null); setProgress({ completed: 0, current: null });
    setReview({ scope: scopeKey, started: false, targets: targets.map((target) => ({
      id: target.id, name: target.name, address: target.address, revision: tree.comparison_revisions?.[target.id],
    })) });
  }
  function cancel() {
    if (run.current) { run.current.cancelled = true; setStopping(true); }
  }
  function close() { if (running) cancel(); else setReview(null); }
  function inspect(target: { id: number; name: string; address: string | null }) {
    if (running) return;
    open([target]); setSelected(target.id);
    setReview({ scope: scopeKey, started: true, targets: [{ ...target, revision: tree.comparison_revisions?.[target.id] }] });
  }
  async function confirm(gateway: string) {
    if (!review || review.scope !== scopeKey || review.started || running) return;
    if (!tree.server_incarnation || tree.project_incarnation === undefined) { setError(t("compare.noRevision")); return; }
    const owned = { cancelled: false }; run.current = owned;
    const reviewed = review;
    setReview({ ...review, started: true }); setRunning(true); setStopping(false); setError(null);
    const live = () => run.current === owned && liveScope.current === reviewed.scope;
    try {
      await runComparisonQueue({ targets: reviewed.targets,
        scope: { serverIncarnation: tree.server_incarnation, projectIncarnation: tree.project_incarnation }, gateway,
        cancelled: () => !live() || owned.cancelled,
        compare: api.compareDevice,
        describe: (code) => t(`compare.${code}`),
        result: (id, value) => { if (!live()) return;
          setRecords((previous) => ({ scope: reviewed.scope, entries: new Map([
            ...(previous.scope === reviewed.scope ? previous.entries : []), [id, value],
          ]) }));
          setSelected((previous) => previous ?? id);
        },
        progress: (completed, current) => { if (live()) setProgress({ completed, current }); },
      });
    } catch (reason) { if (live()) setError(api.errorMessage(reason)); }
    finally { if (live()) { setRunning(false); setStopping(false); run.current = null; } }
  }
  return { entry, open, inspect, close, cancel, confirm, selected, setSelected, running, stopping, progress, error,
    review: review?.scope === scopeKey ? review : null };
}
export type DeviceComparisons = ReturnType<typeof useDeviceComparisons>;
