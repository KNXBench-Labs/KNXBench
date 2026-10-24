//! Two ways to pick a project file: native OS dialogs
//! (`@tauri-apps/plugin-dialog`, only functional inside the Tauri shell —
//! `window.__TAURI__` marks that) and the server-mount picker
//! (`FsPicker.tsx`) for the plain web build. `App.tsx` calls these
//! instead of choosing between the two itself.
//!
//! Native dialogs start in the desktop shell's default dialog directory
//! (`default_dialog_directory`, the same policy as its Rust-side dialogs), so
//! an AppImage never opens one inside its read-only mount. When the shell
//! reports none or the query fails, the dialog keeps the toolkit default.
import { invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";
import { open as tauriOpen, save as tauriSave } from "@tauri-apps/plugin-dialog";
import { openMountPicker, saveMountPicker } from "./FsPicker";

interface Filter {
  name: string;
  extensions: string[];
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

/** The shell's start folder for native dialogs, or null; never throws. */
async function defaultDialogDirectory(): Promise<string | null> {
  try {
    const directory = await invoke<string | null>("default_dialog_directory");
    return typeof directory === "string" && directory !== "" ? directory : null;
  } catch {
    return null;
  }
}

/** `defaultName` inside the start folder, or the bare name as before. */
async function saveDefaultPath(defaultName: string): Promise<string> {
  const directory = await defaultDialogDirectory();
  if (directory === null) return defaultName;
  try {
    return await join(directory, defaultName);
  } catch {
    return defaultName;
  }
}

export async function pickOpenPath(filters: Filter[]): Promise<string | null> {
  if (isTauri()) {
    const directory = await defaultDialogDirectory();
    const path = await tauriOpen(
      directory === null ? { multiple: false, filters } : { multiple: false, filters, defaultPath: directory },
    );
    return typeof path === "string" ? path : null;
  }
  return openMountPicker(filters);
}

export async function pickSavePath(filters: Filter[], defaultName: string): Promise<string | null> {
  if (isTauri()) {
    const path = await tauriSave({ filters, defaultPath: await saveDefaultPath(defaultName) });
    return typeof path === "string" ? path : null;
  }
  return saveMountPicker(defaultName);
}
