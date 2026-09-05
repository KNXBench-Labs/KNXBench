import { describe, expect, it, vi, beforeEach } from "vitest";
import * as api from "./api";

function mockFetchOnce(body: unknown, ok = true, status = 200) {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok,
      status,
      statusText: "",
      headers: new Headers({ "content-length": ok ? "2" : "0" }),
      json: async () => body,
    }),
  );
}

describe("api", () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it("importProject posts the path and returns the parsed tree", async () => {
    mockFetchOnce({ installations: [] });
    const tree = await api.importProject("/x.knxproj");
    expect(tree).toEqual({ installations: [] });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/import");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/x.knxproj" });
  });

  it("deviceDetail issues a GET to /api/device/:id", async () => {
    mockFetchOnce({ id: 1, name: "D1" });
    await api.deviceDetail(1);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/device/1");
    expect(init?.method ?? "GET").toBe("GET");
  });

  it("throws the server's error message on a non-ok response", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.undo()).rejects.toThrow("no project open");
  });

  it("setIndividualAddress sends camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setIndividualAddress(7, "1.1.1");
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 7, address: "1.1.1" });
  });
});
