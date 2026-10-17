/** Source-group parameter workbench; navigation never owns values or write authority. */
import { useId, useState, type ReactNode } from "react";
import type { ModuleScope, ParameterField, ParameterSection } from "./api";
import { useTranslate } from "./i18n";
import { indexParameterWorkspace, matchesParameterSearch } from "./parameterWorkspace";
import { matrixStyle, planParameterMatrix, type GroupLayoutView } from "./parameterMatrix";
import "./parameterWorkspace.css";

interface Props {
  sections: readonly ParameterSection[];
  renderField: (field: ParameterField, scope: ModuleScope | null) => ReactNode;
}

export default function ParameterWorkspace({ sections, renderField }: Props) {
  const t = useTranslate();
  const id = useId();
  const [active, setActive] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const { rows, groups } = indexParameterWorkspace(sections);
  // Closed by default: no area opens until chosen; a vanished choice closes again.
  const activeKey = groups.some(group => group.key === active) ? active : null;
  const searching = query.trim() !== "";
  const closed = !searching && activeKey === null;
  const scopeLabel = (scope: ModuleScope | null) => scope === null ? t("parameters.deviceScope")
    : scope.moduleId ?? t("parameters.moduleNumber", { number: scope.moduleNode });
  const groupLabel = (group: typeof groups[number]) => group.ungrouped ? t("parameters.workspace.ungrouped")
    : group.label || t("parameters.groupNumber", { number: group.number });
  const visible = new Set(rows.filter(row => searching ? matchesParameterSearch(row, query) : row.groupKey === activeKey).map(row => row.key));
  const current = groups.find(group => group.key === activeKey);
  const selectedRow = rows.find(row => row.key === selected);
  const selectedField = selectedRow?.field;
  const label = (field: ParameterField) => field.text ?? field.name ?? field.etsId;
  const selectedGroup = groups.find(group => group.key === selectedRow?.groupKey);
  // ADR-0137: a matrix only for a chosen area with a declared grid, never while searching.
  const layoutView: GroupLayoutView | null = !searching && current ? current.layout : null;
  const matrix = layoutView?.kind === "matrix" ? planParameterMatrix(layoutView.layout,
    rows.filter(row => row.groupKey === current!.key).map(row => ({ key: row.key, cell: row.field.cell }))) : null;
  const resultCounts = new Map<string, number>();
  for (const row of rows) if (!searching || visible.has(row.key)) resultCounts.set(row.groupKey, (resultCounts.get(row.groupKey) ?? 0) + 1);

  return <div className="parameter-workspace">
    <div className="parameter-workspace-toolbar">
      <label className="parameter-workspace-search">
        <span>{t("parameters.workspace.search")}</span>
        <input type="search" value={query} placeholder={t("parameters.workspace.searchHint")}
          onChange={event => setQuery(event.target.value)} />
      </label>
      <span className="parameter-workspace-count" role="status">
        {t("parameters.workspace.count", { visible: visible.size, total: rows.length })}
      </span>
      {searching && <button type="button" onClick={() => setQuery("")}>{t("parameters.workspace.clear")}</button>}
    </div>
    <div className="parameter-workspace-layout">
      <nav className="parameter-workspace-nav" aria-label={t("parameters.workspace.areas")}>
        <h4>{t("parameters.workspace.areas")}</h4>
        {groups.map((group, index) => <div key={group.key} className="parameter-workspace-nav-entry">
          {(index === 0 || group.sectionKey !== groups[index - 1].sectionKey) &&
            <span className="parameter-workspace-scope">{scopeLabel(group.scope)}</span>}
          <button type="button" aria-current={!searching && group.key === activeKey ? "page" : undefined}
            onClick={() => { setActive(group.key); setQuery(""); }}>
            <span>{groupLabel(group)}</span><span className="parameter-workspace-nav-count">{resultCounts.get(group.key) ?? 0}</span>
          </button>
        </div>)}
      </nav>
      <div className="parameter-workspace-editor" id={`${id}-editor`}>
        {closed ? <p className="parameter-workspace-closed">{t("parameters.workspace.closed")}</p> : <>
          <header className="parameter-workspace-heading">
            <div><span>{searching ? t("parameters.workspace.allAreas") : current && scopeLabel(current.scope)}</span>
              <h4>{searching ? t("parameters.workspace.results") : current && groupLabel(current)}</h4></div>
            <span>{t("parameters.workspace.sourceGroups")}</span>
          </header>
          {layoutView && <LayoutNote view={layoutView} />}
          {visible.size > 0 && !matrix && <div className="parameter-workspace-columns" aria-hidden="true">
            <span>{t("parameters.workspace.parameter")}</span><span>{t("parameters.workspace.value")}</span>
          </div>}
        </>}
        {/* One stable source-ordered parent: navigation/search never remounts an editor. */}
        <div className="parameter-workspace-rows" data-layout={matrix ? "matrix" : undefined}
          style={matrix ? { gridTemplateColumns: matrix.templateColumns } : undefined}>
          {rows.map(row => <div key={row.key} data-parameter-row={row.field.etsId}
            className="parameter-workspace-row" hidden={!visible.has(row.key)}
            style={matrix?.fields.has(row.key) ? matrixStyle(matrix.fields.get(row.key)!) : undefined}
            data-selected={selected === row.key} data-access={row.field.access} data-display-only={row.field.kind === "None"}
            onFocusCapture={() => setSelected(row.key)} onClick={() => setSelected(row.key)}>
            <div className="parameter-workspace-field">
              {searching && <span className="parameter-workspace-result-path">{scopeLabel(row.scope)} / {groupLabel(groups.find(group => group.key === row.groupKey)!)}</span>}
              {matrix?.fields.has(row.key) && row.field.cell && !matrix.unplaced.includes(row.key) &&
                <span className="sr-only">{t("parameters.matrix.position", { row: row.field.cell.row, column: row.field.cell.column })}</span>}
              {renderField(row.field, row.scope)}
            </div>
            {row.field.kind !== "None" && <button type="button" className="parameter-workspace-info"
              aria-label={t("parameters.workspace.fieldInfo", { name: label(row.field) })}
              aria-controls={`${id}-context`} onClick={() => setSelected(row.key)}><span aria-hidden="true">i</span></button>}
          </div>)}
          {/* Captions are siblings after the rows, so they never shift editor identity. */}
          {matrix && [...matrix.columnHeaders, ...matrix.rowHeaders, ...matrix.labels].map(caption =>
            <span key={caption.key} className="parameter-matrix-caption" data-caption={caption.key.split(":")[0]}
              style={matrixStyle(caption.area)}>{caption.text}</span>)}
          {matrix?.unplacedHeading && <h5 key="matrix:unplaced" className="parameter-matrix-unplaced"
            style={matrixStyle(matrix.unplacedHeading)}>{t("parameters.matrix.unplaced")}</h5>}
        </div>
        {searching && visible.size === 0 && <p className="parameter-workspace-empty" role="status">{t("parameters.workspace.noMatches")}</p>}
      </div>
      <aside className="parameter-workspace-context" id={`${id}-context`} aria-label={t("parameters.workspace.information")}>
        {selectedField ? <section>
          <h5>{t("parameters.workspace.information")}</h5>
          <h4>{label(selectedField)}</h4>
          <p>{selectedRow && scopeLabel(selectedRow.scope)} / {selectedGroup && groupLabel(selectedGroup)}</p>
          {!visible.has(selectedRow!.key) && <p className="parameter-workspace-hidden-note">{t("parameters.workspace.selectedHidden")}</p>}
          <dl>
            <dt>{t("parameters.workspace.identifier")}</dt><dd>{selectedField.etsId}</dd>
            <dt>{t("parameters.workspace.type")}</dt><dd>{selectedField.kind}</dd>
            {selectedField.kind !== "None" && <>
              <dt>{t("parameters.workspace.source")}</dt><dd>{selectedField.valueSource}</dd>
              <dt>{t("parameters.workspace.reportedValue")}</dt><dd>{selectedField.value ?? t("parameters.none")}</dd>
            </>}
            {selectedField.min !== null && <><dt>{t("parameters.workspace.minimum")}</dt><dd>{selectedField.min}</dd></>}
            {selectedField.max !== null && <><dt>{t("parameters.workspace.maximum")}</dt><dd>{selectedField.max}</dd></>}
            {selectedField.access !== null && <><dt>{t("parameters.workspace.access")}</dt><dd>{selectedField.access}</dd></>}
          </dl>
        </section> : <p>{t("parameters.workspace.selectHint")}</p>}
      </aside>
    </div>
  </div>;
}

const ISSUE_KEYS = {
  cellMalformed: "parameters.matrix.issue.cellMalformed",
  cellOutOfRange: "parameters.matrix.issue.cellOutOfRange",
  missingCell: "parameters.matrix.issue.missingCell",
  ambiguousCell: "parameters.matrix.issue.ambiguousCell",
  unshownContent: "parameters.matrix.issue.unshownContent",
} as const;

/** Says why an area is or is not a matrix; reasons and issue kinds stay visible. */
function LayoutNote({ view }: { view: GroupLayoutView }) {
  const t = useTranslate();
  if (view.kind === "rows") {
    return <p className="parameter-workspace-layout-note">
      {t(`parameters.matrix.${view.reason}`, { layout: view.declared ?? "", status: view.status ?? "" })}</p>;
  }
  const { layout } = view;
  const issueText = (issue: typeof layout.issues[number]) => {
    const values = { element: issue.element, reference: issue.reference ?? "", cell: issue.cell ?? "", kind: issue.kind };
    const key = Object.hasOwn(ISSUE_KEYS, issue.kind) ? ISSUE_KEYS[issue.kind as keyof typeof ISSUE_KEYS] : "parameters.matrix.issue.unknown";
    return t(key, values);
  };
  return <>
    <p className="parameter-workspace-layout-note">{t("parameters.matrix.declared",
      { layout: layout.declaredLayout ?? "", rows: layout.rows.length, columns: layout.columns.length })}</p>
    {layout.issues.length > 0 && <details className="parameter-matrix-issues">
      <summary>{t("parameters.matrix.issues", { count: layout.issues.length })}</summary>
      <ul>{layout.issues.map((issue, index) => <li key={index}>{issueText(issue)}</li>)}</ul>
    </details>}
  </>;
}
