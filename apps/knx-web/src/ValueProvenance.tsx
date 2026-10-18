/** Field-local localized provenance, retaining unknown server layer names verbatim. */
import { useTranslate, type MessageKey } from "./i18n";
const LAYERS: Record<string, { name: MessageKey; help: MessageKey }> = {
  Program: { name: "valueSource.program", help: "valueSource.programHelp" },
  ProgramRef: { name: "valueSource.programRef", help: "valueSource.programRefHelp" },
  Instance: { name: "valueSource.instance", help: "valueSource.instanceHelp" },
  Inferred: { name: "valueSource.inferred", help: "valueSource.inferredHelp" },
  UserEdit: { name: "valueSource.userEdit", help: "valueSource.userEditHelp" },
};
// A value stated on the device instance is the ordinary case for an imported
// project, so a field heading says nothing about it (UI-02). The layer stays in
// the model and API; a deliberately opened technical reference names it. Each
// layer is shown in exactly one place: the heading, or else the reference.
const ROUTINE_LAYERS: ReadonlySet<string> = new Set(["Instance"]);
export default function ValueProvenance({ layer, placement = "heading" }: {
  layer: string | null; placement?: "heading" | "reference";
}) {
  const t = useTranslate();
  if (layer === null || ROUTINE_LAYERS.has(layer) !== (placement === "reference")) return null;
  const keys = Object.hasOwn(LAYERS, layer) ? LAYERS[layer] : undefined;
  return <span className="provenance-badge" data-source-layer={layer}
    title={t(keys?.help ?? "valueSource.unknownHelp")}>
    {t("valueSource.label", { source: keys ? t(keys.name) : layer })}
  </span>;
}
