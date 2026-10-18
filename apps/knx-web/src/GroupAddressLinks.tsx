/** Members of one group address, shared by every group-address view and the Inspector. */
import ResizableTable from "./ResizableTable";
import { useState } from "react";
import * as api from "./api";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { directionLabel } from "./groupAddressView";
import { canAddMembers } from "./groupAddressMembers";
import { useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import DeviceLink from "./DeviceLink";
import GroupAddressMemberAdd from "./GroupAddressMemberAdd";

const GROUP_ADDRESS_MEMBERS_COLUMNS = [
  { id: "device", width: 220, min: 120 },
  { id: "object", width: 240, min: 120 },
  { id: "direction", width: 160, min: 100 },
  { id: "actions", width: 140, min: 100 }
];

// A member is one communication object's link (Send or Receive), not a
// device: an object linked both ways is two members, as it holds two links.
function useUnlink(ga: GroupAddressNode, link: GroupAddressLinkNode, onTreeUpdate: (tree: ProjectTree) => void) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // `Command::UnlinkComObject` removes whatever link exists, whichever
  // installation the address lives in (see `Inspector.tsx`'s
  // `GroupLinkRow`, the same command from the object's side), so it is
  // offered for every member.
  async function unlink() {
    if (busy) return;
    setError(null); setBusy(true);
    try {
      onTreeUpdate(await api.unlinkComObject(link.com_object_id, ga.id, link.direction));
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  return { error, busy, unlink };
}

function MemberDevice({ link }: { link: GroupAddressLinkNode }) {
  const t = useTranslate();
  return <>
    <DeviceLink deviceId={link.device_id}>{link.device_name ?? t("addressTable.unknownDevice", { id: link.device_id })}</DeviceLink>
    {link.device_address && <small className="mono"> {link.device_address}</small>}
  </>;
}

function MemberObject({ link }: { link: GroupAddressLinkNode }) {
  const t = useTranslate();
  return <>{link.com_object_name ?? t("addressTable.unnamedObject")}<small className="mono"> #{link.com_object_number}</small></>;
}

function UnlinkButton(props: { ga: GroupAddressNode; link: GroupAddressLinkNode; state: ReturnType<typeof useUnlink> }) {
  const { ga, link, state } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  return <>
    <button type="button" onClick={state.unlink} disabled={state.busy}
      aria-label={t("addressTable.unlinkFrom", { object: link.com_object_name ?? `#${link.com_object_number}`, address: formatGa(ga.address) })}>
      {t("inspector.unlink")}
    </button>
    {state.error && <span className="field-error">{state.error}</span>}
  </>;
}

interface RowProps { ga: GroupAddressNode; link: GroupAddressLinkNode; onTreeUpdate: (tree: ProjectTree) => void }

function LinkRow(props: RowProps) {
  const t = useTranslate();
  const state = useUnlink(props.ga, props.link, props.onTreeUpdate);
  return <tr>
    <td><MemberDevice link={props.link} /></td>
    <td><MemberObject link={props.link} /></td>
    <td>{directionLabel(t, props.link.direction)}</td>
    <td><UnlinkButton ga={props.ga} link={props.link} state={state} /></td>
  </tr>;
}

function LinkItem(props: RowProps) {
  const t = useTranslate();
  const state = useUnlink(props.ga, props.link, props.onTreeUpdate);
  return <li className="ga-member">
    <span className="ga-member-device"><MemberDevice link={props.link} /></span>
    <span className="ga-member-object"><MemberObject link={props.link} /> · {directionLabel(t, props.link.direction)}</span>
    <span className="ga-member-unlink"><UnlinkButton ga={props.ga} link={props.link} state={state} /></span>
  </li>;
}

export default function GroupAddressLinks(props: {
  ga: GroupAddressNode;
  tree: ProjectTree;
  onTreeUpdate: (tree: ProjectTree) => void;
  /** `list` for the narrow Inspector; the workspace keeps its table. */
  variant?: "table" | "list";
}) {
  const { ga: selected, tree, onTreeUpdate, variant = "table" } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  // Imported duplicates of one link stay visible as separate rows.
  const key = (link: GroupAddressLinkNode, index: number) => `${link.com_object_id}-${link.direction}-${index}`;
  const heading = t("addressTable.linksFor", { address: formatGa(selected.address) });
  return (
    <section className={variant === "list" ? "address-links-panel ga-members-compact" : "address-links-panel"} aria-label={t("addressTable.links")}>
      {variant === "list" ? <h3>{heading}</h3> : <h2>{heading}</h2>}
      {selected.links.length === 0 ? (
        <p role="status">{t("addressTable.noLinksYet")}</p>
      ) : variant === "list" ? (
        <ul className="ga-members">
          {selected.links.map((link, index) => <LinkItem key={key(link, index)} ga={selected} link={link} onTreeUpdate={onTreeUpdate} />)}
        </ul>
      ) : (
        <ResizableTable tableId="group-address-members" columns={GROUP_ADDRESS_MEMBERS_COLUMNS} className="workspace-table">
          <thead>
            <tr>
              <th>{t("addressTable.participant")}</th>
              <th>{t("addressTable.function")}</th>
              <th>{t("addressTable.direction")}</th>
              <th><span className="sr-only">{t("inspector.unlink")}</span></th>
            </tr>
          </thead>
          <tbody>
            {selected.links.map((link, index) => <LinkRow key={key(link, index)} ga={selected} link={link} onTreeUpdate={onTreeUpdate} />)}
          </tbody>
        </ResizableTable>
      )}
      {canAddMembers(tree, selected.id)
        ? <GroupAddressMemberAdd ga={selected} tree={tree} onTreeUpdate={onTreeUpdate} />
        : <p className="inspector-description">{t("inspector.restrictedToOneInstallation", {
          action: t("inspector.restrictedAction.addMembers"), entity: t("inspector.entity.groupAddresses"),
        })}</p>}
    </section>
  );
}
