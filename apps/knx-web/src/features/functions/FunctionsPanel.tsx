/** Project Functions in the workbench: per room and per group address (Goal B1b). */
// Views over the A4 routes only (docs/contracts/project-functions.md): no
// domain rule is repeated here beyond a blank-name hint. Functions load on
// explicit opening of the disclosure, never on mount. Every edit binds to
// the read that rendered it; after a success the project tree is re-read so
// Undo, Save and the revision stay truthful.
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import * as api from "../../api";
import type { ProjectTree } from "../../bindings/ProjectTree";
import type { FunctionNode } from "../../bindings/FunctionNode";
import GroupAddressPicker from "../../GroupAddressPicker";
import { useTranslate, type Translate } from "../../i18n";
import { flattenBuildingParts, owningInstallation } from "../../treeUtils";
import {
  COMPLETIONS, contextOf, createFunctionsClient, isFunctionsRefused, isFunctionsStale, sameFunctionsContext,
  type FunctionAction, type FunctionDetailsDraft, type FunctionsClient, type FunctionsContext, type FunctionsRead,
} from "./client";
import {
  detailsOf, detailsProblem, EMPTY_DETAILS, functionsInPart, membershipsOf, optionalFieldValue, sameDetails,
} from "./model";

type Notice = { kind: "stale" } | { kind: "refused"; text: string } | { kind: "error"; text: string };

const defaultClient = createFunctionsClient();

function treeKey(tree: ProjectTree): string {
  return `${tree.server_incarnation ?? ""}/${tree.project_incarnation ?? ""}/${tree.snapshot_revision ?? ""}`;
}

/** One Functions read shared by a panel's controls; loads only while `open`. */
function useFunctions(tree: ProjectTree, open: boolean, client: FunctionsClient, onApplied: (tree: ProjectTree) => void) {
  const t = useTranslate();
  const [read, setRead] = useState<FunctionsRead | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  // A project switch drops the old read at once; it must never be edited.
  const project = `${tree.server_incarnation ?? ""}/${tree.project_incarnation ?? ""}`;
  useEffect(() => { setRead(null); setNotice(null); }, [project]);

  const load = useCallback(async () => {
    const owned = ++generation.current;
    try {
      const next = await client.read();
      if (alive.current && owned === generation.current) setRead(next);
    } catch (error) {
      if (alive.current && owned === generation.current) setNotice({ kind: "error", text: t("functions.readFailed", { message: api.errorMessage(error) }) });
    }
  }, [client, t]);

  const key = treeKey(tree);
  useEffect(() => { if (open) void load(); }, [open, key, load]);

  /** Re-reads Functions and the project tree after a refusal; never writes. */
  const reload = useCallback(async () => {
    setNotice(null);
    try { onApplied(await api.currentProject()); } catch (error) {
      setNotice({ kind: "error", text: t("functions.reloadFailed", { message: api.errorMessage(error) }) });
    }
    await load();
  }, [load, onApplied, t]);

  const edit = useCallback(async (action: FunctionAction, base: FunctionsContext): Promise<boolean> => {
    setBusy(true); setNotice(null);
    ++generation.current;
    try {
      const result = await client.edit(base, action);
      if (!alive.current) return true;
      setRead(result);
      try { onApplied(await api.currentProject()); } catch (error) {
        if (alive.current) setNotice({ kind: "error", text: t("functions.reloadFailed", { message: api.errorMessage(error) }) });
      }
      return true;
    } catch (error) {
      if (!alive.current) return false;
      if (isFunctionsStale(error)) setNotice({ kind: "stale" });
      else if (isFunctionsRefused(error)) setNotice({ kind: "refused", text: api.errorMessage(error) });
      else setNotice({ kind: "error", text: api.errorMessage(error) });
      return false;
    } finally {
      if (alive.current) setBusy(false);
    }
  }, [client, onApplied, t]);

  return { read, notice, busy, edit, reload };
}

function NoticeView(props: { notice: Notice | null; busy: boolean; onReload: () => void }) {
  const t = useTranslate();
  const { notice } = props;
  if (notice === null) return null;
  return <div className="functions-notice" role="alert">
    {notice.kind === "stale" && t("functions.stale")}
    {notice.kind === "refused" && <>{t("functions.refused")} <span className="functions-server-text">{notice.text}</span></>}
    {notice.kind === "error" && notice.text}
    {notice.kind === "stale" && <button type="button" className="functions-quiet" disabled={props.busy} onClick={props.onReload}>{t("functions.reload")}</button>}
  </div>;
}

function Provenance(props: { sourceId: string }) {
  const t = useTranslate();
  return <span className="functions-provenance" title={t("functions.sourceIdHint")}>
    {t("functions.sourceId")} <code>{props.sourceId}</code>
  </span>;
}

function completionLabel(t: Translate, completion: string): string {
  return (COMPLETIONS as readonly string[]).includes(completion) ? t(`functions.completion.${completion as typeof COMPLETIONS[number]}`) : completion;
}

/** Name, number, type, comment, description and completion, edited as one replacement. */
function DetailsForm(props: {
  initial: FunctionDetailsDraft; busy: boolean; submitLabel: string;
  /** Submitting waits for an explicit decision (stored details changed underneath). */
  blocked?: boolean;
  onSubmit: (draft: FunctionDetailsDraft) => void; onCancel?: () => void; children?: ReactNode;
}) {
  const t = useTranslate();
  const [draft, setDraft] = useState(props.initial);
  const [problem, setProblem] = useState<string | null>(null);
  const base = props.initial;
  const optional = (field: "functionType" | "number" | "comment" | "description", label: string) =>
    <label className="inspector-field">{label}
      <input value={draft[field] ?? ""} disabled={props.busy}
        onChange={(event) => setDraft({ ...draft, [field]: optionalFieldValue(event.target.value, base[field]) })} />
    </label>;
  return <form className="functions-details" onSubmit={(event) => {
    event.preventDefault();
    if (props.blocked) return;
    if (detailsProblem(draft) === "blank") { setProblem(t("functions.blankName")); return; }
    setProblem(null); props.onSubmit(draft);
  }}>
    <label className="inspector-field">{t("functions.name")}
      <input value={draft.name} disabled={props.busy} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
    </label>
    {optional("number", t("functions.number"))}
    {optional("functionType", t("functions.functionType"))}
    {optional("comment", t("functions.comment"))}
    {optional("description", t("functions.description"))}
    <label className="inspector-field">{t("functions.completionLabel")}
      <select value={draft.completion} disabled={props.busy}
        onChange={(event) => setDraft({ ...draft, completion: event.target.value as FunctionDetailsDraft["completion"] })}>
        {COMPLETIONS.map((value) => <option key={value} value={value}>{completionLabel(t, value)}</option>)}
      </select>
    </label>
    {problem && <p className="field-error" role="alert">{problem}</p>}
    {props.children}
    <div className="functions-actions">
      <button type="submit" className="primary-action" disabled={props.busy || props.blocked || sameDetails(draft, base) && props.onCancel !== undefined}>{props.submitLabel}</button>
      {props.onCancel && <button type="button" className="functions-quiet" disabled={props.busy} onClick={props.onCancel}>{t("functions.cancel")}</button>}
    </div>
  </form>;
}

function FunctionItem(props: {
  node: FunctionNode; read: FunctionsRead; tree: ProjectTree; busy: boolean;
  edit: (action: FunctionAction, base: FunctionsContext) => Promise<boolean>;
}) {
  const { node, read, tree, busy, edit } = props;
  const t = useTranslate();
  // The draft remembers the read and details it started from (Goal B1b).
  const [editing, setEditing] = useState<{ base: FunctionsContext; details: FunctionDetailsDraft } | null>(null);
  const [forced, setForced] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [member, setMember] = useState<number | null>(null);
  const [role, setRole] = useState("");
  const [target, setTarget] = useState<number | null>(null);
  const installation = tree.installations.find((candidate) => candidate.id === node.installationId);
  const parts = useMemo(() => flattenBuildingParts(installation?.buildings ?? [], []), [installation]);
  const memberIds = new Set(node.members.map((entry) => entry.groupAddressId));
  const addable = (installation?.group_addresses ?? []).filter((ga) => !memberIds.has(ga.id));
  const current = detailsOf(node);
  const changedUnderneath = editing !== null && !sameDetails(editing.details, current);

  async function submitDetails(draft: FunctionDetailsDraft) {
    if (editing === null) return;
    // Unchanged underneath: the newer read is a safe binding. Changed: only on request.
    if (changedUnderneath && !forced) return;
    const base = sameFunctionsContext(editing.base, read) ? editing.base : contextOf(read);
    if (await edit({ kind: "update", id: node.id, details: draft }, base)) { setEditing(null); setForced(false); }
  }

  return <li className="functions-item" data-function-id={node.id}>
    <details>
      <summary>
        <span className="functions-name">{node.name}</span>
        <span className="provenance-badge">{completionLabel(t, node.completion)}</span>
      </summary>
      <dl className="functions-facts">
        {node.number !== null && <><dt>{t("functions.number")}</dt><dd>{node.number}</dd></>}
        {node.functionType !== null && <><dt>{t("functions.functionType")}</dt><dd><code>{node.functionType}</code></dd></>}
        {node.comment !== null && <><dt>{t("functions.comment")}</dt><dd>{node.comment}</dd></>}
        {node.description !== null && <><dt>{t("functions.description")}</dt><dd>{node.description}</dd></>}
      </dl>
      <Provenance sourceId={node.sourceId} />
      <h4>{t("functions.members", { count: node.members.length })}</h4>
      {node.members.length === 0 ? <p className="inspector-description">{t("functions.noMembers")}</p> : <ul className="functions-members">
        {node.members.map((entry) => <li key={entry.id} data-member-id={entry.id}>
          <span className="mono">{entry.address}</span> <span>{entry.groupAddressName}</span>
          {entry.name !== "" && <span className="functions-member-name">{entry.name}</span>}
          {entry.role !== null && <span className="provenance-badge">{entry.role}</span>}
          <Provenance sourceId={entry.sourceId} />
          <button type="button" className="functions-quiet" disabled={busy}
            aria-label={t("functions.removeMemberLabel", { address: entry.address, function: node.name })}
            onClick={() => void edit({ kind: "removeMember", functionId: node.id, memberId: entry.id }, contextOf(read))}>
            {t("functions.removeMember")}
          </button>
        </li>)}
      </ul>}
      <div className="functions-add-member">
        <GroupAddressPicker addresses={addable} objectDpt={null} value={member} onChange={setMember} />
        <label className="inspector-field">{t("functions.role")}
          <input value={role} disabled={busy} onChange={(event) => setRole(event.target.value)} />
        </label>
        <button type="button" disabled={busy || member === null} onClick={async () => {
          if (member === null) return;
          if (await edit({ kind: "addMember", functionId: node.id, groupAddressId: member, name: "", role: role === "" ? null : role }, contextOf(read))) {
            setMember(null); setRole("");
          }
        }}>{t("functions.addMember")}</button>
      </div>
      {editing === null
        ? <button type="button" className="functions-quiet" disabled={busy} onClick={() => setEditing({ base: contextOf(read), details: current })}>{t("functions.editDetails")}</button>
        : <DetailsForm initial={editing.details} busy={busy} blocked={changedUnderneath && !forced} submitLabel={changedUnderneath && forced ? t("functions.applyMineAnyway") : t("functions.applyDetails")}
            onSubmit={(draft) => void submitDetails(draft)} onCancel={() => { setEditing(null); setForced(false); }}>
            {changedUnderneath && <div className="functions-notice" role="status">
              {t("functions.changedUnderneath", { name: current.name, completion: completionLabel(t, current.completion) })}
              {!forced && <button type="button" className="functions-quiet" onClick={() => setForced(true)}>{t("functions.keepMine")}</button>}
              <button type="button" className="functions-quiet" onClick={() => { setEditing({ base: contextOf(read), details: current }); setForced(false); }}>{t("functions.takeCurrent")}</button>
            </div>}
          </DetailsForm>}
      <div className="functions-move">
        <label className="inspector-field">{t("functions.moveTo")}
          <select value={target ?? ""} disabled={busy} onChange={(event) => setTarget(event.target.value === "" ? null : Number(event.target.value))}>
            <option value="">{t("functions.chooseRoom")}</option>
            {parts.filter((entry) => entry.node.id !== node.location.buildingPartId).map((entry) =>
              <option key={entry.node.id} value={entry.node.id}>{entry.path}</option>)}
          </select>
        </label>
        <button type="button" disabled={busy || target === null} onClick={async () => {
          if (target !== null && await edit({ kind: "move", id: node.id, location: target }, contextOf(read))) setTarget(null);
        }}>{t("functions.move")}</button>
      </div>
      {confirmDelete
        ? <div className="functions-notice" role="group">
          {node.members.length === 0 ? t("functions.deleteConfirmEmpty") : t("functions.deleteConfirm", { count: node.members.length })}
          <button type="button" disabled={busy} onClick={() => void edit({ kind: "delete", id: node.id }, contextOf(read))}>{t("functions.deleteConfirmButton")}</button>
          <button type="button" className="functions-quiet" disabled={busy} onClick={() => setConfirmDelete(false)}>{t("functions.cancel")}</button>
        </div>
        : <button type="button" className="functions-quiet" disabled={busy} onClick={() => setConfirmDelete(true)}>{t("functions.delete")}</button>}
    </details>
  </li>;
}

/** The Functions located in one building part, with create/edit/move/delete. */
export function BuildingPartFunctions(props: {
  tree: ProjectTree; buildingPartId: number; onApplied: (tree: ProjectTree) => void; client?: FunctionsClient;
}) {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const [creating, setCreating] = useState(0);
  const { read, notice, busy, edit, reload } = useFunctions(props.tree, open, props.client ?? defaultClient, props.onApplied);
  const identified = props.tree.server_incarnation !== undefined && props.tree.server_incarnation !== null;
  const known = owningInstallation(props.tree, "building_part", props.buildingPartId) !== undefined;
  const here = read === null ? [] : functionsInPart(read.functions, props.buildingPartId);
  return <details className="functions-panel" onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>{t("functions.title")}{read !== null && ` (${here.length})`}</summary>
    {!identified || !known ? <p className="inspector-description">{t("functions.unavailable")}</p>
      : read === null ? (notice === null ? <p className="inspector-description" role="status"><span className="task-status-text">{t("functions.loading")}</span></p> : null)
      : <>
        <p className="inspector-description">{t("functions.explanation")}</p>
        {here.length === 0 && <p className="inspector-description">{t("functions.none")}</p>}
        <ul className="functions-list">{here.map((node) =>
          <FunctionItem key={node.id} node={node} read={read} tree={props.tree} busy={busy} edit={edit} />)}</ul>
        <h4>{t("functions.create")}</h4>
        <DetailsForm key={creating} initial={EMPTY_DETAILS} busy={busy} submitLabel={t("functions.createButton")}
          onSubmit={async (details) => {
            if (await edit({ kind: "create", location: props.buildingPartId, details }, contextOf(read))) setCreating((value) => value + 1);
          }} />
      </>}
    <NoticeView notice={notice} busy={busy} onReload={() => void reload()} />
  </details>;
}

/** Which Functions use one group address; removing a membership comes before deleting it. */
export function GroupAddressFunctions(props: {
  tree: ProjectTree; groupAddressId: number; onApplied: (tree: ProjectTree) => void; client?: FunctionsClient;
}) {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const { read, notice, busy, edit, reload } = useFunctions(props.tree, open, props.client ?? defaultClient, props.onApplied);
  const memberships = read === null ? [] : membershipsOf(read.functions, props.groupAddressId);
  return <details className="functions-panel" onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>{t("functions.title")}{read !== null && ` (${memberships.length})`}</summary>
    {read === null ? (notice === null ? <p className="inspector-description" role="status"><span className="task-status-text">{t("functions.loading")}</span></p> : null)
      : memberships.length === 0 ? <p className="inspector-description">{t("functions.noMemberships")}</p>
      : <>
        <p className="inspector-description">{t("functions.membershipHint")}</p>
        <ul className="functions-members">{memberships.map(({ function: node, member }) =>
          <li key={member.id} data-member-id={member.id}>
            <span className="functions-name">{node.name}</span> <span>{node.location.name}</span>
            {member.role !== null && <span className="provenance-badge">{member.role}</span>}
            <button type="button" className="functions-quiet" disabled={busy}
              aria-label={t("functions.removeMemberLabel", { address: member.address, function: node.name })}
              onClick={() => void edit({ kind: "removeMember", functionId: node.id, memberId: member.id }, contextOf(read))}>
              {t("functions.removeMember")}
            </button>
          </li>)}</ul>
      </>}
    <NoticeView notice={notice} busy={busy} onReload={() => void reload()} />
  </details>;
}
