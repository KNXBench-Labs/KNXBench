/** Device editor marks for parts not programmed yet: a heading summary and affected-tab notes. */
// Issue #40, WP-37 step 1. Reads the tree's existing per-part flags only; an
// absent `pending_programming` (unknown) shows nothing here, the Inspector's
// programming section already says the state is not reported.
import { useMemo } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { partsText } from "./ProgrammingChips";
import { pendingParts, type ProgrammingPart } from "./programmingState";
import { collectDevices } from "./treeUtils";
import { useUiLanguage } from "./uiLanguage";

/** The device editor's tabs, in display order; `DeviceWorkspace` labels them. */
export const DEVICE_TABS = ["objects", "parameters", "identity", "diagnostics", "restricted", "instructions"] as const;
export type DeviceTab = (typeof DEVICE_TABS)[number];

/**
 * The parts each tab shows: communication objects carry the group-address
 * links, parameters the application's settings. Others show none.
 */
const TAB_PARTS: Partial<Record<DeviceTab, readonly ProgrammingPart[]>> = {
  objects: ["group_addresses"],
  parameters: ["application", "parameters"],
};

/** The device's pending parts in display order; none when the tree does not report them. */
export function useDevicePendingParts(tree: ProjectTree, deviceId: number): ProgrammingPart[] {
  return useMemo(() => {
    const device = collectDevices(tree).get(deviceId);
    return device ? pendingParts(device) : [];
  }, [tree, deviceId]);
}

/** The pending parts a tab shows, in display order. */
export function tabPendingParts(pending: readonly ProgrammingPart[], tab: DeviceTab): ProgrammingPart[] {
  const parts = TAB_PARTS[tab] ?? [];
  return pending.filter((part) => parts.includes(part));
}

/** One sentence naming `parts`, for the summary, a tab's tooltip and its description. */
export function usePendingSentence(): (parts: ProgrammingPart[]) => string {
  const t = useTranslate();
  const [language] = useUiLanguage();
  return (parts) => t("deviceEditor.programmingPending", { parts: partsText(t, language, parts) });
}

/** The heading line; nothing when no part is pending or the state is unknown. */
export function DevicePendingSummary({ parts }: { parts: ProgrammingPart[] }) {
  const sentence = usePendingSentence();
  if (parts.length === 0) return null;
  return <p className="device-pending-summary" data-pending-parts={parts.join(" ")}>
    <span className="device-pending-dot" aria-hidden="true" />{sentence(parts)}
  </p>;
}
