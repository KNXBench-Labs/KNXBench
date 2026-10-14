/** Reload/tab-close guard (browser and desktop webview) while an editor draft exists. */
// The workbench's own exits (Open, Import, New, Quit, desktop window close)
// ask through the draft dialog. A reload or tab close cannot show that
// dialog; the only thing a page may do is ask the engine to confirm (it shows
// its own wording: the browser's, or WebKitGTK's Stay/Leave dialog in the
// desktop shell). The draft state is read at unload time, so the listener is
// installed once.

/** Installs the guard on `target` (normally `window`); returns its removal. */
export function installDraftUnloadGuard(target: EventTarget, hasDraft: () => boolean): () => void {
  const listener = (event: Event) => {
    if (!hasDraft()) return;
    event.preventDefault();
    // Engines without preventDefault() support for beforeunload only honour
    // a truthy legacy returnValue.
    (event as Event & { returnValue: unknown }).returnValue = true;
  };
  target.addEventListener("beforeunload", listener);
  return () => target.removeEventListener("beforeunload", listener);
}
