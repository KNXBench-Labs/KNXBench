/** Missing-name wizard: scan, select and edit names, preview, then apply one batch (WP-51.3a). */
/*
 * The scan and the preview are bound to one project snapshot; any edit,
 * any project change and closing the dialog retire the shown preview,
 * and only the shown preview can be applied. Totals appear only for a
 * complete scan, duplicates are only warned about, and more than
 * `MAX_MISSING_NAME_SELECTION` entries are never submitted (Q13, Q17).
 * Read-only projects: the web UI has no read-only signal (the server sends
 * no flag), so the wizard relies on the server refusing the apply.
 */
import { useEffect, useRef, useState } from "react";
import * as api from "../../api";
import type { ProjectTree } from "../../bindings/ProjectTree";
import Overlay from "../../Overlay";
import { useTranslate } from "../../i18n";
import { useGroupAddressFormat } from "../../gaNotation";
import { isProjectSaving, projectSavingMessage } from "../../projectSaving";
import {
  applyMissingNames, cancelMissingNames, isStaleMissingName, MAX_MISSING_NAME_SELECTION, MISSING_NAME_PAGE_SIZE,
  previewMissingNames, reevaluateMissingNames, scanMissingNames, targetKey, type EvidenceGap, type MissingNameBinding,
  type MissingNamePreview, type MissingNameProposal, type MissingNameRefusal, type MissingNameScan, type NameTargetId,
  type NameTargetKind,
} from "./client";
import {
  applyReevaluation, draftParents, duplicateKeys, groupByKind, isParentKind, isSelectable, mergePage, nameProblem,
  occurrenceKey, selectedDrafts, selectedEntries, setName, setSelected, type Draft, type Drafts,
} from "./drafts";
import "./missingNames.css";

const REFUSALS = {
  missingNameSelection: "missingNames.refusal.selection",
  missingNameBudget: "missingNames.refusal.budget",
  missingNameDuplicate: "missingNames.refusal.duplicate",
  missingNameTarget: "missingNames.refusal.target",
  missingNameReservedTarget: "missingNames.refusal.target",
  missingNameTargetNotBlank: "missingNames.refusal.notBlank",
  missingNameInvalidName: "missingNames.refusal.invalidName",
  missingNameRefused: "missingNames.refusal.refused",
  missingNamePersistenceFailed: "missingNames.refusal.persistence",
} as const;

const KIND_LABELS = {
  project: "missingNames.kind.project",
  installation: "missingNames.kind.installation",
  building_part: "missingNames.kind.building_part",
  area: "missingNames.kind.area",
  line: "missingNames.kind.line",
  device: "missingNames.kind.device",
  group_range: "missingNames.kind.group_range",
  group_address: "missingNames.kind.group_address",
  function: "missingNames.kind.function",
} as const satisfies Record<NameTargetKind, string>;

const ORIGIN_LABELS = {
  automatic: "missingNames.origin.automatic",
  manual: "missingNames.origin.manual",
  manual_required: "missingNames.origin.manual_required",
} as const;

type Problem = { lead: string; detail: string | null };

function treeBinding(tree: ProjectTree): MissingNameBinding | null {
  const { server_incarnation, project_incarnation, snapshot_revision } = tree;
  if (server_incarnation == null || project_incarnation == null || snapshot_revision == null) return null;
  return { serverIncarnation: server_incarnation, projectIncarnation: project_incarnation, snapshotRevision: snapshot_revision };
}

const sameBinding = (a: MissingNameBinding, b: MissingNameBinding) => a.serverIncarnation === b.serverIncarnation
  && a.projectIncarnation === b.projectIncarnation && a.snapshotRevision === b.snapshotRevision;

/** A raw 16-bit group address as main/middle/sub, for the user's notation. */
const groupAddressText = (raw: number) => `${raw >> 11}/${(raw >> 8) & 7}/${raw & 255}`;

export default function MissingNamesWizard(props: {
  tree: ProjectTree;
  /** After an apply: the re-read project. */
  onTreeUpdate: (tree: ProjectTree) => void;
  /** The server says the shown tree is old: the caller re-reads it. */
  onStale: () => void;
  onClose: () => void;
}) {
  const t = useTranslate();
  const format = useGroupAddressFormat();
  const [scan, setScan] = useState<MissingNameScan | null>(null);
  const [drafts, setDrafts] = useState<Drafts>(new Map());
  const [changedParents, setChangedParents] = useState<ReadonlyMap<string, NameTargetId>>(new Map());
  const [preview, setPreview] = useState<MissingNamePreview | null>(null);
  const [outdated, setOutdated] = useState(false);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [applied, setApplied] = useState<number | null>(null);
  const tokenRef = useRef<string | null>(null);
  const binding = treeBinding(props.tree);

  function retire() {
    if (tokenRef.current !== null) cancelMissingNames(tokenRef.current);
    tokenRef.current = null;
    setPreview(null);
  }
  // A scan and its preview belong to one snapshot: a project change outdates both.
  const firstBinding = useRef(true);
  useEffect(() => {
    if (firstBinding.current) { firstBinding.current = false; return; }
    if (scan && binding && sameBinding(scan.binding, binding)) return;
    if (applied !== null) return;
    retire();
    setOutdated(true);
    setProblem({ lead: t("missingNames.stale"), detail: null });
  }, [binding?.serverIncarnation, binding?.projectIncarnation, binding?.snapshotRevision]);
  // Closing the dialog forgets the preview on the server too.
  useEffect(() => () => { if (tokenRef.current !== null) cancelMissingNames(tokenRef.current); }, []);
  useEffect(() => { void loadPage(0, true); }, []);

  function explain(error: unknown, fallback: "missingNames.failed" | "missingNames.scanFailed"): Problem {
    if (isStaleMissingName(error)) {
      setOutdated(true);
      props.onStale();
      return { lead: t("missingNames.stale"), detail: null };
    }
    const refusal = error as Partial<MissingNameRefusal>;
    const key = typeof refusal.kind === "string" && Object.hasOwn(REFUSALS, refusal.kind)
      ? REFUSALS[refusal.kind as keyof typeof REFUSALS] : fallback;
    return { lead: t(key), detail: error instanceof Error ? error.message : String(error) };
  }

  async function loadPage(offset: number, fresh: boolean) {
    retire();
    setBusy(true); setProblem(null); setNotice(null);
    try {
      const page = await scanMissingNames({ offset });
      // A page of another snapshot invalidates every draft of the old one.
      const reset = fresh || !scan || !sameBinding(scan.binding, page.binding);
      if (reset && !fresh) setProblem({ lead: t("missingNames.stale"), detail: null });
      setDrafts((current) => mergePage(reset ? new Map() : current, page.items));
      if (reset) setChangedParents(new Map());
      setScan(page);
      setOutdated(false);
    } catch (error) {
      setProblem(explain(error, "missingNames.scanFailed"));
    } finally {
      setBusy(false);
    }
  }

  function noteParent(draft: Draft | undefined) {
    if (!draft || !isParentKind(draft.proposal.candidate.target)) return;
    const target = draft.proposal.candidate.target;
    setChangedParents((current) => new Map(current).set(targetKey(target), target));
  }
  function editName(key: string, name: string) {
    if (preview) retire();
    noteParent(drafts.get(key));
    setDrafts((current) => setName(current, key, name));
  }
  function select(keys: readonly string[], selected: boolean) {
    if (preview) retire();
    keys.forEach((key) => noteParent(drafts.get(key)));
    setDrafts((current) => setSelected(current, keys, selected));
  }

  async function reevaluate() {
    if (!scan || busy || changedParents.size === 0) return;
    retire();
    setBusy(true); setProblem(null); setNotice(null);
    try {
      const result = await reevaluateMissingNames(scan.binding, [...changedParents.values()], draftParents(drafts));
      setDrafts((current) => applyReevaluation(current, result));
      setChangedParents(new Map());
      setNotice(t("missingNames.reevaluated", { count: result.invalidated_targets.length }));
    } catch (error) {
      setProblem(explain(error, "missingNames.failed"));
    } finally {
      setBusy(false);
    }
  }

  const chosen = selectedDrafts(drafts);
  const tooMany = chosen.length > MAX_MISSING_NAME_SELECTION;
  const invalid = chosen.filter((draft) => nameProblem(draft) !== null).length;
  // A changed parent outdates dependent suggestions until Re-evaluate has run.
  const canPreview = !!scan && !busy && !outdated && changedParents.size === 0 && chosen.length > 0 && !tooMany
    && invalid === 0;

  async function showPreview() {
    if (!scan || !canPreview) return;
    retire();
    setBusy(true); setProblem(null); setNotice(null);
    try {
      const plan = await previewMissingNames(scan.binding, selectedEntries(drafts));
      tokenRef.current = plan.token;
      setPreview(plan);
    } catch (error) {
      setProblem(explain(error, "missingNames.failed"));
    } finally {
      setBusy(false);
    }
  }

  async function confirm() {
    if (!preview || busy) return;
    setBusy(true); setProblem(null);
    try {
      await applyMissingNames(preview.token);
      tokenRef.current = null;
      setApplied(preview.changed);
      api.currentProject().then(props.onTreeUpdate, (error: unknown) => setProblem({ lead: api.errorMessage(error), detail: null }));
    } catch (error) {
      if (isProjectSaving(error)) {
        // A Save is writing: nothing was named and the preview stays valid for a retry.
        setProblem({ lead: projectSavingMessage(error) ?? "", detail: null });
        return;
      }
      // Stale or refused: nothing was named. The server keeps a failed preview, so cancel it.
      retire();
      setProblem(explain(error, "missingNames.failed"));
    } finally {
      setBusy(false);
    }
  }

  const duplicates = duplicateKeys(drafts);
  const sessionOnly = (preview?.persistence ?? scan?.persistence) === "session-only";
  const discovery = scan?.discovery;
  const counts = discovery?.counts;

  function gapText(gap: EvidenceGap): string {
    if (gap.gap === "unselected_parent") return t("missingNames.gap.unselectedParent", { parent: targetKey(gap.parent) });
    const key = gap.gap === "missing" ? "missingNames.gap.missing" : gap.gap === "ambiguous" ? "missingNames.gap.ambiguous"
      : "missingNames.gap.insufficient";
    return t(key, { detail: gap.detail });
  }

  function contextParts(item: MissingNameProposal): string[] {
    const context = item.candidate.context;
    const parts: string[] = [];
    if (context.individual_address) parts.push(t("missingNames.context.address", { value: context.individual_address }));
    if (context.group_address != null) parts.push(t("missingNames.context.address", { value: format(groupAddressText(context.group_address)) }));
    if (context.group_range_span) parts.push(t("missingNames.context.range", {
      from: format(groupAddressText(context.group_range_span[0])), to: format(groupAddressText(context.group_range_span[1])) }));
    if (context.topology_area_address != null) parts.push(t("missingNames.context.area", { value: context.topology_area_address }));
    if (context.topology_line_address != null) parts.push(t("missingNames.context.line", { value: context.topology_line_address }));
    if (context.building_part_kind) parts.push([context.building_part_kind, context.building_part_number].filter(Boolean).join(" "));
    if (context.source_ets_id) parts.push(t("missingNames.context.etsId", { value: context.source_ets_id }));
    return parts;
  }

  function row(item: MissingNameProposal) {
    const key = targetKey(item.candidate.target);
    const draft = drafts.get(key);
    const label = `${t(KIND_LABELS[item.candidate.kind])} ${key}`;
    const identity = item.candidate.identity_status;
    const selectable = isSelectable(item);
    // Re-evaluate replaces the draft's proposal: show its provenance, not the scan's.
    const current = draft?.proposal.candidate.occurrence_index === item.candidate.occurrence_index ? draft.proposal : item;
    const { origin, sources, gaps } = current.provenance;
    const issue = draft?.selected ? nameProblem(draft) : null;
    return <li key={occurrenceKey(item)} className="missing-names-entry" data-missing-entry={occurrenceKey(item)}>
      <input type="checkbox" checked={draft?.selected ?? false} disabled={busy || !selectable}
        aria-label={t("missingNames.select", { label })} onChange={(event) => select([key], event.currentTarget.checked)} />
      <div className="missing-names-entry-body">
        <input className="missing-names-name" value={draft?.name ?? ""} disabled={busy || !selectable} data-missing-name={key}
          aria-label={t("missingNames.nameLabel", { label })} aria-invalid={issue !== null}
          placeholder={current.proposed_name === null ? t("missingNames.manualPlaceholder") : undefined}
          onInput={(event) => editName(key, event.currentTarget.value)} />
        <p className="missing-names-meta">
          <span className="missing-names-id">{t("missingNames.stableId", { id: key })}
            {item.candidate.occurrence_index > 0 && ` · ${t("missingNames.occurrence", { n: item.candidate.occurrence_index + 1 })}`}</span>
          {contextParts(item).map((part) => <span key={part}>{part}</span>)}
        </p>
        <p className="missing-names-meta">
          <span>{t(ORIGIN_LABELS[origin])}</span>
          {sources.map((source, index) => <span key={index}>{`${source.kind}: ${source.value}`}</span>)}
        </p>
        {gaps.length > 0 && <ul className="missing-names-gaps">
          {gaps.map((gap, index) => <li key={index}>{gapText(gap)}</li>)}</ul>}
        {identity.status === "duplicate_id" && <p className="missing-names-note">{t("missingNames.identity.duplicate", { count: identity.occurrences })}</p>}
        {identity.status === "invalid_zero_id" && <p className="missing-names-note">{t("missingNames.identity.zero")}</p>}
        {draft?.recheck === "updated" && <p className="missing-names-note" data-recheck="updated">{t("missingNames.recheck.updated")}</p>}
        {draft?.recheck === "kept" && <p className="missing-names-note" data-recheck="kept">{t("missingNames.recheck.kept")}</p>}
        {duplicates.has(key) && <p className="missing-names-warning" data-duplicate={key}>{t("missingNames.duplicate")}</p>}
        {issue === "blank" && <p className="field-error">{t("missingNames.problem.blank")}</p>}
        {issue === "tooLong" && <p className="field-error">{t("missingNames.problem.tooLong", { max: item.candidate.max_name_codepoints })}</p>}
      </div>
    </li>;
  }

  const groups = groupByKind(scan?.items ?? []);
  const from = discovery ? discovery.offset + 1 : 0;
  const to = discovery ? discovery.offset + (scan?.items.length ?? 0) : 0;
  return <Overlay className="missing-names-panel" labelledBy="missing-names-title" onClose={busy ? () => {} : props.onClose}>
    <header className="missing-names-heading">
      <h2 id="missing-names-title">{t("missingNames.title")}</h2>
      <p>{t("missingNames.subtitle")}</p>
    </header>
    {sessionOnly && <p className="missing-names-session" data-session-only>{t("missingNames.sessionOnly")}</p>}
    {!scan && busy && <p role="status">{t("missingNames.scanning")}</p>}
    {discovery && counts && applied === null && <section className="missing-names-scan" aria-labelledby="missing-names-count">
      <div className="missing-names-summary">
        <h3 id="missing-names-count">{discovery.completeness === "complete"
          ? t("missingNames.count.complete", { count: counts.total_candidates ?? 0 })
          : t("missingNames.count.incomplete", { count: counts.candidates_found_in_scan, scanned: counts.entities_scanned })}</h3>
        {discovery.completeness === "incomplete" && <span className="missing-names-badge" data-incomplete>{t("missingNames.incomplete")}</span>}
        <button type="button" disabled={busy} onClick={() => void loadPage(0, true)}>{t("missingNames.scanAgain")}</button>
      </div>
      {scan.items.length === 0 && <p>{t("missingNames.none")}</p>}
      {groups.map((group) => {
        const keys = group.items.filter(isSelectable).map((item) => targetKey(item.candidate.target));
        const all = keys.length > 0 && keys.every((key) => drafts.get(key)?.selected);
        const kind = t(KIND_LABELS[group.kind]);
        return <section key={group.kind} className="missing-names-group" data-missing-group={group.kind}>
          <h4><label><input type="checkbox" checked={all} disabled={busy || keys.length === 0}
            aria-label={t("missingNames.selectGroup", { kind })} onChange={(event) => select(keys, event.currentTarget.checked)} />
            {` ${kind} (${group.items.length})`}</label></h4>
          <ul>{group.items.map(row)}</ul>
        </section>;
      })}
      {scan.items.length > 0 && <nav className="missing-names-pages" aria-label={t("missingNames.pages")}>
        <button type="button" disabled={busy || discovery.offset === 0}
          onClick={() => void loadPage(Math.max(0, discovery.offset - MISSING_NAME_PAGE_SIZE), false)}>{t("missingNames.previousPage")}</button>
        <span>{t("missingNames.page", { from, to })}</span>
        <button type="button" disabled={busy || !discovery.has_more_in_scan}
          onClick={() => void loadPage(discovery.offset + scan.items.length, false)}>{t("missingNames.nextPage")}</button>
      </nav>}
      {changedParents.size > 0 && <div className="missing-names-reevaluate">
        <p>{t("missingNames.reevaluateHint")}</p>
        <button type="button" disabled={busy || outdated} data-missing-reevaluate onClick={() => void reevaluate()}>{t("missingNames.reevaluate")}</button>
      </div>}
      <p className="missing-names-selected" data-selected-count={chosen.length}>
        {t("missingNames.selected", { selected: chosen.length, max: MAX_MISSING_NAME_SELECTION })}</p>
      {tooMany && <p className="field-error" role="alert" data-too-many>{t("missingNames.tooMany",
        { selected: chosen.length, max: MAX_MISSING_NAME_SELECTION, excess: chosen.length - MAX_MISSING_NAME_SELECTION })}</p>}
      {invalid > 0 && <p className="field-error">{t("missingNames.invalidSelected", { count: invalid })}</p>}
    </section>}
    {notice && <p role="status" className="missing-names-notice">{notice}</p>}
    {problem && <div className="missing-names-problem" role="alert"><strong>{problem.lead}</strong>
      {problem.detail && <><p>{problem.detail}</p><small>{t("toast.error.messageIsEnglish")}</small></>}</div>}
    {preview && applied === null && <section className="missing-names-preview" aria-labelledby="missing-names-preview-title">
      <h3 id="missing-names-preview-title">{t("missingNames.previewTitle", { count: preview.changed })}</h3>
      <ul className="missing-names-preview-rows">{preview.rows.map((entry) => {
        const key = targetKey(entry.target);
        return <li key={key} data-preview-row={key}>
          <span>{`${t(KIND_LABELS[entry.kind])} ${key}`}</span>
          <span className="missing-names-unnamed">{entry.before.trim() || t("missingNames.unnamed")}</span>
          <span aria-hidden="true">→</span>
          <strong>{entry.after}</strong>
        </li>;
      })}</ul>
      {preview.warnings.length > 0 && <div className="missing-names-warnings">
        <h4>{t("missingNames.warningsTitle")}</h4>
        <ul>{preview.warnings.map((warning) => <li key={targetKey(warning.target)} data-preview-warning={targetKey(warning.target)}>
          {t("missingNames.warning", { name: warning.proposed_name, count: warning.conflicts_with.length })}</li>)}</ul>
      </div>}
      <p className="missing-names-hint">{t("missingNames.undoHint")}</p>
    </section>}
    {applied !== null && <p role="status" className="missing-names-done">{t("missingNames.applied", { count: applied })}</p>}
    <footer className="missing-names-actions">
      <button type="button" disabled={busy} onClick={props.onClose}>{t("missingNames.close")}</button>
      {preview && applied === null && <button type="button" disabled={busy} onClick={retire}>{t("missingNames.back")}</button>}
      {!preview && applied === null && <button type="button" data-missing-preview disabled={!canPreview}
        onClick={() => void showPreview()}>{t("missingNames.preview")}</button>}
      {applied === null && <button type="button" className="primary-action" data-missing-confirm disabled={busy || !preview}
        onClick={() => void confirm()}>{preview ? t("missingNames.confirm", { count: preview.changed }) : t("missingNames.confirmIdle")}</button>}
    </footer>
  </Overlay>;
}
