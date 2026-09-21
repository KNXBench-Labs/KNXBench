import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "./api";
import { subscribeSessionExpired } from "./session";

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
  // T01b: `request()` publishes "the server wants a session" through
  // `session.ts` rather than importing anything that renders. Subscribing
  // here is how these tests see it happen.
  const sessionExpired = vi.fn();
  let unsubscribe: () => void = () => undefined;

  beforeEach(() => {
    vi.unstubAllGlobals();
    sessionExpired.mockClear();
    unsubscribe = subscribeSessionExpired(sessionExpired);
  });

  afterEach(() => {
    unsubscribe();
  });

  it("writeBusValue sends the selected input format explicitly", async () => {
    mockFetchOnce({
      encodedPayload: "[10]",
      service: "GroupValueWrite",
      decodedEcho: { kind: "value", dpt: "DPST-21-1", text: "00010000" },
    });
    await api.writeBusValue("1/2/3", "DPST-21-1", "10", "hexadecimal");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/bus/write");
    expect(JSON.parse(init.body as string)).toEqual({
      destination: "1/2/3",
      dpt: "DPST-21-1",
      inputFormat: "hexadecimal",
      value: "10",
    });
  });

  it("importProject posts the path and client token, and returns the parsed tree", async () => {
    mockFetchOnce({ installations: [] });
    const tree = await api.importProject("/x.knxproj", "a-client-token");
    expect(tree).toEqual({ installations: [] });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/import");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/x.knxproj", clientToken: "a-client-token" });
  });

  // Fix round 4, F11: the same assertion for the native open, which had
  // none. Dropping `clientToken` from this one body left every gate green
  // while the banner froze on "Starting…" for every `.knxdb` — the
  // import/open asymmetry is the shape three earlier rounds kept.
  it("openProject posts the path and client token, and returns the parsed tree", async () => {
    mockFetchOnce({ installations: [] });
    const tree = await api.openProject("/x.knxdb", "a-client-token");
    expect(tree).toEqual({ installations: [] });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/open");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/x.knxdb", clientToken: "a-client-token" });
  });

  it("newProject posts the camelCase creation body, discardChanges spelled out", async () => {
    mockFetchOnce({ installations: [] });
    await api.newProject({
      name: "Scratch",
      installationName: "Ground floor",
      language: "de-DE",
      groupAddressStyle: "TwoLevel",
    });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/new");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({
      name: "Scratch",
      installationName: "Ground floor",
      language: "de-DE",
      groupAddressStyle: "TwoLevel",
      discardChanges: false,
    });
  });

  it("newProject sends discardChanges only when it was asked for", async () => {
    mockFetchOnce({ installations: [] });
    await api.newProject({
      name: "Scratch",
      installationName: "",
      language: "en",
      groupAddressStyle: "ThreeLevel",
      discardChanges: true,
    });
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string).discardChanges).toBe(true);
  });

  it("newProject's 409 is recognisable as the unsaved-changes refusal, and nothing else is", async () => {
    mockFetchOnce({ error: "the open project has unsaved changes" }, false, 409);
    const conflict = await api
      .newProject({ name: "x", installationName: "", language: "en", groupAddressStyle: "Free" })
      .then(() => null)
      .catch((e: unknown) => e);
    expect(api.errorMessage(conflict)).toContain("unsaved changes");
    expect(api.isUnsavedChangesConflict(conflict)).toBe(true);

    mockFetchOnce({ error: "no project open" }, false, 400);
    const other = await api
      .newProject({ name: "x", installationName: "", language: "en", groupAddressStyle: "Free" })
      .then(() => null)
      .catch((e: unknown) => e);
    expect(api.isUnsavedChangesConflict(other)).toBe(false);
    expect(api.isUnsavedChangesConflict(new Error("not from request() at all"))).toBe(false);
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

  it("setComObjectFlag posts to /api/com-object-flag with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.setComObjectFlag(3, "Communication", true);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/com-object-flag");
    expect(JSON.parse(init.body as string)).toEqual({
      comObjectId: 3,
      flag: "Communication",
      value: true,
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

  it("createBuildingPart posts to /api/building-parts with camelCase parentId", async () => {
    mockFetchOnce({ installations: [] });
    await api.createBuildingPart("Main building", "Building", 5);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts");
    expect(JSON.parse(init.body as string)).toEqual({
      name: "Main building",
      kind: "Building",
      parentId: 5,
    });
  });

  it("deleteBuildingPart issues a DELETE to /api/building-parts/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteBuildingPart(9);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts/9");
    expect(init.method).toBe("DELETE");
  });

  it("renameBuildingPart issues a PATCH with the new name", async () => {
    mockFetchOnce({ installations: [] });
    await api.renameBuildingPart(9, "Living room");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/building-parts/9");
    expect(init.method).toBe("PATCH");
    expect(JSON.parse(init.body as string)).toEqual({ name: "Living room" });
  });

  it("moveDeviceToBuildingPart posts a part id", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToBuildingPart(9, 7);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/move-device-to-building-part");
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, partId: 7 });
  });

  it("moveDeviceToBuildingPart posts null to un-place the device", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToBuildingPart(9, null);
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, partId: null });
  });

  it("linkComObject posts to /api/group-links with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.linkComObject(3, 9, "Send");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-links");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({
      comObjectId: 3,
      gaId: 9,
      direction: "Send",
    });
  });

  it("unlinkComObject issues a DELETE to /api/group-links with a body", async () => {
    mockFetchOnce({ installations: [] });
    await api.unlinkComObject(3, 9, "Receive");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-links");
    expect(init.method).toBe("DELETE");
    expect(JSON.parse(init.body as string)).toEqual({
      comObjectId: 3,
      gaId: 9,
      direction: "Receive",
    });
  });

  it("createArea posts name and address", async () => {
    mockFetchOnce({ installations: [] });
    await api.createArea("Ground floor", 1);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/areas");
    expect(JSON.parse(init.body as string)).toEqual({ name: "Ground floor", address: 1 });
  });

  it("deleteArea issues a DELETE to /api/areas/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteArea(4);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/areas/4");
    expect(init.method).toBe("DELETE");
  });

  it("createLine posts to /api/lines with camelCase field names", async () => {
    mockFetchOnce({ installations: [] });
    await api.createLine(4, "Main line", 1, "MT-0");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/lines");
    expect(JSON.parse(init.body as string)).toEqual({
      areaId: 4,
      name: "Main line",
      address: 1,
      mediumRef: "MT-0",
    });
  });

  it("deleteLine issues a DELETE to /api/lines/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteLine(7);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/lines/7");
    expect(init.method).toBe("DELETE");
  });

  it("moveDeviceToLine posts a line id", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToLine(9, 7);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/move-device");
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, lineId: 7 });
  });

  it("moveDeviceToLine posts null to unassign", async () => {
    mockFetchOnce({ installations: [] });
    await api.moveDeviceToLine(9, null);
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({ deviceId: 9, lineId: null });
  });

  it("catalogManufacturers issues a GET to /api/catalog/manufacturers", async () => {
    mockFetchOnce([{ id: "M1", name: "ACME" }]);
    const rows = await api.catalogManufacturers();
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/catalog/manufacturers");
    expect(init?.method ?? "GET").toBe("GET");
    expect(rows).toEqual([{ id: "M1", name: "ACME" }]);
  });

  it("catalogItems with no filters omits the query string", async () => {
    mockFetchOnce([]);
    await api.catalogItems();
    const [url] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/catalog/items");
  });

  it("catalogItems encodes manufacturer and search as query params", async () => {
    mockFetchOnce([]);
    await api.catalogItems("M1", "switch actuator");
    const [url] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/catalog/items?manufacturer=M1&search=switch+actuator");
  });

  it("createDevice posts camelCase field names with a line id", async () => {
    mockFetchOnce({ installations: [] });
    await api.createDevice(7, "cat-1", "New actuator");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/devices");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({
      lineId: 7,
      catalogItemId: "cat-1",
      name: "New actuator",
    });
  });

  it("createDevice omits lineId when creating an unassigned device", async () => {
    mockFetchOnce({ installations: [] });
    await api.createDevice(null, "cat-1", "New actuator");
    const [, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(init.body as string)).toEqual({
      catalogItemId: "cat-1",
      name: "New actuator",
    });
  });

  it("createDevice returns the tree together with machine-readable diagnostics", async () => {
    mockFetchOnce({
      tree: { installations: [] },
      diagnostics: [{ kind: "programlessProduct", catalogItemId: "cat-1" }],
    });
    const response = await api.createDevice(null, "cat-1", "Passive device");
    expect(response.tree.installations).toEqual([]);
    expect(response.diagnostics).toEqual([{ kind: "programlessProduct", catalogItemId: "cat-1" }]);
  });

  it("installProductPackage posts the selected package as multipart data", async () => {
    mockFetchOnce({ sha256: "abc", scheme: 11, skipped: false, members: [], unknown: 0, conflicts: 0 });
    const file = new File(["package"], "vendor.knxprod", { type: "application/zip" });
    const report = await api.installProductPackage(file);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/catalog/install");
    expect(init.method).toBe("POST");
    expect(init.body).toBeInstanceOf(FormData);
    expect((init.body as FormData).get("file")).toBe(file);
    expect(report.scheme).toBe(11);
  });

  it("deleteDevice issues a DELETE to /api/devices/:id", async () => {
    mockFetchOnce({ installations: [] });
    await api.deleteDevice(12);
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/devices/12");
    expect(init.method).toBe("DELETE");
  });

  it("exportGroupAddressesCsv posts the path to /api/group-addresses/csv-export", async () => {
    mockFetchOnce({ warnings: [] });
    const report = await api.exportGroupAddressesCsv("/data/group-addresses.csv");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-addresses/csv-export");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/data/group-addresses.csv" });
    expect(report).toEqual({ warnings: [] });
  });

  it("exportGroupAddressesCsv surfaces the server's error message on a 400", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.exportGroupAddressesCsv("/data/group-addresses.csv")).rejects.toThrow(
      "no project open",
    );
  });

  it("importGroupAddressesCsv posts the path to /api/group-addresses/csv-import", async () => {
    mockFetchOnce({
      tree: { installations: [] },
      report: {
        separator: ",",
        rowsRead: 1,
        created: 1,
        updated: 0,
        unchanged: 0,
        ignoredColumns: [],
        problems: [],
      },
    });
    const response = await api.importGroupAddressesCsv("/data/in.csv");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/group-addresses/csv-import");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/data/in.csv" });
    expect(response.report.created).toBe(1);
    expect(response.tree.installations).toEqual([]);
  });

  it("importGroupAddressesCsv surfaces the server's row-naming error message on a 400", async () => {
    mockFetchOnce(
      { error: "1 row(s) rejected, nothing applied: row 4: unknown group address style" },
      false,
      400,
    );
    await expect(api.importGroupAddressesCsv("/data/bad.csv")).rejects.toThrow(
      "1 row(s) rejected, nothing applied: row 4: unknown group address style",
    );
  });

  it("exportDocumentation posts the path to /api/project/documentation-export", async () => {
    mockFetchOnce({ warnings: [] });
    const report = await api.exportDocumentation("/data/project.html");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/documentation-export");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/data/project.html" });
    expect(report).toEqual({ warnings: [] });
  });

  it("exportDocumentation surfaces the server's error message on a 400", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.exportDocumentation("/data/project.html")).rejects.toThrow(
      "no project open",
    );
  });

  it("diffProject posts the path to /api/project/diff", async () => {
    mockFetchOnce({ infoChanges: [], installations: [] });
    const report = await api.diffProject("/data/compare.knxdb");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/project/diff");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ path: "/data/compare.knxdb" });
    expect(report).toEqual({ infoChanges: [], installations: [] });
  });

  it("diffProject surfaces the server's error message on a 400", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.diffProject("/data/compare.knxdb")).rejects.toThrow("no project open");
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
  // ── T01b / ADR-0026 — the three auth endpoints and the 401 seam ─────────

  it("authStatus asks the server, and reads back both flags", async () => {
    mockFetchOnce({ required: true, authenticated: false });
    const status = await api.authStatus();
    expect(status).toEqual({ required: true, authenticated: false });
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/auth/status");
    // A GET: it must not refresh the session's idle clock, and the server
    // only promises that for the GET.
    expect(init?.method).toBeUndefined();
  });

  it("login puts the password in the body and nowhere else", async () => {
    mockFetchOnce({ authenticated: true });
    await api.login("correct horse battery staple");
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/auth/login");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ password: "correct horse battery staple" });
    // The one assertion worth spelling out: no query string, ever.
    expect(String(url)).not.toContain("correct");
  });

  it("login rejects with the server's status so the screen can tell 401 from 400", async () => {
    mockFetchOnce({ error: "invalid password" }, false, 401);
    await expect(api.login("wrong")).rejects.toThrow("invalid password");
    expect(sessionExpired).not.toHaveBeenCalled();
  });

  it("logout posts with no body at all", async () => {
    mockFetchOnce({ authenticated: false });
    await api.logout();
    const [url, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("/api/auth/logout");
    expect(init.method).toBe("POST");
    expect(init.body).toBeUndefined();
  });

  it("publishes a session expiry when an ordinary call is refused", async () => {
    mockFetchOnce({ error: "authentication required" }, false, 401);
    await expect(api.undo()).rejects.toThrow("authentication required");
    expect(sessionExpired).toHaveBeenCalledTimes(1);
  });

  it("says nothing about the session when a call fails for any other reason", async () => {
    mockFetchOnce({ error: "no project open" }, false, 400);
    await expect(api.undo()).rejects.toThrow("no project open");
    expect(sessionExpired).not.toHaveBeenCalled();
  });

  it("publishes a session expiry from the multipart upload too", async () => {
    // `installProductPackage` is the one call that does not go through
    // `request()` — it needs FormData's own boundary — so it is also the
    // one that could quietly miss the seam.
    mockFetchOnce({ error: "authentication required" }, false, 401);
    await expect(api.installProductPackage(new File([], "p.knxprod"))).rejects.toThrow(
      "authentication required",
    );
    expect(sessionExpired).toHaveBeenCalledTimes(1);
  });
});
