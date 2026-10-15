/** Chips and the explorer dot for parts of a device that are not programmed yet. */
// Rendered once per device row, thousands of times in a large project: no
// hook here subscribes to a store. Texts come from the caller (the table) or
// from one label function computed at the explorer's root.
import { createContext, useContext, useMemo } from "react";
import type { DeviceNode } from "./bindings/DeviceNode";
import { useTranslate, type Translate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { PART_LABELS, pendingParts, type ProgrammingPart } from "./programmingState";

const listFormats = new Map<string, Intl.ListFormat>();

/** "Address, Parameters and Group addresses", in `language`. */
export function partsText(t: Translate, language: string, parts: ProgrammingPart[]): string {
  let format = listFormats.get(language);
  if (!format) {
    format = new Intl.ListFormat(language, { type: "conjunction" });
    listFormats.set(language, format);
  }
  return format.format(parts.map((part) => t(PART_LABELS[part].long)));
}

/** A table-local renderer; equivalent states can share immutable presentation. */
export function createProgrammingChips(t: Translate, language: string) {
  // Four booleans plus unknown: at most seventeen entries, owned by this
  // table/locale. Reuse the immutable element too, so unrelated updates do
  // not reconcile and translate every identical chip subtree again.
  const views = new Map<string, ReturnType<typeof programmingChipView>>();
  return (device: Pick<DeviceNode, "pending_programming">) => {
    const key = device.pending_programming ? pendingParts(device).join(" ") : "unknown";
    let view = views.get(key);
    if (!view) {
      view = programmingChipView(device, t, language);
      views.set(key, view);
    }
    return view;
  };
}

/** Device view cell: one chip per pending part, nothing visible when none is pending. */
function programmingChipView(device: Pick<DeviceNode, "pending_programming">, t: Translate, language: string) {
  if (!device.pending_programming) {
    return <span className="programming-chips" data-state="unknown" role="img"
      aria-label={t("devices.programmingUnknown")} title={t("devices.programmingUnknown")}>—</span>;
  }
  const parts = pendingParts(device);
  if (parts.length === 0) {
    return <span className="programming-chips" data-state="clean" role="img" aria-label={t("devices.programmingClean")} />;
  }
  const label = t("devices.programmingPending", { parts: partsText(t, language, parts) });
  return <span className="programming-chips" data-state="pending" title={label} role="img" aria-label={label}>
    {parts.map((part) => <span key={part} className="programming-chip" data-part={part} aria-hidden="true">
      {t(PART_LABELS[part].short)}</span>)}
  </span>;
}

type PendingLabel = (parts: ProgrammingPart[]) => string;

/** The explorer's dot label, provided once at its root. */
export const PendingLabelContext = createContext<PendingLabel | null>(null);

/** The label function for the explorer root: one per language, memoised per combination. */
export function usePendingLabel(): PendingLabel {
  const t = useTranslate();
  const [language] = useUiLanguage();
  return useMemo(() => {
    const labels = new Map<string, string>();
    return (parts) => {
      const key = parts.join(" ");
      let label = labels.get(key);
      if (label === undefined) {
        label = t("explorer.pendingProgramming", { parts: partsText(t, language, parts) });
        labels.set(key, label);
      }
      return label;
    };
  }, [t, language]);
}

/** Explorer marker: a small dot when any part is pending, named for screen readers. */
export function PendingProgrammingDot({ device }: { device: Pick<DeviceNode, "pending_programming"> }) {
  const label = useContext(PendingLabelContext);
  const parts = pendingParts(device);
  if (!label || parts.length === 0) return null;
  const text = label(parts);
  return <span className="tree-pending-dot" role="img" aria-label={text} title={text} />;
}
