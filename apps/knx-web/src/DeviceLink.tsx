/** Scoped device navigation by ID or one unique project-address match. */
import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import { collectDevices } from "./treeUtils";
import { catalogueMatchesTree, type DeviceCatalog } from "./deviceList";
import { useTranslate } from "./i18n";

interface DeviceNavigation {
  devices: Map<number, DeviceNode>;
  addresses: Map<string, DeviceNode[]>;
  open: (id: number) => void;
  openList: () => void;
}
const Navigation = createContext<DeviceNavigation | null>(null);

export function DeviceNavigationProvider({ tree, catalogue, onOpen, onList, children }: {
  tree: ProjectTree | null; catalogue?: DeviceCatalog | null; onOpen: (id: number) => void; onList: () => void; children: ReactNode;
}) {
  const index = useMemo(() => {
    const devices = tree ? collectDevices(tree) : new Map<number, DeviceNode>();
    if (tree && catalogue && catalogueMatchesTree(catalogue, tree)) {
      for (const row of catalogue.devices) if (!devices.has(row.id)) devices.set(row.id, row.device);
    }
    const addresses = new Map<string, DeviceNode[]>();
    for (const device of devices.values()) {
      if (device.address !== null) addresses.set(device.address, [...(addresses.get(device.address) ?? []), device]);
    }
    return { devices, addresses };
  }, [tree, catalogue]);
  return <Navigation.Provider value={{ ...index, open: onOpen, openList: onList }}>{children}</Navigation.Provider>;
}

export function useDeviceNavigation() { return useContext(Navigation); }

/** Known ids navigate directly; observed addresses need exactly one project device. */
export default function DeviceLink({ deviceId, address, children }: {
  deviceId?: number; address?: string | null; children?: ReactNode;
}) {
  const navigation = useDeviceNavigation();
  const t = useTranslate();
  if (!navigation) return <>{children ?? address}</>;
  const matches = address == null ? [] : navigation.addresses.get(address) ?? [];
  const device = deviceId !== undefined ? navigation.devices.get(deviceId)
    : matches.length === 1 ? matches[0] : undefined;
  const label = children ?? device?.name ?? address ?? `#${deviceId}`;
  if (!device) {
    const hint = t(deviceId !== undefined ? "devices.missingDevice"
      : matches.length > 1 ? "devices.ambiguousAddress" : "devices.noAddressMatch");
    return <span className="device-link-unresolved" title={hint}>{label}<span className="sr-only"> — {hint}</span></span>;
  }
  return <button type="button" className="device-link table-select" title={t("devices.openNamed", { name: device.name })}
    aria-label={t("devices.openNamed", { name: device.name })}
    onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") event.stopPropagation(); }}
    onClick={(event) => { event.stopPropagation(); navigation.open(device.id); }}>{label}</button>;
}
