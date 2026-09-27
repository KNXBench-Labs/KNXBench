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
 * Ends the application without asking again: callers have already run the
 * unsaved-changes check (File › Quit on a clean project, or the quit-confirm
 * dialog's "discard"). `destroy()`, not `close()`: `close()` emits
 * `tauri://close-requested`, which `onWindowCloseRequested` below answers
 * with the very check that just passed — for a modified project that would
 * reopen the dialog the user has just dismissed. Needs
 * `core:window:allow-destroy` in `capabilities/default.json`. Ending the
 * process is the other half, and it lives in Rust —
 * `apps/knx-desktop/src-tauri/src/lib.rs`'s `on_window_event` clause turns
 * the main window's destruction into `AppHandle::exit(0)`.
 *
 * The import is dynamic for the same reason `diagnosticsWindow.ts`'s is:
 * the module must not be pulled into the browser bundle's startup path,
 * where there is no Tauri to talk to.
 */
export async function quitApp(): Promise<void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().destroy();
}

/**
 * Routes the window manager's close (title-bar ×, Alt+F4, a compositor
 * keybinding) through `shouldClose` — KNOWN_LIMITATIONS §132. While a JS
 * listener for `tauri://close-requested` exists, the pinned `tauri` crate
 * calls `prevent_close()` itself (`manager/window.rs`, `on_window_event`),
 * and `@tauri-apps/api`'s `onCloseRequested` destroys the window after the
 * handler unless it called `preventDefault()`. So `shouldClose` returning
 * `false` keeps the window, and `true` closes it exactly as before.
 *
 * Returns the unlisten function; call it on unmount.
 */
export async function onWindowCloseRequested(shouldClose: () => boolean): Promise<() => void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().onCloseRequested((event) => {
    if (!shouldClose()) event.preventDefault();
  });
}
