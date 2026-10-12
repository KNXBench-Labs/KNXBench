/** Deliver a JSON snapshot to the user's browser without a server-side path. */
export function downloadLocalJson(contents: string, filename: string): void {
  downloadLocalBlob(new Blob([contents], { type: "application/json;charset=utf-8" }), filename);
}

/** Deliver inert bytes through the existing local browser download mechanism. */
export function downloadLocalBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.append(anchor);
  try {
    anchor.click();
  } finally {
    anchor.remove();
    // Some browsers do not begin the download until after the click returns.
    window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }
}
