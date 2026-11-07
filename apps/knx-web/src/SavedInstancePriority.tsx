/** Read-only row showing an object's captured saved-project priority declaration, never wire priority. */
import type { CapturedSavedInstanceSource } from "./bindings/CapturedSavedInstanceSource";
import type { SavedInstancePriorityView } from "./bindings/SavedInstancePriorityView";
import type { SavedPriorityUnavailableView } from "./bindings/SavedPriorityUnavailableView";
import { useTranslate, type MessageKey, type Translate } from "./i18n";

const UNAVAILABLE_KEYS: Record<SavedPriorityUnavailableView, MessageKey> = {
  NotCaptured: "inspector.savedPriority.unavailable.NotCaptured",
  UnsupportedSourceSchema: "inspector.savedPriority.unavailable.UnsupportedSourceSchema",
  ModuleScope: "inspector.savedPriority.unavailable.ModuleScope",
  AmbiguousOwner: "inspector.savedPriority.unavailable.AmbiguousOwner",
  AmbiguousDeclaration: "inspector.savedPriority.unavailable.AmbiguousDeclaration",
  SourceIntegrity: "inspector.savedPriority.unavailable.SourceIntegrity",
  ResourceLimit: "inspector.savedPriority.unavailable.ResourceLimit",
  SourceBindingMismatch: "inspector.savedPriority.unavailable.SourceBindingMismatch",
};

/** Exact literal, quoted so empty, whitespace and case differences stay visible. */
function quoted(raw: string): string {
  return JSON.stringify(raw);
}

function sourceText(t: Translate, source: CapturedSavedInstanceSource): string {
  return t("inspector.savedPriority.source", {
    schema: source.source_schema,
    object: source.object_source.ets_id,
  });
}

function stateText(t: Translate, view: SavedInstancePriorityView): { text: string; value: string | null } {
  switch (view.status) {
    case "declared":
      switch (view.token_status) {
        case "Known": return { text: t("inspector.savedPriority.declared.known"), value: view.raw };
        case "Empty": return { text: t("inspector.savedPriority.declared.empty"), value: quoted(view.raw) };
        case "Unknown": return { text: t("inspector.savedPriority.declared.unknown"), value: quoted(view.raw) };
        default: return { text: t("inspector.savedPriority.unrecognisedState"), value: quoted(view.raw) };
      }
    case "absent":
      return {
        text: view.omission === "InstanceElementOmitted"
          ? t("inspector.savedPriority.absent.InstanceElementOmitted")
          : view.omission === "AttributeOmitted"
            ? t("inspector.savedPriority.absent.AttributeOmitted")
            : t("inspector.savedPriority.unrecognisedState"),
        value: null,
      };
    case "unavailable": {
      // Own keys only: an inherited name such as `constructor` is not a reason.
      const known = Object.prototype.hasOwnProperty.call(UNAVAILABLE_KEYS, view.reason);
      return { text: known ? t(UNAVAILABLE_KEYS[view.reason]) : t("inspector.savedPriority.unrecognisedState"), value: null };
    }
    default:
      // A newer server's state is named as unknown, never shown as a default.
      return { text: t("inspector.savedPriority.unrecognisedState"), value: null };
  }
}

export function SavedInstancePriorityRow(props: { priority: SavedInstancePriorityView }) {
  const t = useTranslate();
  const view = props.priority;
  const { text, value } = stateText(t, view);
  // Visible text, not a hover title, so keyboard, touch and screen readers reach it.
  const source = view.status === "declared" || view.status === "absent" ? sourceText(t, view.source) : null;
  return <div className="com-object-saved-priority" data-status={view.status}>
    <span className="saved-priority-label">{t("inspector.savedPriority.label")}</span>
    {value !== null && <code className="saved-priority-raw">{value}</code>}
    <span className="saved-priority-state">{text}</span>
    {source !== null && <span className="field-hint saved-priority-source">{source}</span>}
    <span className="field-hint">{t("inspector.savedPriority.hint")}</span>
  </div>;
}
