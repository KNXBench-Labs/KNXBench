/** Verifies shared exclusion editing, validation, locking, and snapshot-bound removal. */
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

it("disarms removal when a sibling update changes the protected occurrence", async () => {
  saveLineScanExclusions(["2.3.41", "2.3.42"]);
  await render();
  await act(async () => host!.querySelector<HTMLButtonElement>("li button")!.click());

  await act(async () => saveLineScanExclusions(["2.3.42"]));
  const remainingButton = host!.querySelector<HTMLButtonElement>("li button")!;
  expect(remainingButton.textContent).toBe("Remove exclusion");

  await act(async () => remainingButton.click());
  expect(loadLineScanExclusions()).toEqual(["2.3.42"]);
});

it("rejects invalid and duplicate additions without changing storage", async () => {
  saveLineScanExclusions(["2.3.42"]);
  await render();
  const input = host!.querySelector<HTMLInputElement>("input")!;
  const addButton = host!.querySelector<HTMLButtonElement>(".line-scan-add-exclusion button")!;

  await act(async () => {
    Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(
      input,
      "not-an-address",
    );
    input.dispatchEvent(new Event("input", { bubbles: true }));
    addButton.click();
  });
  expect(host!.querySelector("[role='alert']")!.textContent).toContain("complete dotted");
  expect(loadLineScanExclusions()).toEqual(["2.3.42"]);

  await act(async () => {
    Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(
      input,
      "2.3.42",
    );
    input.dispatchEvent(new Event("input", { bubbles: true }));
    addButton.click();
  });
  expect(host!.querySelector("[role='alert']")!.textContent).toContain("already excluded");
  expect(loadLineScanExclusions()).toEqual(["2.3.42"]);
});

it("disables every mutation control while a scan owns the snapshot", async () => {
  saveLineScanExclusions(["2.3.42"]);
  await render(1, true);
  expect(Array.from(host!.querySelectorAll<HTMLInputElement | HTMLButtonElement>("input, button")))
    .toSatisfy((controls: Array<HTMLInputElement | HTMLButtonElement>) => controls.every((item) => item.disabled));
  expect(loadLineScanExclusions()).toEqual(["2.3.42"]);
});
