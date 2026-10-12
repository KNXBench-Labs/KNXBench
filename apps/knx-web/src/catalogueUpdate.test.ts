/** Pins the size and version formatting of the catalogue update view. */
import { describe, expect, it } from "vitest";
import { formatBytes, installedVersionList } from "./catalogueUpdate";

describe("catalogue update presentation", () => {
  it("formats sizes in binary units per locale", () => {
    expect(formatBytes(512, "en")).toBe("512 B");
    expect(formatBytes(2048, "en")).toBe("2.0 KiB");
    expect(formatBytes(1536 * 1024, "de")).toBe("1,5 MiB");
    expect(formatBytes(5 * 1024 ** 4, "en")).toBe("5,120.0 GiB");
  });

  it("lists installed versions as written, naming missing ones", () => {
    const program = { id: "A", manufacturerId: "M", name: null, applicationNumber: "1", applicationVersion: "3", status: "newVersion" as const,
      installedVersions: [{ id: "A1", applicationVersion: "1.0" }, { id: "A2", applicationVersion: null }] };
    expect(installedVersionList(program, "not declared")).toBe("1.0, not declared");
  });
});
