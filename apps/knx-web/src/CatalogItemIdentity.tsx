/** Read-only identity and provenance shared by catalogue rows and cleanup previews. */
import type { CatalogItem } from "./api";
import { useTranslate } from "./i18n";

export default function CatalogItemIdentity({ item }: { item: CatalogItem }) {
  const t = useTranslate();
  const unknown = t("catalog.details.unknown");
  return <span className="catalog-item-identity">
    <span>{t("catalog.details.references", { product: item.productRefId ?? unknown, binding: item.hardware2programRefId ?? unknown })}</span>
    {item.details ? <>
      <span>{t("catalog.details.application", { name: item.details.applicationName ?? unknown,
        id: item.details.applicationProgramId ?? unknown, version: item.details.applicationVersion ?? unknown })}</span>
      <span>{t("catalog.details.placement", { id: item.id, section: item.details.catalogSectionId ?? unknown })}</span>
      <span>{t("catalog.details.sources", { sources: item.details.sourceNames.length ? item.details.sourceNames.join(", ") : unknown })}</span>
    </> : <>
      <span>{t("catalog.details.placement", { id: item.id, section: unknown })}</span>
      <span>{t("catalog.details.unavailable")}</span>
    </>}
  </span>;
}
