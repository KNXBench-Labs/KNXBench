/** Localized closed-set acquisition refusals; provider error text never becomes UI copy. */
import { useTranslate, type TranslatableKey } from "./i18n";
import type { OnlineErrorInfo } from "./onlineCatalogueApi";

const errors: Record<string, TranslatableKey> = {
  unavailable: "online.error.unavailable",
  unknownSource: "online.error.unavailable",
  invalidBaseUrl: "online.error.invalidBaseUrl",
  noIndex: "online.empty",
  unknownEntry: "online.error.selectionChanged",
  selectionChanged: "online.error.selectionChanged",
  notDownloadable: "online.unsupported",
  needsLogin: "online.error.login",
  tokenRejected: "online.error.login",
  invalidCredentials: "online.error.invalidCredentials",
  forbidden: "online.error.forbidden",
  notFound: "online.error.notFound",
  dailyLimit: "online.error.dailyLimit",
  rateLimited: "online.error.rateLimited",
  serviceUnavailable: "online.error.serviceUnavailable",
  busy: "online.error.busy",
  cancelled: "online.error.cancelled",
  transport: "online.error.transport",
  integrity: "online.error.integrity",
  vaultKeyMissing: "online.secretProblem",
  vaultOriginMismatch: "online.secretProblem",
  vault: "online.secretProblem",
  storage: "online.error.storage",
};

export default function OnlineCatalogueError({
  error,
}: {
  error: OnlineErrorInfo;
}) {
  const t = useTranslate();
  return (
    <div className="field-error online-catalogue-error" role="alert">
      <p>{t(errors[error.kind] ?? "online.error.transport")}</p>
      {error.resetAt && <p>{t("online.resetAt", { time: error.resetAt })}</p>}
      {error.retryAfterSecs != null && (
        <p>{t("online.retryAfter", { seconds: error.retryAfterSecs })}</p>
      )}
    </div>
  );
}
