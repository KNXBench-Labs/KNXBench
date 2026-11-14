/** Owns volatile comparison results and the explicit reviewed, cancellable read-only run. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { runComparisonQueue, type ComparisonEntry, type ComparisonTarget, type ComparisonTransport } from "./deviceComparisons";
import { projectActivityContext, type ActivityController, type ActivityHandle } from "./activityController";
import { useTranslate } from "./i18n";

export function useDeviceComparisons(tree: ProjectTree | null, activity: ActivityController) {
  const t = useTranslate();
  const scopeKey = tree ? JSON.stringify([tree.server_incarnation, tree.project_incarnation]) : null;
  // A withdrawn scope never regains authority, even if its incarnation later reappears.
  const liveScope = useRef({ key: scopeKey });
  if (liveScope.current.key !== scopeKey) liveScope.current = { key: scopeKey };
  const scope = liveScope.current;
  const [records, setRecords] = useState<{ scope: typeof scope; entries: Map<number, ComparisonEntry> }>({ scope, entries: new Map() });
  const [review, setReview] = useState<{ scope: typeof scope; targets: ComparisonTarget[]; started: boolean; connection?: { gateway: string; transport: ComparisonTransport } } | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [stopping, setStopping] = useState(false);
  const [progress, setProgress] = useState({ completed: 0, current: null as ComparisonTarget | null });
  const [error, setError] = useState<string | null>(null);
  const run = useRef<{ cancelled: boolean; activity: ActivityHandle } | null>(null);
  const activitySerial = useRef(0);
  const activityId = useRef<number | null>(null);
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; if (run.current) run.current.cancelled = true; };
  }, []);
  useEffect(() => {
    // Withdrawal cancels the queue, not the still-pending read's ownership.
    if (run.current) { run.current.cancelled = true; run.current.activity.update({ state: "cancelling" }); setStopping(true); }
  }, [scope]);

  function entry(id: number): ComparisonEntry | null {
    const value = tree && records.scope === scope ? records.entries.get(id) : null;
    if (!value) return null;
    return value.revision !== tree?.comparison_revisions?.[id] ? { ...value, tag: "outdated" } : value;
  }
  function open(targets: readonly { id: number; name: string; address: string | null }[]) {
    if (!tree || scope !== liveScope.current || run.current) return;
    activityId.current = null;
    setError(null); setSelected(null); setProgress({ completed: 0, current: null });
    setReview({ scope, started: false, targets: targets.map((target) => ({
      id: target.id, name: target.name, address: target.address, revision: tree.comparison_revisions?.[target.id],
    })) });
  }
  function cancel() {
    if (run.current) { run.current.cancelled = true; run.current.activity.update({ state: "cancelling" }); setStopping(true); }
  }
  function close() { if (run.current) cancel(); else { activityId.current = null; setReview(null); } }
  function inspect(target: { id: number; name: string; address: string | null }) {
    if (!tree || scope !== liveScope.current || run.current) return;
    open([target]); setSelected(target.id);
    setReview({ scope, started: true, targets: [{ ...target, revision: tree.comparison_revisions?.[target.id] }] });
  }
  async function confirm(gateway: string, transport: ComparisonTransport = "udp") {
    if (!tree || !review || review.scope !== scope || scope !== liveScope.current || review.started || run.current) return;
    if (!tree.server_incarnation || tree.project_incarnation === undefined) { setError(t("compare.noRevision")); return; }
    const owned = { cancelled: false, activity: activity.begin({ kind: "compare", operationId: String(++activitySerial.current),
      projectScope: scope, access: "read", entityId: review.targets.map(target => target.id).join(",") }, { destination: "comparison",
        ...projectActivityContext(tree), entities: review.targets.map(target => ({ label: target.name, address: target.address })),
      }) }; run.current = owned;
    activityId.current = owned.activity.id;
    let failed = false;
    const reviewed = review;
    setReview({ ...review, started: true, connection: { gateway, transport } }); setRunning(true); setStopping(false); setError(null);
    const live = () => mounted.current && run.current === owned && liveScope.current === reviewed.scope;
    try {
      await runComparisonQueue({ targets: reviewed.targets,
        scope: { serverIncarnation: tree.server_incarnation, projectIncarnation: tree.project_incarnation }, gateway,
        cancelled: () => !live() || owned.cancelled,
        transport, compare: transport === "tcp" ? api.compareDeviceTcp : api.compareDevice,
        describe: (code) => t(`compare.${code}`),
        result: (id, value) => { if (!live()) return;
          setRecords((previous) => ({ scope: reviewed.scope, entries: new Map([
            ...(previous.scope === reviewed.scope ? previous.entries : []), [id, value],
          ]) }));
          setSelected((previous) => previous ?? id);
        },
        progress: (completed, current) => {
          owned.activity.update({ progress: { completed: String(completed), total: String(reviewed.targets.length) } });
          if (live()) setProgress({ completed, current });
        },
      });
    } catch (reason) {
      failed = true;
      owned.activity.finish(!live() || owned.cancelled ? "cancelled" : "failed", live() && !owned.cancelled ? api.errorMessage(reason) : undefined);
      if (live()) setError(api.errorMessage(reason));
    }
    finally {
      if (!failed) owned.activity.finish(owned.cancelled || !live() ? "cancelled" : "succeeded");
      if (run.current === owned) {
        run.current = null;
        if (mounted.current) { setRunning(false); setStopping(false); }
      }
    }
  }
  return { entry, open, inspect, close, cancel, confirm, selected, setSelected, running, stopping, progress, error,
    ownsActivity: (id: number) => activityId.current === id && scope === liveScope.current,
    activityId: activityId.current, review: review?.scope === scope ? review : null };
}
export type DeviceComparisons = ReturnType<typeof useDeviceComparisons>;
