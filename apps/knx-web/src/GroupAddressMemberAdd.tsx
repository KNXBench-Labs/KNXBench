/** Adds one member: device, then communication object, then direction. */
import { useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { dptDisplay } from "./dptLabels";
import { useDeviceObjectInventory } from "./groupAddressObjectInventory";
import {
  MEMBER_DIRECTIONS, existingDirections, isDuplicateMember, memberCandidates, memberDraftIdentity,
  memberObjectChoices, type MemberDirection,
} from "./groupAddressMembers";
import { directionLabel } from "./groupAddressView";
import { useTranslate, type MessageKey } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { projectContextKey } from "./treeUtils";

const DIRECTION_KEYS: Record<MemberDirection, MessageKey> = {
  Send: "inspector.direction.send",
  Receive: "inspector.direction.receive",
  Both: "inspector.direction.both",
};

interface Draft { deviceId: number | null; objectId: number | null; direction: MemberDirection }
const EMPTY_DRAFT: Draft = { deviceId: null, objectId: null, direction: "Send" };

/** The object's DPT as the device editor shows it: its own, else the program default. */
const objectDpt = (object: ComObjectNode): string | null => object.dpt ?? object.program_dpt;

export default function GroupAddressMemberAdd(props: {
  ga: GroupAddressNode;
  tree: ProjectTree;
  onTreeUpdate: (tree: ProjectTree) => void;
}) {
  const { ga, tree, onTreeUpdate } = props;
  const t = useTranslate();
  const [language] = useUiLanguage();
  const identity = memberDraftIdentity(tree, ga.id);
  const projectKey = projectContextKey(tree);
  const candidates = useMemo(() => memberCandidates(tree, ga.id), [tree, ga.id]);
  const inventory = useDeviceObjectInventory(candidates, tree, projectKey);
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);
  const [error, setError] = useState<string | null>(null);
  const [added, setAdded] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const alive = useRef(true);
  const pending = useRef(false);
  const generation = useRef(0);
  const currentIdentity = useRef(identity);
  // Synchronously, so a reply that settles in the same tick as the switch
  // already sees the new generation.
  if (currentIdentity.current !== identity) { generation.current++; currentIdentity.current = identity; }

  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  // Another address, project load or server process: the draft and any
  // pending reply belong to the old one and are dropped, not re-targeted.
  useEffect(() => {
    pending.current = false; setBusy(false);
    setDraft(EMPTY_DRAFT); setError(null); setAdded(null); setOpen(false);
  }, [identity]);
  // A device that left the admitted list (moved, deleted) cannot stay chosen.
  useEffect(() => {
    if (draft.deviceId !== null && !candidates.some((device) => device.id === draft.deviceId)) setDraft(EMPTY_DRAFT);
  }, [candidates, draft.deviceId]);

  const entry = draft.deviceId === null ? undefined : inventory.entries.get(draft.deviceId);
  const loaded = draft.deviceId === null ? undefined : inventory.current.get(draft.deviceId);
  const objects = useMemo(() => memberObjectChoices(loaded?.objects ?? []), [loaded]);
  const chosen: ComObjectNode | undefined = objects.find((object) => object.id === draft.objectId);
  const duplicate = chosen !== undefined && isDuplicateMember(chosen, ga.id, draft.direction);
  // An object chosen before a re-read must still exist in the fresh list.
  useEffect(() => {
    if (loaded && draft.objectId !== null && !loaded.objects.some((object) => object.id === draft.objectId)) {
      setDraft((current) => ({ ...current, objectId: null }));
    }
  }, [loaded, draft.objectId]);

  function chooseDevice(value: string) {
    const deviceId = value === "" ? null : Number(value);
    setDraft((current) => ({ ...current, deviceId, objectId: null }));
    setError(null); setAdded(null);
    // Choosing the device is the explicit request to read its objects.
    if (deviceId !== null && inventory.current.get(deviceId) === undefined) inventory.load(deviceId);
  }

  async function submit() {
    if (pending.current || chosen === undefined || duplicate) return;
    const owned = generation.current;
    const object = chosen, direction = draft.direction;
    pending.current = true; setBusy(true); setError(null); setAdded(null);
    try {
      const updated = await api.linkComObject(object.id, ga.id, direction);
      // The command already ran on the server: its snapshot goes to the
      // application, which accepts it only for the same project lifetime
      // and a newer revision. The draft reacts only if it is still ours.
      onTreeUpdate(updated);
      if (!alive.current || generation.current !== owned) return;
      setAdded(t("addressTable.members.added", {
        object: `#${object.number} ${object.name ?? t("addressTable.unnamedObject")}`,
        direction: t(DIRECTION_KEYS[direction]),
      }));
      setDraft((current) => ({ ...current, objectId: null }));
    } catch (reason) {
      if (alive.current && generation.current === owned) setError(api.errorMessage(reason));
    } finally {
      if (alive.current && generation.current === owned) { pending.current = false; setBusy(false); }
    }
  }

  if (!open) {
    return <div className="ga-member-add">
      <button type="button" className="ga-member-add-open" onClick={() => setOpen(true)}>{t("addressTable.members.add")}</button>
      {added && <p className="ga-member-added" role="status">{added}</p>}
    </div>;
  }

  const objectState = draft.deviceId === null ? null
    : entry?.status === "failed" ? "failed" : loaded ? "loaded" : "loading";
  return <form className="ga-member-add ga-member-form" aria-label={t("addressTable.members.addTitle", { address: ga.address })}
    onSubmit={(event) => { event.preventDefault(); void submit(); }}>
    <p className="ga-member-rule">{t("addressTable.members.installationRule")}</p>
    {candidates.length === 0 ? <p role="status">{t("addressTable.members.noCandidates")}</p> : <>
      <label>{t("addressTable.members.device")}
        <select data-ga-member-device value={draft.deviceId ?? ""} disabled={busy} onChange={(event) => chooseDevice(event.target.value)}>
          <option value="">{t("addressTable.members.chooseDevice")}</option>
          {candidates.map((device) => <option key={device.id} value={device.id}>
            {device.address ?? t("addressTable.members.noAddress")} — {device.name}
          </option>)}
        </select>
      </label>
      {objectState === "loading" && <p role="status"><span className="task-status-text">{t("addressTable.members.loadingObjects")}</span></p>}
      {objectState === "failed" && entry?.status === "failed" && <p className="field-error" role="alert">
        {t("addressTable.members.objectsFailed", { message: entry.mismatch ? t("gaWorkspace.inventory.mismatch") : entry.message ?? t("gaWorkspace.inventory.unknownError") })}
        {" "}<button type="button" onClick={() => draft.deviceId !== null && inventory.load(draft.deviceId)}>{t("gaWorkspace.retryObjects")}</button>
      </p>}
      {objectState === "loaded" && (objects.length === 0
        ? <p role="status">{t("addressTable.members.noObjects")}</p>
        : <label>{t("addressTable.members.object")}
          <select data-ga-member-object value={draft.objectId ?? ""} disabled={busy}
            onChange={(event) => { setDraft((current) => ({ ...current, objectId: event.target.value === "" ? null : Number(event.target.value) })); setError(null); setAdded(null); }}>
            <option value="">{t("addressTable.members.chooseObject")}</option>
            {objects.map((object) => {
              const existing = [...existingDirections(object, ga.id)];
              return <option key={object.id} value={object.id}>
                #{object.number} {object.name ?? t("addressTable.unnamedObject")}
                {object.function_text ? ` — ${object.function_text}` : ""}
                {existing.length > 0 ? ` (${t("addressTable.members.alreadyLinked", { directions: existing.map((d) => directionLabel(t, d)).join(", ") })})` : ""}
              </option>;
            })}
          </select>
        </label>)}
      {chosen && <p className="ga-member-dpt">{t("addressTable.members.dptPair", {
        // Shown side by side, never judged here: the server's DPT outcome
        // for the address is the existing rule and appears after linking.
        object: objectDpt(chosen) === null ? t("addressTable.noDpt") : dptDisplay(objectDpt(chosen)!, language),
        address: ga.dpts.length > 0 ? ga.dpts.map((dpt) => dptDisplay(dpt, language)).join(" · ") : t("addressTable.noDpt"),
      })}</p>}
      <label>{t("addressTable.members.direction")}
        <select data-ga-member-direction value={draft.direction} disabled={busy}
          onChange={(event) => { setDraft((current) => ({ ...current, direction: event.target.value as MemberDirection })); setError(null); setAdded(null); }}>
          {MEMBER_DIRECTIONS.map((direction) => <option key={direction} value={direction}>{t(DIRECTION_KEYS[direction])}</option>)}
        </select>
      </label>
      {duplicate && <p className="field-error" role="status">{t("addressTable.members.duplicate")}</p>}
    </>}
    {error && <p className="field-error" role="alert">{error}</p>}
    {added && <p className="ga-member-added" role="status">{added}</p>}
    <div className="ga-member-actions">
      <button type="submit" className="primary-action" disabled={busy || chosen === undefined || duplicate}>{t("addressTable.members.submit")}</button>
      <button type="button" disabled={busy} onClick={() => { setOpen(false); setDraft(EMPTY_DRAFT); setError(null); }}>{t("addressTable.members.close")}</button>
    </div>
  </form>;
}
