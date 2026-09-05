//! Two ways to pick a project file: native OS dialogs
//! (`@tauri-apps/plugin-dialog`, only functional inside the Tauri shell —
//! `window.__TAURI__` marks that) and the server-mount picker
//! (`FsPicker.tsx`) for the plain web build. `App.tsx` calls these
//! instead of choosing between the two itself.
import { open as tauriOpen, save as tauriSave } from "@tauri-apps/plugin-dialog";
import { openMountPicker, saveMountPicker } from "./FsPicker";

interface Filter {
  name: string;
  extensions: string[];
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

export async function pickOpenPath(filters: Filter[]): Promise<string | null> {
  if (isTauri()) {
    const path = await tauriOpen({ multiple: false, filters });
    return typeof path === "string" ? path : null;
  }
  return openMountPicker(filters);
}

export async function pickSavePath(filters: Filter[], defaultName: string): Promise<string | null> {
  if (isTauri()) {
    const path = await tauriSave({ filters, defaultPath: defaultName });
    return typeof path === "string" ? path : null;
  }
  return saveMountPicker(defaultName);
}
