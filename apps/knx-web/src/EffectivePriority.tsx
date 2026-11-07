/** Read-only row for an object's effective project priority and its winning source. */
import type { EffectivePrioritySource } from "./bindings/EffectivePrioritySource";
import type { EffectivePriorityUnresolved } from "./bindings/EffectivePriorityUnresolved";
import type { EffectivePriorityView } from "./bindings/EffectivePriorityView";
import { useTranslate, type MessageKey, type Translate } from "./i18n";

const REASON_KEYS: Record<EffectivePriorityUnresolved, MessageKey> = {
  InstanceUnavailable: "inspector.effectivePriority.unresolved.InstanceUnavailable",
  InstanceEmpty: "inspector.effectivePriority.unresolved.InstanceEmpty",
  InstanceUnknown: "inspector.effectivePriority.unresolved.InstanceUnknown",
  ProductNotConsulted: "inspector.effectivePriority.unresolved.ProductNotConsulted",
  NoProgramReference: "inspector.effectivePriority.unresolved.NoProgramReference",
  NoProductDatabase: "inspector.effectivePriority.unresolved.NoProductDatabase",
  ProgramNotInDatabase: "inspector.effectivePriority.unresolved.ProgramNotInDatabase",
  ProductObjectNotFound: "inspector.effectivePriority.unresolved.ProductObjectNotFound",
  ProductLiteralAbsent: "inspector.effectivePriority.unresolved.ProductLiteralAbsent",
  ProductEmpty: "inspector.effectivePriority.unresolved.ProductEmpty",
  ProductUnknown: "inspector.effectivePriority.unresolved.ProductUnknown",
};

const VALUES = new Set(["Low", "High", "Alert"]);

/** Own keys only: an inherited name such as `constructor` is not a known state. */
function own<T extends object>(table: T, key: string): key is Extract<keyof T, string> {
  return Object.prototype.hasOwnProperty.call(table, key);
}

function sourceText(t: Translate, source: EffectivePrioritySource | null | undefined): string | null {
  if (source === null || source === undefined) return null;
  if (source.kind === "instance") return t("inspector.effectivePriority.source.instance");
  if (source.kind === "product") {
    const layer = source.layer === "ProgramRef" ? t("inspector.effectivePriority.layer.ProgramRef")
      : source.layer === "Program" ? t("inspector.effectivePriority.layer.Program") : null;
    const omission = source.omission === "AttributeOmitted" ? t("inspector.effectivePriority.omission.AttributeOmitted")
      : source.omission === "InstanceElementOmitted" ? t("inspector.effectivePriority.omission.InstanceElementOmitted") : null;
    if (layer !== null && omission !== null) return t("inspector.effectivePriority.source.product", { layer, omission });
  }
  // A newer server's source is named as unknown, never attributed to a known layer.
  return t("inspector.effectivePriority.unrecognisedState");
}

function stateText(t: Translate, view: EffectivePriorityView): { text: string; value: string | null } {
  switch (view.status) {
    case "resolved":
      return VALUES.has(view.value)
        ? { text: t("inspector.effectivePriority.resolved"), value: view.value }
        : { text: t("inspector.effectivePriority.unrecognisedState"), value: JSON.stringify(view.value) };
    case "unresolved": {
      const text = own(REASON_KEYS, view.reason) ? t(REASON_KEYS[view.reason]) : t("inspector.effectivePriority.unrecognisedState");
      // Exact literal, quoted so empty, whitespace and case differences stay visible.
      return { text, value: typeof view.raw === "string" ? JSON.stringify(view.raw) : null };
    }
    default:
      return { text: t("inspector.effectivePriority.unrecognisedState"), value: null };
  }
}

export function EffectivePriorityRow(props: { priority: EffectivePriorityView }) {
  const t = useTranslate();
  const view = props.priority;
  const { text, value } = stateText(t, view);
  const source = view.status === "resolved" || view.status === "unresolved" ? sourceText(t, view.source) : null;
  return <div className="com-object-saved-priority com-object-effective-priority" data-status={view.status}>
    <span className="saved-priority-label">{t("inspector.effectivePriority.label")}</span>
    {value !== null && <code className="saved-priority-raw">{value}</code>}
    <span className="saved-priority-state">{text}</span>
    {source !== null && <span className="field-hint saved-priority-source">{source}</span>}
    <span className="field-hint">{t("inspector.effectivePriority.hint")}</span>
  </div>;
}
