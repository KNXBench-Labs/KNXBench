/** Expandable per-table entity list of a project diff with before and after values. */
import { createContext, useCallback, useContext, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FieldChange, MatchKind, ProjectDiffReport } from "./api";
import { useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { DIFF_FILTER_THRESHOLD, filterEntries, installationTables } from "./projectDiffView";
import type { DiffEntry, DiffTableView, EntryStatus, KeyFormat } from "./projectDiffView";
import { virtualWindow } from "./virtualWindow";

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

// Which tables of the shown report the user opened, by table id. A row of a
// long table unmounts when it scrolls out of the window (KL-60); this keeps
// its nested tables open when it comes back. One map per mount: the panel
// remounts this view for every comparison (`key={generation}`), so a new
// report starts collapsed.
const OpenTables = createContext<Map<string, boolean> | null>(null);

// A plain disclosure: a native button with `aria-expanded`, so Enter and
// Space come from the platform, collapsed until the user asks.
function Disclosure(props: { label: string; children: ReactNode; memoryKey?: string }) {
  const { memoryKey } = props;
  const openTables = useContext(OpenTables);
  const [expanded, setExpandedState] = useState(() => (memoryKey ? openTables?.get(memoryKey) : undefined) ?? false);
  const setExpanded = (toggle: (value: boolean) => boolean) => {
    const next = toggle(expanded);
    setExpandedState(next);
    if (memoryKey) openTables?.set(memoryKey, next);
  };
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

const STATUS_ORDER: EntryStatus[] = ["added", "removed", "changed", "ambiguous"];

// KL-60 windowing. Rows are measured once rendered (and re-measured when a
// nested table opens); rows not yet seen count with an estimate. The
// fallback viewport height applies before layout and in DOM-less tests.
const ROW_ESTIMATE_PX = 32;
const OVERSCAN_PX = 320;
const VIEWPORT_FALLBACK_PX = 448;

function PlainEntryList(props: { entries: DiffEntry[] }) {
  return (
    <ul className="project-diff-entries">
      {props.entries.map((entry) => (
        <li key={entry.id}>
          <EntryRow entry={entry} />
        </li>
      ))}
    </ul>
  );
}

function MeasuredEntry(props: {
  entry: DiffEntry;
  position: number;
  setSize: number;
  onHeight: (id: string, height: number) => void;
}) {
  const { entry, position, setSize, onHeight } = props;
  const ref = useRef<HTMLLIElement | null>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const report = () => {
      const height = element.getBoundingClientRect().height;
      if (height > 0) onHeight(entry.id, height);
    };
    report();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(report);
    observer.observe(element);
    return () => observer.disconnect();
  }, [entry.id, onHeight]);
  return (
    <li ref={ref} aria-posinset={position} aria-setsize={setSize}>
      <EntryRow entry={entry} />
    </li>
  );
}

// A long table: free-text and status filters, a live match count, and a
// scroll viewport that renders only the rows near the visible area. Every
// entry stays reachable by scrolling; nothing is paged away.
function FilteredEntryList(props: { entries: DiffEntry[] }) {
  const t = useTranslate();
  const { entries } = props;
  const [query, setQuery] = useState("");
  const [statuses, setStatuses] = useState<ReadonlySet<EntryStatus>>(new Set());
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(VIEWPORT_FALLBACK_PX);
  const [, setMeasured] = useState(0);
  const heights = useRef(new Map<string, number>());
  const viewportRef = useRef<HTMLDivElement | null>(null);
  // Real heights replace estimates as rows render. Two corrections keep the
  // view steady: growth of rows above the visible area is added to
  // `scrollTop`, and a list scrolled to its end stays at its end.
  const atEnd = useRef(false);
  const pendingShift = useRef(0);
  const layout = useRef<{ index: Map<string, number>; offsets: number[] }>({ index: new Map(), offsets: [] });

  const counts = useMemo(() => {
    const result: Record<EntryStatus, number> = { added: 0, removed: 0, changed: 0, ambiguous: 0 };
    for (const entry of entries) result[entry.status] += 1;
    return result;
  }, [entries]);
  const present = STATUS_ORDER.filter((status) => counts[status] > 0);
  const matching = useMemo(() => filterEntries(entries, query, statuses), [entries, query, statuses]);
  const rowHeights = matching.map((entry) => heights.current.get(entry.id) ?? ROW_ESTIMATE_PX);
  const rows = virtualWindow(rowHeights, scrollTop, viewportHeight, OVERSCAN_PX);
  const offsets = [0];
  for (const height of rowHeights) offsets.push(offsets[offsets.length - 1] + height);
  layout.current = { index: new Map(matching.map((entry, i) => [entry.id, i])), offsets };

  const onHeight = useCallback((id: string, height: number) => {
    const known = heights.current.get(id) ?? ROW_ESTIMATE_PX;
    if (heights.current.has(id) && Math.abs(known - height) < 0.5) return;
    const index = layout.current.index.get(id);
    const top = viewportRef.current?.scrollTop ?? 0;
    if (index !== undefined && layout.current.offsets[index] + known <= top) pendingShift.current += height - known;
    heights.current.set(id, height);
    setMeasured((version) => version + 1);
  }, []);

  useLayoutEffect(() => {
    const element = viewportRef.current;
    if (!element) return;
    if (element.clientHeight > 0) {
      setViewportHeight((current) => (current === element.clientHeight ? current : element.clientHeight));
    }
    if (atEnd.current) {
      element.scrollTop = element.scrollHeight;
    } else if (pendingShift.current !== 0) {
      element.scrollTop += pendingShift.current;
    }
    pendingShift.current = 0;
  });

  function backToTop() {
    setScrollTop(0);
    atEnd.current = false;
    pendingShift.current = 0;
    if (viewportRef.current) viewportRef.current.scrollTop = 0;
  }

  function toggleStatus(status: EntryStatus) {
    setStatuses((current) => {
      const next = new Set(current);
      if (!next.delete(status)) next.add(status);
      return next;
    });
    backToTop();
  }

  return (
    <>
      <div className="project-diff-filter">
        <input
          type="search"
          aria-label={t("projectDiff.filterLabel")}
          placeholder={t("projectDiff.filterPlaceholder")}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            backToTop();
          }}
          onKeyDown={(event) => {
            // Innermost first: Escape clears a typed filter and stops there;
            // on an empty field it reaches the panel and closes the report.
            if (event.key !== "Escape" || query === "") return;
            event.preventDefault();
            event.stopPropagation();
            setQuery("");
            backToTop();
          }}
        />
        {present.length > 1 && (
          <div role="group" aria-label={t("projectDiff.statusFilterGroup")} className="project-diff-status-filter">
            {present.map((status) => (
              <button key={status} type="button" aria-pressed={statuses.has(status)} onClick={() => toggleStatus(status)}>
                {t("projectDiff.statusFilterOption", { status: t(STATUS_KEYS[status]), count: counts[status] })}
              </button>
            ))}
          </div>
        )}
      </div>
      <p className="project-diff-match-count" role="status">
        {t("projectDiff.matchCount", { shown: matching.length, total: entries.length })}
      </p>
      {matching.length === 0 ? (
        <p className="project-diff-no-match">{t("projectDiff.noMatch")}</p>
      ) : (
        <div
          ref={viewportRef}
          className="project-diff-viewport"
          role="region"
          tabIndex={0}
          aria-label={t("projectDiff.viewportLabel")}
          onKeyDown={(event) => {
            // End/Home jump instantly and record the intent: a smooth native
            // jump would stop short once newly measured rows grow the list.
            if (event.target !== event.currentTarget || (event.key !== "End" && event.key !== "Home")) return;
            event.preventDefault();
            const element = event.currentTarget;
            atEnd.current = event.key === "End";
            element.scrollTop = event.key === "End" ? element.scrollHeight : 0;
            setScrollTop(element.scrollTop);
          }}
          onScroll={(event) => {
            const element = event.currentTarget;
            atEnd.current = element.scrollHeight > element.clientHeight &&
              element.scrollTop + element.clientHeight >= element.scrollHeight - 1;
            setScrollTop(element.scrollTop);
          }}
        >
          <ul className="project-diff-entries" style={{ paddingTop: rows.before, paddingBottom: rows.after }}>
            {matching.slice(rows.start, rows.end).map((entry, offset) => (
              <MeasuredEntry
                key={entry.id}
                entry={entry}
                position={rows.start + offset + 1}
                setSize={matching.length}
                onHeight={onHeight}
              />
            ))}
          </ul>
        </div>
      )}
    </>
  );
}

function EntryList(props: { entries: DiffEntry[] }) {
  return props.entries.length > DIFF_FILTER_THRESHOLD ? (
    <FilteredEntryList entries={props.entries} />
  ) : (
    <PlainEntryList entries={props.entries} />
  );
}

function TableSection(props: { view: DiffTableView }) {
  const t = useTranslate();
  const { view } = props;
  return (
    <Disclosure
      memoryKey={view.id}
      label={t("projectDiff.tableToggle", { table: t(view.labelKey), count: view.entries.length })}
    >
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
  // UI memory only, never part of the report.
  const [openTables] = useState(() => new Map<string, boolean>());

  return (
    <OpenTables.Provider value={openTables}>
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
    </OpenTables.Provider>
  );
}
