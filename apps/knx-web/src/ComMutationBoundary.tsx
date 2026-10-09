/** Stable per-object mutation admission and lifetime-bound result publication. */
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
type Mutation = (operation: () => Promise<ProjectTree>) => Promise<ProjectTree | undefined>;
const MutationContext = createContext<Mutation>(async operation => operation());
export const useComMutation = () => useContext(MutationContext);
export default function ComMutationBoundary({ children, onBusy }: { children: ReactNode; onBusy: (busy: boolean) => void }) {
  const alive = useRef(true), running = useRef(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  async function run(operation: () => Promise<ProjectTree>): Promise<ProjectTree | undefined> {
    if (running.current || !alive.current) return undefined;
    running.current = true; setBusy(true); onBusy(true);
    try { const tree = await operation(); return alive.current ? tree : undefined; }
    finally { running.current = false; if (alive.current) { setBusy(false); onBusy(false); } }
  }
  return <MutationContext.Provider value={run}><fieldset className="com-editor-fields" disabled={busy} aria-busy={busy}>{children}</fieldset></MutationContext.Provider>;
}
