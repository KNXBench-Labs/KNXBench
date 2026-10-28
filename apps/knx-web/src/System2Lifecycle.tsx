/** Application-level owner of the System-2 status lifetime and the post-settlement reload. */
import { useEffect, useRef } from "react";
import { currentProject } from "./api";
import System2BeliefNotice from "./System2BeliefNotice";
import type { ProjectTree } from "./bindings/ProjectTree";
import { isSystem2Active, markSystem2BeliefCurrent, markSystem2BeliefStale, refreshSystem2Status, useSystem2Status } from "./system2Status";

const POLL_MS = 1000;

/**
 * Mounted once at the application root, so it survives every tab and view switch. It polls only
 * while a known operation is active (never idle, never per row) and, once per settled end,
 * reloads the project and acknowledges it. A failed or superseded reload leaves the shared
 * belief explicitly stale until the user retries.
 */
export default function System2Lifecycle({ onTreeUpdate }: { onTreeUpdate: (tree: ProjectTree) => void | Promise<void> }) {
  const { status, belief, pendingKey } = useSystem2Status();
  const active = isSystem2Active(status);
  const update = useRef(onTreeUpdate); update.current = onTreeUpdate;
  const generation = useRef(0);

  useEffect(() => {
    if (!active) return;
    const timer = window.setInterval(() => void refreshSystem2Status(), POLL_MS);
    return () => window.clearInterval(timer);
  }, [active]);

  useEffect(() => {
    if (belief !== "refreshing" || pendingKey === null) return;
    const ticket = ++generation.current;
    const key = pendingKey;
    currentProject().then(async (tree) => {
      if (ticket !== generation.current) return;
      await update.current(tree);
      if (ticket === generation.current) markSystem2BeliefCurrent(key);
    }, () => { if (ticket === generation.current) markSystem2BeliefStale(key); })
      .catch(() => { if (ticket === generation.current) markSystem2BeliefStale(key); });
    return () => { generation.current += 1; };
  }, [belief, pendingKey]);
  return <System2BeliefNotice />;
}
