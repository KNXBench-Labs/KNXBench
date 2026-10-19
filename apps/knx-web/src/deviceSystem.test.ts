/** System-family wire admission and label regressions (WP-20). */
import { describe, expect, it } from "vitest";
import { isDeviceSystem, maskHex, systemLabel, SYSTEM_FAMILIES, type DeviceSystem } from "./deviceSystem";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { translateFor } from "./i18n";

const families = ["system1", "system2", "system7", "systemB", "system300", "coupler",
  "knxnetIpRouter", "usbInterface", "irDecoder", "rfBidirectional", "rfUnidirectional"] as const;

describe("device system family", () => {
  it("knows exactly the server's closed family vocabulary", () => {
    expect([...SYSTEM_FAMILIES]).toEqual([...families]);
  });
  it.each([
    [{ state: "family", mask: 0x0012, family: "system1" }, "System 1", "System 1"],
    [{ state: "family", mask: 0x0025, family: "system2" }, "System 2", "System 2"],
    [{ state: "family", mask: 0x0701, family: "system7" }, "System 7", "System 7"],
    [{ state: "family", mask: 0x07b0, family: "systemB" }, "System B", "System B"],
    [{ state: "family", mask: 0x0300, family: "system300" }, "System 300", "System 300"],
    [{ state: "family", mask: 0x0912, family: "coupler" }, "Coupler", "Koppler"],
    [{ state: "family", mask: 0x091a, family: "knxnetIpRouter" }, "KNXnet/IP router", "KNXnet/IP-Router"],
    [{ state: "family", mask: 0x0310, family: "usbInterface" }, "USB interface", "USB-Schnittstelle"],
    [{ state: "family", mask: 0x0810, family: "irDecoder" }, "IR decoder", "IR-Decoder"],
    [{ state: "family", mask: 0x2010, family: "rfBidirectional" }, "RF bidirectional", "RF bidirektional"],
    [{ state: "family", mask: 0x2110, family: "rfUnidirectional" }, "RF unidirectional", "RF unidirektional"],
    [{ state: "unknownMask", mask: 0x0afd }, "Unknown (mask 0AFDh)", "Unbekannt (Maske 0AFDh)"],
    [{ state: "noMaskVersion" }, "No mask version", "Keine Maskenversion"],
    [{ state: "invalidMask", declared: "MV-07" }, "Unreadable mask “MV-07”", "Unlesbare Maske „MV-07“"],
    [{ state: "noApplicationProgram" }, "No application program", "Kein Applikationsprogramm"],
    [{ state: "applicationNotInDatabase" }, "Program not installed", "Programm nicht installiert"],
    [{ state: "applicationNotOfProduct" }, "Program does not belong to this product",
      "Programm gehört nicht zu diesem Produkt"],
  ] as [DeviceSystem, string, string][])("labels %j in English and German", (system, english, german) => {
    const label = systemLabel(system)!;
    expect(translateFor("en", label.key, label.params)).toBe(english);
    expect(translateFor("de", label.key, label.params)).toBe(german);
  });
  it("leaves an unresolved product to the row's product resolution text", () => {
    expect(systemLabel({ state: "noProduct" })).toBeNull();
  });
  it("formats masks as four upper-case hex digits with the h suffix", () => {
    expect(maskHex(0x0afd)).toBe("0AFDh");
    expect(maskHex(0x12)).toBe("0012h");
    expect(maskHex(0xffff)).toBe("FFFFh");
  });
  it.each([
    { state: "noProduct" }, { state: "noApplicationProgram" }, { state: "applicationNotInDatabase" },
    { state: "applicationNotOfProduct" }, { state: "noMaskVersion" },
    { state: "invalidMask", declared: "" }, { state: "unknownMask", mask: 0 }, { state: "unknownMask", mask: 0xffff },
    ...families.map((family) => ({ state: "family", mask: 0x0701, family })),
  ])("admits %j", (value) => {
    expect(isDeviceSystem(value)).toBe(true);
  });
  it.each([
    null, "family", {}, { state: "invented" }, { state: "family", mask: 0x0701 },
    { state: "family", mask: 0x0701, family: "system9" }, { state: "family", mask: "0701", family: "system7" },
    { state: "unknownMask", mask: -1 }, { state: "unknownMask", mask: 0x10000 }, { state: "unknownMask", mask: 1.5 },
    { state: "unknownMask" }, { state: "invalidMask", declared: 7 }, { state: "invalidMask" },
  ])("refuses %j", (value) => {
    expect(isDeviceSystem(value)).toBe(false);
  });
  it("has every family label in both catalogues", () => {
    for (const family of families) {
      const key = systemLabel({ state: "family", mask: 0, family })!.key;
      expect(en[key]).toBeTruthy();
      expect(de[key]).toBeTruthy();
    }
  });
});
