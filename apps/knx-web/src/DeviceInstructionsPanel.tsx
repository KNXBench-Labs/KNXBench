/** Device-editor tab host for the installation instructions editor. */
// Host side of docs/contracts/instruction-editor-host.md. The editor owns
// draft, preview and apply; this host hands it the tree's real snapshot
// identity, or `null` (read-only) when the tree has none, and re-reads the
// authoritative project after every Apply, no-op included. The editor does
// not await `onApplied`, so a failed re-read is reported here, as a
// confirmed change whose refresh failed — never as a failed change.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstructionClient } from "./features/rtf/client";
import { InstructionEditor } from "./features/rtf/InstructionEditor";
import { useTranslate } from "./i18n";
import { snapshotContextFromTree } from "./snapshotIdentity";
import { useUiLanguage } from "./uiLanguage";

export interface DeviceInstructionsPanelProps {
  deviceId: number;
  tree: ProjectTree;
  /** The workbench's own snapshot publication (it refuses stale trees). */
  onApplied: (tree: ProjectTree) => void | Promise<void>;
  onDirtyChange?: (dirty: boolean) => void;
  /** Test seam; production uses the editor's authenticated HTTP client. */
  client?: InstructionClient;
}

export default function DeviceInstructionsPanel({ deviceId, tree, onApplied, onDirtyChange, client }: DeviceInstructionsPanelProps) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const context = snapshotContextFromTree(tree);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const mounted = useRef(true);
  const dirtyReport = useRef(onDirtyChange);
  dirtyReport.current = onDirtyChange;
  // The draft dies with this instance (other device, view or project), so the
  // host's draft flag must not outlive it.
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; dirtyReport.current?.(false); };
  }, []);
  const refresh = useCallback(async () => {
    setRefreshError(null);
    try {
      await onApplied(await api.currentProject());
    } catch (error) {
      if (mounted.current) setRefreshError(error instanceof Error ? error.message : String(error));
    }
  }, [onApplied]);

  return (
    <div className="device-instructions">
      {refreshError !== null && <p className="device-instructions-refresh" role="alert">{t("deviceInstructions.refreshFailed", { reason: refreshError })}</p>}
      <InstructionEditor deviceId={deviceId} context={context} language={uiLanguage === "de" ? "de" : "en"}
        client={client} onApplied={() => void refresh()} onDirtyChange={(dirty) => dirtyReport.current?.(dirty)} />
    </div>
  );
}
