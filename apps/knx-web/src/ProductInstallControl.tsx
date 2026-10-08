/** Product file picker shared by catalog and device wizard: packages and legacy databases. */
import { useState } from "react";
import * as api from "./api";
import type { CatalogInstallReport, LegacyInstallReport } from "./api";
import { CatalogInstallReportView } from "./CatalogInstallReport";
import { LegacyInstallReportView } from "./LegacyInstallReport";
import LegacyPasswordDialog from "./LegacyPasswordDialog";
import { useTranslate } from "./i18n";
import {
  installProductFile,
  legacyPasswordRefusal,
  type LegacyPasswordRefusal,
  type ProductInstallOutcome,
} from "./legacyInstall";

/** What the file picker offers: product packages and legacy product databases. */
export const PRODUCT_FILE_ACCEPT = ".knxprod,.vd3,.vd4,.vd5,application/zip";

export default function ProductInstallControl(props: {
  label: string;
  className: string;
  /** A package or database was installed: reload whatever lists it. */
  onInstalled: () => void | Promise<void>;
}) {
  const { label, className, onInstalled } = props;
  const t = useTranslate();
  const [installing, setInstalling] = useState(false);
  const [outcome, setOutcome] = useState<ProductInstallOutcome<CatalogInstallReport, LegacyInstallReport> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [prompt, setPrompt] = useState<{ file: File; reason: LegacyPasswordRefusal } | null>(null);

  async function install(file: File, legacy?: { password: string; remember: boolean }) {
    setError(null);
    setOutcome(null);
    setPrompt(null);
    setInstalling(true);
    try {
      setOutcome(await installProductFile(file, {
        installPackage: api.installProductPackage,
        installLegacy: api.installLegacyProductDatabase,
      }, legacy));
      await onInstalled();
    } catch (e) {
      const reason = legacyPasswordRefusal(e);
      if (reason) setPrompt({ file, reason });
      else setError(api.errorMessage(e));
    } finally {
      setInstalling(false);
    }
  }

  return (
    <>
      <label className={className}>
        {installing ? t("catalog.installing") : label}
        <input type="file" accept={PRODUCT_FILE_ACCEPT} disabled={installing}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void install(file);
            e.currentTarget.value = "";
          }} />
      </label>
      {outcome?.kind === "package" && <CatalogInstallReportView report={outcome.report} />}
      {outcome?.kind === "legacy" && <LegacyInstallReportView report={outcome.report} />}
      {error && <p className="field-error" role="alert">{error}</p>}
      {prompt && (
        <LegacyPasswordDialog fileName={prompt.file.name} reason={prompt.reason}
          onSubmit={(password, remember) => void install(prompt.file, { password, remember })}
          onCancel={() => setPrompt(null)} />
      )}
    </>
  );
}
