/** Shows until acknowledged that programming marks may lag a settled System-2 run. */
import { useTranslate } from "./i18n";
import { retrySystem2Refresh, useSystem2Status } from "./system2Status";

export default function System2BeliefNotice() {
  const t = useTranslate();
  const { belief } = useSystem2Status();
  if (belief === "current") return null;
  return <p role="status" className="form-warning" data-system2-belief={belief}>
    {t(belief === "refreshing" ? "system2.belief.refreshing" : "system2.belief.stale")}
    {belief === "stale" && <> <button type="button" data-system2-belief-retry onClick={retrySystem2Refresh}>{t("system2.belief.retry")}</button></>}
  </p>;
}
