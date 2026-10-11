/** Verifies that release discovery contacts only the public official repository. */
import { afterEach, expect, it, vi } from "vitest";
import { fetchGitHubReleases, GITHUB_RELEASES_API } from "./githubReleases";
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); });
it("requests published releases without cookies, tokens, referrer or project data", async () => {
 const fetcher = vi.fn().mockResolvedValue(new Response("[]", { status: 200 }));
 vi.stubGlobal("fetch", fetcher);
 expect(await fetchGitHubReleases()).toEqual([]);
 expect(fetcher).toHaveBeenCalledWith(GITHUB_RELEASES_API, expect.objectContaining({ credentials: "omit", referrerPolicy: "no-referrer", redirect: "error", signal: expect.any(AbortSignal) }));
});

it.each([403, 404, 429, 500])("reports HTTP %s as unavailable, never up-to-date", async status => {
 vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("{}", { status })));
 await expect(fetchGitHubReleases()).rejects.toThrow("unavailable");
});
it.each(["not JSON", "{}", JSON.stringify(Array(101).fill({}))])("rejects malformed or unbounded release data", async body => {
 vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(body)));
 await expect(fetchGitHubReleases()).rejects.toThrow();
});
it("bounds the actual streamed response, not an untrusted content-length header", async () => {
 vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(" ".repeat(4 * 1024 * 1024 + 1))));
 await expect(fetchGitHubReleases()).rejects.toThrow("exceeds limit");
});
it("aborts a request at ten seconds and honors caller cancellation", async () => {
 vi.useFakeTimers();
 let signal: AbortSignal | undefined;
 vi.stubGlobal("fetch", vi.fn((_url, init) => {
   signal = init.signal;
   return new Promise((_resolve, reject) => { if (signal!.aborted) reject(new Error("aborted")); else signal!.addEventListener("abort", () => reject(new Error("aborted"))); });
 }));
 const pending = expect(fetchGitHubReleases()).rejects.toThrow("aborted");
 await vi.advanceTimersByTimeAsync(9999); expect(signal!.aborted).toBe(false);
 await vi.advanceTimersByTimeAsync(1); await pending; expect(signal!.aborted).toBe(true);
 const owner = new AbortController(); const cancelled = expect(fetchGitHubReleases(owner.signal)).rejects.toThrow("aborted");
 owner.abort(); await cancelled;
});
