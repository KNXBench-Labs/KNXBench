/** Existing directional unlink actions, shared by every group-address view. */
import { useState } from "react";
import * as api from "./api";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { directionLabel } from "./groupAddressView";
import { useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import DeviceLink from "./DeviceLink";
function LinkRow(props: {
  ga: GroupAddressNode;
  link: GroupAddressLinkNode;
  onTreeUpdate: (tree: ProjectTree) => void;
}) {
  const { ga, link, onTreeUpdate } = props;
  const formatGa = useGroupAddressFormat();
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  // `Command::UnlinkComObject` carries no `installations[0]`-only
  // restriction (see `Inspector.tsx`'s `GroupLinkRow`, which calls the
  // same command from the communication object's side), so this is offered
  // for every address. Linking from this side is not: choosing a
  // communication object needs a device/object picker that does not exist
  // yet, and the Inspector's existing `NewGroupLinkRow` remains the one
  // place a link is created.
  async function unlink() {
    setError(null);
    try {
      onTreeUpdate(await api.unlinkComObject(link.com_object_id, ga.id, link.direction));
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <tr>
      <td>
        <DeviceLink deviceId={link.device_id}>{link.device_name ?? t("addressTable.unknownDevice", { id: link.device_id })}</DeviceLink>
        {link.device_address && <small className="mono"> {link.device_address}</small>}
      </td>
      <td>
        {link.com_object_name ?? t("addressTable.unnamedObject")}
        <small className="mono"> #{link.com_object_number}</small>
      </td>
      <td>{directionLabel(t, link.direction)}</td>
      <td>
        <button
          onClick={unlink}
          aria-label={t("addressTable.unlinkFrom", {
            object: link.com_object_name ?? `#${link.com_object_number}`,
            address: formatGa(ga.address),
          })}
        >
          {t("inspector.unlink")}
        </button>
        {error && <span className="field-error">{error}</span>}
      </td>
    </tr>
  );
}

export default function GroupAddressLinks(props: { ga: GroupAddressNode; onTreeUpdate: (tree: ProjectTree) => void }) {
  const { ga: selected, onTreeUpdate } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  return (
        <section className="address-links-panel" aria-label={t("addressTable.links")}>
          <h2>{t("addressTable.linksFor", { address: formatGa(selected.address) })}</h2>
          {selected.links.length === 0 ? (
            <p role="status">{t("addressTable.noLinksYet")}</p>
          ) : (
            <table className="workspace-table">
              <thead>
                <tr>
                  <th>{t("addressTable.participant")}</th>
                  <th>{t("addressTable.function")}</th>
                  <th>{t("addressTable.direction")}</th>
                  <th>
                    <span className="sr-only">{t("inspector.unlink")}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {selected.links.map((link) => (
                  <LinkRow
                    key={`${link.com_object_id}-${link.direction}`}
                    ga={selected}
                    link={link}
                    onTreeUpdate={onTreeUpdate}
                  />
                ))}
              </tbody>
            </table>
          )}
        </section>
  );
}
