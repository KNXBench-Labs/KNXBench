/** Horizontal oldest-to-newest timeline of stored project versions and the working state. */
import { useEffect, useRef } from "react";
import { useTranslate } from "./i18n";
import { chronologicalVersions, type ProjectVersion } from "./projectHistory";

/** Selecting only reports the id; restore, delete and their confirmations stay in the dialog. */
export default function ProjectHistoryTimeline(props: {
  versions: readonly ProjectVersion[]; selectedId: number | null; disabled: boolean; language: string;
  undoSteps: number; redoSteps: number; onSelect: (id: number) => void;
}) {
  const t = useTranslate();
  const selectedRef = useRef<HTMLButtonElement | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  // Open at the newest end first, then reveal the selection, so a remount after Cancel keeps an older pick in view.
  useEffect(() => { if (scrollRef.current) scrollRef.current.scrollLeft = scrollRef.current.scrollWidth; }, []);
  useEffect(() => { selectedRef.current?.scrollIntoView?.({ inline: "nearest", block: "nearest" }); }, [props.selectedId]);
  const format = new Intl.DateTimeFormat(props.language === "de" ? "de" : "en", { dateStyle: "medium", timeStyle: "short" });
  return <section className="project-history-timeline-section" aria-labelledby="project-history-timeline-title">
    <h3 id="project-history-timeline-title">{t("projectHistory.timelineTitle")}</h3>
    <p className="project-history-timeline-note">{t("projectHistory.timelineNote")}</p>
    <div className="project-history-timeline-scroll" ref={scrollRef}>
      <ol className="project-history-timeline" aria-labelledby="project-history-timeline-title">
        {chronologicalVersions(props.versions).map((version) => <li key={version.id} data-version-id={version.id}>
          <button type="button" ref={props.selectedId === version.id ? selectedRef : undefined} aria-pressed={props.selectedId === version.id}
            disabled={props.disabled} onClick={() => props.onSelect(version.id)}>
            <strong>{version.label}</strong>{version.reason === "named" && <span>{t("projectHistory.reason.named")}</span>}
            <time dateTime={version.createdAt} title={version.createdAt}>{format.format(new Date(version.createdAt))}</time>
            <small>#{version.id}</small></button></li>)}
        <li className="project-history-timeline-current" data-timeline-current aria-current="step">
          <div><strong>{t("projectHistory.timelineCurrent")}</strong>
            <span>{t("projectHistory.timelineCurrentDetail", { undo: props.undoSteps, redo: props.redoSteps })}</span></div></li>
      </ol>
    </div>
  </section>;
}
