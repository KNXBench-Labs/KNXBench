/** U20: feeds the flow reducer from the monitor's own poll loop; one model per session. */
// The monitor panel owns polling (cursor, pause, identity guards). It hands
// every admitted batch to `admit`, which keeps one `FlowModel` per session
// identity, fetches each new generation's participant snapshot exactly once,
// and schedules one timer for the next value deadline. Nothing here polls,
// connects or writes.
import { useCallback, useEffect, useRef, useState } from "react";
import {
  admitRows,
  createFlowModel,
  expireSlots,
  nextExpiryAt,
  provideContext,
  type FlowModel,
  type FlowRowInput,
} from "./flowModel";
import type { FlowSnapshot } from "./flowWire";

export interface FlowIdentity {
  serverIncarnation: string;
  sessionId: number;
}

export interface FlowSourceDiagnostics {
  gaps: number; pruned: number; capacity?: number;
  context: "synced" | "stale" | "unverified";
  ended: boolean; error: string | null;
}
export interface FlowFeed {
  source?: FlowSourceDiagnostics;
  model: FlowModel | null;
  /** Changes whenever the model changed; renderers key on it. */
  version: number;
  admit(identity: FlowIdentity, rows: readonly FlowRowInput[]): void;
  reset(): void;
}

/** The monitor's clock: monotonic, unaffected by wall-clock changes. */
export const flowNow = (): number => performance.now();

const sameSession = (model: FlowModel, identity: FlowIdentity) =>
  model.identity.serverIncarnation === identity.serverIncarnation && model.identity.sessionId === identity.sessionId;

export function useFlowFeed(
  fetchSnapshot: (sessionId: number, generation: string) => Promise<FlowSnapshot>,
  projectScope?: string,
): FlowFeed {
  const modelRef = useRef<FlowModel | null>(null);
  const fetchRef = useRef(fetchSnapshot);
  fetchRef.current = fetchSnapshot;
  const scopeRef = useRef(projectScope);
  scopeRef.current = projectScope;
  const [version, setVersion] = useState(0);
  const bump = useCallback(() => setVersion((v) => v + 1), []);

  const admit = useCallback((identity: FlowIdentity, rows: readonly FlowRowInput[]) => {
    let model = modelRef.current;
    if (model === null || !sameSession(model, identity)) {
      model = createFlowModel(identity);
      modelRef.current = model;
    }
    const owner = model;
    for (const generation of admitRows(owner, rows, flowNow())) {
      const requestedScope = scopeRef.current;
      // The reply only ever reaches the model that asked (`owner`); after a
      // session change that model is no longer rendered, so a late reply
      // cannot touch the new session. A refused or failed snapshot draws
      // the waiting rows raw instead of leaving them queued forever.
      Promise.resolve()
        .then(() => fetchRef.current(identity.sessionId, generation))
        .then(
          (snapshot) => {
            if (!provideContext(owner, generation, snapshot, flowNow(), requestedScope)) provideContext(owner, generation, null, flowNow());
            bump();
          },
          () => {
            provideContext(owner, generation, null, flowNow());
            bump();
          },
        );
    }
    bump();
  }, [bump]);

  const reset = useCallback(() => {
    modelRef.current = null;
    bump();
  }, [bump]);

  // One timer for the earliest deadline. A late timer (background tab) is
  // harmless: renderers ask for values at the current time anyway.
  useEffect(() => {
    const model = modelRef.current;
    if (model === null) return;
    const next = nextExpiryAt(model);
    if (next === null) return;
    const id = setTimeout(() => {
      expireSlots(model, flowNow());
      bump();
    }, Math.max(0, next - flowNow()));
    return () => clearTimeout(id);
  }, [version, bump]);

  return { model: modelRef.current, version, admit, reset };
}
