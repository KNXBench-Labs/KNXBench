/** Verifies the shared exclusion editor preserves and removes legacy occurrences safely. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import LineScanExclusionsEditor from "./LineScanExclusionsEditor";
import { loadLineScanExclusions, saveLineScanExclusions } from "./lineScanExclusions";
import { resetSettingsForTests } from "./settingsStore";

let host: HTMLDivElement | undefined;
afterEach(() => {
  host?.remove();
  host = undefined;
  resetSettingsForTests();
});

async function render(editors = 1, disabled = false) {
  host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<>{Array.from({ length: editors }, (_, index) => (
    <LineScanExclusionsEditor key={index} disabled={disabled} />
  ))}</>));
  return root;
}

it("marks invalid and duplicate legacy occurrences without rewriting them", async () => {
  saveLineScanExclusions(["2.3.42", "2.3.42", " bad "]);
  await render();
  expect(host!.querySelectorAll("[data-invalid='true']")).toHaveLength(2);
  expect(loadLineScanExclusions()).toEqual(["2.3.42", "2.3.42", " bad "]);
});

it("two-click removal deletes only the selected equal occurrence", async () => {
  saveLineScanExclusions(["2.3.42", "2.3.42", " bad "]);
  await render(2);
  const secondEditor = host!.querySelectorAll(".line-scan-exclusions")[1]!;
  const duplicateButton = secondEditor.querySelectorAll<HTMLButtonElement>("li button")[1]!;
  await act(async () => duplicateButton.click());
  expect(loadLineScanExclusions()).toEqual(["2.3.42", "2.3.42", " bad "]);
  await act(async () => duplicateButton.click());
  expect(loadLineScanExclusions()).toEqual(["2.3.42", " bad "]);
  expect(host!.querySelectorAll("code")[0]!.textContent).toBe("2.3.42");
});

it("rejects invalid and duplicate additions and disables every mutation", async () => {
  saveLineScanExclusions(["2.3.42"]);
  await render(1, true);
  expect(Array.from(host!.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")))
    .toSatisfy((controls: Array<HTMLInputElement | HTMLButtonElement>) => controls.every((item) => item.disabled));
  expect(loadLineScanExclusions()).toEqual(["2.3.42"]);
});
