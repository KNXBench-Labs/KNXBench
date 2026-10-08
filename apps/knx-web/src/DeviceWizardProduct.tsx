/** Add-device wizard step 1: find a catalogue product or install a package first. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogInstallReport, CatalogItem, CatalogManufacturer } from "./api";
import { CatalogInstallReportView } from "./CatalogInstallReport";
import type { Translate } from "./i18n";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import { useProductLanguage } from "./productLanguage";

/** Same debounce the catalog workspace uses before asking the server. */
const SEARCH_DEBOUNCE_MS = 200;

export default function DeviceWizardProduct(props: {
  selected: CatalogItem | null;
  onSelect: (item: CatalogItem) => void;
  /** A package was installed: it stays installed whatever happens next. */
  onInstalled: () => void;
  t: Translate;
}) {
  const { selected, onSelect, onInstalled, t } = props;
  const [language] = useProductLanguage();
  const [manufacturers, setManufacturers] = useState<CatalogManufacturer[]>([]);
  const [manufacturer, setManufacturer] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [report, setReport] = useState<CatalogInstallReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);
  const [reload, setReload] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  useEffect(() => {
    api.catalogManufacturers().then(setManufacturers).catch((e) => setError(api.errorMessage(e)));
  }, [reload]);

  useEffect(() => {
    const handle = setTimeout(() => {
      const id = ++requestRef.current;
      api.catalogItems(manufacturer || undefined, search.trim() || undefined, language)
        .then((rows) => {
          if (id !== requestRef.current) return;
          setItems(rows);
          setLoaded(true);
        })
        .catch((e) => {
          if (id === requestRef.current) setError(api.errorMessage(e));
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(handle);
  }, [manufacturer, search, language, reload]);

  async function install(file: File) {
    setError(null);
    setReport(null);
    setInstalling(true);
    try {
      setReport(await api.installProductPackage(file));
      onInstalled();
      setReload((n) => n + 1);
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setInstalling(false);
    }
  }

  return (
    <div className="device-wizard-step">
      <p className="project-wizard-intro">{t("deviceWizard.product.intro")}</p>
      <div className="project-wizard-row">
        <select value={manufacturer} aria-label={t("deviceWizard.product.manufacturer")}
          onChange={(e) => setManufacturer(e.target.value)}>
          <option value="">{t("catalog.allManufacturers")}</option>
          {manufacturers.map((m) => <option key={m.id} value={m.id}>{m.name ?? m.id}</option>)}
        </select>
        <input ref={searchRef} type="search" value={search} aria-label={t("deviceWizard.product.search")}
          placeholder={t("catalog.searchPlaceholder")} onChange={(e) => setSearch(e.target.value)} />
      </div>
      {loaded && items.length === 0 && <p className="project-wizard-empty">{t("catalog.noMatches")}</p>}
      <ul className="device-wizard-results" aria-label={t("deviceWizard.product.results")}>
        {items.map((item) => (
          <li key={item.id}>
            <button type="button" aria-pressed={selected?.id === item.id}
              className={selected?.id === item.id ? "device-wizard-result selected" : "device-wizard-result"}
              onClick={() => onSelect(item)}>
              <span className="device-wizard-result-name">
                {item.name ?? item.id}
                {language !== null && fellBack(item.name, item.nameLanguage) && (
                  <LanguageFallbackBadge selected={language} source={item.sourceLanguage} />
                )}
              </span>
              {item.number && <span className="device-wizard-result-meta">{item.number}</span>}
              {item.visibleDescription && <span className="device-wizard-result-meta">{item.visibleDescription}</span>}
            </button>
          </li>
        ))}
      </ul>
      <label className="catalog-install device-wizard-install">
        {installing ? t("catalog.installing") : t("deviceWizard.product.install")}
        <input type="file" accept=".knxprod,application/zip" disabled={installing}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void install(file);
            e.currentTarget.value = "";
          }} />
      </label>
      {report && <CatalogInstallReportView report={report} />}
      {error && <p className="field-error" role="alert">{error}</p>}
    </div>
  );
}
