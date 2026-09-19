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
 * Closes the main window. That is the whole job on this side: this only
 * needs `core:window:allow-close` in `capabilities/default.json`, the
 * narrowest permission that does anything here. Ending the process is the
 * other half, and it lives in Rust — `apps/knx-desktop/src-tauri/src/lib.rs`'s
 * `on_window_event` clause turns the main window's destruction into
 * `AppHandle::exit(0)`, which is also why the window manager's own close
 * button quits the whole application, not just this window.
 *
 * The import is dynamic for the same reason `diagnosticsWindow.ts`'s is:
 * the module must not be pulled into the browser bundle's startup path,
 * where there is no Tauri to talk to.
 */
export async function quitApp(): Promise<void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
