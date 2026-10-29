/** The one Report problem entry: provider, opener hook and the shortcut button. */
import { createContext, useContext, type ReactNode } from "react";
import { createPortal } from "react-dom";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useFileMenuItemProps } from "./FileMenu";
import { useTranslate } from "./i18n";
import type { ReportProblemRequest } from "./reportProblem";
import ReportProblemWizard from "./ReportProblemWizard";
import type { ToastNotification } from "./toast";

export type OpenReport = (request: ReportProblemRequest) => void;
const ReportProblem = createContext<OpenReport | null>(null);

/**
 * Mounted once by the main workbench, which owns the open request so the
 * command palette can open it too. Every report entry (File menu, palette,
 * download panels, device diagnostics) opens this single wizard, preselected
 * by its request; the diagnostics companion has none.
 */
export function ReportProblemProvider({ request, onOpen, onClose, tree, notifications, children }: {
  request: ReportProblemRequest | null;
  onOpen: OpenReport;
  onClose: () => void;
  tree: ProjectTree | null;
  notifications: readonly ToastNotification[];
  children: ReactNode;
}) {
  return <ReportProblem.Provider value={onOpen}>
    {children}
    {request && createPortal(
      <ReportProblemWizard request={request} tree={tree} notifications={notifications} onClose={onClose} />,
      document.body,
    )}
  </ReportProblem.Provider>;
}

/** Opens the wizard; `null` outside the workbench. */
export function useReportProblem(): OpenReport | null {
  return useContext(ReportProblem);
}

/**
 * A shortcut into the wizard. Inside the File menu it is a menu item; elsewhere
 * a plain button. `from` names the entry for tests and styling only.
 */
export function ReportProblemButton({ request, from, label, disabled = false }: {
  request: ReportProblemRequest;
  from: string;
  label?: string;
  disabled?: boolean;
}) {
  const t = useTranslate();
  const entry = useFileMenuItemProps();
  const open = useReportProblem();
  if (!open) return null;
  return <button type="button" {...entry} data-report-problem-open={from} disabled={disabled}
    onClick={() => open(request)}>{label ?? t("reportProblem.button")}</button>;
}
