/** Explicit per-selection admission/profile consent; never uploads from an effect. */
import { useContext, useState } from "react";
import { CatalogueWizardOrigin, useCatalogueJobs } from "./CatalogueJobsContext";
import CatalogueJobPanel from "./CatalogueJobPanel";
import { useTranslate } from "./i18n";
export default function CatalogueJobSelection({file,onDone}: {file: File; onDone: () => void}) {
 const {controller,state} = useCatalogueJobs(); const origin = useContext(CatalogueWizardOrigin); const t = useTranslate();
 const [profile,setProfile] = useState<"standard" | "large">("standard"); const [ack,setAck] = useState(false);
 const occupied = state.status !== null || state.pending || state.uncertain;
 return <div className="catalog-update-preview catalogue-job-selection" data-catalogue-job-selection>
  <h3>{file.name}</h3>
  {!occupied && <>
   <label>{t("catalogJob.profile")}<select value={profile} onChange={event => {setProfile(event.target.value as "standard" | "large");setAck(false);}}>
    <option value="standard">{t("catalogJob.profile.standard")}</option><option value="large">{t("catalogJob.profile.large")}</option>
   </select></label>
   <p>{t("catalogJob.bounds")}</p>
   <p className="catalog-update-warning">{t("catalogJob.growth")}</p>
   {profile === "large" && <label><input type="checkbox" checked={ack} onChange={event => setAck(event.target.checked)} />{t("catalogJob.largeRisk")}</label>}
   {state.previousUncertain && <p role="alert">{t("catalogJob.previousUnknown")}</p>}
   <button type="button" className="primary-action" disabled={profile === "large" && !ack} onClick={() => {void controller.start(file,profile,ack,origin);onDone();}}>{t("catalogJob.start")}</button>
  </>}
  {occupied && <><p>{t("catalogJob.occupied")}</p><CatalogueJobPanel /></>}
  <button type="button" onClick={onDone}>{t("catalogUpdate.cancel")}</button>
 </div>;
}
