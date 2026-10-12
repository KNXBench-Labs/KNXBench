/** Four group-address presentation choices stored in application settings. */
import { useEffect, useState } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

export const GROUP_ADDRESS_VIEWS = ["tree", "columns", "atlas", "matrix"] as const;
export type GroupAddressPresentationView = typeof GROUP_ADDRESS_VIEWS[number];
const KEY = "groupAddressView";

export function loadGroupAddressView(storage: Pick<Storage, "getItem">): GroupAddressPresentationView {
  try {
    const value = storage.getItem(KEY);
    return GROUP_ADDRESS_VIEWS.find(view => view === value) ?? "tree";
  } catch {
    return "tree";
  }
}

export function saveGroupAddressView(
  storage: Pick<Storage, "setItem">,
  view: GroupAddressPresentationView,
): boolean {
  try {
    storage.setItem(KEY, view);
    return true;
  } catch {
    return false;
  }
}

export function useGroupAddressView(): [GroupAddressPresentationView, (view: GroupAddressPresentationView) => void] {
  const revision = useSettingsRevision();
  const [view, setView] = useState(() => loadGroupAddressView(settingsStorage));
  useEffect(() => { setView(loadGroupAddressView(settingsStorage)); }, [revision]);
  return [view, next => { setView(next); saveGroupAddressView(settingsStorage, next); }];
}
