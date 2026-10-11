/** Fixed-origin, credential-free GitHub adapter; release content is never executed. */
export const GITHUB_RELEASES_API = "https://api.github.com/repos/KNXBench-Labs/KNXBench/releases?per_page=100";
export const RELEASE_REQUEST_TIMEOUT_MS = 10_000;
export const MAX_RELEASE_RESPONSE_BYTES = 4 * 1024 * 1024;

export async function fetchGitHubReleases(signal?: AbortSignal): Promise<unknown[]> {
  const controller = new AbortController();
  const abort = () => controller.abort();
  if (signal?.aborted) abort();
  signal?.addEventListener("abort", abort, { once: true });
  const timeout = setTimeout(abort, RELEASE_REQUEST_TIMEOUT_MS);
  let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
  try {
    const response = await fetch(GITHUB_RELEASES_API, {
      headers: { Accept: "application/vnd.github+json" }, credentials: "omit",
      referrerPolicy: "no-referrer", redirect: "error", signal: controller.signal,
    });
    if (!response.ok || !response.body) throw new Error("Release discovery unavailable");
    reader = response.body.getReader();
    const decoder = new TextDecoder();
    let size = 0, text = "";
    for (;;) {
      const part = await reader.read();
      if (part.done) break;
      size += part.value.byteLength;
      if (size > MAX_RELEASE_RESPONSE_BYTES) throw new Error("Release response exceeds limit");
      text += decoder.decode(part.value, { stream: true });
    }
    text += decoder.decode();
    const rows: unknown = JSON.parse(text);
    if (!Array.isArray(rows) || rows.length > 100) throw new Error("Invalid release response");
    return rows;
  } finally {
    clearTimeout(timeout);
    signal?.removeEventListener("abort", abort);
    await reader?.cancel().catch(() => {});
  }
}
