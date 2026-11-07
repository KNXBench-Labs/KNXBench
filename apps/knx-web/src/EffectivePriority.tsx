/** Effective project priority row with its winning source, and the user's own priority edit. */
import type { EffectivePrioritySource } from "./bindings/EffectivePrioritySource";
import type { EffectivePriorityUnresolved } from "./bindings/EffectivePriorityUnresolved";
import type { EffectivePriorityView } from "./bindings/EffectivePriorityView";
import { useState } from "react";
import * as api from "./api";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useComMutation } from "./ComMutationBoundary";
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

function sourceText(t: Translate, view: EffectivePriorityView): string | null {
  const source: EffectivePrioritySource | null | undefined = view.source;
  // Unresolved without a selected literal has no source; a resolved value always must.
  if (source === null || source === undefined) {
    return view.status === "resolved" ? t("inspector.effectivePriority.unrecognisedState") : null;
  }
  if (source.kind === "userEdit") return t("inspector.effectivePriority.source.userEdit");
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
  const source = view.status === "resolved" || view.status === "unresolved" ? sourceText(t, view) : null;
  return <div className="com-object-effective-priority" data-status={view.status}>
    <span className="saved-priority-label">{t("inspector.effectivePriority.label")}</span>
    {value !== null && <code className="saved-priority-raw">{value}</code>}
    <span className="saved-priority-state">{text}</span>
    {source !== null && <span className="field-hint saved-priority-source">{source}</span>}
    <span className="field-hint">{t("inspector.effectivePriority.hint")}</span>
  </div>;
}

/** The user's current edit, read from the effective view: only `userEdit` names one. */
export function priorityEdit(view: EffectivePriorityView): api.ObjectPriorityToken | null {
  return view.status === "resolved" && view.source.kind === "userEdit" && VALUES.has(view.value)
    ? view.value
    : null;
}

/** Sets or clears the user's priority edit; programming refuses a value the product does not state. */
export function PriorityEditField(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const t = useTranslate();
  const mutate = useComMutation();
  const [error, setError] = useState<string | null>(null);
  const current = priorityEdit(com.effective_priority);

  async function change(value: string) {
    setError(null);
    const next = value === "" ? null : (value as api.ObjectPriorityToken);
    try {
      const tree = await mutate(() => api.setComObjectPriority(com.id, next));
      if (tree) onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  const id = `com-priority-edit-${com.id}`;
  return <div className="com-object-priority-edit">
    <label htmlFor={id}>{t("inspector.priorityEdit.label")}</label>
    <select id={id} value={current ?? ""} aria-describedby={`${id}-hint`}
      onChange={(event) => void change(event.target.value)}>
      <option value="">{t("inspector.priorityEdit.none")}</option>
      {(["Low", "High", "Alert"] as const).map((token) => <option key={token} value={token}>{token}</option>)}
    </select>
    <span id={`${id}-hint`} className="field-hint">{t("inspector.priorityEdit.hint")}</span>
    {error && <span className="field-error">{error}</span>}
  </div>;
}
