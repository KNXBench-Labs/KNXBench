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

  it("setDeviceDescription posts to /api/device-description with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setDeviceDescription(7, "Flur, links");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/device-description");
    expect(JSON.parse(init.body as string)).toEqual({
      deviceId: 7,
      description: "Flur, links",
    });
  });

  it("setComObjectDescription posts to /api/com-object-description with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setComObjectDescription(3, "Aktoreingang 1");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/com-object-description");
    expect(JSON.parse(init.body as string)).toEqual({
      comObjectId: 3,
      description: "Aktoreingang 1",
    });
  });

  it("createGroupAddress omits rangeId when not given", async () => {
    mockFetchOnce({ installations: [] });
    await api.createGroupAddress("Light on/off", "1/1/1");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-addresses");
    expect(JSON.parse(init.body as string)).toEqual({ name: "Light on/off", address: "1/1/1" });
  });

  it("createGroupAddress includes rangeId when given", async () => {
    mockFetchOnce({ installations: [] });
    await api.createGroupAddress("Light on/off", "1/1/1", 3);
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({
      name: "Light on/off",
      address: "1/1/1",
      rangeId: 3,
    });
  });

  it("createGroupRange posts to /api/group-ranges with camelCase parentId", async () => {
    mockFetchOnce({ installations: [] });
    await api.createGroupRange("Lighting", "1/0/0", "1/7/255", 5);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-ranges");
    expect(JSON.parse(init.body as string)).toEqual({
      name: "Lighting",
      start: "1/0/0",
      end: "1/7/255",
      parentId: 5,
    });
  });

  it("deleteGroupRange issues a DELETE to /api/group-ranges/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteGroupRange(9);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-ranges/9");
    expect(init.method).toBe("DELETE");
  });

  it("renameGroupRange issues a PATCH with the new name", async () => {
    mockFetchOnce({ installations: [] });
    await api.renameGroupRange(9, "Blinds");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-ranges/9");
    expect(init.method).toBe("PATCH");
    expect(JSON.parse(init.body as string)).toEqual({ name: "Blinds" });
  });

  it("errorMessage unwraps an Error's message without doubling 'Error: '", () => {
    expect(api.errorMessage(new Error("no project open"))).toBe("no project open");
  });

  it("errorMessage falls back to String(e) for a non-Error throw", () => {
    expect(api.errorMessage("plain string")).toBe("plain string");
    expect(api.errorMessage(42)).toBe("42");
  });

  it("errorMessage round-trips a rejection thrown by request()", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    try {
      await api.undo();
      throw new Error("expected api.undo() to reject");
    } catch (e) {
      expect(api.errorMessage(e)).toBe("no project open");
    }
  });
});
