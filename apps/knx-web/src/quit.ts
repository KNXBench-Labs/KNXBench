/** Whether a Quit item means anything on this platform, and what it does when it does. */
// F4. A browser tab cannot close itself — `window.close()` is refused for
// any document the script did not open — so outside the Tauri shell a Quit
// item would be a control that does nothing, which is worse than no
// control. The detection is `filePicker.ts`'s `isTauri`, reused rather
// than written a second time: two answers to "are we in the shell?" is one
// answer too many.
import { isTauri } from "./filePicker";

/** Whether the File menu should carry a Quit item at all. */
export function canQuit(): boolean {
  return isTauri();
}

/**
 * Closes the editing window. The desktop shell builds exactly one window
 * up front (`apps/knx-desktop/src-tauri/src/lib.rs`'s `setup`), so closing
 * it ends the process — with the one exception of a diagnostics companion
 * still being open, recorded in KNOWN_LIMITATIONS.md §103.
 *
 * `close()` rather than a process-level `exit()`: the latter needs
 * `tauri-plugin-process` on both sides, and this needs only
 * `core:window:allow-close` in `capabilities/default.json` — the narrowest
 * permission that does the job. The import is dynamic for the same reason
 * `diagnosticsWindow.ts`'s is: the module must not be pulled into the
 * browser bundle's startup path, where there is no Tauri to talk to.
 */
export async function quitApp(): Promise<void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
