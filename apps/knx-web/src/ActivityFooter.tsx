/** Shows delayed concurrent activity and session history without hiding bus connection status. */
import { useId, useRef, useState } from "react";
import { useActivities, useActivityController } from "./ActivityContext";
import { activityHighlighted, activityTerminal, type ActivityEntry } from "./activityController";
import { useTranslate } from "./i18n";
import "./ActivityFooter.css";

export default function ActivityFooter({ canOpenDetails, onOpenDetails }: {
  canOpenDetails: (entry: ActivityEntry) => boolean;
  onOpenDetails: (entry: ActivityEntry) => void;
}) {
  const entries = useActivities();
  const controller = useActivityController();
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const regionId = useId();
  const toggle = useRef<HTMLButtonElement>(null);
  const highlighted = entries.filter(activityHighlighted);
  const summary = highlighted.find(row => row.state === "failed" && !row.acknowledged)
    ?? highlighted.find(row => !activityTerminal(row.state)) ?? highlighted.at(-1);
  const active = entries.filter(row => !activityTerminal(row.state));
  const recent = entries.filter(row => activityTerminal(row.state)).sort((a, b) => (b.finishedOrder ?? 0) - (a.finishedOrder ?? 0));
  function title(entry: ActivityEntry, compact = false) {
    const entities = entry.entities ?? [];
    const shown = compact ? entities.slice(0, 1) : entities;
    return [t(`backgroundActivity.kind.${entry.identity.kind}`), entry.source, entry.projectName,
      ...shown.map(entity => [entity.label, entity.address].filter(Boolean).join(" · ")),
      compact && entities.length > 1 ? t("backgroundActivity.moreDevices", { count: entities.length - 1 }) : undefined,
    ].filter(Boolean).join(" · ");
  }
  function description(entry: ActivityEntry) {
    return `${t(`backgroundActivity.state.${entry.state}`)}${entry.resultKey ? ` · ${t(entry.resultKey)}` : ""}${entry.phaseKey ? ` · ${t(entry.phaseKey)}` : entry.phase ? ` · ${entry.phase}` : ""}`;
  }
  function row(entry: ActivityEntry) {
    const moving = entry.visible && (entry.state === "running" || entry.state === "cancelling");
    return <li key={entry.id} data-activity-id={entry.id} data-state={entry.state}>
      <strong>{title(entry)}</strong>
      <span className={moving ? "task-status-text" : undefined}>{description(entry)}</span>
      {entry.progress && !activityTerminal(entry.state) && <span>{t("backgroundActivity.counted", entry.progress)}</span>}
      {(entry.error || entry.errorKey) && <span className="activity-footer-error">{entry.errorKey ? t(entry.errorKey) : entry.error}</span>}
      <div className="activity-footer-actions">
        {canOpenDetails(entry) && <button type="button" onClick={() => { setOpen(false); toggle.current?.focus(); onOpenDetails(entry); }}>{t("backgroundActivity.details")}</button>}
        {entry.state === "failed" && !entry.acknowledged && <button type="button" onClick={() => { toggle.current?.focus(); controller.acknowledge(entry.id); }}>{t("backgroundActivity.acknowledge")}</button>}
      </div>
    </li>;
  }
  return <section className="activity-footer" aria-label={t("backgroundActivity.title")}
    onKeyDown={event => { if (open && event.key === "Escape") { event.stopPropagation(); setOpen(false); toggle.current?.focus(); } }}>
    <span className="activity-footer-summary" data-state={summary?.state} role="status" aria-live="polite" aria-atomic="true">
      {summary && <span className={summary.state === "running" || summary.state === "cancelling" ? "task-status-text" : undefined}>
        {title(summary, true)} · {description(summary)}
        {highlighted.length > 1 ? ` · ${t("backgroundActivity.more", { count: highlighted.length - 1 })}` : ""}
      </span>}
    </span>
    <button ref={toggle} type="button" className="activity-footer-toggle" aria-expanded={open} aria-controls={regionId}
      onClick={() => setOpen(value => !value)}>{t("backgroundActivity.title")}{entries.length ? ` (${entries.length})` : ""}</button>
    {open && <div id={regionId} className="activity-footer-panel" role="region" aria-label={t("backgroundActivity.title")}>
      <h2>{t("backgroundActivity.title")}</h2>
      <p>{t("backgroundActivity.sessionOnly")}</p>
      {active.length > 0 && <><h3>{t("backgroundActivity.active")}</h3><ul>{active.map(row)}</ul></>}
      {recent.length > 0 && <><h3>{t("backgroundActivity.recent")}</h3><ul>{recent.map(row)}</ul></>}
      {entries.length === 0 && <p>{t("backgroundActivity.empty")}</p>}
      <button type="button" onClick={() => { setOpen(false); toggle.current?.focus(); }}>{t("backgroundActivity.close")}</button>
    </div>}
  </section>;
}
