/** Complete versioned display catalogue and lossless reference-presentation contracts. */
import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import { DPT_OPTIONS, DPT_LABEL_VERSION, dptDisplay, dptEntry, retainedDptOptions } from "./dptLabels";
it("covers the independently audited 454 IDs with both localized names in numeric order", () => {
  const csv = readFileSync(new URL("../../../docs/spec-audits/2026-10-07-dpt-types.csv", import.meta.url), "utf8");
  const ids = csv.trim().split("\n").slice(1).map(line => line.split(",")[0]);
  const numbers = DPT_OPTIONS.map(entry => entry.number);
  expect(DPT_LABEL_VERSION).toBe("02.02.01");
  expect(DPT_OPTIONS).toHaveLength(454); expect(new Set(numbers).size).toBe(454);
  expect(new Set(numbers)).toEqual(new Set(ids));
  expect(numbers).toEqual([...numbers].sort((a, b) => {
    const ap = a.split(".").map(Number), bp = b.split(".").map(Number);
    return ap[0] - bp[0] || ap[1] - bp[1];
  }));
  for (const entry of DPT_OPTIONS) {
    expect(entry.en.trim()).not.toBe(""); expect(entry.de.trim()).not.toBe("");
    expect(dptDisplay(entry.id, "en")).toBe(`${entry.en} (${entry.number})`);
    expect(dptDisplay(entry.id, "de")).toBe(`${entry.de} (${entry.number})`);
  }
});
it("distinguishes main-only, zero subtypes, four-digit subtypes and unknown references", () => {
  expect(dptDisplay("DPST-14-0", "de")).toBe("Beschleunigung (14.000)");
  expect(dptDisplay("DPST-1-1200", "en")).toBe("Consumer / producer (1.1200)");
  expect(dptDisplay("DPT-9", "de")).toBe("9.*");
  expect(dptDisplay("DPST-999-999", "en")).toBe("999.999");
  expect(dptDisplay(null, "en")).toBe("—");
  for (const invalid of ["9.001", "DPST-9-1suffix", " DPST-9-1", "DPST-9007199254740993-1", "vendor:opaque"])
    expect(dptEntry(invalid)).toBeUndefined();
});
it("retains stored aliases and unlisted dirty drafts without inventing subtype membership", () => {
  expect(dptDisplay("DPST-09-001", "de")).toBe("Temperatur (9.001)");
  expect(retainedDptOptions("DPST-09-001", "DPST-09-001")).toEqual(["DPST-09-001"]);
  expect(retainedDptOptions("DPT-9", "vendor:opaque")).toEqual(["DPT-9", "vendor:opaque"]);
  expect(retainedDptOptions("DPST-9-1", "DPST-5-1")).toEqual([]);
  expect(retainedDptOptions("", "")).toEqual([]);
});
it("falls back to English display vocabulary for additional UI packs", () => {
  expect(dptDisplay("DPST-9-1", "tlh")).toBe("Temperature (9.001)");
});

it("uses definition-checked terminology for status-command and thermoelectric quantities", () => {
  expect(dptDisplay("DPST-200-100", "de")).toBe("Heizen / Kühlen mit Status / Befehl (200.100)");
  expect(dptDisplay("DPST-14-73", "de")).toBe("Thermokraft (V/K) (14.073)");
  expect(dptDisplay("DPST-14-1200", "en")).toBe("Meter volume flow (m³/h) (14.1200)");
});
