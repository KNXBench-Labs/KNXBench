/** Expandable per-table entity list of a project diff with before and after values. */
import { useEffect, useId, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FieldChange, MatchKind, ProjectDiffReport } from "./api";
import { useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { DIFF_PAGE_SIZE, installationTables } from "./projectDiffView";
import type { DiffEntry, DiffTableView, EntryStatus, KeyFormat } from "./projectDiffView";

// Status is always spelled out as a word; the symbol is decoration for
// sighted users scanning a long list and is hidden from screen readers,
// and colour (see `styles.css`) only reinforces both.
const STATUS_SYMBOLS: Record<EntryStatus, string> = {
  added: "+",
  removed: "−",
  changed: "~",
  ambiguous: "?",
};

const STATUS_KEYS: Record<EntryStatus, MessageKey> = {
  added: "projectDiff.entityStatus.added",
  removed: "projectDiff.entityStatus.removed",
  changed: "projectDiff.entityStatus.changed",
  ambiguous: "projectDiff.entityStatus.ambiguous",
};

const MATCHED_BY_KEYS: Record<MatchKind, MessageKey> = {
  etsId: "projectDiff.matchedBy.etsId",
  naturalKey: "projectDiff.matchedBy.naturalKey",
};

// A plain disclosure: a native button with `aria-expanded`, so Enter and
// Space come from the platform, collapsed until the user asks.
function Disclosure(props: { label: string; children: ReactNode }) {
  const [expanded, setExpanded] = useState(false);
  const contentId = useId();
  return (
    <div className="project-diff-disclosure">
      <button
        type="button"
        className="project-diff-toggle"
        aria-expanded={expanded}
        aria-controls={contentId}
        onClick={() => setExpanded((value) => !value)}
      >
        <span aria-hidden="true">{expanded ? "▾" : "▸"} </span>
        {props.label}
      </button>
      {expanded && (
        <div id={contentId} className="project-diff-disclosure-content">
          {props.children}
        </div>
      )}
    </div>
  );
}

function FieldChangeTable(props: { changes: FieldChange[] }) {
  const t = useTranslate();
  return (
    <table className="project-diff-fields">
      <thead>
        <tr>
          <th scope="col">{t("projectDiff.fieldHeader")}</th>
          <th scope="col">{t("projectDiff.beforeHeader")}</th>
          <th scope="col">{t("projectDiff.afterHeader")}</th>
        </tr>
      </thead>
      <tbody>
        {props.changes.map((change) => (
          <tr key={change.field}>
            <th scope="row">
              <code>{change.field}</code>
            </th>
            <td>{change.left}</td>
            <td>{change.right}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function EntryRow(props: { entry: DiffEntry }) {
  const t = useTranslate();
  const { entry } = props;
  return (
    <>
      <span className={`project-diff-status project-diff-status-${entry.status}`}>
        <span aria-hidden="true">{STATUS_SYMBOLS[entry.status]} </span>
        {t(STATUS_KEYS[entry.status])}
      </span>{" "}
      <span className="project-diff-key">{entry.keyLabel}</span>
      {entry.detail && <span className="project-diff-detail"> — {entry.detail}</span>}
      {entry.matchedBy && (
        <span className="project-diff-meta"> ({t(MATCHED_BY_KEYS[entry.matchedBy])})</span>
      )}
      {entry.ambiguity && (
        <span className="project-diff-meta">
          {" "}
          ({t("projectDiff.ambiguityCandidates", {
            left: entry.ambiguity.leftCandidates,
            right: entry.ambiguity.rightCandidates,
          })})
        </span>
      )}
      {entry.fieldChanges.length > 0 && <FieldChangeTable changes={entry.fieldChanges} />}
      {entry.nested.map((view) => (
        <TableSection key={view.id} view={view} />
      ))}
    </>
  );
}

// Renders at most `visible` rows. "Show more" moves focus to the first
// newly revealed row, so a keyboard user is not dropped on `<body>` when
// the control disappears after the last page.
function EntryList(props: { entries: DiffEntry[] }) {
  const t = useTranslate();
  const { entries } = props;
  const [visible, setVisible] = useState(DIFF_PAGE_SIZE);
  const [focusIndex, setFocusIndex] = useState<number | null>(null);
  const itemRefs = useRef<(HTMLLIElement | null)[]>([]);

  useEffect(() => {
    if (focusIndex === null) return;
    itemRefs.current[focusIndex]?.focus();
    setFocusIndex(null);
  }, [focusIndex]);

  const shown = entries.slice(0, visible);
  const remaining = entries.length - shown.length;

  function showMore() {
    setFocusIndex(visible);
    setVisible((count) => count + DIFF_PAGE_SIZE);
  }

  return (
    <>
      <ul className="project-diff-entries">
        {shown.map((entry, index) => (
          <li
            key={entry.id}
            tabIndex={-1}
            ref={(element) => {
              itemRefs.current[index] = element;
            }}
          >
            <EntryRow entry={entry} />
          </li>
        ))}
      </ul>
      {remaining > 0 && (
        <p className="project-diff-more">
          <span>{t("projectDiff.shownOf", { shown: shown.length, total: entries.length })} </span>
          <button type="button" onClick={showMore}>
            {t("projectDiff.showMore", { count: Math.min(remaining, DIFF_PAGE_SIZE) })}
          </button>
        </p>
      )}
    </>
  );
}

function TableSection(props: { view: DiffTableView }) {
  const t = useTranslate();
  const { view } = props;
  return (
    <Disclosure label={t("projectDiff.tableToggle", { table: t(view.labelKey), count: view.entries.length })}>
      <EntryList entries={view.entries} />
    </Disclosure>
  );
}

function InfoSection(props: { labelKey: MessageKey; changes: FieldChange[] }) {
  const t = useTranslate();
  if (props.changes.length === 0) return null;
  return (
    <Disclosure label={t("projectDiff.tableToggle", { table: t(props.labelKey), count: props.changes.length })}>
      <FieldChangeTable changes={props.changes} />
    </Disclosure>
  );
}

/** Collapsed-by-default detail view below the grouped-count summary. */
export default function ProjectDiffDetails(props: { report: ProjectDiffReport }) {
  const t = useTranslate();
  const formatGroupAddress = useGroupAddressFormat();
  const { report } = props;
  const format: KeyFormat = {
    groupAddress: formatGroupAddress,
    unaddressed: t("projectDiff.key.unaddressed"),
  };
  const multiple = report.installations.length > 1;

  return (
    <section className="project-diff-details" aria-label={t("projectDiff.detailsHeading")}>
      <InfoSection labelKey="projectDiff.projectInfo" changes={report.infoChanges} />
      {report.installations.map((installation) => {
        const tables = installationTables(installation, format);
        if (installation.fieldChanges.length === 0 && tables.length === 0) return null;
        return (
          <div key={installation.id} className="project-diff-installation">
            {multiple && <h3>{t("projectDiff.installationHeading", { id: installation.id })}</h3>}
            <InfoSection labelKey="projectDiff.installationInfo" changes={installation.fieldChanges} />
            {tables.map((view) => (
              <TableSection key={view.id} view={view} />
            ))}
          </div>
        );
      })}
    </section>
  );
}
