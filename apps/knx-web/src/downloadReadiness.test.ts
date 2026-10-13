/** Issue #10: the download page's offline readiness marks, before anything is prepared. */
import { describe, expect, it } from "vitest";
import type { DeviceReadinessRow } from "./api";
import { readinessMarks, readinessReason } from "./downloadReadiness";
import { translateFor } from "./i18n";

const en = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("en", key, params);
const de = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("de", key, params);

function row(address: string | null, readiness: string, category: string | null = null): DeviceReadinessRow {
  return { address, name: `device ${address}`, programRef: "fictional", readiness, category, detail: null, steps: null, octets: null };
}

describe("readinessMarks", () => {
  it("marks only devices a download cannot serve, by address", () => {
    const marks = readinessMarks([
      row("1.1.1", "verified"),
      row("1.1.2", "untested"),
      row("0.2.23", "unsupported", "byte-order"),
      row("1.1.4", "excluded"),
      row(null, "no-address"),
    ]);
    expect([...marks.entries()]).toEqual([
      ["0.2.23", { kind: "unsupported", category: "byte-order", detail: null, programRef: "fictional" }],
      ["1.1.4", { kind: "excluded" }],
    ]);
  });

  it("marks no address that occurs more than once: the row cannot be told apart", () => {
    const marks = readinessMarks([row("1.1.5", "unsupported", "modules"), row("1.1.5", "untested")]);
    expect(marks.size).toBe(0);
  });
});

it("retains the exact technical detail for local guidance", () => {
  const rowWithDetail = { ...row("1.1.9", "unsupported", "modules"), detail: "PRIVATE customer /path" };
  expect(readinessMarks([rowWithDetail]).get("1.1.9")).toMatchObject({ detail: rowWithDetail.detail });
});

describe("readinessReason", () => {
  it("says the byte-order refusal in plain words", () => {
    expect(readinessReason({ kind: "unsupported", category: "byte-order" }, en)).toBe(
      "its application program stores values in little-endian byte order, which KNXBench does not write yet",
    );
    expect(readinessReason({ kind: "unsupported", category: "byte-order" }, de)).toBe(
      "sein Applikationsprogramm speichert Werte in Little-Endian-Byte-Reihenfolge, die KNXBench noch nicht schreibt",
    );
  });

  it("has a plain reason for every category the server grades with", () => {
    const categories = ["not-memory-mapped", "procedure-style", "unmodelled-step", "procedure-shape", "procedure-contents",
      "parameter-evaluation", "modules", "parameter-value", "image-structure", "byte-order", "product-data", "configuration"];
    for (const category of categories) {
      for (const t of [en, de]) {
        const reason = readinessReason({ kind: "unsupported", category }, t);
        expect(reason, category).not.toContain("deviceDownload.");
        // Not the fallback that only repeats the code.
        expect(reason, category).not.toBe(t("deviceDownload.readiness.reason.other", { category }));
      }
    }
  });

  it("shows an unknown category's code instead of guessing a reason", () => {
    expect(readinessReason({ kind: "unsupported", category: "warp-drive" }, en)).toBe("reason code warp-drive");
    expect(readinessReason({ kind: "unsupported", category: null }, en)).toBe("no reason was reported");
    expect(readinessReason({ kind: "excluded" }, en)).toBe("it is excluded from downloads in this project");
  });
});
