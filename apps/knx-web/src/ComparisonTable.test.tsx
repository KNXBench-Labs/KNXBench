/** Named values, translated decoder refusals and always-visible raw bytes. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import ComparisonTable from "./ComparisonTable";
import { setSetting, resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
let dispose = () => {};
afterEach(async () => { await act(async () => dispose()); resetSettingsForTests(); resetUiLanguageForTests(); });
it.each(["en", "de"] as const)("%s shows exact named values and explains nondecodable bits without hiding raw", async (language) => {
  await act(async () => setSetting("uiLanguage", language));
  const host = document.createElement("div");document.body.append(host);const root = createRoot(host);
  dispose = () => {root.unmount();host.remove();};
  await act(async () => root.render(<ComparisonTable rows={[
    { kind: "parameter", label: "Delay · R-1 · bit 5 / 9", address: 0x4400, project: "235", device: "345", reason: null },
    { kind: "parameter", label: "Text · R-2 · bit 0 / 48", address: 0x4410, project: "5 € ok", device: null, reason: "parameterNotDecoded" },
    { kind: "raw", label: "", address: 0x4400, project: "AB AC", device: "FD 64", reason: null },
  ]} />));
  const rows=host.querySelectorAll("tbody tr");expect(rows).toHaveLength(3);
  expect(rows[0].querySelectorAll("td")[0].textContent).toBe("235");
  expect(rows[0].querySelectorAll("td")[1].textContent).toBe("345");
  expect(rows[0].getAttribute("data-different")).toBe("true");
  expect(rows[1].querySelector(".comparison-reason")?.textContent).not.toBe("parameterNotDecoded");
  expect(rows[1].querySelector(".comparison-reason")?.textContent).toContain(language === "de" ? "Rohbytes" : "raw bytes");
  expect(rows[1].textContent).toContain((language === "de" ? de : en)["compare.unknown"]);
  expect(rows[2].textContent).toContain("FD 64");expect(rows[2].closest("details")).toBeNull();
});
