/** App-lifetime injection; catalogue and wizard leaves subscribe without owning transport. */
import { createContext, useContext, useEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { CatalogueJobsController } from "./catalogueJobsController";
import CatalogueJobPanel from "./CatalogueJobPanel";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";
import "./CatalogueJobPanel.css";
const Context = createContext<CatalogueJobsController | null>(null);
export const CatalogueWizardOrigin = createContext<string | null>(null);
export function useCatalogueJobs() {
 const controller = useContext(Context);
 if (!controller) throw new Error("Catalogue jobs require the App-root provider");
 const state = useSyncExternalStore(controller.subscribe,controller.getSnapshot);
 return {controller,state};
}
export function useOptionalCatalogueJobs() {
 const controller = useContext(Context);
 const state = useSyncExternalStore(controller?.subscribe ?? (() => () => {}), controller?.getSnapshot ?? (() => null));
 return controller && state ? {controller,state} : null;
}
export function CatalogueJobsProvider({children, controller: supplied}: {children: ReactNode; controller?: CatalogueJobsController}) {
 const [controller] = useState(() => supplied ?? new CatalogueJobsController());
 useEffect(controller.connect,[controller]);
 return <Context.Provider value={controller}>{children}</Context.Provider>;
}
/** Always mounted in App, outside the conditional catalogue/wizard. No fetch on mount. */
export function CatalogueJobsRootNotice() {
 const {state} = useCatalogueJobs(); const t = useTranslate(); const [open,setOpen] = useState(false);
 const active = state.selecting || state.status?.state === "running" || state.uncertain;
 useEffect(() => {
  if (!active) return;
  const warn = (event: BeforeUnloadEvent) => { event.preventDefault(); event.returnValue = ""; };
  window.addEventListener("beforeunload",warn); return () => window.removeEventListener("beforeunload",warn);
 },[active]);
 if (!state.status && !state.selecting && !state.error) return null;
 return <>
  <aside className="catalogue-job-root" aria-label={t("catalogJob.title")}>
   <button type="button" onClick={() => setOpen(true)}>{t("catalogJob.title")} · {state.status ? t(`catalogJob.state.${state.status.state}`) : t("catalogJob.preparing")}</button>
   {state.committed && <span role="status">{t("catalogJob.committedNotice")}</span>}
  </aside>
  {open && createPortal(<Overlay labelledBy="catalogue-root-job-title" className="project-wizard catalogue-job-dialog" onClose={() => setOpen(false)}>
   <h2 id="catalogue-root-job-title">{t("catalogJob.title")}</h2><CatalogueJobPanel />
   <button type="button" onClick={() => setOpen(false)}>{t("catalogJob.hide")}</button>
  </Overlay>, document.body)}
  {/* Retention is explicit; hiding this view never cancels or disposes. */}
 </>;
}
