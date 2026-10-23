/** Platform-aware Quit and the desktop's unsaved-changes close check. */
import { isTauri } from "./filePicker";

/** A browser cannot close a tab it did not open. */
export function canQuit(): boolean {
  return isTauri();
}

// File > Quit and the discard dialog already obtained approval. Consume it
// on exactly one close-requested event, rather than bypassing native close.
let approvedClose = false;

/**
 * Request native close after the caller's unsaved-changes check. The close
 * event still runs; its one-shot approval avoids reopening the same dialog.
 * Rust's Destroyed/ExitRequested hooks own process shutdown and server drain.
 * The dynamic import keeps Tauri out of the browser startup path.
 */
export async function quitApp(): Promise<void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  approvedClose = true;
  try {
    await getCurrentWindow().close();
  } catch (error) {
    approvedClose = false;
    throw error;
  }
}

/**
 * Route the window manager's close through the same unsaved-changes check.
 * Tauri prevents native close while this listener exists, then destroys the
 * window after the callback unless preventDefault() was called. That accepted
 * destruction is intercepted by Rust before process exit, not a drain bypass.
 */
export async function onWindowCloseRequested(shouldClose: () => boolean): Promise<() => void> {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow().onCloseRequested((event) => {
    if (approvedClose) {
      approvedClose = false;
    } else if (!shouldClose()) {
      event.preventDefault();
    }
  });
}
