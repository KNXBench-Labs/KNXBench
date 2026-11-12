/** Source-group parameter workbench; navigation never owns values or write authority. */
import { useId, useState, type ReactNode } from "react";
import type { ModuleScope, ParameterField, ParameterSection } from "./api";
import { useTranslate } from "./i18n";
import { differingAreas, indexParameterWorkspace, matchesParameterSearch, type ParameterWorkspaceSeparator } from "./parameterWorkspace";
import { matrixStyle, planParameterMatrix, type GroupLayoutView } from "./parameterMatrix";
import "./parameterWorkspace.css";
import { parameterMetadataLabel } from "./parameterMetadata";
import { parameterCaption } from "./parameterCaption";

interface Props {
  sections: readonly ParameterSection[];
  renderField: (field: ParameterField, scope: ModuleScope | null) => ReactNode;
  /** #48: whether a row's value differs from the last download; the server decides, never this view. */
  differs?: (field: ParameterField, scope: ModuleScope | null) => boolean;
}

const NEVER = () => false;

export default function ParameterWorkspace({ sections, renderField, differs = NEVER }: Props) {
  const t = useTranslate();
  const id = useId();
  const [active, setActive] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const { rows, groups, items, passiveLabels } = indexParameterWorkspace(sections);
  // Closed by default: no area opens until chosen; a vanished choice closes again.
  const activeKey = groups.some(group => group.key === active) ? active : null;
  const searching = query.trim() !== "";
  const closed = !searching && activeKey === null;
  const scopeLabel = (scope: ModuleScope | null) => scope === null ? t("parameters.deviceScope")
    : scope.moduleId ?? t("parameters.moduleNumber", { number: scope.moduleNode });
  const caption = (group: typeof groups[number]) => group.ungrouped
    ? { label: t("parameters.workspace.ungrouped"), title: undefined }
    : parameterCaption(group, t("parameters.groupNumber", { number: group.number }), t);
  const groupLabel = (group: typeof groups[number]) => caption(group).label;
  const displayedGroupLabels = new Map(groups.map(group => [group.key, groupLabel(group)]));
  const visible = new Set(rows.filter(row => searching ? matchesParameterSearch(row, query, displayedGroupLabels.get(row.groupKey)) : row.groupKey === activeKey).map(row => row.key));
  const current = groups.find(group => group.key === activeKey);
  const selectedRow = rows.find(row => row.key === selected);
  const selectedField = selectedRow?.field;
  const label = (field: ParameterField) => field.text ?? field.name ?? field.etsId;
  const selectedGroup = groups.find(group => group.key === selectedRow?.groupKey);
  // ADR-0137: a matrix only for a chosen area with a declared grid, never while searching.
  const layoutView: GroupLayoutView | null = !searching && current ? current.layout : null;
  const matrix = layoutView?.kind === "matrix" ? planParameterMatrix(layoutView.layout,
    rows.filter(row => row.groupKey === current!.key).map(row => ({ key: row.key, cell: row.field.cell })),
    passiveLabels.filter(entry => entry.groupKey === current!.key)
      .map(entry => ({ key: `passive:${entry.key}`, text: entry.label.text, cell: entry.label.cell }))) : null;
  const passiveIds = new Map(passiveLabels.map(entry => [`passive:${entry.key}`, entry.label.elementId]));
  const passiveCells = new Map(passiveLabels.map(entry =>
    [`passive:${entry.key}`, `${entry.label.cell.row},${entry.label.cell.column}`]));
  // Button id and source cell of every literal this grid actually shows: such an issue is no
  // unplaced declaration. The cell matters: the grid reader names an Id-less Button by its RefId.
  const shownButtons = new Set(matrix?.labels.flatMap(caption => caption.passive
    ? [shownButtonKey(passiveIds.get(caption.key)!, passiveCells.get(caption.key)!)] : []) ?? []);
  const marks = differingAreas(rows, groups, differs);
  const markNote = t("parameters.lastDownload.section");
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
        {marks.groups.size > 0 && <span id={`${id}-differs`} hidden>{markNote}</span>}
        {groups.map((group, index) => <div key={group.key} className="parameter-workspace-nav-entry">
          {(index === 0 || group.sectionKey !== groups[index - 1].sectionKey) &&
            <span className="parameter-workspace-scope" data-differs-from-download={marks.sections.has(group.sectionKey) ? "true" : undefined}>
              {scopeLabel(group.scope)}{marks.sections.has(group.sectionKey) && <SectionMark note={markNote} />}</span>}
          <button type="button" title={caption(group).title} aria-current={!searching && group.key === activeKey ? "page" : undefined}
            aria-describedby={marks.groups.has(group.key) ? `${id}-differs` : undefined}
            data-differs-from-download={marks.groups.has(group.key) ? "true" : undefined}
            onClick={() => { setActive(group.key); setQuery(""); }}>
            <span>{groupLabel(group)}{marks.groups.has(group.key) && <span className="parameter-section-download-mark" title={markNote} aria-hidden="true" />}</span>
            <span className="parameter-workspace-nav-count">{resultCounts.get(group.key) ?? 0}</span>
          </button>
        </div>)}
      </nav>
      <div className="parameter-workspace-editor" id={`${id}-editor`}>
        {closed ? <p className="parameter-workspace-closed">{t("parameters.workspace.closed")}</p> : <>
          <header className="parameter-workspace-heading">
            <div><span>{searching ? t("parameters.workspace.allAreas") : current && scopeLabel(current.scope)}</span>
              <h4 title={current && !searching ? caption(current).title : undefined}>{searching ? t("parameters.workspace.results") : current && groupLabel(current)}</h4></div>
            <span>{t("parameters.workspace.sourceGroups")}</span>
          </header>
          {layoutView && <LayoutNote view={layoutView} shownButtons={shownButtons} />}
          {visible.size > 0 && !matrix && <div className="parameter-workspace-columns" aria-hidden="true">
            <span>{t("parameters.workspace.parameter")}</span><span>{t("parameters.workspace.value")}</span>
          </div>}
        </>}
        {/* One stable source-ordered parent: navigation/search never remounts an editor. */}
        <div className="parameter-workspace-rows" data-layout={matrix ? "matrix" : undefined}
          style={matrix ? { gridTemplateColumns: matrix.templateColumns } : undefined}>
          {items.map(item => item.kind === "separator"
            // ADR-0139: text belongs to its chosen area; a search lists fields
            // only, and a declared grid shows separators as its own captions.
            ? <SeparatorRow key={`separator:${item.separator.key}`} item={item.separator}
              hidden={searching || matrix !== null || item.separator.groupKey !== activeKey} />
            : (row => <div key={row.key} data-parameter-row={row.field.etsId}
            className="parameter-workspace-row" hidden={!visible.has(row.key)}
            data-indent={!searching && !matrix?.fields.has(row.key) && row.field.indentLevel ? row.field.indentLevel : undefined}
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
          </div>)(item.row))}
          {/* Captions are siblings after the rows, so they never shift editor identity. */}
          {matrix && [...matrix.columnHeaders, ...matrix.rowHeaders, ...matrix.labels].map(caption => caption.passive
            // button-literal-passive-v1: the literal is plain text; no role, focus stop or handler.
            ? <span key={caption.key} className="parameter-matrix-caption parameter-matrix-passive" data-caption="passive"
              data-passive-label={passiveIds.get(caption.key)} style={matrixStyle(caption.area)}>
              <span className="parameter-matrix-passive-text">{caption.text}</span>
              <span className="parameter-matrix-passive-cue">{t("parameters.matrix.passiveButton")}</span>
            </span>
            : <span key={caption.key} className="parameter-matrix-caption" data-caption={caption.key.split(":")[0]}
              title={parameterCaption({ label: caption.text, labelFill: caption.textFill }, t("parameters.caption.text"), t).title}
              style={matrixStyle(caption.area)}>{parameterCaption({ label: caption.text, labelFill: caption.textFill }, t("parameters.caption.text"), t).label}</span>)}
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
            <dt>{t("parameters.workspace.type")}</dt><dd title={selectedField.kind}>{parameterMetadataLabel(t, "kind", selectedField.kind)}</dd>
            {selectedField.kind !== "None" && <>
              <dt>{t("parameters.workspace.source")}</dt><dd title={selectedField.valueSource}>{parameterMetadataLabel(t, "source", selectedField.valueSource)}</dd>
              <dt>{t("parameters.workspace.reportedValue")}</dt><dd>{selectedField.value ?? t("parameters.none")}</dd>
            </>}
            {selectedField.min !== null && <><dt>{t("parameters.workspace.minimum")}</dt><dd>{selectedField.min}</dd></>}
            {selectedField.max !== null && <><dt>{t("parameters.workspace.maximum")}</dt><dd>{selectedField.max}</dd></>}
            {selectedField.access !== null && <><dt>{t("parameters.workspace.access")}</dt><dd title={selectedField.access}>{parameterMetadataLabel(t, "access", selectedField.access)}</dd></>}
            {selectedField.blockAccess != null && <><dt>{t("parameters.workspace.blockAccess")}</dt><dd title={selectedField.blockAccess}>{parameterMetadataLabel(t, "access", selectedField.blockAccess)}</dd></>}
          </dl>
        </section> : <p>{t("parameters.workspace.selectHint")}</p>}
      </aside>
    </div>
  </div>;
}

/**
 * #48: the amber dot on a scope that holds a marked area. The scope label is
 * not focusable, so its words go to screen readers as text, not a description.
 */
function SectionMark({ note }: { note: string }) {
  return <><span className="parameter-section-download-mark" title={note} aria-hidden="true" /><span className="sr-only"> ({note})</span></>;
}

const SEPARATOR_PREFIX_KEYS = {
  Information: "parameters.separator.information",
  Error: "parameters.separator.error",
} as const;

/**
 * ADR-0139: one manufacturer separator, display only. `UIHint` selects the
 * style; an unknown hint shows its text plainly and keeps the token as data.
 * A separator without text can only divide, so it is drawn as a rule.
 */
function SeparatorRow({ item, hidden }: { item: ParameterWorkspaceSeparator; hidden: boolean }) {
  const t = useTranslate();
  const { text, textFill, uiHint, elementId } = item.separator;
  const caption = parameterCaption({ label: text, labelFill: textFill }, t("parameters.caption.text"), t);
  const prefix = uiHint !== null && Object.hasOwn(SEPARATOR_PREFIX_KEYS, uiHint)
    ? SEPARATOR_PREFIX_KEYS[uiHint as keyof typeof SEPARATOR_PREFIX_KEYS] : null;
  return <div className="parameter-workspace-separator" data-ui-hint={uiHint ?? undefined}
    data-separator={elementId ?? undefined} hidden={hidden} title={text !== null ? caption.title : undefined}>
    {text !== null && (uiHint === "Headline" ? <h5>{caption.label}</h5>
      : <p>{prefix && <span className="sr-only">{t(prefix)} </span>}{caption.label}</p>)}
    {(text === null || uiHint === "HorizontalRuler") && <hr />}
  </div>;
}

const ISSUE_KEYS = {
  cellMalformed: "parameters.matrix.issue.cellMalformed",
  cellOutOfRange: "parameters.matrix.issue.cellOutOfRange",
  missingCell: "parameters.matrix.issue.missingCell",
  ambiguousCell: "parameters.matrix.issue.ambiguousCell",
  unshownContent: "parameters.matrix.issue.unshownContent",
} as const;

/** One shown Button: its source reference and its declared `row,column` cell. */
function shownButtonKey(reference: string, cell: string): string {
  return JSON.stringify([reference, cell]);
}

/**
 * Says why an area is or is not a matrix; reasons and issue kinds stay visible.
 * A functional `Button` whose passive label this grid shows keeps its own note
 * (action unsupported) and is not counted as a declaration that was not placed.
 */
function LayoutNote({ view, shownButtons }: { view: GroupLayoutView; shownButtons: ReadonlySet<string> }) {
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
  const labelShown = (issue: typeof layout.issues[number]) => issue.kind === "unshownContent" && issue.element === "Button"
    && issue.reference !== null && issue.cell !== null && shownButtons.has(shownButtonKey(issue.reference, issue.cell));
  const shown = layout.issues.filter(labelShown);
  const unplaced = layout.issues.filter(issue => !labelShown(issue));
  return <>
    <p className="parameter-workspace-layout-note">{t("parameters.matrix.declared",
      { layout: layout.declaredLayout ?? "", rows: layout.rows.length, columns: layout.columns.length })}</p>
    {shown.map((issue, index) => <p key={index} className="parameter-matrix-passive-note" data-passive-note={issue.reference!}>
      {t("parameters.matrix.issue.buttonLabelShown", { element: issue.element, reference: issue.reference! })}</p>)}
    {unplaced.length > 0 && <details className="parameter-matrix-issues">
      <summary>{t("parameters.matrix.issues", { count: unplaced.length })}</summary>
      <ul>{unplaced.map((issue, index) => <li key={index}>{issueText(issue)}</li>)}</ul>
    </details>}
  </>;
}
