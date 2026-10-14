/** Devices whose object lists the matrix may read; every state stays visible. */
import type { GroupAddressObjectInventory } from "./groupAddressObjectInventory";
import { loadableDevices, objectInventoryState } from "./groupAddressObjectInventory";
import { useTranslate } from "./i18n";

export default function GroupAddressObjectInventoryList(props: { inventory: GroupAddressObjectInventory; query: string }) {
  const t = useTranslate();
  const { inventory, query } = props;
  const devices = loadableDevices(inventory.devices).filter(device =>
    query === "" || [device.name, device.address ?? ""].some(value => value.toLowerCase().includes(query)));
  return <div className="ga-inventory-devices" role="group" aria-label={t("gaWorkspace.inventoryDevices")}>
    <p className="ga-evidence-note">{t("gaWorkspace.inventoryDevices")}</p>
    <ul>{devices.map(device => {
      const entry = inventory.entries.get(device.id);
      const state = objectInventoryState(entry, inventory.source);
      const failure = entry?.status === "failed"
        ? (entry.mismatch ? t("gaWorkspace.inventory.mismatch") : entry.message ?? t("gaWorkspace.inventory.unknownError")) : null;
      return <li key={device.id} data-ga-inventory-device={device.id} data-ga-inventory-state={state}>
        <span>{device.name} · {device.address ?? t("gaWorkspace.noDeviceAddress")}
          <small>{t("gaWorkspace.objectCount", { count: device.com_object_count })}</small></span>
        <span className="ga-inventory-actions"><span className={`ga-inventory-state${state === "loading" && failure === null ? " task-status-text" : ""}`}>{failure !== null ? t("gaWorkspace.inventory.failed", { message: failure })
          : t(state === "loaded" ? "gaWorkspace.inventory.loaded" : state === "loading" ? "gaWorkspace.inventory.loading" : "gaWorkspace.inventory.notLoaded")}</span>
        {(state === "not-loaded" || state === "failed") && <button type="button" data-ga-load-objects={device.id}
          onClick={() => inventory.load(device.id)}>{t(state === "failed" ? "gaWorkspace.retryObjects" : "gaWorkspace.loadObjects")}</button>}</span>
      </li>;
    })}</ul>
  </div>;
}
