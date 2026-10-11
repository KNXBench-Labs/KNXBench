/** Field-local localized provenance, retaining unknown server layer names verbatim. */
import { useTranslate, type MessageKey } from "./i18n";
const LAYERS: Record<string, { name: MessageKey; help: MessageKey }> = {
  Program: { name: "valueSource.program", help: "valueSource.programHelp" },
  ProgramRef: { name: "valueSource.programRef", help: "valueSource.programRefHelp" },
  Instance: { name: "valueSource.instance", help: "valueSource.instanceHelp" },
  Inferred: { name: "valueSource.inferred", help: "valueSource.inferredHelp" },
  UserEdit: { name: "valueSource.userEdit", help: "valueSource.userEditHelp" },
};
export default function ValueProvenance({ layer }: { layer: string | null }) {
  const t = useTranslate();
  if (layer === null) return null;
  const keys = Object.hasOwn(LAYERS, layer) ? LAYERS[layer] : undefined;
  return <span className="provenance-badge" data-source-layer={layer}
    title={t(keys?.help ?? "valueSource.unknownHelp")}>
    {t("valueSource.label", { source: keys ? t(keys.name) : layer })}
  </span>;
}
